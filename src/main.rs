use glob::glob;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{fs::File, io::Read};

const TEMP_FILES_GLOB: &str = "/sys/class/hwmon/hwmon*/temp*_input"; // Gets the temperatures
const FAN_CONTROL_FILE: &str = "/proc/acpi/ibm/fan"; // controls the fan speed
const TEMP_INVALID: i64 = i64::min_value();

// Configuration for temperature thresholds and fan speeds
struct Temperatue {
    low: i64,
    high: i64,
    speed: FanSpeed,
}

// Fan speed levels
enum FanSpeed {
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

enum FanLevel {
    FanMax,
    FanMed,
    FanLow,
    FanOff,
    FanInvalid,
}

fn millic_to_c(temp: i64) -> i64 {
    temp / 1000
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

fn read_temp_file(filename: PathBuf) -> i64 {
    let f = File::open(&filename);
    let mut buf: String = Default::default();
    let val: i64;

    if f.is_err() {
        return TEMP_INVALID;
    }

    let _ = f.unwrap().read_to_string(&mut buf);

    // println!("Path: {}", filename.display());
    if !buf.is_empty() {
        let i = buf.trim_end_matches('\n').parse::<i64>();
        match i {
            Ok(res) => val = res,
            Err(_) => val = TEMP_INVALID,
        }
    } else {
        val = TEMP_INVALID;
    }

    return val;
}

fn get_max_temp() -> i64 {
    let mut max_temp = TEMP_INVALID;

    for entry in glob(TEMP_FILES_GLOB).expect("Failed to read glob pattern") {
        match entry {
            Ok(ref path) => {
                max_temp = max_temp.max(read_temp_file(path.to_path_buf()));
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
        // Err("Couldn't find any valid temperature\n");
        return TEMP_INVALID;
    }

    return millic_to_c(max_temp);
}

fn main() {
    println!("Hello, world!");
    println!("Full speed supported: {}", full_speed_supported());
    println!("Max speed: {}", get_max_temp());
}
