use alloy::primitives::Address;
use alloy::providers::Provider;
use alloy::sol;

sol!(
    #[allow(missing_docs)]
    #[derive(Debug)]
    #[sol(rpc)]
    WETH9,
    "artifacts/WETH9.json"
);

#[inline]
pub fn weth9<P>(address: Address, provider: P) -> WETH9::WETH9Instance<P>
where
    P: Provider,
{
    WETH9::WETH9Instance::new(address, provider)
}
