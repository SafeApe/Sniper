use bson::{doc, Document};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Trades {
    pub _id: Option<ObjectId>,
    pub address: String,
    pub isEVM: bool,
    pub chainID: i64,
    pub amount: String, // U256 is not directly supported, convert to string
    pub active: bool,
    pub priceBought: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Wallet {
    pub _id: Option<ObjectId>,
    pub address: String,
    pub chainIDs: Vec<i64>, // If empty, enabled for all chains
    pub privateKey: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub _id: Option<ObjectId>,
    pub userid: u64,
    pub stoploss: i32, // Stop Loss Percentage %
    pub multiWallet: bool,
    pub wallets: Vec<Wallet>,
    pub trades: Vec<Trades>,
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
        }
    }
}

impl From<Wallet> for Document {
    fn from(wallet: Wallet) -> Self {
        doc! {
            "_id": wallet._id,
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
            "stoploss": user.stoploss,
            "multiWallet": user.multiWallet,
            "wallets": user.wallets.into_iter().map(Document::from).collect::<Vec<Document>>(),
            "trades": user.trades.into_iter().map(Document::from).collect::<Vec<Document>>(),
        }
    }
}
impl From<Document> for Trades {
    fn from(doc: Document) -> Self {
        Trades {
            _id: Some(doc.get_object_id("_id").unwrap()),
            address: doc.get_str("address").unwrap().to_string(),
            isEVM: doc.get_bool("isEVM").unwrap(),
            chainID: doc.get_i64("chainID").unwrap(),
            amount: doc.get_str("amount").unwrap().to_string(),
            active: doc.get_bool("active").unwrap(),
            priceBought: doc.get_f64("priceBought").unwrap(),
        }
    }
}

impl From<Document> for Wallet {
    fn from(doc: Document) -> Self {
        Wallet {
            _id: Some(doc.get_object_id("_id").unwrap()),
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
            stoploss: doc.get_i32("stoploss").unwrap(),
            multiWallet: doc.get_bool("multiWallet").unwrap(),
            wallets: doc
                .get_array("wallets")
                .unwrap()
                .into_iter()
                .map(|x| x.as_document().unwrap().clone().into())
                .collect(),
            trades: doc
                .get_array("trades")
                .unwrap()
                .into_iter()
                .map(|x| x.as_document().unwrap().clone().into())
                .collect(),
        }
    }
}
