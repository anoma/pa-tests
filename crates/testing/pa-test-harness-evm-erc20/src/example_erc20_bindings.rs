use alloy::primitives::Address;
use alloy::primitives::U256;
use alloy::providers::Provider;
use alloy::sol;

use crate::addresses::insert_erc20_address;

sol!(
    #[allow(missing_docs)]
    #[derive(Debug)]
    #[sol(rpc)]
    ERC20Example,
    "artifacts/ERC20Example.json"
);

#[inline]
pub fn erc20_example<P>(address: Address, provider: P) -> ERC20Example::ERC20ExampleInstance<P>
where
    P: Provider,
{
    ERC20Example::ERC20ExampleInstance::new(address, provider)
}

pub async fn deploy_example_erc20<P>(provider: P) -> anyhow::Result<Address>
where
    P: Provider,
{
    let deployed = ERC20Example::deploy(provider).await?;

    Ok(*deployed.address())
}

pub async fn deploy_and_mint_example_erc20<P>(
    provider: P,
    mint_to: Address,
    amount: U256,
) -> anyhow::Result<Address>
where
    P: Provider + Clone,
{
    let deployed = ERC20Example::deploy(provider.clone()).await?;

    deployed
        .mint(mint_to, amount)
        .send()
        .await?
        .get_receipt()
        .await?;

    Ok(*deployed.address())
}

pub async fn deploy_and_insert_example_erc20<P>(
    builder: &mut pa_test_harness_core::environment::StateBuilder,
    symbol: &str,
    provider: P,
    mint_to: Address,
    amount: U256,
) -> anyhow::Result<Address>
where
    P: Provider + Clone,
{
    let address = deploy_and_mint_example_erc20(provider, mint_to, amount).await?;

    insert_erc20_address(builder, symbol, address);

    Ok(address)
}
