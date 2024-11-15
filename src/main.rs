use alloy::primitives::{address, U256};
use client::provider::TradingClient;
use config::getConfig;
use std::{collections::HashMap, io, thread};
use utils::create_wallet_pk;
#[path = "./client/client.rs"]
mod client;

#[path = "./bot/bot.rs"]
mod bot;

// mod events;
// mod models;
use dipsniper::config;
use dipsniper::db;
use dipsniper::events;
use dipsniper::models;
use dipsniper::utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // keep the program running
    // let mut input = String::new();
    // io::stdin().read_line(&mut input).unwrap();
    let mut DB = db::DB::new().await?;
    // DB.add_user(db::User {
    //     userid: 1,
    //     wallets: HashMap::new(),
    //     stoploss: 0,
    //     trades: vec![],
    // });
    println!("{:?}", DB.get_user(1).await);
    let tradingCli = TradingClient::initialize().await.unwrap();
    tokio::spawn(async {
        println!("Starting event listener");
        events::CreateEVENTSub().await;
    });
    let a = tradingCli
        .buy(
            utils::convertToAddress("0x732130b66481A8f508569A4E74569C3F95f16Ace"),
            utils::convertToAddress("0x4200000000000000000000000000000000000006"),
            U256::from(100000000000 as i64),
            vec![config::getConfig().testpk.unwrap()],
            8453,
        )
        .await
        .unwrap();

    // println!("{:?}", tradingCli);
    Ok(())
}
