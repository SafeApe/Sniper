use std::str::FromStr;

use alloy::{
    network::{EthereumWallet, NetworkWallet},
    primitives::{address, utils, Address, Uint, U256},
    signers::local::{LocalSigner, PrivateKeySigner},
};
use k256::ecdsa::SigningKey;

pub fn convertToAddress<T>(address: T) -> Address
where
    T: Into<String>,
{
    let straddress = address.into();
    let address = Address::parse_checksummed(&straddress, None).unwrap();
    return address;
}

use rand::rngs::OsRng;
use rand::RngCore;
use secp256k1::SecretKey;

pub fn create_wallet_pk() -> String {
    // Create a random 32-byte array for the secret key
    let mut rng = OsRng;
    let mut secret_key_bytes = [0u8; 32];
    rng.fill_bytes(&mut secret_key_bytes);

    // Create the secret key from the 32-byte array
    let secret_key =
        SecretKey::from_slice(&secret_key_bytes).expect("32 bytes, within curve order");

    // Convert the secret key to a hexadecimal string
    let pk = hex::encode(secret_key.secret_bytes());
    let pkSigner = PrivateKeySigner::from_str(&pk).unwrap();
    pk
}

pub fn create_wallet() -> (String, Address) {
    let pk = create_wallet_pk();
    let pkSigner = PrivateKeySigner::from_str(&pk).unwrap();
    let address = pkSigner.address();
    (pk, address)
}

pub fn wallet_from_pk(pk: &str) -> (EthereumWallet, Address) {
    let pkSigner = PrivateKeySigner::from_str(pk).unwrap();
    let address = pkSigner.address();
    (EthereumWallet::from(pkSigner), address)
}
