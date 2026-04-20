use crate::fan_control::{FanControl, convert_fan_speed, get_fan_rpm};
use nvfans_common::{Request, Response, Temperature, socket_path};
use std::error::Error;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
};

pub struct DaemonServer {
    fan_control: Arc<Mutex<FanControl>>,
}

impl DaemonServer {
    pub fn new(fan_control: Arc<Mutex<FanControl>>) -> DaemonServer {
        DaemonServer { fan_control }
    }

    pub async fn make_connection(&self) -> Result<(), Box<dyn Error>> {
        let socket = socket_path();

        // Ensure parent directory exists
        if let Some(parent) = socket.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Check if another daemon is already listening on the socket
        if Path::new(&socket).exists() {
            match tokio::net::UnixStream::connect(&socket).await {
                Ok(_) => {
                    // Connected successfully, so another daemon is running
                    return Err("Another daemon is already running".into());
                }
                Err(_) => {
                    // Connection failed, socket is stale - safe to remove
                    println!("Removing stale socket file...");
                    std::fs::remove_file(&socket)?;
                }
            }
        }

        let listener = UnixListener::bind(&socket)?;
        println!("Daemon listening on {}", socket.display());

        loop {
            // Wait for a new client to connect
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    println!("Accepted new client connection.");
                    // Clone the Arc to pass to the client task
                    let fan_control_clone = Arc::clone(&self.fan_control);
                    // Spawn a new task to handle this client concurrently.
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream, fan_control_clone).await {
                            eprintln!("Error handling client: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }
    }

    async fn handle_client(
        stream: UnixStream,
        fan_control: Arc<Mutex<FanControl>>,
    ) -> Result<(), Box<dyn Error>> {
        let (reader, mut writer) = tokio::io::split(stream);
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            // Read one line (one full JSON message) from the client
            let bytes_read = reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                // Connection was closed
                println!("Client disconnected.");
                break;
            }

            // Deserialize the request
            let request: Request = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    let response = Response::Error {
                        msg: format!("Failed to parse request: {}", e),
                    };
                    let response_json = serde_json::to_string(&response)? + "\n";
                    writer.write_all(response_json.as_bytes()).await?;
                    line.clear();
                    continue;
                }
            };

            println!("Received request: {:?}", request);

            // Process the request and create a response
            // Lock is scoped to ensure it drops before any await points
            let response = {
                let mut fc = fan_control.lock().expect("Failed to lock FanControl");
                match request {
                    Request::GetFanSpeedStatus => {
                        let current_rule = fc.get_current_rule();
                        Response::FanSpeedStatus {
                            temperature: Temperature {
                                low: current_rule.low,
                                high: current_rule.high,
                                speed: current_rule.speed,
                            },
                        }
                    }
                    // NOTE: We can probably do something with allowing the user to set the fan
                    // speed once and ignores the config, but for now we just write to the fan file
                    // and still follow the config.
                    Request::SetFanSpeed { low, high, speed } => {
                        let result = fc.write_to_fan("level", &convert_fan_speed(speed.clone()));
                        // fc.set_current_rule(low, high, speed.clone());
                        if let Err(e) = result {
                            Response::Error {
                                msg: format!("Failed to set fan speed: {}", e),
                            }
                        } else {
                            Response::Success {
                                msg: format!("Fan speed set to {:?}", speed),
                            }
                        }
                    }
                    Request::GetConfig => {
                        let config = fc.get_config();
                        Response::ConfigResponse {
                            config: config.clone(),
                        }
                    }
                    Request::SetConfig { config } => match fc.set_config(config.clone()) {
                        Ok(()) => Response::Success {
                            msg: "Configuration updated successfully".to_string(),
                        },
                        Err(e) => Response::Error {
                            msg: format!("Failed to write config: {}", e),
                        },
                    },
                    Request::GetFanRPM => {
                        let rpm = get_fan_rpm();
                        if rpm < 0 {
                            Response::Error {
                                msg: "Failed to read fan RPM".to_string(),
                            }
                        } else {
                            Response::FanSpeedRpm { rpm }
                        }
                    }
                }
            };

            // Serialize the response and send it back to the client
            let response_json = serde_json::to_string(&response)? + "\n";
            writer.write_all(response_json.as_bytes()).await?;

            // Clear the buffer for the next message
            line.clear();
        }

        Ok(())
    }
}
