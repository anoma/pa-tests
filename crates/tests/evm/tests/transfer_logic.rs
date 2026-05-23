use anyhow::Context;
#[cfg(feature = "e2e")]
use pa_evm_tests::setup_transfer_e2e_env;
use pa_evm_tests::{
    Needle, commitment_root, execute_tx, expect_integration_panic, prove_actions,
    setup_transfer_integration_env, transfer_chain_id, transfer_forwarder_address,
    transfer_token_address,
};
use pa_test_harness_core::environment::CommitmentTree;
use pa_test_harness_core::environment::Environment;
use pa_test_harness_core::environment::ProtocolAdapter;
use pa_test_harness_evm_action_transfer::{
    TransferActionOverrides, WrapActionOverrides, build_transfer_action_with_overrides,
    build_transfer_action_with_overrides_and_path, build_unwrap_action_with_overrides_and_path,
    build_wrap_action_with_overrides,
};
use rstest::*;

#[rstest]
#[case::integration_test(setup_transfer_integration_env())]
#[cfg_attr(feature = "e2e", case::e2e_test(setup_transfer_e2e_env()))]
#[tokio::test]
async fn happy_wrap_transfer_unwrap<Env: Environment>(
    #[future(awt)]
    #[case]
    env_with_setup: anyhow::Result<Env>,
) -> anyhow::Result<()> {
    let mut env = env_with_setup.context("env setup failed")?;
    let chain_id = transfer_chain_id(&env).await?;
    let forwarder = transfer_forwarder_address(&env)?;
    let token = transfer_token_address(&env)?;

    let before = commitment_root(&env)?;

    let wrapped = build_wrap_action_with_overrides(
        chain_id,
        forwarder,
        token,
        1,
        11,
        WrapActionOverrides::default(),
    )
    .await
    .context("failed to build wrap action")?;
    let tx = prove_actions(&env, &[wrapped.action])
        .await
        .context("failed to prove wrap action")?;
    execute_tx(&mut env, tx)
        .await
        .context("failed to execute wrap action")?;

    let transfer_path = env
        .protocol_adapter()
        .commitment_tree()
        .path_to(wrapped.created_persistent.commitment())
        .context("failed to generate transfer merkle path")?;

    let transferred = build_transfer_action_with_overrides_and_path(
        wrapped.created_persistent,
        forwarder,
        token,
        17,
        TransferActionOverrides::default(),
        Some(transfer_path),
    )
    .context("failed to build transfer action")?;
    let tx = prove_actions(&env, &[transferred.action])
        .await
        .context("failed to prove transfer action")?;
    execute_tx(&mut env, tx)
        .await
        .context("failed to execute transfer action")?;

    let unwrap_path = env
        .protocol_adapter()
        .commitment_tree()
        .path_to(transferred.created_persistent.commitment())
        .context("failed to generate unwrap merkle path")?;

    let unwrapped = build_unwrap_action_with_overrides_and_path(
        transferred.created_persistent,
        forwarder,
        token,
        21,
        pa_test_harness_evm_action_transfer::UnwrapActionOverrides::default(),
        Some(unwrap_path),
    )
    .context("failed to build unwrap action")?;
    let tx = prove_actions(&env, &[unwrapped.action])
        .await
        .context("failed to prove unwrap action")?;
    execute_tx(&mut env, tx)
        .await
        .context("failed to execute unwrap action")?;

    let after = commitment_root(&env)?;
    anyhow::ensure!(before != after, "commitment tree root must change");

    Ok(())
}

#[rstest]
#[case::integration_test(
    setup_transfer_integration_env(),
    expect_integration_panic(Needle::Static("Signature must be 65 bytes long"))
)]
#[tokio::test]
async fn transfer_negative_invalid_permit_signature_len<Env: Environment>(
    #[future(awt)]
    #[case]
    env_with_setup: anyhow::Result<Env>,
    #[case] assert_err: impl FnOnce(anyhow::Result<Env::Transaction>) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let env = env_with_setup.context("env setup failed")?;
    let chain_id = transfer_chain_id(&env).await?;
    let forwarder = transfer_forwarder_address(&env)?;
    let token = transfer_token_address(&env)?;

    let bad = build_wrap_action_with_overrides(
        chain_id,
        forwarder,
        token,
        1,
        23,
        WrapActionOverrides::invalid_permit_signature_length(),
    )
    .await
    .context("failed to build invalid wrap action")?;

    assert_err(prove_actions(&env, &[bad.action]).await)
}

#[rstest]
#[case::integration_test(
    setup_transfer_integration_env(),
    expect_integration_panic(Needle::Static("Invalid signature"))
)]
#[tokio::test]
async fn transfer_negative_invalid_auth_signature<Env: Environment>(
    #[future(awt)]
    #[case]
    env_with_setup: anyhow::Result<Env>,
    #[case] assert_err: impl FnOnce(anyhow::Result<Env::Transaction>) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let env = env_with_setup.context("env setup failed")?;
    let chain_id = transfer_chain_id(&env).await?;
    let forwarder = transfer_forwarder_address(&env)?;
    let token = transfer_token_address(&env)?;

    let wrapped = build_wrap_action_with_overrides(
        chain_id,
        forwarder,
        token,
        1,
        31,
        WrapActionOverrides::default(),
    )
    .await
    .context("failed to build wrap action")?;

    let bad = build_transfer_action_with_overrides(
        wrapped.created_persistent,
        forwarder,
        token,
        37,
        TransferActionOverrides::invalid_auth_signature(),
    )
    .context("failed to build invalid transfer action")?;

    assert_err(prove_actions(&env, &[bad.action]).await)
}
