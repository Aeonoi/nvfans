use crate::suspend_detector::SuspendDetector;
use glob::glob;
use std::{
    collections::VecDeque,
    fs::{File, read_to_string},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::exit,
    time::Instant,
};

const TEMP_FILES_GLOB: &str = "/sys/class/hwmon/hwmon*/temp*_input"; // Gets the temperatures
const FAN_CONTROL_FILE: &str = "/proc/acpi/ibm/fan"; // Controls the fan speed
const TEMP_INVALID: i64 = i64::MIN;
const CONFIG_FILE: &str = "/etc/nvfans.conf";
const TICK_HYSTERESIS: i64 = 4;
const FAST_TICK_HYSTERESIS: i64 = 2; // Speeds up the time to change fan speeds
const TEMP_OFFSET: i64 = 2; // Offsets the temperature when determining whether to switch fan speeds
const TEMP_HISTORY_SIZE: usize = 30; // Arbitrary history size

pub const DEFAULT_WATCHDOG_SECS: i64 = 120;
pub const WATCHDOG_GRACE_PERIOD_SECS: i64 = 2;

const fn millic_to_c(temp: i64) -> i64 {
    temp / 1000
}

fn convert_number_to_fan_speed(value: &str) -> FanSpeed {
    match value {
        "0" => FanSpeed::Level0,
        "1" => FanSpeed::Level1,
        "2" => FanSpeed::Level2,
        "3" => FanSpeed::Level3,
        "4" => FanSpeed::Level4,
        "5" => FanSpeed::Level5,
        "6" => FanSpeed::Level6,
        "7" => FanSpeed::Level7,
        "full-speed" => FanSpeed::FullSpeed,
        "auto" => FanSpeed::Auto,
        "Auto" => FanSpeed::Auto,
        _ => FanSpeed::Auto,
    }
}

fn convert_fan_speed(fan_speed: FanSpeed) -> String {
    match fan_speed {
        FanSpeed::Level0 => String::from("0"),
        FanSpeed::Level1 => String::from("1"),
        FanSpeed::Level2 => String::from("2"),
        FanSpeed::Level3 => String::from("3"),
        FanSpeed::Level4 => String::from("4"),
        FanSpeed::Level5 => String::from("5"),
        FanSpeed::Level6 => String::from("6"),
        FanSpeed::Level7 => String::from("7"),
        FanSpeed::FullSpeed => String::from("full-speed"),
        FanSpeed::Auto => String::from("auto"),
    }
}

#[derive(Clone, Debug)]
struct Temperature {
    name: String,
    low: i64,
    high: i64,
    speed: FanSpeed,
}

impl PartialEq for Temperature {
    fn eq(&self, other: &Self) -> bool {
        self.speed == other.speed
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
enum FanSpeed {
    Level0,
    Level1,
    Level2,
    Level3,
    Level4,
    Level5,
    Level6,
    Level7,
    FullSpeed,
    Auto,
}

impl PartialEq<Temperature> for FanSpeed {
    fn eq(&self, other: &Temperature) -> bool {
        *self == other.speed
    }
}

#[derive(PartialEq)]
pub enum ResumeState {
    ResumeNotDetected,
    ResumeDetected,
}

#[derive(PartialEq)]
pub enum SetFanStatus {
    FanLevelNotSet,
    FanLevelSet,
    FanLevelInvalid,
    FanLevelError,
}

fn full_speed_supported() -> bool {
    let f = File::open(FAN_CONTROL_FILE);
    if f.is_err() {
        return false;
    }
    let mut data = vec![];

    if f.is_ok() {
        let _ = f.unwrap().read_to_end(&mut data);
    }

    let content = String::from_utf8_lossy(&data);
    let mut found = false;

    if content.find("full-speed").is_some() {
        found = true;
    }

    found
}

fn read_config_file() -> (Vec<Temperature>, bool) {
    let default_config: Vec<Temperature> = [
        Temperature {
            name: "level 0".to_string(),
            low: 0,
            high: 60,
            speed: FanSpeed::Level0,
        },
        Temperature {
            name: "level 2".to_string(),
            low: 60,
            high: 75,
            speed: FanSpeed::Level2,
        },
        Temperature {
            name: "level 3".to_string(),
            low: 75,
            high: 80,
            speed: FanSpeed::Level3,
        },
        Temperature {
            name: "level 4".to_string(),
            low: 80,
            high: 85,
            speed: FanSpeed::Level4,
        },
        Temperature {
            name: "level 5".to_string(),
            low: 85,
            high: 90,
            speed: FanSpeed::Level5,
        },
        Temperature {
            name: "level 7".to_string(),
            low: 90,
            high: 100,
            speed: FanSpeed::Level7,
        },
    ]
    .to_vec();

    let exists = Path::new(CONFIG_FILE).exists();
    let full_speed_supported = full_speed_supported();

    println!("{}", CONFIG_FILE);
    if exists {
        let mut config: Vec<Temperature> = vec![];
        let lines = read_to_string(CONFIG_FILE);
        if lines.is_ok() {
            for (i, line) in lines.unwrap().lines().enumerate() {
                let data: Vec<&str> = line.split(",").collect();
                if data.len() == 3 {
                    let low = data[0].parse::<i64>();
                    if low.is_err() {
                        eprintln!("Error with reading config values. Using default config");
                        return (default_config, true);
                    }
                    let high = data[1].parse::<i64>();
                    if high.is_err() {
                        eprintln!("Error with reading config values. Using default config");
                        return (default_config, true);
                    }
                    let mut speed = data[2];
                    if speed.contains(' ') {
                        eprintln!("Incorrect speed value for {speed}, must not contain any spaces");
                        speed = "auto";
                    }
                    if speed == "full-speed" {
                        if !full_speed_supported {
                            eprintln!(
                                "Full-speed is not supported on your laptop, will use level 7 as the maximum."
                            );
                            speed = "7";
                        } else {
                            println!(
                                "NOTE: It is not advised to set fan speed over the limit that the BIOS can handle. Use full-speed with caution!"
                            );
                        }
                    }
                    println!(
                        "[CFG] At ranges {} - {} fan is set to level {speed}",
                        low.clone().unwrap(),
                        high.clone().unwrap()
                    );
                    config.push(Temperature {
                        name: format!("level {}", i),
                        low: low.unwrap(),
                        high: high.unwrap(),
                        speed: convert_number_to_fan_speed(speed),
                    });
                }
            }
        } else {
            eprintln!("Error opening file, using default config");
            return (default_config, true);
        }
        if config.is_empty() {
            eprintln!(
                "Config file contains no valid temperature ranges. Using default config instead."
            );
            return (default_config, true);
        }
        (config, false)
    } else {
        println!("[CFG] No config file found. Defaulting to application default config");
        (default_config, true)
    }
}

pub struct FanControl {
    current_rule: Temperature,
    temperature_configs: Vec<Temperature>,
    run: bool,
    first_tick: bool,
    suspend_detector: SuspendDetector,
    last_watchdog_ping: Instant,
    pending_sleep: bool,
    pending_resume: bool,
    tick: i64,
    temp_history: VecDeque<i64>,
}

impl FanControl {
    pub fn new() -> FanControl {
        let config = read_config_file();
        FanControl {
            current_rule: Temperature {
                name: "level 0".to_string(),
                low: 0,
                high: 100,
                speed: FanSpeed::Auto,
            },
            temperature_configs: config.0,
            run: true,
            first_tick: true,
            suspend_detector: SuspendDetector::new(),
            last_watchdog_ping: Instant::now(),
            pending_sleep: false,
            pending_resume: false,
            tick: TICK_HYSTERESIS,
            temp_history: VecDeque::new(),
        }
    }

    pub fn get_pending_sleep_state(&mut self) -> bool {
        self.pending_sleep
    }

    pub fn get_pending_resume_state(&mut self) -> bool {
        self.pending_resume
    }

    pub fn set_pending_sleep_state(&mut self, new_state: bool) {
        self.temp_history.clear();
        self.pending_sleep = new_state
    }

    pub fn set_pending_resume_state(&mut self, new_state: bool) {
        self.pending_resume = new_state
    }

    pub fn get_run_state(&mut self) -> bool {
        self.run
    }

    pub fn reset(&mut self) {
        println!("[FAN] Quit requested or error has occured, reenabling thinkpad_acpi fan control");
        if self.write_to_fan("level", "auto").is_ok() {
            self.write_watchdog_timeout(0);
        }
    }

    pub fn unset_first_tick(&mut self) {
        self.first_tick = false
    }

    fn exit_if_first_tick(&mut self) {
        if self.first_tick {
            eprintln!("Quitting due to failure during first run\n");
            exit(1); // FIX: may not be thread safe
        }
    }

    pub fn read_temp_file(&mut self, filename: PathBuf) -> i64 {
        let f = File::open(&filename);
        let mut buf: String = Default::default();
        let val: i64;

        if f.is_err() {
            return TEMP_INVALID;
        }

        let mut reader = BufReader::new(f.unwrap());

        let result = reader.read_to_string(&mut buf);

        if result.is_err() {
            return TEMP_INVALID;
        }

        if !buf.is_empty() {
            let i = buf.trim_end_matches('\n').parse::<i64>();
            match i {
                Ok(res) => val = res,
                Err(_) => val = TEMP_INVALID,
            }
        } else {
            val = TEMP_INVALID;
        }

        val
    }

    pub fn get_max_temp(&mut self) -> i64 {
        let mut max_temp = TEMP_INVALID;

        for entry in glob(TEMP_FILES_GLOB).expect("Failed to read glob pattern") {
            match entry {
                Ok(ref path) => {
                    max_temp = max_temp.max(self.read_temp_file(path.to_path_buf()));
                    // println!(
                    //     "{:?} has temperature: {}",
                    //     path.display(),
                    //     read_temp_file(path.to_path_buf())
                    // )
                }
                Err(e) => println!("{:?}", e),
            }
        }

        if max_temp == TEMP_INVALID {
            eprintln!("Couldn't find any valid temperature\n");
            self.exit_if_first_tick();
            return TEMP_INVALID;
        }

        millic_to_c(max_temp)
    }

    pub fn set_fan_level(&mut self) -> SetFanStatus {
        let max_temp = self.get_max_temp();

        let mut stuck_penalty = 0;

        if self.tick > 0 {
            self.tick -= 1;
        }

        if max_temp == TEMP_INVALID {
            let status = self.write_to_fan("level", "7");
            if status.is_err() {
                return SetFanStatus::FanLevelError;
            }
            return SetFanStatus::FanLevelInvalid;
        }

        let count = match self.temp_history.len() {
            0 => 1,
            len => len as i64,
        };

        if count == TEMP_HISTORY_SIZE as i64 {
            self.temp_history.pop_front();
        }

        self.temp_history.push_back(max_temp);

        // NOTE: It is possible that a mean normalization is a little better at determing the
        // correct temperature to compare with.
        let avg_temp = (self.temp_history.iter().sum::<i64>() / count) - TEMP_OFFSET;

        // TODO: For default configs, we want to raise the fan speed level when necessary, i.e.
        // when we have been stuck on the same fan level for a while.
        for rule in self.temperature_configs.clone() {
            if rule == self.current_rule {
                if self.tick > 0 {
                    return SetFanStatus::FanLevelNotSet;
                }
                stuck_penalty = 3;
            }
            if rule.high - stuck_penalty >= avg_temp && rule.low <= avg_temp {
                if self.current_rule != rule {
                    self.current_rule = rule.clone();
                    let mut value = convert_fan_speed(rule.speed);
                    self.tick = TICK_HYSTERESIS;

                    // Turn to fan speed 6 and wait to see if we really want to turn to fan speed
                    // level 7 to remove any unncessary drastic fan spin up
                    // Some newer devices and some higher end ThinkPads can have a difference of
                    // >2000 RPM change from level 6 -> level 7
                    if rule.speed == FanSpeed::Level7 {
                        value = convert_fan_speed(FanSpeed::Level6);
                        self.current_rule = Temperature {
                            name: rule.name,
                            low: rule.low,
                            high: rule.high,
                            speed: FanSpeed::Level6,
                        };
                        self.tick = FAST_TICK_HYSTERESIS;
                    }

                    let status = self.write_to_fan("level", &value);
                    if status.is_err() {
                        return SetFanStatus::FanLevelError;
                    }
                    println!("[FAN] Temperature now {}C, fan set to {}", max_temp, value);
                    return SetFanStatus::FanLevelSet;
                } else {
                    return SetFanStatus::FanLevelNotSet;
                }
            }
        }

        SetFanStatus::FanLevelInvalid
    }

    pub fn set_fan_to_previous(&mut self) {
        let status =
            self.write_to_fan("level", convert_fan_speed(self.current_rule.speed).as_str());
        if status.is_err() {
            self.reset();
            panic!("Error setting back to previous fan speed");
        }
    }

    pub fn write_to_fan(&mut self, command: &str, value: &str) -> std::io::Result<()> {
        let exists = Path::new(FAN_CONTROL_FILE).exists();
        if exists {
            let mut f = File::options()
                .write(true)
                .read(true)
                .truncate(false)
                .create(false)
                .open(FAN_CONTROL_FILE);
            if f.is_ok() {
                let mut writer = BufWriter::new(f.as_mut().unwrap());
                let string_to_write = format!("{} {}", command, value);
                let bytes_written = writer.write(string_to_write.as_bytes());
                if bytes_written.is_err() {
                    self.exit_if_first_tick();
                    self.reset();
                    panic!(
                        "Error writing to {}, did you enable fan_control=1?",
                        FAN_CONTROL_FILE
                    );
                } else {
                    self.last_watchdog_ping = Instant::now();
                    Ok(())
                }
            } else {
                self.exit_if_first_tick();
                self.reset();
                panic!(
                    "Error opening {}, do you have sudo access?",
                    FAN_CONTROL_FILE
                );
            }
        } else {
            self.exit_if_first_tick();
            self.reset();
            panic!(
                "{} does not exist. Is thinkpad_acpi loaded properly?",
                FAN_CONTROL_FILE
            );
        }
    }

    //                      WATCHDOG STUFF              //

    pub fn write_watchdog_timeout(&mut self, timeout: i64) {
        assert!(timeout >= 0, "Timeout is smaller than 0");
        assert!(
            timeout <= DEFAULT_WATCHDOG_SECS,
            "Timeout is greater than 120"
        );
        let binding = timeout.to_string();
        let timeout_s = binding.as_str();
        let status = self.write_to_fan("watchdog", timeout_s);
        if status.is_err() {
            self.exit_if_first_tick();
        }
    }

    pub fn maybe_ping_watchdog(&mut self) {
        if self.suspend_detector.check() == ResumeState::ResumeDetected {
            // On resume, some models need a manual fan write again, or they will
            // revert to "auto".
            println!("Clock jump detected, possible resume. Rewriting fan level\n");
            let status = self.write_to_fan("level", &convert_fan_speed(self.current_rule.speed));
            if status.is_err() {
                self.exit_if_first_tick();
            }
        }

        if self.last_watchdog_ping.elapsed().as_secs()
            < (DEFAULT_WATCHDOG_SECS - WATCHDOG_GRACE_PERIOD_SECS)
                .try_into()
                .unwrap()
        {
            return;
        }

        // Transitioning from level 0 -> level 0 can cause a brief fan spinup on
        // some models, so don't reset the timer by write_fan_level().
        self.write_watchdog_timeout(DEFAULT_WATCHDOG_SECS);
    }
}
