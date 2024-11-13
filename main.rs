use alloy::primitives::{address, U256};
use client::provider::TradingClient;
use config::getConfig;
use std::{collections::HashMap, io, thread};
#[path = "./client/client.rs"]
mod client;

mod config;
mod db;
mod events;
mod models;
mod utils;

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
    let tradingCli =
        TradingClient::initialize("wss://mainnet.infura.io/ws/v3/b6bf7d3508c941499b10025c0776eaf8")
            .await
            .unwrap();
    tokio::spawn(async {
        println!("Starting event listener");
        events::CreateEVENTSub().await;
    });
    tradingCli
        .buy(
            utils::convertToAddress("0x963189eA3ec15B9974bF222F1487f15348750366"),
            utils::convertToAddress("0x963189eA3ec15B9974bF222F1487f15348750366"),
            U256::from(1000000),
            vec![],
            1,
        )
        .await;
    println!(
        "{:?}",
        utils::convertToAddress("0x2184E68a05F736dc85758d82254C5D24a269d177")
    );
    println!("{:?}", tradingCli);
    Ok(())
}
