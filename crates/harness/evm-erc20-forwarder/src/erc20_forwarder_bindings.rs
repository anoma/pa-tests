use alloy::primitives::Address;
use alloy::primitives::B256;
use alloy::providers::Provider;
use anomapay_erc20_forwarder_bindings::generated::erc20_forwarder::ERC20Forwarder;
use anyhow::Context;

use crate::state::addresses::insert_erc20_forwarder_v1_address;

#[inline]
pub fn erc20_forwarder<P>(
    address: Address,
    provider: P,
) -> ERC20Forwarder::ERC20ForwarderInstance<P>
where
    P: Provider,
{
    ERC20Forwarder::ERC20ForwarderInstance::new(address, provider)
}

pub async fn deploy_erc20_forwarder<P>(
    provider: P,
    protocol_adapter: Address,
    logic_ref: B256,
    guardian: Address,
) -> anyhow::Result<Address>
where
    P: Provider + Clone,
{
    let deployed = ERC20Forwarder::deploy(provider, protocol_adapter, logic_ref, guardian)
        .await
        .context("failed to deploy ERC20Forwarder")?;

    Ok(*deployed.address())
}

pub async fn deploy_and_insert_erc20_forwarder<P>(
    builder: &mut pa_test_harness_core::environment::StateBuilder,
    provider: P,
    protocol_adapter: Address,
    logic_ref: B256,
    guardian: Address,
) -> anyhow::Result<Address>
where
    P: Provider + Clone,
{
    let address = deploy_erc20_forwarder(provider, protocol_adapter, logic_ref, guardian).await?;

    insert_erc20_forwarder_v1_address(builder, address);

    Ok(address)
}
