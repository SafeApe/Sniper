use std::os::unix::net::UnixListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener as TokioUnixListener;

const SOCKET_PATH: &str = "/tmp/sniperSocket";

pub async fn CreateEVENTSub() -> Result<(), Box<dyn std::error::Error>> {
    // Remove the socket if it already exists
    if std::path::Path::new(SOCKET_PATH).exists() {
        std::fs::remove_file(SOCKET_PATH)?;
    }

    // Create the Unix socket listener
    let listener = TokioUnixListener::bind(SOCKET_PATH)?;

    println!("Server running at {}", SOCKET_PATH);

    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                tokio::spawn(async move {
                    let mut buffer = String::with_capacity(128);
                    match stream.read_to_string(&mut buffer).await {
                        Ok(_) => {
                            println!("Received message: {}", buffer);
                        }
                        Err(e) => {
                            eprintln!("Error reading from stream: {}", e);
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
}
