use std::{
    fs::File,
    io::{BufRead, BufReader},
};

const STAT_FILE: &str = "/proc/stat";

pub struct CpuUsage {
    tick_info: Vec<i64>,
    prev_usage: f64,
}

// Calculating for CPU usage: https://umatechnology.org/c-program-to-find-cpu-usage-in-linux/
impl CpuUsage {
    pub fn new() -> Self {
        CpuUsage {
            tick_info: vec![-1; 8],
            prev_usage: f64::NAN,
        }
    }

    pub fn set_cpu_times(&mut self, info: Vec<i64>) {
        self.tick_info = info.clone();
    }

    pub fn get_prev_usage(&mut self) -> f64 {
        self.prev_usage
    }

    pub fn set_prev_usage(&mut self, prev: f64) {
        self.prev_usage = prev;
    }

    pub fn get_cpu_times(&mut self) -> Vec<i64> {
        let f = File::open(STAT_FILE);
        if f.is_err() {
            eprintln!("Error opening {STAT_FILE}");
            return self.tick_info.clone();
        }
        let mut reader = BufReader::new(f.unwrap());
        let mut info = vec![0; 8];

        let mut s = String::default();
        let status = reader.read_line(&mut s);
        // Handle read better since it is not guaranteed that the first line is the line that we
        // want
        if status.is_ok() {
            let data: Vec<&str> = s
                .split_whitespace()
                .filter(|x| !x.is_empty() && x.parse::<i64>().is_ok())
                .collect();
            for i in 0..info.len() {
                info[i] = data[i].parse::<i64>().unwrap();
            }
        }
        info
    }

    pub fn get_cpu_usage(&mut self, end: Vec<i64>) -> f64 {
        let idle_diff = (end[3] + end[4]) - (self.tick_info[3] + self.tick_info[4]);
        let total_diff = (end[0] + end[1] + end[2] + end[3] + end[4] + end[5] + end[6] + end[7])
            - (self.tick_info[0]
                + self.tick_info[1]
                + self.tick_info[2]
                + self.tick_info[3]
                + self.tick_info[4]
                + self.tick_info[5]
                + self.tick_info[6]
                + self.tick_info[7]);

        (100.0 * (total_diff as f64 - idle_diff as f64)) / total_diff as f64
    }
}
