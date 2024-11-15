use crate::utils::{create_wallet, create_wallet_pk};
use bson::{doc, Document};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct tradeset {
    pub trailingstoploss: f32, //If 0 turned off else the percentage to trail
    pub trailingstoplosspercentage: f32, //We sell this percentage after hitting trailingstoploss percent of the price and decrease the price again to trail
    pub trailingtakeprofit: f32,         //If 0 turned off else the percentage to trail
    pub trailingprofitpercentage: f32, //We sell this percentage after hitting trailingtakeprofit percent of the price and increase the price again to trail
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Trades {
    pub _id: Option<ObjectId>,
    pub userid: u64,
    pub address: String,
    pub isEVM: bool,
    pub chainID: i64,
    pub amount: String, // U256 is not directly supported, convert to string
    pub active: bool,
    pub priceBought: f64,
    pub stoploss: u64,   //If this is 0, stoploss is turned off
    pub takeprofit: u64, //If this is 0, takeprofit is turned off
    pub trail: Option<tradeset>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Wallet {
    pub _id: Option<ObjectId>,
    pub address: String,
    pub chainIDs: Vec<i64>, // If empty, enabled for all chains
    pub privateKey: String,
    pub userid: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub _id: Option<ObjectId>,
    pub userid: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TradeSettings {
    // These are the settings for default trades
    userid: u64,
    multiwallet: bool,
    stoploss: f32,
    takeprofit: f32,
    trailing: bool,        //If this is true, the following trailing settings are used
    trailingstoploss: f32, //If 0 turned off else the percentage to trail
    trailingstoplosspercentage: f32, //We sell this percentage after hitting trailingstoploss percent of the price and decrease the price again to trail
    trailingtakeprofit: f32,         //If 0 turned off else the percentage to trail
    trailingprofitpercentage: f32, //We sell this percentage after hitting trailingtakeprofit percent of the price and increase the price again to trail
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CopyTrade {
    //To copy trades from another user can be added as feature later
    userid: u64,
    active: bool,
    address: String,
    chains: Vec<i64>,
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
                    "trailingstoploss": trail.trailingstoploss as f64,
                    "trailingstoplosspercentage": trail.trailingstoplosspercentage as f64,
                    "trailingtakeprofit": trail.trailingtakeprofit as f64,
                    "trailingprofitpercentage": trail.trailingprofitpercentage as f64,
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
            userid: doc.get_i64("userid").unwrap() as u64,
            address: doc.get_str("address").unwrap().to_string(),
            isEVM: doc.get_bool("isEVM").unwrap(),
            chainID: doc.get_i64("chainID").unwrap(),
            amount: doc.get_str("amount").unwrap().to_string(),
            active: doc.get_bool("active").unwrap(),
            priceBought: doc.get_f64("priceBought").unwrap(),
            stoploss: doc.get_i64("stoploss").unwrap() as u64,
            takeprofit: doc.get_i64("takeprofit").unwrap() as u64,
            trail: match doc.get_document("trail") {
                Ok(trail) => Some(tradeset {
                    trailingstoploss: trail.get_f64("trailingstoploss").unwrap() as f32,
                    trailingstoplosspercentage: trail.get_f64("trailingstoplosspercentage").unwrap()
                        as f32,
                    trailingtakeprofit: trail.get_f64("trailingtakeprofit").unwrap() as f32,
                    trailingprofitpercentage: trail.get_f64("trailingprofitpercentage").unwrap()
                        as f32,
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
            userid: doc.get_i64("userid").unwrap() as u64,
            address: doc.get_str("address").unwrap().to_string(),
            chainIDs: doc
                .get_array("chainIDs")
                .unwrap()
                .into_iter()
                .map(|x| x.as_i64().unwrap())
                .collect(),
            privateKey: doc.get_str("privateKey").unwrap().to_string(),
        }
    }
}

impl From<Document> for User {
    fn from(doc: Document) -> Self {
        User {
            _id: Some(doc.get_object_id("_id").unwrap()),
            userid: doc.get_i64("userid").unwrap() as u64,
        }
    }
}

impl User {
    pub fn default(userid: u64) -> User {
        User { _id: None, userid }
    }
    pub fn new(userid: u64) -> User {
        User { _id: None, userid }
    }
}

impl Wallet {
    pub fn new(userid: u64) -> Wallet {
        let (privateKey, address) = create_wallet();
        Wallet {
            _id: None,
            userid,
            address: address.to_string(),
            chainIDs: vec![],
            privateKey,
        }
    }
    pub fn default(userid: u64, address: String, chainIDs: Vec<i64>, privateKey: String) -> Wallet {
        Wallet {
            _id: None,
            userid,
            address,
            chainIDs,
            privateKey,
        }
    }
}

impl Trades {
    pub fn new(
        userid: u64,
        address: String,
        isEVM: bool,
        chainID: i64,
        amount: String,
        active: bool,
        priceBought: f64,
        stoploss: u64,
        takeprofit: u64,
        trail: Option<tradeset>,
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
