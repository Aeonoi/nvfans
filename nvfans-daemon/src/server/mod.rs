pub const SOCKET_PATH: &str = "/tmp/nvfans.sock";

use crate::fan_control::FanControl;
use nvfans_common::{FanSpeed, Request, Response, Temperature};
use std::error::Error;
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
        // Clean up the socket file if it already exists
        if std::fs::metadata(SOCKET_PATH).is_ok() {
            println!("Removing existing socket file...");
            std::fs::remove_file(SOCKET_PATH)?;
        }

        let listener = UnixListener::bind(SOCKET_PATH)?;
        println!("Daemon listening on {}", SOCKET_PATH);

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
            let response = match request {
                Request::GetFanSpeedStatus => {
                    // Lock the FanControl to get current status
                    let mut fc = fan_control.lock().expect("Failed to lock FanControl");
                    let current_temp = fc.get_max_temp();
                    Response::FanSpeedStatus {
                        temperature: Temperature {
                            low: 0, // Placeholder
                            high: current_temp,
                            speed: FanSpeed::Level0, // Placeholder
                        },
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
