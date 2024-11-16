use alloy::primitives::{Address,U256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener as TokioUnixListener;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// use crate::models::{TokenInfo, PairInfo, UserState};
use crate::db::DB;
use crate::client::provider::TradingClient;
use crate::models::{trailset, Wallet};
use crate::utils;
const SOCKET_PATH: &str = "/tmp/sniperSocket";

#[derive(Debug, Serialize, Deserialize)]
pub struct BuyTrailSettings {
    pub trailing_stop_loss: f32,           // If 0 turned off else the percentage to trail
    pub trailing_stop_loss_percentage: f32, // Sell percentage after hitting trailing_stop_loss
    pub trailing_take_profit: f32,         // If 0 turned off else the percentage to trail
    pub trailing_profit_percentage: f32,   // Sell percentage after hitting trailing_take_profit
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuyMessage {
    pub pair_address: String,
    pub token_in: String,
    pub token_out: String,
    pub amount: String,
    pub wallets: Vec<String>,
    pub chain_id: u64,
    pub takeprofit: u64,
    pub stoploss: u64,
    pub trail: Option<BuyTrailSettings>,
    pub mev: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SocketMessage {
    // TokenInfo {
    //     address: String,
    //     user_id: i64,
    // },
    Buy {
        message: BuyMessage,
        user_id: i64,
    },
    Sell {
        percentage: f64,
        user_id: i64,
    },
    UpdateTrade {
        user_id: i64,
    },
}

pub struct EventHandler {
    db: Arc<DB>,
    trading_client: Arc<TradingClient>,
}

impl EventHandler {
    pub fn new(db: Arc<DB>, trading_client: Arc<TradingClient>) -> Self {
        Self {
            db,
            trading_client,
        }
    }

    async fn handle_token_info(&self, address: String, user_id: i64) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let wallets = self.db.get_wallets(user_id).await?;
        if wallets.is_empty() {
            return Ok("No wallets found. Please add a wallet first.".to_string());
        }

        // Token info retrieval implementation
        Ok("Token info retrieved successfully".to_string())
    }

    async fn handle_buy(&self, buy_message: BuyMessage, user_id: i64) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Convert string addresses to Address type
        let pair = utils::convertToAddress(&buy_message.pair_address);
        let token_in = utils::convertToAddress(&buy_message.token_in);
        let token_out = utils::convertToAddress(&buy_message.token_out);
        
        // Convert amount string to U256
        let amount = U256::from(buy_message.amount.parse::<u128>()
            .map_err(|e| format!("Invalid amount format: {}", e))?);

        // Get user's wallets if none provided
        // let wallets = if buy_message.wallets.is_empty() {
        //     self.db.get_wallets(user_id).await?
        // } else {
        //     buy_message.wallets
        // };
        let wallets = buy_message.wallets;

        // Forward to trading client
        self.trading_client.buy(
            pair,
            token_in,
            token_out,
            amount,
            wallets,
            buy_message.chain_id,
            buy_message.stoploss,
            buy_message.takeprofit,
            buy_message.trail.map(|t| trailset {
                trailing_stop_loss: t.trailing_stop_loss,
                trailing_stop_loss_percentage: t.trailing_stop_loss_percentage,
                trailing_take_profit: t.trailing_take_profit,
                trailing_take_profit_percentage: t.trailing_profit_percentage,
            }),
            buy_message.mev
        ).await?;

        Ok("Buy order executed successfully".to_string())
    }

    async fn handle_message(&self, message: SocketMessage) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        match message {
            // SocketMessage::TokenInfo { address, user_id } => {
            //     self.handle_token_info(address, user_id).await
            // }
            SocketMessage::Buy { message, user_id } => {
                self.handle_buy(message, user_id).await
            }
            SocketMessage::Sell { percentage, user_id } => {
                // Implement sell logic
                Ok("Sell order received".to_string())
            }
            SocketMessage::UpdateTrade { user_id } => {
                // Implement wallet switching
                Ok("Update trade request received ".to_string())
            }
        }
    }
}

pub async fn start_event_handler(handler:EventHandler) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if std::path::Path::new(SOCKET_PATH).exists() {
        std::fs::remove_file(SOCKET_PATH)?;
    }

    let listener = TokioUnixListener::bind(SOCKET_PATH)?;
    println!("Event handler listening on {}", SOCKET_PATH);

    // let db = Arc::new(DB::new().await?);
    // let trading_client = Arc::new(TradingClient::new().await?);
    // let handler = EventHandler::new(db, trading_client);

    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                let mut buffer = Vec::new();
                match stream.read_to_end(&mut buffer).await {
                    Ok(_) => {
                        if let Ok(message) = serde_json::from_slice::<SocketMessage>(&buffer) {
                            match handler.handle_message(message).await {
                                Ok(response) => {
                                    let _ = stream.write_all(response.as_bytes()).await;
                                }
                                Err(e) => {
                                    eprintln!("Error handling message: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading from stream: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
}
