use anyhow::Context;
use pa_test_harness_core::environment::CommitmentTree as _;
use pa_test_harness_core::environment::Environment;
use pa_test_harness_core::environment::ProtocolAdapter as _;
use pa_test_harness_core::environment::Prover as _;
use pa_test_harness_core::witness::ActionWitnesses;

pub type EvmIntegrationEnv = pa_test_harness_evm::envs::integration_tests::Environment;

pub use pa_test_harness_core::{commitment_root, execute_tx, prove_actions};
