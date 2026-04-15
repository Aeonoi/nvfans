use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Returns the socket path following XDG conventions.
/// Prefers `$XDG_RUNTIME_DIR/nvfans.sock` for user services,
/// falls back to `/tmp/nvfans.sock` if not available.
pub fn socket_path() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(|dir| PathBuf::from(dir).join("nvfans.sock"))
        .unwrap_or_else(|| PathBuf::from("/tmp/nvfans.sock"))
}

#[derive(PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum FanSpeed {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Temperature {
    pub low: i64,
    pub high: i64,
    pub speed: FanSpeed,
}

impl PartialEq for Temperature {
    fn eq(&self, other: &Self) -> bool {
        self.speed == other.speed
    }
}

impl PartialEq<Temperature> for FanSpeed {
    fn eq(&self, other: &Temperature) -> bool {
        *self == other.speed
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    GetFanSpeedStatus,
    SetFanSpeed { speed: String }, // String in the format of a number 0-7, "full", or "auto"
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    FanSpeedStatus { temperature: Temperature },
    Success { msg: String },
    Error { msg: String },
}
