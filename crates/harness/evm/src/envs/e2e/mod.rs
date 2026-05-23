use alloy::node_bindings::AnvilInstance;
use alloy::primitives::B256;
use alloy::providers::DynProvider;
use anoma_pa_evm_bindings::generated::protocol_adapter::ProtocolAdapter as PaContract;
use anoma_rm_risc0::action_tree::MerkleTree as ArmTree;
use anoma_rm_risc0::merkle_path::MerklePath;
use anoma_rm_risc0::transaction::Transaction as ArmTxn;
use anyhow::Context;
use heliax_ap_orchestrator_sdk::QueueClient;
use pa_test_harness_core::environment::CommitmentTree as CoreCommitmentTree;
use pa_test_harness_core::environment::Environment as CoreEnvironment;
use pa_test_harness_core::environment::ProtocolAdapter as CoreProtocolAdapter;
use pa_test_harness_core::environment::Prover as CoreProver;
use pa_test_harness_core::environment::State;
use pa_test_harness_core::environment::Transaction as CoreTransaction;
use pa_test_harness_core::witness::ActionWitnesses;
use risc0_zkvm::Digest;

pub mod config;
mod evm_convert;
mod evm_execute;
mod prover;
mod queue;
mod setup;

pub use config::E2eConfig;

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

pub struct Prover {
    queue: QueueClient,
}

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
        let tx = transaction.arm_txn;

        self.assert_root_consistency(&tx).await?;

        let pa_tx: PaContract::Transaction = tx.into();

        evm_execute::execute_on_pa(&self.pa, pa_tx).await?;

        self.commitment_tree.leaves.extend(created_commitments);

        Ok(())
    }

    fn commitment_tree(&self) -> &Self::CommitmentTree {
        &self.commitment_tree
    }
}

impl ProtocolAdapter {
    async fn assert_root_consistency(&self, tx: &ArmTxn) -> anyhow::Result<()> {
        let local_root = self.commitment_tree.root()?;
        let pa_root = self
            .pa
            .latestCommitmentTreeRoot()
            .call()
            .await
            .context("failed to query latest commitment tree root from protocol adapter")?;

        let local_root_b256 = B256::from_slice(local_root.as_bytes());
        anyhow::ensure!(
            local_root_b256 == pa_root,
            "commitment tree root mismatch before execution: local={local_root_b256:?}, pa={pa_root:?}"
        );

        for (action_idx, action) in tx.actions.iter().enumerate() {
            for (unit_idx, unit) in action.compliance_units.iter().enumerate() {
                let instance = unit.get_instance().with_context(|| {
                    format!("failed to decode compliance instance for action {action_idx} unit {unit_idx}")
                })?;

                let consumed_root =
                    B256::from_slice(instance.consumed_commitment_tree_root.as_bytes());
                let contained = self
                    .pa
                    .isCommitmentTreeRootContained(consumed_root)
                    .call()
                    .await
                    .with_context(|| {
                        format!(
                            "failed to query root containment for action {action_idx} unit {unit_idx}"
                        )
                    })?;

                anyhow::ensure!(
                    contained,
                    "consumed commitment tree root not found in PA for action {action_idx} unit \
                     {unit_idx}: root={consumed_root:?}, pa_latest={pa_root:?}, \
                     local_latest={local_root_b256:?}"
                );
            }
        }

        Ok(())
    }
}

impl CoreProver for Prover {
    type Transaction = Transaction;

    async fn prove(&self, actions: &[ActionWitnesses]) -> anyhow::Result<Self::Transaction> {
        prover::prove_via_queue(&self.queue, actions).await
    }
}

impl CoreCommitmentTree for CommitmentTree {
    fn root(&self) -> anyhow::Result<Digest> {
        if self.leaves.is_empty() {
            return Ok(*anoma_rm_risc0::compliance::INITIAL_ROOT);
        }

        Ok(self.build_tree().root()?)
    }

    fn path_to(&self, leaf: Digest) -> anyhow::Result<MerklePath> {
        Ok(self.build_tree().generate_path(&leaf)?)
    }
}

impl CommitmentTree {
    fn build_tree(&self) -> ArmTree {
        let mut leaves = self.leaves.clone();
        if leaves.is_empty() || leaves.len().is_power_of_two() {
            leaves.push(*anoma_rm_risc0::merkle_path::PADDING_LEAF);
        }
        ArmTree::new(leaves)
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
