pub mod addresses;
pub mod keys;

#[cfg(feature = "example-erc20-bindings")]
pub mod example_erc20_bindings;

#[cfg(feature = "ierc20-bindings")]
pub mod ierc20_bindings;

#[cfg(feature = "weth-bindings")]
pub mod weth_bindings;
