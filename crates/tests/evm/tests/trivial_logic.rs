use anyhow::Context;
#[cfg(feature = "e2e")]
use pa_evm_tests::EvmE2eEnv;
use pa_evm_tests::{
    EvmIntegrationEnv, Needle, commitment_root, execute_tx, expect_integration_panic,
    prove_actions, tamper_integration_first_logic_seal,
};
use pa_test_harness_core::environment::Environment;
use pa_test_harness_evm_action_trivial::{
    TrivialActionOverrides, build_action_with_overrides, build_actions,
};
use rstest::*;

#[rstest]
#[case::integration_test(EvmIntegrationEnv::setup_bare())]
#[cfg_attr(feature = "e2e", case::e2e_test(EvmE2eEnv::setup_bare()))]
#[tokio::test]
async fn trivial_happy_flow<Env: Environment>(
    #[future(awt)]
    #[case]
    env: anyhow::Result<Env>,
) -> anyhow::Result<()> {
    let mut env = env.context("env setup failed")?;

    let before = commitment_root(&env)?;

    let actions = build_actions(2, 1).context("failed to build trivial actions")?;
    let tx = prove_actions(&env, &actions).await?;

    execute_tx(&mut env, tx).await?;

    let after = commitment_root(&env)?;

    anyhow::ensure!(before != after, "commitment tree root must change");
    Ok(())
}

#[rstest]
#[case::integration_test(
    EvmIntegrationEnv::setup_bare(),
    expect_integration_panic(Needle::Regexp(
        regex::Regex::new(
            r#"proving failed: [^\n]*\n\s*left: 1[^\n]*\n\s*right: 0"#,
        )
        .unwrap(),
    )),
)]
#[tokio::test]
async fn trivial_negative_flow_quantity_must_be_zero<Env: Environment>(
    #[future(awt)]
    #[case]
    env: anyhow::Result<Env>,
    #[case] assert_err: impl FnOnce(anyhow::Result<Env::Transaction>) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let env = env.expect("env setup failed");

    let bad = build_action_with_overrides(7, TrivialActionOverrides::invalid_nonzero_quantity())
        .expect("failed to build invalid trivial action");

    assert_err(prove_actions(&env, &[bad]).await)
}

#[rstest]
#[case::integration_test(
    EvmIntegrationEnv::setup_bare(),
    expect_integration_panic(Needle::Static("assertion failed: self.resource.is_ephemeral"))
)]
#[tokio::test]
async fn trivial_negative_flow_resource_must_be_ephemeral<Env: Environment>(
    #[future(awt)]
    #[case]
    env: anyhow::Result<Env>,
    #[case] assert_err: impl FnOnce(anyhow::Result<Env::Transaction>) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let env = env.expect("env setup failed");

    let bad =
        build_action_with_overrides(8, TrivialActionOverrides::invalid_consumed_non_ephemeral())
            .expect("failed to build invalid trivial action");

    assert_err(prove_actions(&env, &[bad]).await)
}

#[rstest]
#[case::integration_test(
    EvmIntegrationEnv::setup_bare(),
    tamper_integration_first_logic_seal,
    expect_integration_panic(Needle::Static(
        "protocol adapter preflight failed: server returned an error response: error \
         code 3: execution reverted: payload, data: \"0x08c379a0"
    ))
)]
#[tokio::test]
async fn trivial_negative_flow_invalid_seal<Env: Environment>(
    #[future(awt)]
    #[case]
    env: anyhow::Result<Env>,
    #[case] tamper: impl FnOnce(&mut Env::Transaction) -> anyhow::Result<()>,
    #[case] assert_err: impl FnOnce(anyhow::Result<()>) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut env = env.context("env setup failed")?;

    let actions = build_actions(1, 11).context("failed to build trivial actions")?;
    let mut tx = prove_actions(&env, &actions)
        .await
        .context("valid witnesses should prove before tampering")?;

    tamper(&mut tx).context("failed to tamper transaction proof")?;

    assert_err(execute_tx(&mut env, tx).await)
}
