use alloy::primitives::Address;
use alloy::providers::Provider;
use alloy::sol;

sol!(
    #[allow(missing_docs)]
    #[derive(Debug)]
    #[sol(rpc)]
    IERC20,
    "artifacts/IERC20.json"
);

#[inline]
pub fn ierc20<P>(address: Address, provider: P) -> IERC20::IERC20Instance<P>
where
    P: Provider,
{
    IERC20::IERC20Instance::new(address, provider)
}
