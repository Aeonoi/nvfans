pub const SOCKET_PATH: &str = "/tmp/nvfans.sock";

use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
};

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    SayHello { name: String },
    GetStatus,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Hello { message: String },
    Status { temperature: f32, fan_speed: u32 },
    Error { msg: String },
}

pub async fn make_connection() -> Result<(), Box<dyn Error>> {
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
                // Spawn a new task to handle this client
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream).await {
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
            Request::SayHello { name } => Response::Hello {
                message: format!("Hello, {}!", name),
            },
            Request::GetStatus => Response::Status {
                // In a real app, you'd get this from the hardware
                temperature: 68.5,
                fan_speed: 1337,
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
