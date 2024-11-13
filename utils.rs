use alloy::primitives::{address, utils, Address, Uint, U256};

pub fn convertToAddress<T>(address: T) -> Address
where
    T: Into<String>,
{
    let straddress = address.into();
    let address = Address::parse_checksummed(&straddress, None).unwrap();
    return address;
}
