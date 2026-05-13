use anyhow::Context;
use pa_test_harness_core::environment::CommitmentTree as _;
use pa_test_harness_core::environment::Environment;
use pa_test_harness_core::environment::ProtocolAdapter as _;
use pa_test_harness_core::environment::Prover as _;
use pa_test_harness_core::witness::ActionWitnesses;

pub type EvmIntegrationEnv = pa_test_harness_evm::envs::integration_tests::Environment;

pub async fn prove_actions<Env: Environment>(
    env: &Env,
    actions: &[ActionWitnesses],
) -> anyhow::Result<Env::Transaction> {
    env.prover()
        .prove(actions)
        .await
        .context("failed to prove action witnesses")
}

pub async fn execute_tx<Env: Environment>(
    env: &mut Env,
    tx: Env::Transaction,
) -> anyhow::Result<()> {
    env.protocol_adapter_mut()
        .execute(tx)
        .await
        .context("failed to execute transaction on protocol adapter")
}

pub fn commitment_root<Env: Environment>(env: &Env) -> anyhow::Result<risc0_zkvm::Digest> {
    env.protocol_adapter()
        .commitment_tree()
        .root()
        .context("failed to compute commitment tree root")
}
