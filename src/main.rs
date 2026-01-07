mod cpu_usage;
mod fan_control;
mod suspend_detector;
use signal_hook::{
    consts::{SIGINT, SIGUSR1, SIGUSR2},
    iterator::Signals,
};
use std::{
    env,
    process::exit,
    sync::mpsc::channel,
    thread::{self, sleep},
    time::Duration,
};

use crate::fan_control::{FanControl, SetFanStatus};

#[derive(Debug)]
enum SignalState {
    Interrupt,
    Sleep,
    Resume,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args();

    if args.len() != 1 {
        println!("nvfans: Customizable ThinkPad fan daemon.\n\n");
        println!("  [any argument]     Show this help\n\n");
        println!("See the nvfans(1) man page for details.\n");
        return Ok(());
    }

    let mut fan_control = FanControl::new();

    let mut signals = Signals::new([SIGINT, SIGUSR1, SIGUSR2])?;
    let (sender, receiver) = channel();

    thread::spawn(move || {
        for sig in signals.forever() {
            if sig == SIGINT {
                sender.send(SignalState::Interrupt).unwrap();
            } else if sig == SIGUSR1 {
                sender.send(SignalState::Sleep).unwrap();
            } else if sig == SIGUSR2 {
                sender.send(SignalState::Resume).unwrap();
            }
        }
    });

    let mut fan_control_enabled = true;
    while fan_control.get_run_state() {
        let response = receiver.try_recv();
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
