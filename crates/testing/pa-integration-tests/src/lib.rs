use std::future::Future;

use anyhow::Context;
use pa_test_harness_core::environment::CommitmentTree as _;
use pa_test_harness_core::environment::Environment as _;
use pa_test_harness_core::environment::ProtocolAdapter as _;
use pa_test_harness_core::environment::Prover as _;
use pa_test_harness_core::witness::ActionWitnesses;

pub type EvmIntegrationEnv = pa_test_harness_evm::envs::integration_tests::Environment;
pub type EvmTransaction = pa_test_harness_evm::envs::integration_tests::Transaction;

pub async fn setup_bare_env() -> anyhow::Result<EvmIntegrationEnv> {
    EvmIntegrationEnv::setup()
        .await
        .context("failed to set up bare EVM integration env")
}

pub async fn with_bare_env<F, Fut>(test_fn: F) -> anyhow::Result<()>
where
    F: FnOnce(EvmIntegrationEnv) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let env = setup_bare_env().await?;
    test_fn(env).await
}

pub async fn prove_actions(
    env: &EvmIntegrationEnv,
    actions: &[ActionWitnesses],
) -> anyhow::Result<EvmTransaction> {
    env.prover()
        .prove(actions)
        .await
        .context("failed to prove action witnesses")
}

pub async fn execute_tx(env: &mut EvmIntegrationEnv, tx: EvmTransaction) -> anyhow::Result<()> {
    env.protocol_adapter_mut()
        .execute(tx)
        .await
        .context("failed to execute transaction on protocol adapter")
}

pub fn commitment_root(env: &EvmIntegrationEnv) -> anyhow::Result<risc0_zkvm::Digest> {
    env.protocol_adapter()
        .commitment_tree()
        .root()
        .context("failed to compute commitment tree root")
}
