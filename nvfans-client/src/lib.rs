mod connection;

use connection::Client;
use nvfans_common::{FanSpeed, Temperature};
use std::error::Error;

pub fn run() -> Result<(), Box<dyn Error>> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        println!("Connecting to daemon...");

        let client = Client::new()
            .await
            .map_err(|e| format!("Failed to connect: {}", e))?;
        println!("Connected!");

        println!("\nSending GetStatus request...");
        let status_response = match client.get_status().await {
            Ok(response) => response,
            Err(e) => return Err(format!("Failed to get status: {}", e).into()),
        };
        println!("Daemon responded: {:?}\n", status_response);

        let set_response = match client.set_fan_speed(String::from("1")).await {
            Ok(response) => response,
            Err(e) => return Err(format!("Failed to set fan speed: {}", e).into()),
        };
        println!("Daemon responded: {:?}\n", set_response);

        // Get the current config and store it for later
        let config = match client.get_config().await {
            Ok(response) => match response {
                nvfans_common::Response::ConfigResponse { config } => config,
                _ => return Err("Unexpected response type from get_config".into()),
            },
            Err(e) => return Err(format!("Failed to get config: {}", e).into()),
        };
        println!("Current config: {:?}", config);

        let new_config: Vec<Temperature> = vec![
            Temperature {
                low: 0,
                high: 60,
                speed: FanSpeed::Level0,
            },
            Temperature {
                low: 60,
                high: 75,
                speed: FanSpeed::Level2,
            },
            Temperature {
                low: 75,
                high: 80,
                speed: FanSpeed::Level3,
            },
            Temperature {
                low: 80,
                high: 85,
                speed: FanSpeed::Level4,
            },
            Temperature {
                low: 85,
                high: 100,
                speed: FanSpeed::Level5,
            },
            Temperature {
                low: 100,
                high: 255,
                speed: FanSpeed::Level7,
            },
        ];

        let set_config_response = match client.set_config(new_config).await {
            Ok(response) => response,
            Err(e) => return Err(format!("Failed to set config: {}", e).into()),
        };
        println!("Daemon responded: {:?}\n", set_config_response);

        // Reset back to original config using the extracted response
        let reset_response = match client.set_config(config).await {
            Ok(response) => response,
            Err(e) => return Err(format!("Failed to reset config: {}", e).into()),
        };
        println!("Daemon responded: {:?}\n", reset_response);

        let rpm_response = match client.get_fan_rpm().await {
            Ok(response) => response,
            Err(e) => return Err(format!("Failed to get fan RPM: {}", e).into()),
        };

        println!("Daemon responded: {:?}\n", rpm_response);

        Ok(())
    })
}
