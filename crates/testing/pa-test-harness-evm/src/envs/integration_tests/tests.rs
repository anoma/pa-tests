use anoma_pa_evm_bindings::generated::protocol_adapter::ProtocolAdapter as PaContract;
use anoma_rm_risc0::action_tree::MerkleTree as ArmTree;
use anoma_rm_risc0::compliance::ComplianceWitness;
use anoma_rm_risc0::nullifier_key::NullifierKey;
use anoma_rm_risc0::resource::Resource;
use anoma_rm_risc0::resource_logic::TrivialLogicWitness;
use anoma_rm_risc0::transaction::Transaction as ArmTxn;
use pa_test_harness_core::witness::ActionWitnesses;
use pa_test_harness_core::witness::ComplianceUnitWitnesses;
use risc0_zkvm::Digest;

use super::Transaction;

#[test]
fn constrain_trivial() {
    _ = Transaction::create(&build_trivial_action_witnesses_many(8)).unwrap();
}

#[test]
fn arm_txn_into_pa_tx_conversion_smoke() {
    let tx = Transaction::create(&build_trivial_action_witnesses_many(2)).unwrap();
    let arm_tx: ArmTxn = tx.into();

    let pa_tx: PaContract::Transaction = arm_tx.into();
    assert!(!pa_tx.actions.is_empty());
    assert!(!pa_tx.deltaProof.is_empty());
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
