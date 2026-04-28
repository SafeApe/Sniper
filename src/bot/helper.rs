use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::collections::HashMap;
use regex::Regex;
use crate::getConfig;
use alloy::primitives::Address;
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub struct PairData {
    pub chain: String,
    pub exchange: String,
    pub pair: String,
    pub base: String,
    pub quote: String,
    pub created_at: i64,
    pub version: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct DexScreenerResponse {
    pairs: Vec<DexScreenerPair>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DexScreenerPair {
    chainId: String,
    dexId: String,
    pairAddress: String,
    baseToken: Token,
    quoteToken: Token,
    pairCreatedAt: Option<i64>,
    labels: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Token {
    address: String,
}

pub struct PairHandler {
    dexscreener_api: String,
    dexguru_api: String,
    dextools_api: String,
    client: Client,
}

impl PairHandler {
    pub fn new() -> Self {
        Self {
            dexscreener_api: "https://api.dexscreener.com/latest/dex/search/?q=".to_string(),
            dexguru_api: "https://api.dex.guru/v3/tokens/search/".to_string(),
            dextools_api: "https://www.dextools.io/shared/search/pair?query=".to_string(),
            client: Client::new(),
        }
    }

    pub async fn dexscreener_parse(&self, token_or_name: &str) -> Result<Vec<PairData>, Box<dyn std::error::Error>> {
        let config = getConfig();
        let url = format!("{}{}", self.dexscreener_api, token_or_name);
        let response = self.client.get(&url).send().await?;
        let data: DexScreenerResponse = response.json().await?;
        
        let mut pairs = Vec::new();
        for pair in data.pairs {
            pairs.push(PairData {
                chain: pair.chainId,
                exchange: pair.dexId,
                pair: pair.pairAddress,
                base: pair.baseToken.address,
                quote: pair.quoteToken.address,
                created_at: pair.pairCreatedAt.unwrap_or_else(|| {
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64
                }),
                // if v3 in labels version = 3 else 4
                version: pair.labels.as_ref().map_or(4, |labels| if labels.contains(&"v3".to_string()) { 3 } else { 4 }),
            });
        }
        Ok(pairs)
    }

    pub async fn search(&self, token_or_name: &str) -> Result<Vec<PairData>, Box<dyn std::error::Error>> {
        self.dexscreener_parse(token_or_name).await
    }
}

pub struct SignatureFinder {
    bytes4_api: String,
    openchain_api: String,
    etherface_api: String,
    client: Client,
}

impl SignatureFinder {
    pub fn new() -> Self {
        Self {
            bytes4_api: "https://raw.githubusercontent.com/ethereum-lists/4bytes/master/signatures/".to_string(),
            openchain_api: "https://api.openchain.xyz/signature-database/v1/lookup?filter=false&function=".to_string(),
            etherface_api: "https://api.etherface.io/v1/signatures/hash/all/".to_string(),
            client: Client::new(),
        }
    }

    pub async fn get_hash_from_openchain(&self, hashes: &[String]) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut hash_url = self.openchain_api.clone();
        for hash in hashes {
            if hash.replace("0x", "").len() == 8 {
                hash_url.push_str(&format!("{},", hash));
            }
        }
        hash_url.pop(); // Remove last comma

        let response = self.client.get(&hash_url).send().await?;
        let result: serde_json::Value = response.json().await?;
        
        let mut hash_name = HashMap::new();
        if let Some(functions) = result["result"]["function"].as_object() {
            for (hash, signatures) in functions {
                if let Some(signatures_array) = signatures.as_array() {
                    if !signatures_array.is_empty() {
                        if let Some(name) = signatures_array[0]["name"].as_str() {
                            hash_name.insert(hash.to_string(), name.to_string());
                        }
                    }
                }
            }
        }
        
        Ok(hash_name)
    }
}

// Helper functions
pub fn get_eth_addresses(text: &str) -> Vec<String> {
    let eth_pattern = Regex::new(r"^0x[a-fA-F0-9]{40}$").unwrap();
    eth_pattern
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .collect()
}

pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

pub fn get_token_url(token: &str, network: &str) -> String {
    let config = getConfig();
    let explorer = &config.networks[network].explorer;
    format!("{}/token/{}", explorer, token)
}