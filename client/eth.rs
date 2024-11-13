use alloy::{
    primitives::Address,
    providers::{fillers::FillProvider, Provider, ProviderBuilder, WsConnect},
    signers::local::PrivateKeySigner,
    Network,
};
use eyre::Result;
use std::sync::Arc;

// Define the Web3Client struct with a generic Provider
pub struct Web3Client<P>
where
    P: FillProvider,
{
    provider: Arc<P>,
    wallet_address: Address,
    with_signer: bool,
}

impl<P> Web3Client<P>
where
    P: Provider,
{
    pub async fn new(provider: P, wallet_address: Address, with_signer: bool) -> Result<Self> {
        Ok(Self {
            provider: Arc::new(provider),
            wallet_address,
            with_signer,
        })
    }

    // Helper method to create a client with recommended fillers
    pub async fn with_recommended_fillers(
        url: &str,
        wallet_address: Address,
        with_signer: bool,
    ) -> Result<Self> {
        let ws = WsConnect::new(url);
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .on_ws(ws)
            .await?;

        Self::new(provider, wallet_address, with_signer).await
    }

    // Example method using the provider
    pub async fn get_balance(&self, address: Address) -> Result<U256> {
        let balance = self.provider.get_balance(address, None).await?;
        Ok(balance)
    }

    // Add buy method
    pub async fn buy(&self, token_address: Address, amount: U256) -> Result<()> {
        let tx = self
            .provider
            .transaction_builder()
            .to(token_address)
            .value(amount)
            .from(self.wallet_address)
            .build()?;

        if self.with_signer {
            // Handle signed transaction
            // You'll need to add signer logic here
        } else {
            let pending_tx = self.provider.send_transaction(tx).await?;
            let receipt = pending_tx.await?;
            println!(
                "Buy transaction confirmed in block: {}",
                receipt.block_number
            );
        }

        Ok(())
    }

    // Add sell method
    pub async fn sell(&self, token_address: Address, amount: U256) -> Result<()> {
        let tx = self
            .provider
            .transaction_builder()
            .to(token_address)
            .value(amount)
            .from(self.wallet_address)
            .build()?;

        if self.with_signer {
            // Handle signed transaction
            // You'll need to add signer logic here
        } else {
            let pending_tx = self.provider.send_transaction(tx).await?;
            let receipt = pending_tx.await?;
            println!(
                "Sell transaction confirmed in block: {}",
                receipt.block_number
            );
        }

        Ok(())
    }
}

// Example usage
#[tokio::test]
async fn test_web3_client() -> Result<()> {
    let url = "ws://localhost:8545";
    let wallet_address = Address::zero(); // Replace with actual address

    // Create client with recommended fillers
    let client = Web3Client::with_recommended_fillers(url, wallet_address, false).await?;

    // Or create with custom provider
    let custom_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .on_ws(url)
        .await?;

    let client = Web3Client::new(custom_provider, wallet_address, false).await?;

    Ok(())
}
