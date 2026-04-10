use nvfans_common::{Request, Response};
use std::error::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::Mutex,
};

const SOCKET_PATH: &str = "/tmp/nvfans.sock";

/// A client for the nvfans daemon.
///
/// This client maintains a persistent connection to the daemon's Unix socket.
pub struct Client {
    // We use a Mutex to allow for safe concurrent access to the stream
    // from multiple async tasks if needed.
    stream: Mutex<UnixStream>,
}

impl Client {
    /// Connects to the nvfans daemon socket.
    pub async fn new() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let stream = UnixStream::connect(SOCKET_PATH).await?;
        Ok(Self {
            stream: Mutex::new(stream),
        })
    }

    /// Sends a request to the daemon and awaits a response.
    async fn send_request(
        &self,
        request: Request,
    ) -> Result<Response, Box<dyn Error + Send + Sync>> {
        let mut locked_stream = self.stream.lock().await;

        // Serialize the request and send it
        let request_json = serde_json::to_string(&request)? + "\n";
        locked_stream.write_all(request_json.as_bytes()).await?;

        // Wait for the response
        let (reader, _) = locked_stream.split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        // Deserialize and return the response
        let response = serde_json::from_str(&line)?;
        println!("Received response: {:?}", response);
        Ok(response)
    }

    /// Sends a `GetStatus` request.
    pub async fn get_status(&self) -> Result<Response, Box<dyn Error + Send + Sync>> {
        let request = Request::GetFanSpeedStatus;
        self.send_request(request).await
    }
}
