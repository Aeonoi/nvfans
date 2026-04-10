mod connection;

use connection::Client;
use std::error::Error;

pub fn run() -> Result<(), Box<dyn Error>> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        println!("Connecting to daemon...");

        if let Ok(client) = Client::new().await {
            println!("Connected!");
            println!("\nSending GetStatus request...");
            match client.get_status().await {
                Ok(response) => println!("Daemon responded: {:?}", response),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    });

    Ok(())
}
