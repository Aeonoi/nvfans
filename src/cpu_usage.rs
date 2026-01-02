use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use crate::fan_control::TickRule;

const STAT_FILE: &str = "/proc/stat";

pub struct CpuUsage {
    tick_info: Vec<Vec<i64>>,
    current_tick: i64,
}

// Calculating for CPU usage: https://umatechnology.org/c-program-to-find-cpu-usage-in-linux/
impl CpuUsage {
    pub fn new() -> Self {
        CpuUsage {
            tick_info: vec![vec![-1; 8]; 3],
            current_tick: 0,
        }
    }

    pub fn get_cpu_times(&mut self) {
        let f = File::open(STAT_FILE);
        if f.is_err() {
            eprintln!("Error opening {STAT_FILE}");
        }
        let mut reader = BufReader::new(f.unwrap());
        let mut info = vec![0; 8];

        let mut s = String::default();
        let status = reader.read_line(&mut s);
        if status.is_ok() {
            let data: Vec<&str> = s
                .split(" ")
                .filter(|x| !x.is_empty() && x.parse::<i64>().is_ok())
                .collect();
            for i in 0..info.len() {
                info[i] = data[i].parse::<i64>().unwrap();
            }
        }

        self.tick_info[self.current_tick as usize] = info;
        self.current_tick += 1;
        if self.current_tick == 3 {
            self.current_tick = 0
        }
    }

    pub fn calculate_idle_and_total(&mut self, start: Vec<i64>, end: Vec<i64>) -> (f64, f64) {
        let idle_diff = (end[3] + end[4]) - (start[3] + start[4]);
        let total_diff = (end[0] + end[1] + end[2] + end[3] + end[4] + end[5] + end[6] + end[7])
            - (start[0]
                + start[1]
                + start[2]
                + start[3]
                + start[4]
                + start[5]
                + start[6]
                + start[7]);

        (idle_diff as f64, total_diff as f64)
    }

    pub fn get_cpu_usage(&mut self, idle_diff: f64, total_diff: f64) -> f64 {
        let usage = (1.0 - (idle_diff / total_diff)) * 100.0;
        usage
    }

    pub fn get_cpu_usage_diff(&mut self) -> TickRule {
        let info_t =
            self.calculate_idle_and_total(self.tick_info[0].clone(), self.tick_info[1].clone());
        let mut avg_cpu_usage = self.get_cpu_usage(info_t.0, info_t.1);
        // we have gone through the three ticks
        if self.tick_info[2][0] != -1 {
            let next_info_t =
                self.calculate_idle_and_total(self.tick_info[1].clone(), self.tick_info[2].clone());
            let next_cpu_usage = self.get_cpu_usage(next_info_t.0, next_info_t.1);
            avg_cpu_usage = (avg_cpu_usage + next_cpu_usage) / 2.0;
        }

        if avg_cpu_usage >= 12.5 {
            return TickRule::InfrequentTick;
        }
        TickRule::NormalTick
    }
}
