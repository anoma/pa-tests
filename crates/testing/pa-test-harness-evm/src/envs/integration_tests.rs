use alloy::node_bindings::Anvil;
use alloy::node_bindings::AnvilInstance;
use alloy::primitives::B256;
use alloy::primitives::Bytes;
use alloy::primitives::b256;
use alloy::primitives::utils::parse_ether;
use alloy::providers::DynProvider;
use alloy::providers::Provider;
use alloy::providers::ProviderBuilder;
use alloy::providers::ext::AnvilApi;
use alloy_chains::NamedChain;
use anoma_pa_evm_bindings::generated::protocol_adapter::Compliance;
use anoma_pa_evm_bindings::generated::protocol_adapter::Logic;
use anoma_pa_evm_bindings::generated::protocol_adapter::ProtocolAdapter;
use anoma_rm_risc0::action::Action;
use anoma_rm_risc0::action_tree::MerkleTree as ArmTree;
use anoma_rm_risc0::compliance::ComplianceInstance;
use anoma_rm_risc0::compliance_unit::ComplianceUnit;
use anoma_rm_risc0::delta_proof::DeltaWitness;
use anoma_rm_risc0::logic_instance::LogicInstance;
use anoma_rm_risc0::logic_proof::LogicVerifier;
use anoma_rm_risc0::logic_proof::LogicVerifierInputs;
use anoma_rm_risc0::merkle_path::MerklePath;
use anoma_rm_risc0::transaction::{Delta, Transaction as ArmTxn};
use anyhow::Context;
use pa_test_harness_core::environment::CommitmentTree as CommitmentTreeTrait;
use pa_test_harness_core::environment::Environment as EnvironmentTrait;
use pa_test_harness_core::environment::ProtocolAdapter as ProtocolAdapterTrait;
use pa_test_harness_core::environment::Prover as ProverTrait;
use pa_test_harness_core::environment::State;
use pa_test_harness_core::environment::StateBuilder;
use pa_test_harness_core::witness::{ActionWitnesses, LogicWitness};
use risc0_zkvm::{Digest, Groth16Receipt, InnerReceipt, MaybePruned};
use sha2::{Digest as _, Sha256};

use crate::pa::{deploy_protocol_adapter, protocol_adapter};
use crate::state::actors::insert_default_signer;
use crate::state::chains::insert_chain_id;
use crate::state::pa::insert_pa_address;

pub struct IntegrationTestEnvironment {
    _anvil: AnvilInstance,
    state: State,
    prover: IntegrationTestProver,
    protocol_adapter: IntegrationTestProtocolAdapter,
}

pub struct IntegrationTestProtocolAdapter {
    pa: ProtocolAdapter::ProtocolAdapterInstance<DynProvider>,
    commitment_tree: IntegrationTestCommitmentTree,
}

#[derive(Default)]
pub struct IntegrationTestProver;

#[derive(Default)]
pub struct IntegrationTestCommitmentTree {
    leaves: Vec<Digest>,
}

pub struct Transaction {
    arm_txn: ArmTxn,
}

impl IntegrationTestEnvironment {
    #[cfg(all(feature = "pa-bindings", feature = "mock-risc0-bindings"))]
    pub async fn setup() -> anyhow::Result<Self> {
        let chain = LocalChain::spawn().await?;

        let pa_address = deploy_protocol_adapter(&chain.provider).await?;
        let pa = protocol_adapter(pa_address, chain.provider.clone());

        let chain_id = chain.provider.get_chain_id().await?;
        let named_chain = NamedChain::try_from(chain_id)
            .map_err(|_| anyhow::anyhow!("unsupported chain id {chain_id}"))?;

        let state = {
            let mut builder = StateBuilder::new();

            insert_default_signer(&mut builder, chain.provider.clone());
            insert_chain_id(&mut builder, named_chain);
            insert_pa_address(&mut builder, pa_address);

            builder.finalize()
        };

        Ok(Self {
            _anvil: chain.anvil,
            state,
            prover: IntegrationTestProver,
            protocol_adapter: IntegrationTestProtocolAdapter {
                pa,
                commitment_tree: IntegrationTestCommitmentTree::default(),
            },
        })
    }

    pub fn protocol_adapter_instance(
        &self,
    ) -> &ProtocolAdapter::ProtocolAdapterInstance<DynProvider> {
        &self.protocol_adapter.pa
    }
}

impl EnvironmentTrait for IntegrationTestEnvironment {
    type Transaction = Transaction;
    type ProtocolAdapter = IntegrationTestProtocolAdapter;
    type Prover = IntegrationTestProver;

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

impl ProtocolAdapterTrait for IntegrationTestProtocolAdapter {
    type Transaction = Transaction;
    type CommitmentTree = IntegrationTestCommitmentTree;

    async fn execute(&mut self, transaction: Self::Transaction) -> anyhow::Result<()> {
        let tx = transaction.arm_txn;
        let delta_bytes = delta_proof_bytes_from_witness(&tx)?;
        let pa_tx = to_protocol_adapter_transaction(&tx.actions, delta_bytes, None)?;

        execute_on_pa(&self.pa, pa_tx).await?;

        for action in &tx.actions {
            for unit in &action.compliance_units {
                let instance = unit.get_instance()?;
                self.commitment_tree.append(instance.created_commitment)?;
            }
        }

        Ok(())
    }

    fn commitment_tree(&self) -> &Self::CommitmentTree {
        &self.commitment_tree
    }

    fn commitment_tree_mut(&mut self) -> &mut Self::CommitmentTree {
        &mut self.commitment_tree
    }
}

impl ProverTrait for IntegrationTestProver {
    type Transaction = Transaction;

    async fn prove(&self, actions: &[ActionWitnesses]) -> anyhow::Result<Self::Transaction> {
        Transaction::create(actions)
    }
}

impl CommitmentTreeTrait for IntegrationTestCommitmentTree {
    fn append(&mut self, commitment: Digest) -> anyhow::Result<()> {
        self.leaves.push(commitment);
        Ok(())
    }

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

impl Transaction {
    #[inline]
    pub fn create(witnesses: &[ActionWitnesses]) -> anyhow::Result<Self> {
        constrain_txn(witnesses)
    }
}

impl From<Transaction> for ArmTxn {
    #[inline]
    fn from(transaction: Transaction) -> Self {
        transaction.arm_txn
    }
}

struct LocalChain {
    anvil: AnvilInstance,
    provider: DynProvider,
}

impl LocalChain {
    async fn spawn() -> anyhow::Result<Self> {
        let anvil = Anvil::new().spawn();

        let signer = alloy::signers::local::PrivateKeySigner::from_bytes(&b256!(
            "7ad4b84636a3fa408827e7202f6da39287bbf099d1fab6250d3b56e03e77586b"
        ))?;
        let deployer = signer.address();

        let provider = ProviderBuilder::new()
            .wallet(signer)
            .connect_http(anvil.endpoint_url())
            .erased();

        provider
            .anvil_set_balance(deployer, parse_ether("100").expect("parse ether"))
            .await?;

        Ok(Self { anvil, provider })
    }
}

fn delta_proof_bytes_from_witness(tx: &ArmTxn) -> anyhow::Result<Vec<u8>> {
    let proved = tx.clone().generate_delta_proof()?;
    let bytes = match proved.delta_proof {
        Delta::Proof(proof) => proof.to_bytes().to_vec(),
        Delta::Witness(_) => {
            return Err(anyhow::anyhow!(
                "delta proof generation returned witness unexpectedly"
            ));
        }
    };
    Ok(bytes)
}

fn to_protocol_adapter_transaction(
    actions: &[Action],
    delta_proof: Vec<u8>,
    aggregation_proof: Option<Vec<u8>>,
) -> anyhow::Result<ProtocolAdapter::Transaction> {
    let evm_actions = actions
        .iter()
        .map(to_pa_action)
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(ProtocolAdapter::Transaction {
        actions: evm_actions,
        deltaProof: Bytes::from(delta_proof),
        aggregationProof: Bytes::from(aggregation_proof.unwrap_or_default()),
    })
}

fn to_pa_action(action: &Action) -> anyhow::Result<ProtocolAdapter::Action> {
    let logic_inputs = build_logic_inputs(action)?;
    let compliance_inputs = action
        .compliance_units
        .iter()
        .map(to_pa_compliance)
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(ProtocolAdapter::Action {
        logicVerifierInputs: logic_inputs,
        complianceVerifierInputs: compliance_inputs,
    })
}

fn build_logic_inputs(action: &Action) -> anyhow::Result<Vec<Logic::VerifierInput>> {
    let compliance_instances = action
        .compliance_units
        .iter()
        .map(ComplianceUnit::get_instance)
        .collect::<Result<Vec<ComplianceInstance>, _>>()?;

    let tags = compliance_instances
        .iter()
        .flat_map(|instance| vec![instance.consumed_nullifier, instance.created_commitment])
        .collect::<Vec<_>>();
    let logics = compliance_instances
        .iter()
        .flat_map(|instance| vec![instance.consumed_logic_ref, instance.created_logic_ref])
        .collect::<Vec<_>>();

    if tags.len() != action.logic_verifier_inputs.len() {
        anyhow::bail!(
            "logic input count mismatch: tags={}, inputs={}",
            tags.len(),
            action.logic_verifier_inputs.len()
        );
    }

    let action_tree_root = ArmTree::new(tags.clone()).root()?;
    let mut out = Vec::with_capacity(tags.len());

    for (index, (tag, expected_vk)) in tags.iter().zip(logics.iter()).enumerate() {
        let input = action
            .logic_verifier_inputs
            .iter()
            .find(|input| &input.tag == tag)
            .ok_or_else(|| anyhow::anyhow!("logic input tag not found"))?;

        if input.verifying_key != *expected_vk {
            anyhow::bail!("logic verifying key mismatch");
        }

        let is_consumed = index % 2 == 0;
        let verifier = input
            .clone()
            .to_logic_verifier(is_consumed, action_tree_root)?;
        out.push(to_pa_logic(verifier)?);
    }

    Ok(out)
}

fn to_pa_logic(logic: LogicVerifier) -> anyhow::Result<Logic::VerifierInput> {
    let instance = logic.get_instance()?;
    let resource_payload = instance
        .app_data
        .resource_payload
        .into_iter()
        .map(to_pa_blob)
        .collect();
    let discovery_payload = instance
        .app_data
        .discovery_payload
        .into_iter()
        .map(to_pa_blob)
        .collect();
    let external_payload = instance
        .app_data
        .external_payload
        .into_iter()
        .map(to_pa_blob)
        .collect();
    let application_payload = instance
        .app_data
        .application_payload
        .into_iter()
        .map(to_pa_blob)
        .collect();

    Ok(Logic::VerifierInput {
        tag: B256::from_slice(instance.tag.as_bytes()),
        verifyingKey: B256::from_slice(logic.verifying_key.as_bytes()),
        appData: Logic::AppData {
            resourcePayload: resource_payload,
            discoveryPayload: discovery_payload,
            externalPayload: external_payload,
            applicationPayload: application_payload,
        },
        proof: Bytes::from(logic.proof.unwrap_or_default()),
    })
}

fn to_pa_compliance(compliance: &ComplianceUnit) -> anyhow::Result<Compliance::VerifierInput> {
    let instance = compliance.get_instance()?;

    Ok(Compliance::VerifierInput {
        proof: Bytes::from(compliance.proof.clone().unwrap_or_default()),
        instance: Compliance::Instance {
            consumed: Compliance::ConsumedRefs {
                nullifier: B256::from_slice(instance.consumed_nullifier.as_bytes()),
                logicRef: B256::from_slice(instance.consumed_logic_ref.as_bytes()),
                commitmentTreeRoot: B256::from_slice(
                    instance.consumed_commitment_tree_root.as_bytes(),
                ),
            },
            created: Compliance::CreatedRefs {
                commitment: B256::from_slice(instance.created_commitment.as_bytes()),
                logicRef: B256::from_slice(instance.created_logic_ref.as_bytes()),
            },
            unitDeltaX: B256::from_slice(anoma_rm_risc0::utils::words_to_bytes(&instance.delta_x)),
            unitDeltaY: B256::from_slice(anoma_rm_risc0::utils::words_to_bytes(&instance.delta_y)),
        },
    })
}

fn to_pa_blob(blob: anoma_rm_risc0::logic_instance::ExpirableBlob) -> Logic::ExpirableBlob {
    Logic::ExpirableBlob {
        deletionCriterion: blob.deletion_criterion as u8,
        blob: Bytes::from(anoma_rm_risc0::utils::words_to_bytes(&blob.blob).to_vec()),
    }
}

async fn execute_on_pa(
    pa: &ProtocolAdapter::ProtocolAdapterInstance<DynProvider>,
    tx: ProtocolAdapter::Transaction,
) -> anyhow::Result<alloy::rpc::types::TransactionReceipt> {
    if let Err(err) = pa.execute(tx.clone()).call().await {
        return Err(anyhow::anyhow!(
            "protocol adapter call reverted before send: {err:?}"
        ));
    }

    let receipt = pa.execute(tx).send().await?.get_receipt().await?;
    if !receipt.status() {
        return Err(anyhow::anyhow!("protocol adapter execution failed"));
    }
    Ok(receipt)
}

fn encode_seal(verifying_key: Digest, journal: Digest) -> Vec<u8> {
    let proof = {
        let mut hasher = Sha256::new();

        hasher.update(verifying_key);
        hasher.update(journal);

        hasher.finalize().to_vec()
    };

    bincode::serialize(&InnerReceipt::Groth16(Groth16Receipt::new(
        proof,
        MaybePruned::Pruned(Digest::default()),
        // NB: this will yield a selector equal to [0xFF, 0xFF, 0xFF, 0xFF]
        Digest::new([u32::MAX; 8]),
    )))
    .unwrap()
}

#[inline]
fn logic_instance_to_journal(instance: &LogicInstance) -> anyhow::Result<Digest> {
    let words = risc0_zkvm::serde::to_vec(instance)
        .context("failed to convert logic instance to risc0-zkvm words")?;

    Ok(journal_digest_from_words(&words))
}

#[inline]
fn compliance_instance_to_journal(instance: &ComplianceInstance) -> anyhow::Result<Digest> {
    let words = risc0_zkvm::serde::to_vec(instance)
        .context("failed to convert compliance instance to risc0-zkvm words")?;

    Ok(journal_digest_from_words(&words))
}

#[inline]
fn journal_digest_from_words(words: &[u32]) -> Digest {
    let raw: [u8; 32] =
        Sha256::digest(&anoma_rm_risc0::utils::words_to_bytes(words).to_vec()).into();

    raw.into()
}

fn constrain_txn(action_witnesses: &[ActionWitnesses]) -> anyhow::Result<Transaction> {
    let mut actions = Vec::with_capacity(action_witnesses.len());
    let mut rcvs = Vec::with_capacity(action_witnesses.len());

    for (action_idx, action_witness) in action_witnesses.iter().enumerate() {
        anyhow::ensure!(
            !action_witness.compliance_units.is_empty(),
            "action {action_idx} has no compliance units"
        );

        let mut compliance_units = Vec::with_capacity(action_witness.compliance_units.len());
        let mut logic_verifier_inputs =
            Vec::with_capacity(action_witness.compliance_units.len() * 2);

        for (unit_idx, unit_witnesses) in action_witness.compliance_units.iter().enumerate() {
            let compliance_instance =
                unit_witnesses
                    .compliance_witness
                    .constrain()
                    .with_context(|| {
                        format!(
                            "failed to constrain compliance unit {unit_idx} of action {action_idx}"
                        )
                    })?;

            let consumed_logic_instance = constrain_logic_witness(
                &unit_witnesses.consumed_logic_witness,
                action_idx,
                unit_idx,
                true,
            )?;
            let created_logic_instance = constrain_logic_witness(
                &unit_witnesses.created_logic_witness,
                action_idx,
                unit_idx,
                false,
            )?;

            anyhow::ensure!(
                consumed_logic_instance.tag == compliance_instance.consumed_nullifier,
                "action {action_idx} unit {unit_idx} consumed tag must equal consumed nullifier"
            );
            anyhow::ensure!(
                created_logic_instance.tag == compliance_instance.created_commitment,
                "action {action_idx} unit {unit_idx} created tag must equal created commitment"
            );

            let consumed_vk = verifying_key_for_witness(&unit_witnesses.consumed_logic_witness);
            let created_vk = verifying_key_for_witness(&unit_witnesses.created_logic_witness);

            anyhow::ensure!(
                consumed_vk == compliance_instance.consumed_logic_ref,
                "action {action_idx} unit {unit_idx} consumed verifying key mismatch"
            );
            anyhow::ensure!(
                created_vk == compliance_instance.created_logic_ref,
                "action {action_idx} unit {unit_idx} created verifying key mismatch"
            );

            let consumed_logic_proof = {
                let journal = logic_instance_to_journal(&consumed_logic_instance)?;
                encode_seal(consumed_vk, journal)
            };
            let created_logic_proof = {
                let journal = logic_instance_to_journal(&created_logic_instance)?;
                encode_seal(created_vk, journal)
            };
            let compliance_proof = {
                let journal = compliance_instance_to_journal(&compliance_instance)?;
                encode_seal(*anoma_rm_risc0::constants::COMPLIANCE_VK, journal)
            };

            compliance_units.push(ComplianceUnit {
                proof: Some(compliance_proof),
                instance: {
                    anoma_rm_risc0::utils::words_to_bytes(
                        &risc0_zkvm::serde::to_vec(&compliance_instance)
                            .context("failed to serialize compliance instance words")?,
                    )
                    .to_vec()
                },
            });
            logic_verifier_inputs.push(LogicVerifierInputs {
                tag: consumed_logic_instance.tag,
                verifying_key: consumed_vk,
                app_data: consumed_logic_instance.app_data,
                proof: Some(consumed_logic_proof),
            });
            logic_verifier_inputs.push(LogicVerifierInputs {
                tag: created_logic_instance.tag,
                verifying_key: created_vk,
                app_data: created_logic_instance.app_data,
                proof: Some(created_logic_proof),
            });

            rcvs.push(unit_witnesses.compliance_witness.rcv.clone());
        }

        anyhow::ensure!(
            logic_verifier_inputs.len() == compliance_units.len() * 2,
            "action {action_idx} must have exactly 2 logic inputs per \
             compliance unit: logic={} compliance={}",
            logic_verifier_inputs.len(),
            compliance_units.len()
        );

        let action = Action {
            compliance_units,
            logic_verifier_inputs,
        };

        actions.push(action);
    }

    let delta = Delta::Witness(
        DeltaWitness::from_bytes_vec(&rcvs)
            .context("failed to construct delta witness from rcv values")?,
    );
    let arm_txn = ArmTxn::create(actions, delta)
        .generate_delta_proof()
        .context("failed to gelerate delta proof")?;

    Ok(Transaction { arm_txn })
}

fn constrain_logic_witness(
    logic_witness: &impl LogicWitness,
    action_idx: usize,
    unit_idx: usize,
    expected_consumed: bool,
) -> anyhow::Result<LogicInstance> {
    let instance = logic_witness.constrain().with_context(|| {
        format!("failed to constrain logic instance {unit_idx} of action {action_idx}")
    })?;

    if instance.is_consumed != expected_consumed {
        let expected = if expected_consumed {
            "consumed"
        } else {
            "created"
        };
        anyhow::bail!("action {action_idx} unit {unit_idx} witness is not {expected} as expected");
    }

    Ok(instance)
}

#[inline]
fn verifying_key_for_witness(witness: &impl LogicWitness) -> anoma_rm_risc0::Digest {
    witness.verifying_key()
}

#[cfg(test)]
mod tests {
    use anoma_rm_risc0::compliance::ComplianceWitness;
    use anoma_rm_risc0::nullifier_key::NullifierKey;
    use anoma_rm_risc0::resource::Resource;
    use anoma_rm_risc0::resource_logic::TrivialLogicWitness;
    use pa_test_harness_core::witness::ComplianceUnitWitnesses;

    use super::*;

    #[test]
    fn constrain_trivial() {
        _ = Transaction::create(&build_trivial_action_witnesses_many(8)).unwrap();
    }

    fn build_trivial_action_witnesses_many(actions_count: usize) -> Vec<ActionWitnesses> {
        (0..actions_count)
            .map(build_trivial_action_witnesses)
            .collect()
    }

    fn build_trivial_action_witnesses(idx: usize) -> ActionWitnesses {
        let (compliance, consumed, created) = build_action_fixture(idx);

        ActionWitnesses {
            compliance_units: vec![ComplianceUnitWitnesses {
                compliance_witness: Box::new(compliance),
                consumed_logic_witness: Box::new(consumed),
                created_logic_witness: Box::new(created),
            }],
        }
    }

    fn build_action_fixture(
        idx: usize,
    ) -> (ComplianceWitness, TrivialLogicWitness, TrivialLogicWitness) {
        let seed = (idx as u8).wrapping_add(1);

        let nf_key = NullifierKey::from_bytes([seed; 32]);
        let nk_commitment = nf_key.commit();
        let logic_ref = *anoma_rm_risc0::constants::PADDING_LOGIC_VK;

        let consumed_resource = Resource {
            logic_ref,
            label_ref: Digest::default(),
            quantity: 0,
            value_ref: Digest::default(),
            is_ephemeral: true,
            nonce: [seed; 32],
            nk_commitment,
            rand_seed: [seed.wrapping_add(11); 32],
        };

        let consumed_nullifier = consumed_resource
            .nullifier(&nf_key)
            .expect("failed to compute consumed nullifier");

        let created_resource = Resource {
            logic_ref,
            label_ref: Digest::default(),
            quantity: 0,
            value_ref: Digest::default(),
            is_ephemeral: true,
            nonce: consumed_nullifier
                .as_bytes()
                .try_into()
                .expect("nullifier must be 32 bytes"),
            nk_commitment,
            rand_seed: [seed.wrapping_add(33); 32],
        };

        let compliance_witness = ComplianceWitness::from_resources(
            consumed_resource,
            *anoma_rm_risc0::compliance::INITIAL_ROOT,
            nf_key.clone(),
            created_resource,
        );

        let compliance_instance = compliance_witness
            .constrain()
            .expect("compliance constrain must pass");

        let action_tree_root = ArmTree::new(vec![
            compliance_instance.consumed_nullifier,
            compliance_instance.created_commitment,
        ])
        .root()
        .expect("action tree root");

        let consumed_logic_witness = TrivialLogicWitness {
            resource: consumed_resource,
            action_tree_root,
            is_consumed: true,
            nf_key: nf_key.clone(),
        };
        let created_logic_witness = TrivialLogicWitness {
            resource: created_resource,
            action_tree_root,
            is_consumed: false,
            nf_key,
        };

        (
            compliance_witness,
            consumed_logic_witness,
            created_logic_witness,
        )
    }
}
