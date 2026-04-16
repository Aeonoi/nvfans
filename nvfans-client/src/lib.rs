mod connection;

pub use connection::Client;

// Re-export commonly used types for convenience
pub use nvfans_common::{FanSpeed, Response, Temperature};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
