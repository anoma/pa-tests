use anyhow::Context;
use pa_evm_tests::{EvmIntegrationEnv, commitment_root, execute_tx, prove_actions};
use pa_test_harness_core::environment::Environment;
use pa_test_harness_evm_action_trivial::{
    TrivialActionOverrides, build_action_with_overrides, build_actions,
};
use rstest::*;

#[rstest]
#[case::integration_test(EvmIntegrationEnv::setup())]
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
#[case::integration_test(EvmIntegrationEnv::setup())]
#[tokio::test]
#[should_panic(expected = "left: 1\n right: 0")]
async fn trivial_negative_flow_quantity_must_be_zero<Env: Environment>(
    #[future(awt)]
    #[case]
    env: anyhow::Result<Env>,
) {
    let env = env.expect("env setup failed");

    let bad = build_action_with_overrides(7, TrivialActionOverrides::invalid_nonzero_quantity())
        .expect("failed to build invalid trivial action");

    _ = prove_actions(&env, &[bad]).await;
}

#[rstest]
#[case::integration_test(EvmIntegrationEnv::setup())]
#[tokio::test]
#[should_panic(expected = "assertion failed: self.resource.is_ephemeral")]
async fn trivial_negative_flow_resource_must_be_ephemeral<Env: Environment>(
    #[future(awt)]
    #[case]
    env: anyhow::Result<Env>,
) {
    let env = env.expect("env setup failed");

    let bad =
        build_action_with_overrides(8, TrivialActionOverrides::invalid_consumed_non_ephemeral())
            .expect("failed to build invalid trivial action");

    _ = prove_actions(&env, &[bad]).await;
}
