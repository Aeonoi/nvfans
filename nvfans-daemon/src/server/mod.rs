pub const SOCKET_PATH: &str = "/tmp/nvfans.sock";

use nvfans_common::{FanSpeed, Request, Response, Temperature};
use std::error::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    sync::mpsc::{Receiver, Sender},
};

use crate::fan_control::FanControl;

pub struct DaemonServer {
    sender: Sender<Request>,
    receiver: Receiver<Request>,
}

impl DaemonServer {
    pub fn new(sender: Sender<Request>, receiver: Receiver<Request>) -> DaemonServer {
        DaemonServer {
            sender: sender,
            receiver,
        }
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
                    // Spawn a new task to handle this client concurrently.
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream).await {
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

    async fn handle_client(stream: UnixStream) -> Result<(), Box<dyn Error>> {
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
                Request::GetFanSpeedStatus => Response::FanSpeedStatus {
                    temperature: Temperature {
                        low: 68,
                        high: 100,
                        speed: FanSpeed::Level0,
                    },
                },
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
