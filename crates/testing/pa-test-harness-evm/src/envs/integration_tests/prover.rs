use anoma_rm_risc0::action::Action;
use anoma_rm_risc0::compliance::ComplianceInstance;
use anoma_rm_risc0::compliance_unit::ComplianceUnit;
use anoma_rm_risc0::delta_proof::DeltaWitness;
use anoma_rm_risc0::logic_instance::LogicInstance;
use anoma_rm_risc0::logic_proof::LogicVerifierInputs;
use anoma_rm_risc0::transaction::{Delta, Transaction as ArmTxn};
use anyhow::Context;
use pa_test_harness_core::witness::{ActionWitnesses, LogicWitness};
use risc0_zkvm::{Digest, Groth16Receipt, InnerReceipt, MaybePruned};
use sha2::{Digest as _, Sha256};

use super::Transaction;

impl Transaction {
    #[inline]
    pub fn create(witnesses: &[ActionWitnesses]) -> anyhow::Result<Self> {
        constrain_txn(witnesses)
    }
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
        .context("failed to generate delta proof")?;

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
