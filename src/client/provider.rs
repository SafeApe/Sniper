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
    EIP1559: bool,
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
    pub async fn initialize() -> Result<Self> {
        // Create WS connection
        let networks = getConfig().networks;
        let mut EVMproviders = HashMap::new();
        for (name, network) in &networks {
            let ws = WsConnect::new(&network.rpc);
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
                EIP1559: network.eip1599,
            };
            EVMproviders.insert(network.chain_id, net_provider);
            println!("Network: {} initialized", name);
        }
        println!("EVM {:?}", EVMproviders);
        // Initialize signer with private key
        Ok(Self { EVMproviders })
    }
    pub async fn get_fixed_transaction_request(
        &self,
        mut tx: TransactionRequest,
        wallet: EthereumWallet,
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
        amount: U256,
        wallets: Vec<String>,
        chain_id: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let provider = self.EVMproviders.get(&chain_id).unwrap();
        let tx = provider
            .sniperSwapper
            .swap(pair, tokenIn, amount)
            .into_transaction_request();

        if wallets.len() == 0 {
            println!("No wallets provided");
            return Err("No wallets provided".into());
        }
        let (wallet1, address1) = wallet_from_pk(wallets[0].as_str());
        let stx = self
            .get_fixed_transaction_request(
                tx.clone(),
                wallet1.clone(),
                address1.clone(),
                chain_id.clone(),
                provider,
                amount.clone(),
            )
            .await?;
        let tx_built = stx.build(&wallet1).await?;
        let resp = provider
            .provider
            .send_tx_envelope(tx_built)
            .await?
            .get_receipt()
            .await?;
        println!("Receipt: {:?}", resp);
        // println!("Wallet Address: {:?}",wallet1.);

        // let txs = tx
        //     .into_batch(wallets.iter().map(|wallet| {
        //         let signer = PrivateKeySigner::new(wallet);
        //         provider.provider.sign_transaction(tx.clone(), signer)
        //     }))
        //     .await
        //     .unwrap();
        // let signed_tx = provider.provider.sign_transaction(tx).await.unwrap();
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
}
