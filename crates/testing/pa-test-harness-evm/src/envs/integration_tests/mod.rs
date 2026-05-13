use alloy::node_bindings::AnvilInstance;
use alloy::providers::DynProvider;
use anoma_pa_evm_bindings::generated::protocol_adapter::ProtocolAdapter as PaContract;
use anoma_rm_risc0::action_tree::MerkleTree as ArmTree;
use anoma_rm_risc0::merkle_path::MerklePath;
use anoma_rm_risc0::transaction::Transaction as ArmTxn;
use pa_test_harness_core::environment::CommitmentTree as CoreCommitmentTree;
use pa_test_harness_core::environment::Environment as CoreEnvironment;
use pa_test_harness_core::environment::ProtocolAdapter as CoreProtocolAdapter;
use pa_test_harness_core::environment::Prover as CoreProver;
use pa_test_harness_core::environment::State;
use pa_test_harness_core::environment::Transaction as CoreTransaction;
use pa_test_harness_core::witness::ActionWitnesses;
use risc0_zkvm::Digest;

mod evm_convert;
mod evm_execute;
mod prover;
mod setup;

#[cfg(test)]
mod tests;

/// Integration test execution environment.
///
/// Setup contract:
/// - All fields on this environment and its nested structures are public on purpose.
/// - Setup code should mutate/inspect the concrete environment directly.
/// - Test execution code should accept `impl pa_test_harness_core::environment::Environment`
///   and use typed state helpers instead of concrete fields.
pub struct Environment {
    pub anvil: AnvilInstance,
    pub state: State,
    pub prover: Prover,
    pub protocol_adapter: ProtocolAdapter,
}

pub struct ProtocolAdapter {
    pub pa: PaContract::ProtocolAdapterInstance<DynProvider>,
    pub commitment_tree: CommitmentTree,
}

#[derive(Default)]
pub struct Prover;

#[derive(Default)]
pub struct CommitmentTree {
    leaves: Vec<Digest>,
}

pub struct Transaction {
    arm_txn: ArmTxn,
}

impl CoreEnvironment for Environment {
    type Transaction = Transaction;
    type ProtocolAdapter = ProtocolAdapter;
    type Prover = Prover;

    fn prover(&self) -> &Self::Prover {
        &self.prover
    }

    fn state(&self) -> &State {
        &self.state
    }

    fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    fn protocol_adapter(&self) -> &Self::ProtocolAdapter {
        &self.protocol_adapter
    }

    fn protocol_adapter_mut(&mut self) -> &mut Self::ProtocolAdapter {
        &mut self.protocol_adapter
    }
}

impl CoreProtocolAdapter for ProtocolAdapter {
    type Transaction = Transaction;
    type CommitmentTree = CommitmentTree;

    async fn execute(&mut self, transaction: Self::Transaction) -> anyhow::Result<()> {
        let created_commitments: Vec<Digest> = transaction.created_commitments()?.collect();
        let pa_tx: PaContract::Transaction = transaction.into();

        evm_execute::execute_on_pa(&self.pa, pa_tx).await?;

        self.commitment_tree.leaves.extend(created_commitments);

        Ok(())
    }

    fn commitment_tree(&self) -> &Self::CommitmentTree {
        &self.commitment_tree
    }
}

impl CoreProver for Prover {
    type Transaction = Transaction;

    async fn prove(&self, actions: &[ActionWitnesses]) -> anyhow::Result<Self::Transaction> {
        Transaction::create(actions)
    }
}

impl CoreCommitmentTree for CommitmentTree {
    fn root(&self) -> anyhow::Result<Digest> {
        if self.leaves.is_empty() {
            return Ok(*anoma_rm_risc0::compliance::INITIAL_ROOT);
        }

        Ok(ArmTree::new(self.leaves.clone()).root()?)
    }

    fn path_to(&self, leaf: Digest) -> anyhow::Result<MerklePath> {
        Ok(ArmTree::new(self.leaves.clone()).generate_path(&leaf)?)
    }
}

impl CoreTransaction for Transaction {
    fn created_commitments(&self) -> anyhow::Result<impl Iterator<Item = Digest> + '_> {
        let commitments = self
            .arm_txn
            .actions
            .iter()
            .flat_map(|action| {
                action.compliance_units.iter().map(|unit| {
                    unit.get_instance()
                        .map(|instance| instance.created_commitment)
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(commitments.into_iter())
    }
}
