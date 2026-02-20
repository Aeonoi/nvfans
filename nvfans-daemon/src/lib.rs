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
    sync::mpsc::channel,
    thread::{self, sleep},
    time::Duration,
};

use crate::{
    fan_control::{FanControl, SetFanStatus},
    server::{Request, make_connection},
};

#[derive(Debug)]
enum SignalState {
    Interrupt,
    Sleep,
    Resume,
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let mut fan_control = FanControl::new();

    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut signals = Signals::new([SIGINT, SIGUSR1, SIGUSR2])?;
    let (signal_sender, signal_receiver) = channel();
    let (server_sender, server_receiver) = channel::<Request>();

    thread::spawn(move || {
        for sig in signals.forever() {
            if sig == SIGINT {
                signal_sender.send(SignalState::Interrupt).unwrap();
            } else if sig == SIGUSR1 {
                signal_sender.send(SignalState::Sleep).unwrap();
            } else if sig == SIGUSR2 {
                signal_sender.send(SignalState::Resume).unwrap();
            }
        }
    });

    rt.spawn(async {
        let _ = make_connection().await;
    });

    let mut fan_control_enabled = true;
    while fan_control.get_run_state() {
        let response = signal_receiver.try_recv();
        if response.is_ok() {
            match response.unwrap() {
                SignalState::Sleep => {
                    fan_control.set_pending_sleep_state(true);
                }
                SignalState::Resume => {
                    fan_control.set_pending_resume_state(true);
                }
                SignalState::Interrupt => {
                    fan_control.reset();
                    exit(0);
                }
            }
        }
        if fan_control_enabled {
            let set = fan_control.set_fan_level();
            if set != SetFanStatus::FanLevelNotSet {
                fan_control.maybe_ping_watchdog();
            }
        }
        if fan_control.get_run_state() {
            sleep(Duration::from_secs(1));
            fan_control.unset_first_tick();
        }
        if fan_control.get_pending_sleep_state() {
            fan_control.set_pending_sleep_state(false);
            println!("[FAN] Fan control disabled for sleep. Turning off fans.");

            if fan_control.write_to_fan("level", "0").is_ok() {
                fan_control.write_watchdog_timeout(0);
            }
            fan_control_enabled = false;
        }
        if fan_control.get_pending_resume_state() {
            fan_control.set_pending_resume_state(false);
            println!("[FAN] Fan control enabled for resume. Restoring fan control.");
            fan_control_enabled = true;
            fan_control.set_fan_to_previous();
            fan_control.write_watchdog_timeout(fan_control::DEFAULT_WATCHDOG_SECS);
        }
    }
    fan_control.reset();
    Ok(())
}
