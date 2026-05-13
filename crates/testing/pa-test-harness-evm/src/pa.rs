use alloy::primitives::Address;
use alloy::providers::Provider;
use anoma_pa_evm_bindings::generated::protocol_adapter::ProtocolAdapter;

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

#[cfg(feature = "mock-risc0-bindings")]
pub async fn deploy_protocol_adapter(
    default_signer: &alloy::providers::DynProvider,
) -> anyhow::Result<Address> {
    use alloy::primitives::FixedBytes;
    use anyhow::Context;

    use crate::mock_risc0_bindings::MOCK_VERIFIER_SELECTOR;
    use crate::mock_risc0_bindings::deploy_mock_risc0_stack;

    let fee_recipient = default_signer
        .get_accounts()
        .await
        .context("failed to retrieve signer accounts")?
        .into_iter()
        .next()
        .context("failed to retrieve default signer account")?;

    let mock_risc0 = deploy_mock_risc0_stack(default_signer, fee_recipient)
        .await
        .context("failed to deploy mock Risc0 verifier stack")?;
    let selector = FixedBytes::<4>::from(MOCK_VERIFIER_SELECTOR);

    let deployed = ProtocolAdapter::deploy(
        default_signer.clone(),
        *mock_risc0.router.address(),
        selector,
        fee_recipient,
    )
    .await
    .context("failed to deploy protocol adapter")?;

    Ok(*deployed.address())
}

#[cfg(feature = "mock-risc0-bindings")]
pub async fn deploy_and_insert_protocol_adapter(
    builder: &mut pa_test_harness_core::environment::StateBuilder,
    default_signer: &alloy::providers::DynProvider,
) -> anyhow::Result<Address> {
    use crate::state::pa::insert_pa_address;

    let address = deploy_protocol_adapter(default_signer).await?;

    insert_pa_address(builder, address);

    Ok(address)
}
