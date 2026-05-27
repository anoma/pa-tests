use alloy::primitives::Address;
use alloy::primitives::B256;
use alloy::providers::Provider;
use anoma_generic_call_forwarder_bindings::generated::generic_call_forwarder::GenericCallForwarder;
use anyhow::Context;

use crate::state::addresses::insert_generic_call_forwarder_v1_address;

#[inline]
pub fn generic_call_forwarder<P>(
    address: Address,
    provider: P,
) -> GenericCallForwarder::GenericCallForwarderInstance<P>
where
    P: Provider,
{
    GenericCallForwarder::GenericCallForwarderInstance::new(address, provider)
}

pub async fn deploy_generic_call_forwarder<P>(
    provider: P,
    protocol_adapter: Address,
    logic_ref: B256,
) -> anyhow::Result<Address>
where
    P: Provider + Clone,
{
    let deployed = GenericCallForwarder::deploy(provider, protocol_adapter, logic_ref)
        .await
        .context("failed to deploy GenericCallForwarder")?;

    Ok(*deployed.address())
}

pub async fn deploy_and_insert_generic_call_forwarder<P>(
    builder: &mut pa_test_harness_core::environment::StateBuilder,
    provider: P,
    protocol_adapter: Address,
    logic_ref: B256,
) -> anyhow::Result<Address>
where
    P: Provider + Clone,
{
    let address = deploy_generic_call_forwarder(provider, protocol_adapter, logic_ref).await?;

    insert_generic_call_forwarder_v1_address(builder, address);

    Ok(address)
}
