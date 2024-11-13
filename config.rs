use std::collections::HashMap;

use config::Config;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Network {
    pub name: String,
    pub rpc: String,
    pub chain_id: u64,
    pub sniperca: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct SConfig {
    pub bot_token: String,
    pub networks: HashMap<String, Network>,
    pub ipc_service_name: String,
    database: String,
    db_uri: String,
}

impl SConfig {
    pub fn getBotToken(&self) -> String {
        self.bot_token.clone()
    }
    pub fn getDataBase(&self) -> String {
        self.database.clone()
    }
    pub fn getDataURI(&self) -> String {
        self.db_uri.clone()
    }
    pub fn getNetwork(&self, network: &str) -> &Network {
        self.networks.get(network).unwrap().clone()
    }
    pub fn getNetworkByChainId(&self, chain_id: u64) -> &Network {
        for (_, network) in &self.networks {
            if network.chain_id == chain_id {
                return network.clone();
            }
        }
        panic!("Network not found")
    }
}

pub fn getConfig() -> SConfig {
    Config::builder()
        .add_source(config::File::with_name("config"))
        .build()
        .unwrap()
        .try_deserialize::<SConfig>()
        .unwrap()
}
