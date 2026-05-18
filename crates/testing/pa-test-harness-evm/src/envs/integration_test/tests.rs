use anoma_pa_evm_bindings::generated::protocol_adapter::ProtocolAdapter as PaContract;
use anoma_rm_risc0::transaction::Transaction as ArmTxn;
use pa_test_harness_core::witness::ActionWitnesses;
use pa_test_harness_evm_action_trivial::build_actions;

use super::Transaction;

#[test]
fn constrain_trivial() {
    _ = Transaction::create(&build_trivial_actions_many(8)).unwrap();
}

#[test]
fn arm_txn_into_pa_tx_conversion_smoke() {
    let tx = Transaction::create(&build_trivial_actions_many(2)).unwrap();
    let arm_tx: ArmTxn = tx.into();

    let pa_tx: PaContract::Transaction = arm_tx.into();
    assert!(!pa_tx.actions.is_empty());
    assert!(!pa_tx.deltaProof.is_empty());
}

fn build_trivial_actions_many(actions_count: usize) -> Vec<ActionWitnesses> {
    build_actions(actions_count, 1).expect("must build trivial action witnesses")
}
