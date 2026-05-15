use anyhow::Context;
use pa_evm_tests::{
    EvmIntegrationEnv, Needle, commitment_root, execute_tx, expect_integration_panic, prove_actions,
};
use pa_test_harness_core::environment::Environment;
use pa_test_harness_evm_action_trivial::{
    TrivialActionOverrides, build_action_with_overrides, build_actions,
};
use rstest::*;

#[rstest]
#[case::integration_test(EvmIntegrationEnv::setup_bare())]
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
