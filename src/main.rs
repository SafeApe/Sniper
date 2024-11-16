use alloy::primitives::{address, U256};
use client::provider::TradingClient;
use config::getConfig;
use std::sync::Arc;
use std::{collections::HashMap, io, thread};
use utils::create_wallet_pk;
#[path = "./client/client.rs"]
mod client;

#[path = "./bot/bot.rs"]
mod bot;

use bot::helper::PairHandler;

use crate::db::DB;

// mod events;
// mod models;
use dipsniper::config;
use dipsniper::db;
use dipsniper::events::{self, start_event_handler, EventHandler};
use dipsniper::models;
use dipsniper::utils;
use dipsniper::client::provider::TradingClient as BaseTradingClient;

use pool_sync::{PoolSync, PoolType, Chain, PoolInfo};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool_sync = PoolSync::builder()
        .add_pool(PoolType::UniswapV2)
        .chain(Chain::Ethereum).rate_limit(1000)
        .build()?;

    // Synchronize pools
    let (pools, last_synced_block) = pool_sync.sync_pools().await?;
    for pool in &pools {
        println!("Pool Address {:?}, Token 0: {:?}, Token 1: {:?}", pool.address(), pool.token0_name(), pool.token1_name());
    }

    println!("Synced {} pools!", pools.len());

    // let pair_handler = PairHandler::new();
    // let res = pair_handler.search("pepe").await.unwrap();
    // println!("{:?}",res);
    return Ok(());


    let db = Arc::new(DB::new().await?);
    let trading_client = Arc::new(BaseTradingClient::initialize().await?);
    
    // Spawn the price monitoring task
    let monitor_client = trading_client.clone();
    tokio::spawn(async move {
        if let Err(e) = monitor_client.monitor_prices().await {
            eprintln!("Price monitoring error: {}", e);
        }
    });

    // Start event handler
    let event_handler = EventHandler::new(db, trading_client);
    start_event_handler(event_handler).await;
    Ok(())
}
