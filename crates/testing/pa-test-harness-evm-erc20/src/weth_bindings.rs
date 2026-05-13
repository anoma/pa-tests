use alloy::primitives::Address;
use alloy::primitives::U256;
use alloy::providers::Provider;
use alloy::sol;

use crate::state::addresses::insert_erc20_address;

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

pub async fn deploy_weth<P>(provider: P) -> anyhow::Result<Address>
where
    P: Provider,
{
    let deployed = WETH9::deploy(provider).await?;

    Ok(*deployed.address())
}

pub async fn deploy_and_mint_weth<P>(
    provider: P,
    mint_to: Address,
    amount: U256,
) -> anyhow::Result<Address>
where
    P: Provider + Clone,
{
    let deployed = WETH9::deploy(provider.clone()).await?;

    deployed
        .deposit()
        .value(amount)
        .send()
        .await?
        .get_receipt()
        .await?;

    deployed
        .transfer(mint_to, amount)
        .send()
        .await?
        .get_receipt()
        .await?;

    Ok(*deployed.address())
}

pub async fn deploy_and_insert_weth<P>(
    builder: &mut pa_test_harness_core::environment::StateBuilder,
    symbol: &str,
    provider: P,
    mint_to: Address,
    amount: U256,
) -> anyhow::Result<Address>
where
    P: Provider + Clone,
{
    let address = deploy_and_mint_weth(provider, mint_to, amount).await?;

    insert_erc20_address(builder, symbol, address);

    Ok(address)
}
