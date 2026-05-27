#[cfg(feature = "e2e")]
pub type EvmE2eEnv = pa_test_harness_evm::envs::e2e::Environment;

pub type EvmIntegrationEnv = pa_test_harness_evm::envs::integration_test::Environment;
pub type EvmIntegrationTx = pa_test_harness_evm::envs::integration_test::Transaction;

use alloy::primitives::{Address, B256, U256};
use anyhow::Context;
use pa_test_harness_core::environment::Environment;
use pa_test_harness_core::environment::StateBuilder;
use pa_test_harness_evm::state::actors::default_signer_in_state;
use pa_test_harness_evm::state::chains::chain_id;
use pa_test_harness_evm::state::pa::pa_address_in_state;
use pa_test_harness_evm_action_generic_call::generic_call_vk;
use pa_test_harness_evm_action_transfer::{sender_keychain, token_transfer_vk};
use pa_test_harness_evm_erc20::example_erc20_bindings::deploy_and_insert_example_erc20;
use pa_test_harness_evm_erc20::example_erc20_bindings::erc20_example;
use pa_test_harness_evm_erc20::ierc20_bindings::ierc20;
use pa_test_harness_evm_erc20::state::addresses::erc20_address;
use pa_test_harness_evm_erc20::weth_bindings::deploy_and_insert_weth;
use pa_test_harness_evm_erc20_forwarder::erc20_forwarder_bindings::deploy_and_insert_erc20_forwarder;
use pa_test_harness_evm_erc20_forwarder::state::addresses::erc20_forwarder_v1_address;
use pa_test_harness_evm_generic_call_forwarder::generic_call_forwarder_bindings::deploy_and_insert_generic_call_forwarder;
use pa_test_harness_evm_generic_call_forwarder::state::addresses::generic_call_forwarder_v1_address;
use pa_test_harness_evm_mock_permit2::deploy_permit2_canonical;

pub use pa_test_harness_core::{commitment_root, execute_tx, prove_actions};

#[derive(Debug)]
pub enum Needle {
    Static(&'static str),
    Regexp(regex::Regex),
}

pub fn expect_integration_panic<T>(
    needle: Needle,
) -> impl FnOnce(anyhow::Result<T>) -> anyhow::Result<()> {
    move |result| {
        let Err(error) = result else {
            anyhow::bail!("expected to find error {needle:?}, but got anyhow::Ok");
        };

        let dbg_error = format!("{error:?}");
        let found_needle = match &needle {
            Needle::Static(s) => dbg_error.contains(s),
            Needle::Regexp(re) => re.is_match(&dbg_error),
        };

        if !found_needle {
            return Err(error.context(format!("couldn't find needle {needle:?} in error")));
        }

        Ok(())
    }
}

pub fn tamper_integration_first_logic_seal(tx: &mut EvmIntegrationTx) -> anyhow::Result<()> {
    let logic_input = tx
        .as_arm_mut()
        .actions
        .first_mut()
        .context("tamper requires at least one action")?
        .logic_verifier_inputs
        .first_mut()
        .context("tamper requires at least one logic verifier input")?;

    let proof = logic_input
        .proof
        .as_mut()
        .context("tamper requires first logic proof")?;

    let mut inner: risc0_zkvm::InnerReceipt = bincode::deserialize(proof)
        .context("tamper requires bincode-encoded inner receipt proof")?;

    let receipt = match &mut inner {
        risc0_zkvm::InnerReceipt::Groth16(receipt) => receipt,
        _ => anyhow::bail!("tamper requires Groth16 inner receipt proof"),
    };

    let byte = receipt
        .seal
        .first_mut()
        .context("tamper requires non-empty inner seal")?;
    *byte ^= 0x01;

    *proof =
        bincode::serialize(&inner).context("tamper must re-serialize modified inner receipt")?;

    Ok(())
}

pub async fn setup_transfer_integration_env() -> anyhow::Result<EvmIntegrationEnv> {
    EvmIntegrationEnv::setup(async |builder: &mut StateBuilder| {
        deploy_permit2_for_transfer(builder).await?;
        setup_transfer_env_on_builder(builder, TransferToken::Example, false).await
    })
    .await
}

pub async fn setup_transfer_generic_call_integration_env() -> anyhow::Result<EvmIntegrationEnv> {
    EvmIntegrationEnv::setup(async |builder: &mut StateBuilder| {
        deploy_permit2_for_transfer(builder).await?;
        setup_transfer_env_on_builder(builder, TransferToken::Weth, true).await
    })
    .await
}

#[cfg(feature = "e2e")]
pub async fn setup_transfer_e2e_env() -> anyhow::Result<EvmE2eEnv> {
    EvmE2eEnv::setup(async |builder: &mut StateBuilder| {
        setup_transfer_env_on_builder(builder, TransferToken::Example, false).await
    })
    .await
}

#[cfg(feature = "e2e")]
pub async fn setup_transfer_generic_call_e2e_env() -> anyhow::Result<EvmE2eEnv> {
    EvmE2eEnv::setup(async |builder: &mut StateBuilder| {
        setup_transfer_env_on_builder(builder, TransferToken::Weth, true).await
    })
    .await
}

pub async fn transfer_chain_id<E>(env: &E) -> anyhow::Result<u64>
where
    E: Environment,
{
    chain_id(env).context("failed to retrieve chain id from state")
}

pub fn transfer_token_address<E>(env: &E) -> anyhow::Result<Address>
where
    E: Environment,
{
    erc20_address(env, "example")
}

pub fn transfer_weth_token_address<E>(env: &E) -> anyhow::Result<Address>
where
    E: Environment,
{
    erc20_address(env, "weth")
}

pub fn transfer_forwarder_address<E>(env: &E) -> anyhow::Result<Address>
where
    E: Environment,
{
    erc20_forwarder_v1_address(env)
}

pub fn generic_call_forwarder_address<E>(env: &E) -> anyhow::Result<Address>
where
    E: Environment,
{
    generic_call_forwarder_v1_address(env)
}

enum TransferToken {
    Example,
    Weth,
}

async fn deploy_permit2_for_transfer(builder: &mut StateBuilder) -> anyhow::Result<()> {
    let provider = default_signer_in_state(builder.as_state())
        .context("failed to retrieve default signer from setup state")?;
    deploy_permit2_canonical(&provider)
        .await
        .map(|_| ())
        .context("failed to deploy Permit2 at canonical address")
}

async fn setup_transfer_env_on_builder(
    builder: &mut StateBuilder,
    token_kind: TransferToken,
    deploy_generic_call_forwarder: bool,
) -> anyhow::Result<()> {
    let provider = default_signer_in_state(builder.as_state())
        .context("failed to retrieve default signer from setup state")?;
    let pa_address = pa_address_in_state(builder.as_state())
        .context("failed to retrieve protocol adapter address from setup state")?;

    let deployer = sender_keychain()
        .context("failed to build sender keychain")?
        .ethereum_addr;

    let token = match token_kind {
        TransferToken::Example => deploy_and_insert_example_erc20(
            builder,
            "example",
            provider.clone(),
            deployer,
            U256::from(1_000_000u64),
        )
        .await
        .context("failed to deploy and insert ERC20Example")?,
        TransferToken::Weth => deploy_and_insert_weth(
            builder,
            "weth",
            provider.clone(),
            deployer,
            U256::from(1_000_000u64),
        )
        .await
        .context("failed to deploy and insert WETH9")?,
    };

    let transfer_logic_ref = B256::from(<[u8; 32]>::from(token_transfer_vk()));
    deploy_and_insert_erc20_forwarder(
        builder,
        provider.clone(),
        pa_address,
        transfer_logic_ref,
        deployer,
    )
    .await
    .context("failed to deploy and insert ERC20 forwarder v1")?;

    if deploy_generic_call_forwarder {
        let generic_call_logic_ref = B256::from(<[u8; 32]>::from(generic_call_vk()));
        deploy_and_insert_generic_call_forwarder(
            builder,
            provider.clone(),
            pa_address,
            generic_call_logic_ref,
        )
        .await
        .context("failed to deploy and insert generic call forwarder v1")?;
    }

    match token_kind {
        TransferToken::Example => {
            erc20_example(token, provider.clone())
                .approve(
                    pa_test_harness_evm_mock_permit2::PERMIT2_CANONICAL_ADDRESS,
                    U256::MAX,
                )
                .send()
                .await
                .context("failed to submit permit2 approval transaction")?
                .get_receipt()
                .await
                .context("failed to fetch permit2 approval receipt")?;
        }
        TransferToken::Weth => {
            ierc20(token, provider.clone())
                .approve(
                    pa_test_harness_evm_mock_permit2::PERMIT2_CANONICAL_ADDRESS,
                    U256::MAX,
                )
                .send()
                .await
                .context("failed to submit WETH permit2 approval transaction")?
                .get_receipt()
                .await
                .context("failed to fetch WETH permit2 approval receipt")?;
        }
    }

    Ok(())
}
