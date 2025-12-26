mod fan_control;
use crate::fan_control::FanControl;

fn main() {
    let mut fan_control = FanControl::new();
    println!(
        "Full speed supported: {}",
        fan_control.full_speed_supported()
    );
    println!("Max speed: {}", fan_control.get_max_temp());
    fan_control.set_fan_level();
}
