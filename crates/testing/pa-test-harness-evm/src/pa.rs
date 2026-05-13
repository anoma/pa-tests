use alloy::primitives::Address;
use alloy::primitives::FixedBytes;
use alloy::providers::Provider;
use anoma_pa_evm_bindings::generated::protocol_adapter::ProtocolAdapter;

use crate::state::pa::insert_pa_address;

#[inline]
pub fn protocol_adapter<P>(
    address: Address,
    provider: P,
) -> ProtocolAdapter::ProtocolAdapterInstance<P>
where
    P: Provider,
{
    ProtocolAdapter::ProtocolAdapterInstance::new(address, provider)
}

pub async fn deploy_protocol_adapter<P>(
    provider: P,
    verifier_router: Address,
    verifier_selector: FixedBytes<4>,
    fee_recipient: Address,
) -> anyhow::Result<Address>
where
    P: Provider,
{
    let deployed =
        ProtocolAdapter::deploy(provider, verifier_router, verifier_selector, fee_recipient)
            .await?;

    Ok(*deployed.address())
}

pub async fn deploy_and_insert_protocol_adapter<P>(
    builder: &mut pa_test_harness_core::environment::StateBuilder,
    provider: P,
    verifier_router: Address,
    verifier_selector: FixedBytes<4>,
    fee_recipient: Address,
) -> anyhow::Result<Address>
where
    P: Provider,
{
    let address =
        deploy_protocol_adapter(provider, verifier_router, verifier_selector, fee_recipient)
            .await?;

    insert_pa_address(builder, address);

    Ok(address)
}
