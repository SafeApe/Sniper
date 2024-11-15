use alloy::{primitives::address, providers::ProviderBuilder, sol};
//SniperSwapper
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    SniperSwapper,
    "assets/abis/sniper-abi.json"
);

// ERC20
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    ERC20,
    "assets/abis/erc20.json"
);
