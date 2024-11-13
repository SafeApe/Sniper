use crate::models::{Trades, User, Wallet};
use mongodb::bson::{doc, Document};
use mongodb::{options::ClientOptions, Client, Collection, Database};
use serde::{Deserialize, Serialize};
use tokio;
// Define the DB struct
pub struct DB {
    client: Client,
    db: Database,
    users_collection: Collection<User>,
    wallets_collection: Collection<Wallet>,
    trades_collection: Collection<Trades>,
}

impl DB {
    // Initialize the DB struct
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        use crate::config::getConfig;
        let conf = &getConfig();
        let client_options = ClientOptions::parse(conf.getDataURI()).await?;
        let client = Client::with_options(client_options)?;
        let db = client.database(conf.getDataBase().as_str());
        db.create_collection("users").await?;
        db.create_collection("wallets").await?;
        db.create_collection("trades").await?;
        let users_collection = db.collection("users");
        let wallets_collection = db.collection("wallets");
        let trades_collection = db.collection("trades");
        Ok(DB {
            client,
            db,
            users_collection,
            wallets_collection,
            trades_collection,
        })
    }

    pub async fn create_user(&self, user: User) -> Result<(), Box<dyn std::error::Error>> {
        self.users_collection.insert_one(user).await?;
        Ok(())
    }

    pub async fn remove_user(&self, userid: u64) -> Result<(), Box<dyn std::error::Error>> {
        let filter = doc! { "userid": userid as u32};
        self.users_collection.delete_one(filter).await?;
        Ok(())
    }

    pub async fn update_user(
        &self,
        userid: u64,
        update: Document,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let filter = doc! { "userid": userid as u32};
        self.users_collection.update_one(filter, update).await?;
        Ok(())
    }

    pub async fn get_user(&self, userid: u64) -> Result<Option<User>, Box<dyn std::error::Error>> {
        let filter = doc! { "userid": userid as u32};
        let user = self.users_collection.find_one(filter).await?;
        match user {
            Some(user) => Ok(Some(user)),
            None => Ok(None),
        }
    }

    pub async fn create_wallet(&self, wallet: Wallet) -> Result<(), Box<dyn std::error::Error>> {
        self.wallets_collection.insert_one(wallet).await?;
        Ok(())
    }

    pub async fn remove_wallet(&self, address: &str) -> Result<(), Box<dyn std::error::Error>> {
        let filter = doc! { "address": address };
        self.wallets_collection.delete_one(filter).await?;
        Ok(())
    }

    pub async fn update_wallet(
        &self,
        address: &str,
        update: Document,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let filter = doc! { "address": address };
        self.wallets_collection.update_one(filter, update).await?;
        Ok(())
    }

    pub async fn create_trade(&self, trade: Trades) -> Result<(), Box<dyn std::error::Error>> {
        self.trades_collection.insert_one(trade).await?;
        Ok(())
    }

    pub async fn remove_trade(&self, address: &str) -> Result<(), Box<dyn std::error::Error>> {
        let filter = doc! { "address": address };
        self.trades_collection.delete_one(filter).await?;
        Ok(())
    }

    pub async fn update_trade(
        &self,
        address: &str,
        update: Document,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let filter = doc! { "address": address };
        self.trades_collection.update_one(filter, update).await?;
        Ok(())
    }
}
