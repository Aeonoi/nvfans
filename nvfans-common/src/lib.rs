use serde::{Deserialize, Serialize};

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
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    FanSpeedStatus { temperature: Temperature },
    Error { msg: String },
}
