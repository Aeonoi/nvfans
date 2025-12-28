use crate::fan_control::ResumeState;
use std::time::{Duration, Instant, SystemTime};
const THRESHOLD_NS: u64 = 200000000; // 0.2 seconds

pub struct SuspendDetector {
    mono_prev: Instant,
    wall_prev: SystemTime,
}

impl SuspendDetector {
    pub fn new() -> Self {
        Self {
            mono_prev: Instant::now(),
            wall_prev: SystemTime::now(),
        }
    }

    pub fn check(&mut self) -> ResumeState {
        let mono_now = Instant::now();
        let wall_now = SystemTime::now();

        let mono_delta = mono_now.duration_since(self.mono_prev);
        let wall_delta = wall_now
            .duration_since(self.wall_prev)
            .unwrap_or(Duration::ZERO);

        self.mono_prev = mono_now;
        self.wall_prev = wall_now;

        if wall_delta > mono_delta + Duration::from_nanos(THRESHOLD_NS) {
            ResumeState::ResumeDetected
        } else {
            ResumeState::ResumeNotDetected
        }
    }
}
