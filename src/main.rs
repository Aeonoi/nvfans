mod fan_control;
mod suspend_detector;
use std::{env, thread::sleep, time::Duration};

use crate::fan_control::{FanControl, SetFanStatus};

fn main() {
    let args = env::args();

    if args.len() != 1 {
        println!("zcfan: Zero-configuration ThinkPad fan daemon.\n\n");
        println!("  [any argument]     Show this help\n\n");
        println!("See the zcfan(1) man page for details.\n");
        return;
    }

    let mut fan_control = FanControl::new();

    let mut fan_control_enabled = true;
    while fan_control.get_run_state() {
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
            println!("Fan control disabled for sleep");

            if fan_control.write_to_fan("level", "auto").is_ok() {
                fan_control.write_watchdog_timeout(0);
            }
            fan_control_enabled = false;
        }
        if fan_control.get_pending_resume_state() {
            fan_control.set_pending_resume_state(false);
            println!("Fan control enabled for resume");
            fan_control_enabled = true;
            // expect(current_rule);
            let _ = fan_control.write_to_fan("level", "auto");
            fan_control.write_watchdog_timeout(fan_control::DEFAULT_WATCHDOG_SECS);
        }
    }
    println!("[FAN] Quit requested, reenabling thinkpad_acpi fan control");
    if fan_control.write_to_fan("level", "auto").is_ok() {
        fan_control.write_watchdog_timeout(0);
    }
}
