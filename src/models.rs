use crate::utils::create_wallet;
use bson::{doc, Document};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct trailset {
    pub trailing_stop_loss: f32, //If 0 turned off else the percentage to trail
    pub trailing_stop_loss_percentage: f32, //We sell this percentage after hitting trailingstoploss percent of the price and decrease the price again to trail
    pub trailing_take_profit: f32,         //If 0 turned off else the percentage to trail
    pub trailing_take_profit_percentage: f32, //We sell this percentage after hitting trailingtakeprofit percent of the price and increase the price again to trail
}

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct Trades {
    pub _id: Option<ObjectId>,
    pub userid: i64,
    pub address: String,
    pub isEVM: bool,
    pub chainID: i64,
    pub amount: String, // U256 is not directly supported, convert to string
    pub active: bool,
    pub priceBought: f64,
    pub stoploss: u64,   //If this is 0, stoploss is turned off
    pub takeprofit: u64, //If this is 0, takeprofit is turned off
    pub trail: Option<trailset>,
}

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct Wallet {
    pub _id: Option<ObjectId>,
    pub address: String,
    pub chainIDs: Vec<i64>, // If empty, enabled for all chains
    pub privateKey: String,
    pub userid: i64,
    pub name: String, // Adding name field for wallet identification
}

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct User {
    pub _id: Option<ObjectId>,
    pub userid: i64,
}

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct TradeSettings {
    // These are the settings for default trades
    pub userid: i64,
    pub multiwallet: bool,
    pub stoploss: f32,
    pub takeprofit: f32,
    pub trailing: bool,        //If this is true, the following trailing settings are used
    pub trailing_stop_loss: f32, //If 0 turned off else the percentage to trail
    pub trailing_stop_loss_percentage: f32, //We sell this percentage after hitting trailingstoploss percent of the price and decrease the price again to trail
    pub trailing_take_profit: f32,         //If 0 turned off else the percentage to trail
    pub trailing_take_profit_percentage: f32, //We sell this percentage after hitting trailingtakeprofit percent of the price and increase the price again to trail
    pub mev_enabled_chains: Vec<i64>, // Chain IDs where MEV is enabled. Empty vector means MEV is disabled on all chains
}

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct CopyTrade {
    //To copy trades from another user can be added as feature later
    userid: i64,
    active: bool,
    address: String,
    chains: Vec<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Token {
    pub _id: Option<ObjectId>,
    pub name: String,
    pub supply: String,  // Using String for large numbers
    pub owner: String,   // Address as string
    pub decimals: u8,
    pub chain: i64,      // Chain ID
    pub pairs: Vec<String>, // Vector of pair addresses
    pub created_at: Option<i64>, // Unix timestamp
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pair {
    pub _id: Option<ObjectId>,
    pub token1: String,  // Address of first token
    pub token2: String,  // Address of second token
    pub pool_version: String, // Version of the pool (e.g., "v2", "v3")
    pub dex: String,     // Name of the DEX (e.g., "Uniswap", "PancakeSwap")
    pub created_at: Option<i64>, // Unix timestamp
}

impl From<Trades> for Document {
    fn from(trade: Trades) -> Self {
        doc! {
            "_id": trade._id,
            "address": trade.address,
            "isEVM": trade.isEVM,
            "chainID": trade.chainID.to_string(),
            "amount": trade.amount,
            "active": trade.active,
            "priceBought": trade.priceBought,
            "stoploss": trade.stoploss.to_string(),
            "takeprofit": trade.takeprofit.to_string(),
            "trail": match trade.trail {
                Some(trail) => doc! {
                    "trailing_stop_loss": trail.trailing_stop_loss as f64,
                    "trailing_stop_loss_percentage": trail.trailing_stop_loss_percentage as f64,
                    "trailing_take_profit": trail.trailing_take_profit as f64,
                    "trailing_take_profit_percentage": trail.trailing_take_profit_percentage as f64,
                },
                None => doc! {},
            },
        }
    }
}

impl From<Wallet> for Document {
    fn from(wallet: Wallet) -> Self {
        doc! {
            "_id": wallet._id,
            "userid": wallet.userid.to_string(),
            "address": wallet.address,
            "chainIDs": wallet.chainIDs,
            "privateKey": wallet.privateKey,
            "name": wallet.name,
        }
    }
}

impl From<User> for Document {
    fn from(user: User) -> Self {
        doc! {
            "_id": user._id,
            "userid": user.userid.to_string(),
        }
    }
}

impl From<Document> for Trades {
    fn from(doc: Document) -> Self {
        Trades {
            _id: Some(doc.get_object_id("_id").unwrap()),
            userid: doc.get_i64("userid").unwrap(),
            address: doc.get_str("address").unwrap().to_string(),
            isEVM: doc.get_bool("isEVM").unwrap(),
            chainID: doc.get_i64("chainID").unwrap(),
            amount: doc.get_str("amount").unwrap().to_string(),
            active: doc.get_bool("active").unwrap(),
            priceBought: doc.get_f64("priceBought").unwrap(),
            stoploss: doc.get_i64("stoploss").unwrap() as u64,
            takeprofit: doc.get_i64("takeprofit").unwrap() as u64,
            trail: match doc.get_document("trail") {
                Ok(trail) => Some(trailset {
                    trailing_stop_loss: trail.get_f64("trailing_stop_loss").unwrap() as f32,
                    trailing_stop_loss_percentage: trail
                        .get_f64("trailing_stop_loss_percentage")
                        .unwrap() as f32,
                    trailing_take_profit: trail.get_f64("trailing_take_profit").unwrap() as f32,
                    trailing_take_profit_percentage: trail
                        .get_f64("trailing_take_profit_percentage")
                        .unwrap() as f32,
                }),
                Err(_) => None,
            },
        }
    }
}

impl From<Document> for Wallet {
    fn from(doc: Document) -> Self {
        Wallet {
            _id: Some(doc.get_object_id("_id").unwrap()),
            userid: doc.get_i64("userid").unwrap(),
            address: doc.get_str("address").unwrap().to_string(),
            chainIDs: doc
                .get_array("chainIDs")
                .unwrap()
                .into_iter()
                .map(|x| x.as_i64().unwrap())
                .collect(),
            privateKey: doc.get_str("privateKey").unwrap().to_string(),
            name: doc.get_str("name").unwrap().to_string(),
        }
    }
}

impl From<Document> for User {
    fn from(doc: Document) -> Self {
        User {
            _id: Some(doc.get_object_id("_id").unwrap()),
            userid: doc.get_i64("userid").unwrap(),
        }
    }
}

impl From<TradeSettings> for Document {
    fn from(settings: TradeSettings) -> Self {
        doc! {
            "userid": settings.userid.to_string(),
            "multiwallet": settings.multiwallet,
            "stoploss": settings.stoploss as f64,
            "takeprofit": settings.takeprofit as f64,
            "trailing": settings.trailing,
            "trailing_stop_loss": settings.trailing_stop_loss as f64,
            "trailing_stop_loss_percentage": settings.trailing_stop_loss_percentage as f64,
            "trailing_take_profit": settings.trailing_take_profit as f64,
            "trailing_take_profit_percentage": settings.trailing_take_profit_percentage as f64,
            "mev_enabled_chains": settings.mev_enabled_chains,
        }
    }
}

impl From<Document> for TradeSettings {
    fn from(doc: Document) -> Self {
        TradeSettings {
            userid: doc.get_i64("userid").unwrap(),
            multiwallet: doc.get_bool("multiwallet").unwrap(),
            stoploss: doc.get_f64("stoploss").unwrap() as f32,
            takeprofit: doc.get_f64("takeprofit").unwrap() as f32,
            trailing: doc.get_bool("trailing").unwrap(),
            trailing_stop_loss: doc.get_f64("trailing_stop_loss").unwrap() as f32,
            trailing_stop_loss_percentage: doc
                .get_f64("trailing_stop_loss_percentage")
                .unwrap() as f32,
            trailing_take_profit: doc.get_f64("trailing_take_profit").unwrap() as f32,
            trailing_take_profit_percentage: doc
                .get_f64("trailing_take_profit_percentage")
                .unwrap() as f32,
            mev_enabled_chains: doc
                .get_array("mev_enabled_chains")
                .unwrap()
                .into_iter()
                .map(|x| x.as_i64().unwrap())
                .collect(),
        }
    }
}

impl From<Token> for Document {
    fn from(token: Token) -> Self {
        doc! {
            "_id": token._id,
            "name": token.name,
            "supply": token.supply,
            "owner": token.owner,
            "decimals": token.decimals as i32,
            "chain": token.chain,
            "pairs": token.pairs,
            "created_at": token.created_at,
        }
    }
}

impl From<Document> for Token {
    fn from(doc: Document) -> Self {
        Token {
            _id: doc.get_object_id("_id").ok(),
            name: doc.get_str("name").unwrap().to_string(),
            supply: doc.get_str("supply").unwrap().to_string(),
            owner: doc.get_str("owner").unwrap().to_string(),
            decimals: doc.get_i32("decimals").unwrap() as u8,
            chain: doc.get_i64("chain").unwrap(),
            pairs: doc.get_array("pairs").unwrap()
                .iter()
                .map(|x| x.as_str().unwrap().to_string())
                .collect(),
            created_at: doc.get_i64("created_at").ok(),
        }
    }
}

impl From<Pair> for Document {
    fn from(pair: Pair) -> Self {
        doc! {
            "_id": pair._id,
            "token1": pair.token1,
            "token2": pair.token2,
            "pool_version": pair.pool_version,
            "dex": pair.dex,
            "created_at": pair.created_at,
        }
    }
}

impl From<Document> for Pair {
    fn from(doc: Document) -> Self {
        Pair {
            _id: doc.get_object_id("_id").ok(),
            token1: doc.get_str("token1").unwrap().to_string(),
            token2: doc.get_str("token2").unwrap().to_string(),
            pool_version: doc.get_str("pool_version").unwrap().to_string(),
            dex: doc.get_str("dex").unwrap().to_string(),
            created_at: doc.get_i64("created_at").ok(),
        }
    }
}

impl User {
    pub fn default(userid: i64) -> User {
        User { _id: None, userid }
    }

    pub fn new(userid: i64) -> User {
        User { _id: None, userid }
    }
}

impl Wallet {
    pub fn new(userid: i64) -> Wallet {
        let (privateKey, address) = create_wallet();
        Wallet {
            _id: None,
            userid,
            address: address.to_string(),
            chainIDs: vec![],
            privateKey,
            name: format!("Wallet {}", &address.to_string()[0..8]), // Default name using first 8 chars of address
        }
    }

    pub fn default(userid: i64, address: String, chainIDs: Vec<i64>, privateKey: String) -> Wallet {
        Wallet {
            _id: None,
            userid,
            address: address.clone(),
            chainIDs,
            privateKey,
            name: format!("Wallet {}", &address[0..8]), // Default name using first 8 chars of address
        }
    }
}

impl Trades {
    pub fn new(
        userid: i64,
        address: String,
        isEVM: bool,
        chainID: i64,
        amount: String,
        active: bool,
        priceBought: f64,
        stoploss: u64,
        takeprofit: u64,
        trail: Option<trailset>,
    ) -> Trades {
        Trades {
            _id: None,
            userid,
            address,
            isEVM,
            chainID,
            amount,
            active,
            priceBought,
            stoploss,
            takeprofit,
            trail,
        }
    }
}

impl TradeSettings {
    pub fn new(userid: i64) -> TradeSettings {
        TradeSettings {
            userid,
            multiwallet: false,
            stoploss: 0.0,
            takeprofit: 0.0,
            trailing: false,
            trailing_stop_loss: 0.0,
            trailing_stop_loss_percentage: 0.0,
            trailing_take_profit: 0.0,
            trailing_take_profit_percentage: 0.0,
            mev_enabled_chains: Vec::new(), // Default to MEV disabled on all chains
        }
    }
}
