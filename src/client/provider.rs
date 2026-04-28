use crate::config::getConfig;
use crate::{client::abi, utils::wallet_from_pk};
use alloy::network::{EthereumWallet, NetworkWallet, TransactionBuilder};
use alloy::rpc::types::TransactionRequest;
use alloy::{
    network::Ethereum,
    primitives::{Address, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, Provider, ProviderBuilder, RootProvider, WsConnect,
    },
    signers::local::PrivateKeySigner,
};
use alloy_pubsub::PubSubFrontend;
use loom::broadcast::flashbots::{self, Flashbots, FlashbotsClient};
use crate::models::trailset;
use eyre::Result;
use std::ops::Deref;
use std::{collections::{HashMap, HashSet}, sync::Arc,fmt};
use tokio::sync::RwLock;

#[derive()]
pub struct NetProvider {
    pub provider: Arc<
        FillProvider<
            JoinFill<
                Identity,
                JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
            >,
            RootProvider<PubSubFrontend>,
            PubSubFrontend,
            Ethereum,
        >,
    >,
    sniperSwapper: abi::SniperSwapper::SniperSwapperInstance<
        PubSubFrontend,
        Arc<
            FillProvider<
                JoinFill<
                    Identity,
                    JoinFill<
                        GasFiller,
                        JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>,
                    >,
                >,
                RootProvider<PubSubFrontend>,
                PubSubFrontend,
                Ethereum,
            >,
        >,
    >,
    EIP1559: bool,
    flashbots: Option<Arc<Flashbots<Arc<FillProvider<JoinFill<Identity, JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>>, RootProvider<PubSubFrontend>, PubSubFrontend, Ethereum>>, PubSubFrontend>>>
}

impl NetProvider {
    pub async fn getSniperSwapper() {}
}

impl fmt::Debug for NetProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NetProvider")
            .field("provider", &self.provider)
            .field("sniperSwapper", &self.sniperSwapper)
            .field("EIP1559", &self.EIP1559)
            .finish()
    }
}
impl std::clone::Clone for NetProvider {
    fn clone(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            sniperSwapper: self.sniperSwapper.clone(),
            EIP1559: self.EIP1559,
            flashbots: self.flashbots.clone()
        }
    }
}

#[derive(Debug, Clone)]
struct ActiveTrade {
    pair: Address,
    token_in: Address,
    token_out: Address,
    amount: U256,
    entry_price: U256,
    stop_loss: u64,
    take_profit: u64,
    trail: Option<trailset>,
    walletPk: Vec<String>,
    chain_id: u64,
}

#[derive(Debug)]
pub struct TradeMonitor {
    // Organized by token_out address for efficient price monitoring
    trades_by_token: HashMap<Address, Vec<ActiveTrade>>,
    // Secondary index by chain_id for efficient chain-specific operations
    active_tokens: HashMap<u64, HashSet<Address>>,
}

impl TradeMonitor {
    fn new() -> Self {
        Self {
            trades_by_token: HashMap::new(),
            active_tokens: HashMap::new(),
        }
    }

    fn add_trade(&mut self, trade: ActiveTrade) {
        // Add to token_out index
        self.trades_by_token
            .entry(trade.token_out)
            .or_default()
            .push(trade.clone());

        // Update chain-specific active tokens
        self.active_tokens
            .entry(trade.chain_id)
            .or_default()
            .insert(trade.token_out);
    }

    fn get_trades_for_token(&self, token: &Address) -> Option<&Vec<ActiveTrade>> {
        self.trades_by_token.get(token)
    }

    fn get_active_tokens_for_chain(&self, chain_id: u64) -> Option<&HashSet<Address>> {
        self.active_tokens.get(&chain_id)
    }
}

#[derive(Debug)]
pub struct TradingClient {
    EVMproviders: HashMap<u64, NetProvider>,
    active_trades: Arc<RwLock<TradeMonitor>>,
}

impl TradingClient {
    pub async fn new() -> Result<Self> {
        TradingClient::initialize().await
    }

    pub async fn initialize() -> Result<Self> {
        // Create WS connection
        let networks = getConfig().networks;
        let mut EVMproviders = HashMap::new();
        for (name, network) in &networks {
            let bundle_signer = PrivateKeySigner::random();
            let ws = WsConnect::new(&network.ws_rpc);
            let provider = ProviderBuilder::new()
                .with_recommended_fillers() // Adds ChainIdFiller, GasFiller, and NonceFiller
                .on_ws(ws)
                .await?;
            let provider = Arc::new(provider);
            let tx_signer = EthereumWallet::new(bundle_signer.clone());
            let sniperSwapper = abi::SniperSwapper::new(
                crate::utils::convertToAddress(&network.sniperca),
                provider.clone(),
            );
            let flashbots_url: &str = network.flashbots_url.as_ref();
            let flashbots = if flashbots_url.is_empty() {
                None
            } else {
                let mut ff = Flashbots::new(provider.clone(), flashbots_url, None);
                if (network.chain_id == 1){
                    ff = ff.with_default_relays();
                }
                Some(Arc::new(ff))
            };

            let net_provider = NetProvider {
                provider,
                sniperSwapper,
                EIP1559: network.eip1599,
                flashbots: flashbots
            };
            EVMproviders.insert(network.chain_id, net_provider);
            println!("Network: {} initialized", name);
        }
        println!("EVM {:?}", EVMproviders);
        // Initialize signer with private key
        Ok(Self { 
            EVMproviders:EVMproviders,
            active_trades: Arc::new(RwLock::new(TradeMonitor::new())),
        })
    }
    pub async fn get_fixed_transaction_request(
        mut tx: TransactionRequest,
        _wallet: EthereumWallet,
        address: Address,
        chain_id: u64,
        provider: &NetProvider,
        amount: U256,
    ) -> Result<TransactionRequest> {
        // In the scenario the chain is an EIP1559 chain, also handle normal legacy transactions if the chain is not EIP1559
        let gas_price = provider.provider.get_gas_price().await?;
        let max_priority_fee_per_gas = provider
            .provider
            .get_max_priority_fee_per_gas()
            .await
            .unwrap_or(gas_price * 2)
            * 2;
        tx.set_from(address);
        tx.set_max_fee_per_gas((gas_price * 120) / 100); // 20% more than gas price for now
        tx.set_max_priority_fee_per_gas(max_priority_fee_per_gas);
        tx.set_chain_id(chain_id);
        tx.set_value(amount);
        tx.set_nonce(provider.provider.get_transaction_count(address).await?);
        let gas_limit = provider.provider.estimate_gas(&tx).await?;
        tx.set_gas_limit(gas_limit);
        Ok(tx)
    }

    pub async fn buy(
        &self,
        pair: Address,
        tokenIn: Address,
        tokenOut: Address,
        amount: U256,
        wallets: Vec<String>,
        chain_id: u64,
        stoploss: u64,
        takeprofit: u64,
        trail: Option<trailset>,
        mev: Option<bool>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bundle_signer = PrivateKeySigner::random();
        let tx_signer = EthereumWallet::new(bundle_signer.clone()); //MEV
        let provider = self.EVMproviders.get(&chain_id).unwrap().clone();  // Clone the provider
        let tx = provider
                .sniperSwapper
                .swap(pair, tokenIn, amount)
                .into_transaction_request();

        if wallets.is_empty() {
            println!("No wallets provided");
            return Err("No wallets provided".into());
        }

        // Create a vector to store all transaction handles
        let mut tx_handles = Vec::new();

        // Process each wallet concurrently
        for wallet_pk in wallets.iter() {
            let tx = tx.clone();
            let provider = provider.clone();
            let amount = amount.clone();
            let wallet_pk = wallet_pk.clone();

            // Spawn a new task for each wallet
            let handle = tokio::spawn(async move {
                let (wallet, address) = wallet_from_pk(&wallet_pk);
                let stx = TradingClient::get_fixed_transaction_request(
                    tx,
                    wallet.clone(),
                    address,
                    chain_id,
                    &provider,
                    amount,
                )
                .await?;
                
                let tx_built = stx.build(&wallet).await?;
                let resp = provider
                    .provider
                    .send_tx_envelope(tx_built)
                    .await?
                    .get_receipt()
                    .await?;
                
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(resp)
            });

            tx_handles.push(handle);
        }

        // Wait for all transactions to complete and collect responses
        let mut responses = Vec::new();
        for handle in tx_handles {
            match handle.await {
                Ok(result) => {
                    match result {
                        Ok(receipt) => {
                            println!("Transaction successful: {:?}", receipt);
                            responses.push(receipt);
                        }
                        Err(e) => println!("Transaction failed: {:?}", e),
                    }
                }
                Err(e) => println!("Task failed: {:?}", e),
            }
        }

        // Add to active trades if at least one transaction was successful
        if !responses.is_empty() {
            let new_trade = ActiveTrade {
                pair,
                token_in: tokenIn,
                token_out: tokenOut,
                amount,
                entry_price: U256::from(0), // You'll need to get the actual entry price
                stop_loss: stoploss,
                take_profit: takeprofit,
                trail,
                walletPk: wallets,
                chain_id,
            };

            let active_trades = self.active_trades.clone();
            let mut trades = active_trades.write().await;
            trades.add_trade(new_trade);
            drop(trades);
            drop(active_trades);
        }

        Ok(())
    }

    // pub async fn buy(&self, token_address: Address, amount: U256) -> Result<()> {
    //     let tx = self
    //         .provider
    //         .transaction_builder()
    //         .to(token_address)
    //         .value(amount)
    //         .from(self.wallet_address)
    //         .build()?;

    //     let signed_tx = self.signer.sign_transaction(&tx).await?;
    //     let pending_tx = self.provider.send_transaction(signed_tx).await?;

    //     let receipt = pending_tx.await?;
    //     println!(
    //         "Buy transaction confirmed in block: {}",
    //         receipt.block_number
    //     );

    //     Ok(())
    // }

    // pub async fn sell(&self, token_address: Address, amount: U256) -> Result<()> {
    //     let tx = self
    //         .provider
    //         .transaction_builder()
    //         .to(token_address)
    //         .value(amount)
    //         .from(self.wallet_address)
    //         .build()?;

    //     let signed_tx = self.signer.sign_transaction(&tx).await?;
    //     let pending_tx = self.provider.send_transaction(signed_tx).await?;

    //     let receipt = pending_tx.await?;
    //     println!(
    //         "Sell transaction confirmed in block: {}",
    //         receipt.block_number
    //     );

    //     Ok(())
    // }

    // Add this method to monitor prices and execute trades
    pub async fn monitor_prices(&self) -> Result<()> {
        loop {
            for (chain_id, provider) in &self.EVMproviders {
                let trades = self.active_trades.read().await;
                if let Some(active_tokens) = trades.get_active_tokens_for_chain(*chain_id) {
                    for token in active_tokens {
                        if let Some(token_trades) = trades.get_trades_for_token(token) {
                            for trade in token_trades {
                                // Get current price from mempool or other source
                                // Check if price meets take_profit or stop_loss conditions
                                // Execute sell if conditions are met
                            }
                        }
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
}
