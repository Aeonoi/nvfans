//! Integration tests for nvfans daemon
//!
//! These tests require a running daemon to connect to.
//!
//! ## Requirements
//!
//! 1. **Running daemon**: The nvfans daemon must be started first
//! 2. **Root privileges**: Tests require sudo/root because the daemon's socket
//!    is created in `/run/nvfans.sock` which is only accessible by root
//!
//! ## How to Run
//!
//! ```bash
//! # Terminal 1: Start the daemon with sudo
//! sudo cargo run --bin nvfans -- --daemon
//!
//! # Terminal 2: Run integration tests with sudo
//! sudo NVFANS_TEST_INTEGRATION=1 cargo test --package nvfans-client -- --ignored
//!
//! # Or run all tests including ignored ones
//! sudo cargo test --package nvfans-client -- --include-ignored
//! ```

use nvfans_client::{Client, FanSpeed, Response, Temperature};

fn check_integration_tests_enabled() {
    if std::env::var("NVFANS_TEST_INTEGRATION").is_err() {
        eprintln!("\nSkipping integration tests. To run them:");
        eprintln!("  1. Start the daemon with sudo: sudo cargo run --bin nvfans -- --daemon");
        eprintln!(
            "  2. Run tests with sudo: sudo NVFANS_TEST_INTEGRATION=1 cargo test --package nvfans-client -- --ignored"
        );
        eprintln!("\nNote: Tests require root privileges to connect to /run/nvfans.sock");
        eprintln!();
    }
}

/// Creates a test client connected to the daemon
async fn create_test_client() -> Client {
    Client::new()
        .await
        .expect("Failed to connect to daemon. Make sure the daemon is running.")
}

#[tokio::test]
#[ignore = "Requires running daemon - run with NVFANS_TEST_INTEGRATION=1 or --include-ignored"]
async fn test_get_status() {
    check_integration_tests_enabled();

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
#[ignore = "Requires running daemon - run with NVFANS_TEST_INTEGRATION=1 or --include-ignored"]
async fn test_set_fan_speed() {
    check_integration_tests_enabled();

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
#[ignore = "Requires running daemon - run with NVFANS_TEST_INTEGRATION=1 or --include-ignored"]
async fn test_get_config() {
    check_integration_tests_enabled();

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
#[ignore = "Requires running daemon - run with NVFANS_TEST_INTEGRATION=1 or --include-ignored"]
async fn test_set_and_reset_config() {
    check_integration_tests_enabled();

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
#[ignore = "Requires running daemon - run with NVFANS_TEST_INTEGRATION=1 or --include-ignored"]
async fn test_get_fan_rpm() {
    check_integration_tests_enabled();

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
