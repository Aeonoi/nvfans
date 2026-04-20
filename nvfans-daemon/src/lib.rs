mod fan_control;
mod server;
mod suspend_detector;
use signal_hook::{
    consts::{SIGINT, SIGUSR1, SIGUSR2},
    iterator::Signals,
};
use std::{
    error::Error,
    process::exit,
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::sync::mpsc::channel;

use crate::{
    fan_control::{FanControl, SetFanStatus},
    server::DaemonServer,
};

#[derive(Debug)]
enum SignalState {
    Interrupt,
    Sleep,
    Resume,
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let fan_control = Arc::new(Mutex::new(FanControl::new()));

    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut signals = Signals::new([SIGINT, SIGUSR1, SIGUSR2])?;
    let (signal_sender, mut signal_receiver) = channel(64);

    // Spawn signal handler task using the runtime's spawn_blocking
    // Use spawn_blocking since signal_hook's Signals::forever() is blocking and !Send
    rt.spawn_blocking(move || {
        println!("[SIGNAL] Signal handler thread started");
        for sig in signals.forever() {
            let signal_state = if sig == SIGINT {
                println!("[SIGNAL] Received SIGINT");
                SignalState::Interrupt
            } else if sig == SIGUSR1 {
                println!("[SIGNAL] Received SIGUSR1 (Sleep)");
                SignalState::Sleep
            } else if sig == SIGUSR2 {
                println!("[SIGNAL] Received SIGUSR2 (Resume)");
                SignalState::Resume
            } else {
                println!("[SIGNAL] Received unknown signal: {}", sig);
                continue;
            };
            // blocking_send is used because we're in a blocking task
            if signal_sender.blocking_send(signal_state).is_err() {
                println!("[SIGNAL] Failed to send signal - channel closed");
                break;
            }
        }
    });

    // Spawn daemon server task
    let fan_control_server = Arc::clone(&fan_control);
    rt.spawn(async move {
        let daemon_server = DaemonServer::new(fan_control_server);
        let _ = daemon_server.make_connection().await;
    });

    let mut fan_control_enabled = true;

    // Main loop
    rt.block_on(async {
        loop {
            let run_state = {
                let mut fc = fan_control.lock().expect("Failed to lock FanControl");

            // Process signals
            while let Ok(signal) = signal_receiver.try_recv() {
                println!("[MAIN] Received signal from channel: {:?}", signal);
                match signal {
                    SignalState::Sleep => {
                        fc.set_pending_sleep_state(true);
                    }
                    SignalState::Resume => {
                        fc.set_pending_resume_state(true);
                    }
                    SignalState::Interrupt => {
                        fc.reset();
                        exit(0);
                    }
                }
            }

                if fan_control_enabled {
                    let set = fc.set_fan_level();
                    if set != SetFanStatus::FanLevelNotSet {
                        fc.maybe_ping_watchdog();
                    }
                }

                if fc.get_pending_sleep_state() {
                    fc.set_pending_sleep_state(false);
                    println!("[FAN] Fan control disabled for sleep. Turning off fans.");

                    if fc.write_to_fan("level", "0").is_ok() {
                        fc.write_watchdog_timeout(0);
                    }
                    fan_control_enabled = false;
                }

                if fc.get_pending_resume_state() {
                    fc.set_pending_resume_state(false);
                    println!("[FAN] Fan control enabled for resume. Restoring fan control.");
                    fan_control_enabled = true;
                    fc.set_fan_to_previous();
                    fc.write_watchdog_timeout(fan_control::DEFAULT_WATCHDOG_SECS);
                }

                let run = fc.get_run_state();
                if run {
                    fc.unset_first_tick();
                }
                run
            };

            if !run_state {
                break;
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        let mut fc = fan_control.lock().expect("Failed to lock FanControl");
        fc.reset();
    });

    Ok(())
}
