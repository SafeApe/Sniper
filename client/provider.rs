use crate::client::abi;
use crate::config::getConfig;
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
use eyre::Result;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone)]
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
}

impl NetProvider {
    pub async fn getSniperSwapper() {}
}

#[derive(Debug)]
pub struct TradingClient {
    EVMproviders: HashMap<u64, NetProvider>, //provider saved by Chain IDs
                                             // To add SolanaProvider Later
}

impl TradingClient {
    pub async fn initialize(ws_url: &str) -> Result<Self> {
        // Create WS connection
        let networks = getConfig().networks;
        let mut EVMproviders = HashMap::new();
        for (name, network) in &networks {
            let ws = WsConnect::new(ws_url);
            let provider = ProviderBuilder::new()
                .with_recommended_fillers() // Adds ChainIdFiller, GasFiller, and NonceFiller
                .on_ws(ws)
                .await?;

            let provider = Arc::new(provider);
            let sniperSwapper = abi::SniperSwapper::new(
                crate::utils::convertToAddress(&network.sniperca),
                provider.clone(),
            );
            let net_provider = NetProvider {
                provider,
                sniperSwapper,
            };
            EVMproviders.insert(network.chain_id, net_provider);
            println!("Network: {} initialized", name);
        }
        // Initialize signer with private key
        Ok(Self { EVMproviders })
    }

    pub async fn buy(
        &self,
        pair: Address,
        tokenIn: Address,
        amount: U256,
        wallets: Vec<String>,
        chain_id: u64,
    ) {
        println!("Buying");
        let provider = self.EVMproviders.get(&chain_id).unwrap();
        let tx = provider
            .sniperSwapper
            .swap(pair, tokenIn, amount)
            .into_transaction_request();
        println!("{:?}", tx);
        // let txs = tx
        //     .into_batch(wallets.iter().map(|wallet| {
        //         let signer = PrivateKeySigner::new(wallet);
        //         provider.provider.sign_transaction(tx.clone(), signer)
        //     }))
        //     .await
        //     .unwrap();
        // let signed_tx = provider.provider.sign_transaction(tx).await.unwrap();
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
}
