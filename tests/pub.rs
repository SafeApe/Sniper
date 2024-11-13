#!/usr/bin/env rust-script
//! # sub.rs
//! ```cargo
//! [dependencies]
//! tokio = { version = "1.8", features = ["rt-multi-thread", "macros", "full"] }
//! serde_json = "1.0.132"
//! ```
//!
use serde_json::json;
use std::os::unix::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream as TokioUnixStream;

const SOCKET_PATH: &str = "/tmp/sniperSocket";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the Unix socket
    let mut stream = TokioUnixStream::connect(SOCKET_PATH).await?;

    // Prepare the JSON data to send
    let json = json!({"message": "Hello from client!"});
    let json_string = json.to_string();

    // Send the JSON data to the server
    stream.write_all(json_string.as_bytes()).await?;
    stream.shutdown().await?;

    Ok(())
}
