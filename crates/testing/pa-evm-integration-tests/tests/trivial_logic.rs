use anyhow::Context;
use pa_evm_integration_tests::commitment_root;
use pa_evm_integration_tests::execute_tx;
use pa_evm_integration_tests::prove_actions;
use pa_evm_integration_tests::with_bare_env;
use pa_test_harness_evm_action_trivial::TrivialActionOverrides;
use pa_test_harness_evm_action_trivial::build_action_with_overrides;
use pa_test_harness_evm_action_trivial::build_actions;

#[tokio::test]
async fn trivial_happy_flow() -> anyhow::Result<()> {
    with_bare_env(|mut env| async move {
        let before = commitment_root(&env)?;
        let actions = build_actions(2, 1).context("failed to build trivial actions")?;
        let tx = prove_actions(&env, &actions).await?;
        execute_tx(&mut env, tx).await?;
        let after = commitment_root(&env)?;

        anyhow::ensure!(before != after, "commitment tree root must change");
        Ok(())
    })
    .await
}

#[tokio::test]
#[should_panic(expected = "left: 1\n right: 0")]
async fn trivial_negative_flow_quantity_must_be_zero() {
    with_bare_env(|env| async move {
        let bad =
            build_action_with_overrides(7, TrivialActionOverrides::invalid_nonzero_quantity())
                .context("failed to build invalid trivial action")?;

        let _ = prove_actions(&env, &[bad]).await;
        Ok(())
    })
    .await
    .expect("negative flow setup must succeed before prove panics");
}

#[tokio::test]
#[should_panic(expected = "assertion failed: self.resource.is_ephemeral")]
async fn trivial_negative_flow_resource_must_be_ephemeral() {
    with_bare_env(|env| async move {
        let bad = build_action_with_overrides(
            8,
            TrivialActionOverrides::invalid_consumed_non_ephemeral(),
        )
        .context("failed to build invalid trivial action")?;

        let _ = prove_actions(&env, &[bad]).await;
        Ok(())
    })
    .await
    .expect("negative flow setup must succeed before prove panics");
}
