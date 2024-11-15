use alloy::primitives::Address;
use std::os::unix::net::UnixListener;
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener as TokioUnixListener;

const SOCKET_PATH: &str = "/tmp/sniperSocket";

pub struct BuyTrailSettings {
    trailingstoploss: f32,           //If 0 turned off else the percentage to trail
    trailingstoplosspercentage: f32, //We sell this percentage after hitting trailingstoploss percent of the price and decrease the price again to trail
    trailingtakeprofit: f32,         //If 0 turned off else the percentage to trail
    trailingprofitpercentage: f32, //We sell this percentage after hitting trailingtakeprofit percent of the price and increase the price again to trail
}

pub struct BuyMessage {
    pair_address: String,
    token_in: String,
    // path_before_token_in: Vec<String>, //In case the token_in is not available in the buyers_wallet
    amount: u128,
    recipient: String,
    slippage_percent: u128,
    takeprofit: u128, // 0 for no take profit
    stoploss: u128,   // 0 for no stop loss
    trail: Option<BuyTrailSettings>,
}

pub struct Event {
    notification: bool,
}

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
