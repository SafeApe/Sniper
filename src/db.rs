use crate::models::{Trades, User, Wallet, TradeSettings, Token, Pair};
use mongodb::{options::ClientOptions, Client, Collection, Database};
use mongodb::bson::{doc, Document};
use std::error::Error;
use futures_util::StreamExt;

// Define custom error type that implements Send + Sync
#[derive(Debug)]
pub struct DBError(String);

impl std::fmt::Display for DBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Database error: {}", self.0)
    }
}

impl std::error::Error for DBError {}

impl From<mongodb::error::Error> for DBError {
    fn from(err: mongodb::error::Error) -> Self {
        DBError(err.to_string())
    }
}

// Define the DB struct
pub struct DB {
    client: Client,
    db: Database,
    users_collection: Collection<User>,
    wallets_collection: Collection<Wallet>,
    trades_collection: Collection<Trades>,
    trade_settings_collection: Collection<TradeSettings>,
    tokens_collection: Collection<Token>,
    pairs_collection: Collection<Pair>,
}

impl DB {
    // Initialize the DB struct
    pub async fn new() -> Result<Self, DBError> {
        use crate::config::getConfig;
        let conf = &getConfig();
        let client_options = ClientOptions::parse(conf.getDataURI()).await?;
        let client = Client::with_options(client_options)?;
        let db = client.database(conf.getDataBase().as_str());
        db.create_collection("users").await?;
        db.create_collection("wallets").await?;
        db.create_collection("trades").await?;
        db.create_collection("trade_settings").await?;
        db.create_collection("tokens").await?;
        db.create_collection("pairs").await?;
        let users_collection = db.collection("users");
        let wallets_collection = db.collection("wallets");
        let trades_collection = db.collection("trades");
        let trade_settings_collection = db.collection("trade_settings");
        let tokens_collection = db.collection("tokens");
        let pairs_collection = db.collection("pairs");
        Ok(DB {
            client,
            db,
            users_collection,
            wallets_collection,
            trades_collection,
            trade_settings_collection,
            tokens_collection,
            pairs_collection,
        })
    }

    pub async fn create_user(&self, user: User) -> Result<(), DBError> {
        let userid = user.userid.clone();
        self.users_collection.insert_one(user).await?;
        self.trade_settings_collection
            .insert_one(TradeSettings::new(userid))
            .await?;
        Ok(())
    }

    pub async fn remove_user(&self, userid: i64) -> Result<(), DBError> {
        let filter = doc! { "userid": userid };
        self.users_collection.delete_one(filter).await?;
        Ok(())
    }

    pub async fn update_user(
        &self,
        userid: i64,
        update: Document,
    ) -> Result<(), DBError> {
        let filter = doc! { "userid": userid };
        self.users_collection.update_one(filter, update).await?;
        Ok(())
    }

    pub async fn get_user(&self, userid: i64) -> Result<Option<User>, DBError> {
        let filter = doc! { "userid": userid };
        let user = self.users_collection.find_one(filter).await?;
        match user {
            Some(user) => Ok(Some(user)),
            None => Ok(None),
        }
    }

    pub async fn create_wallet(&self, wallet: Wallet) -> Result<(), DBError> {
        self.wallets_collection.insert_one(wallet).await?;
        Ok(())
    }

    pub async fn remove_wallet(&self, address: &str) -> Result<(), DBError> {
        let filter = doc! { "address": address };
        self.wallets_collection.delete_one(filter).await?;
        Ok(())
    }

    pub async fn update_wallet(
        &self,
        address: &str,
        update: Document,
    ) -> Result<(), DBError> {
        let filter = doc! { "address": address };
        self.wallets_collection.update_one(filter, update).await?;
        Ok(())
    }

    pub async fn get_wallets(&self, userid: i64) -> Result<Vec<Wallet>, DBError> {
        let filter = doc! { "userid": userid };
        let mut cursor = self.wallets_collection.find(filter).await?;
        let mut wallets = Vec::new();
        while let Some(wallet) = cursor.next().await {
            wallets.push(wallet?);
        }
        Ok(wallets)
    }

    pub async fn update_wallet_name(&self, address: &str, new_name: &str) -> Result<(), DBError> {
        let filter = doc! { "address": address };
        let update = doc! { "$set": { "name": new_name } };
        self.wallets_collection.update_one(filter, update).await?;
        Ok(())
    }

    pub async fn create_trade(&self, trade: Trades) -> Result<(), DBError> {
        self.trades_collection.insert_one(trade).await?;
        Ok(())
    }

    pub async fn remove_trade(&self, address: &str) -> Result<(), DBError> {
        let filter = doc! { "address": address };
        self.trades_collection.delete_one(filter).await?;
        Ok(())
    }

    pub async fn update_trade(
        &self,
        address: &str,
        update: Document,
    ) -> Result<(), DBError> {
        let filter = doc! { "address": address };
        self.trades_collection.update_one(filter, update).await?;
        Ok(())
    }

    // Trade Settings Methods
    pub async fn get_trade_settings(&self, userid: i64) -> Result<Option<TradeSettings>, DBError> {
        let filter = doc! { "userid": userid };
        let settings = self.trade_settings_collection.find_one(filter).await?;
        Ok(settings)
    }

    pub async fn create_trade_settings(&self, settings: TradeSettings) -> Result<(), DBError> {
        self.trade_settings_collection.insert_one(settings).await?;
        Ok(())
    }

    pub async fn update_trade_settings(&self, settings: TradeSettings) -> Result<(), DBError> {
        let filter = doc! { "userid": settings.userid };
        let update = doc! { "$set": mongodb::bson::to_document(&settings).unwrap() };
        self.trade_settings_collection.update_one(filter, update).await?;
        Ok(())
    }

    // Token Methods
    pub async fn create_token(&self, token: Token) -> Result<(), DBError> {
        self.tokens_collection.insert_one(token).await?;
        Ok(())
    }

    pub async fn get_token(&self, address: &str) -> Result<Option<Token>, DBError> {
        let filter = doc! { "address": address };
        Ok(self.tokens_collection.find_one(filter).await?)
    }

    pub async fn get_tokens_by_chain(&self, chain_id: i64) -> Result<Vec<Token>, DBError> {
        let filter = doc! { "chain": chain_id };
        let mut cursor = self.tokens_collection.find(filter).await?;
        let mut tokens = Vec::new();
        while let Some(token) = cursor.next().await {
            tokens.push(token?);
        }
        Ok(tokens)
    }

    pub async fn update_token(&self, address: &str, update: Document) -> Result<(), DBError> {
        let filter = doc! { "address": address };
        self.tokens_collection.update_one(filter, update).await?;
        Ok(())
    }

    pub async fn add_pair_to_token(&self, token_address: &str, pair_address: &str) -> Result<(), DBError> {
        let filter = doc! { "address": token_address };
        let update = doc! { "$addToSet": { "pairs": pair_address } };
        self.tokens_collection.update_one(filter, update).await?;
        Ok(())
    }

    // Pair Methods
    pub async fn create_pair(&self, pair: Pair) -> Result<(), DBError> {
        self.pairs_collection.insert_one(pair).await?;
        Ok(())
    }

    pub async fn get_pair(&self, token1: &str, token2: &str, dex: &str) -> Result<Option<Pair>, DBError> {
        let filter = doc! { 
            "$or": [
                { "token1": token1, "token2": token2 },
                { "token1": token2, "token2": token1 }
            ],
            "dex": dex
        };
        Ok(self.pairs_collection.find_one(filter).await?)
    }

    pub async fn get_pairs_by_token(&self, token_address: &str) -> Result<Vec<Pair>, DBError> {
        let filter = doc! { 
            "$or": [
                { "token1": token_address },
                { "token2": token_address }
            ]
        };
        let mut cursor = self.pairs_collection.find(filter).await?;
        let mut pairs = Vec::new();
        while let Some(pair) = cursor.next().await {
            pairs.push(pair?);
        }
        Ok(pairs)
    }

    pub async fn update_pair(&self, token1: &str, token2: &str, dex: &str, update: Document) -> Result<(), DBError> {
        let filter = doc! { 
            "$or": [
                { "token1": token1, "token2": token2 },
                { "token1": token2, "token2": token1 }
            ],
            "dex": dex
        };
        self.pairs_collection.update_one(filter, update).await?;
        Ok(())
    }
}

// // TESTS
// #[cfg(test)]
// mod tests {
//     use super::*;
//     #[test]
//     async fn test_db() {
//         let db = DB::new().await.unwrap();
//         let user = User {
//             userid: 1248191458,
//         };
//     }
// }
