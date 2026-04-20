//! Integration tests for nvfans daemon
//!
//! These tests require a running daemon to connect to.

use nvfans_client::{Client, FanSpeed, Response, Temperature};

// Note: Tests require a running daemon. Start it before running tests:
// cargo run --bin nvfans-daemon -- --daemon

/// Creates a test client connected to the daemon
async fn create_test_client() -> Client {
    Client::new()
        .await
        .expect("Failed to connect to daemon. Make sure the daemon is running.")
}

#[tokio::test]
async fn test_get_status() {
    let client = create_test_client().await;

    let response = client.get_status().await.expect("Failed to get status");

    match response {
        Response::FanSpeedStatus { temperature } => {
            assert!(temperature.high >= 0, "High temp should be non-negative");
            println!("Got temperature status: {:?}", temperature);
        }
        Response::Error { msg } => panic!("Server returned error: {}", msg),
        _ => panic!("Unexpected response type"),
    }
}

#[tokio::test]
async fn test_set_fan_speed() {
    let client = create_test_client().await;

    let response = client
        .set_fan_speed(0, 100, FanSpeed::Level1)
        .await
        .expect("Failed to set fan speed");

    match response {
        Response::Success { msg } => {
            assert!(msg.contains("1"));
            println!("Success: {}", msg);
        }
        Response::Error { msg } => panic!("Failed to set fan speed: {}", msg),
        _ => panic!("Unexpected response type"),
    }

    // Reset to auto
    let _ = client.set_fan_speed(0, 100, FanSpeed::Auto).await;
}

#[tokio::test]
async fn test_get_config() {
    let client = create_test_client().await;

    let response = client.get_config().await.expect("Failed to get config");

    match response {
        Response::ConfigResponse { config } => {
            assert!(!config.is_empty(), "Config should not be empty");
            println!("Got config with {} temperature rules", config.len());
            for rule in &config {
                assert!(rule.low < rule.high, "Low must be less than high");
            }
        }
        Response::Error { msg } => panic!("Server returned error: {}", msg),
        _ => panic!("Unexpected response type"),
    }
}

#[tokio::test]
async fn test_set_and_reset_config() {
    let client = create_test_client().await;

    // Store original config
    let original_config = match client.get_config().await.expect("Failed to get config") {
        Response::ConfigResponse { config } => config,
        Response::Error { msg } => panic!("Failed to get original config: {}", msg),
        _ => panic!("Unexpected response type"),
    };

    // Create a temporary config
    let test_config: Vec<Temperature> = vec![
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

    // Set the test config
    let response = client
        .set_config(test_config.clone())
        .await
        .expect("Failed to set config");

    match response {
        Response::Success { msg } => println!("Set config success: {}", msg),
        Response::Error { msg } => panic!("Failed to set config: {}", msg),
        _ => panic!("Unexpected response type"),
    }

    // Verify the config was set
    let current_config = match client.get_config().await.expect("Failed to get config") {
        Response::ConfigResponse { config } => config,
        Response::Error { msg } => panic!("Failed to get current config: {}", msg),
        _ => panic!("Unexpected response type"),
    };

    assert_eq!(
        current_config.len(),
        test_config.len(),
        "Config length should match"
    );

    // Reset to original config
    let response = client
        .set_config(original_config)
        .await
        .expect("Failed to reset config");

    match response {
        Response::Success { msg } => println!("Reset config success: {}", msg),
        Response::Error { msg } => panic!("Failed to reset config: {}", msg),
        _ => panic!("Unexpected response type"),
    }
}

#[tokio::test]
async fn test_get_fan_rpm() {
    let client = create_test_client().await;

    let response = client.get_fan_rpm().await.expect("Failed to get fan RPM");

    match response {
        Response::FanSpeedRpm { rpm } => {
            println!("Fan RPM: {}", rpm);
            // RPM should be non-negative
            assert!(rpm >= 0, "RPM should be non-negative");
        }
        Response::Error { msg } => panic!("Server returned error: {}", msg),
        _ => panic!("Unexpected response type: {:?}", response),
    }
}
