use std::collections::HashMap;
use std::panic::AssertUnwindSafe;

use anoma_rm_risc0::action::Action;
use anoma_rm_risc0::compliance_unit::ComplianceUnit;
use anoma_rm_risc0::constants::COMPLIANCE_PK;
use anoma_rm_risc0::delta_proof::DeltaWitness;
use anoma_rm_risc0::logic_instance::{AppData, LogicInstance};
use anoma_rm_risc0::logic_proof::LogicVerifierInputs;
use anoma_rm_risc0::transaction::{Delta, Transaction as ArmTxn};
use anyhow::Context;
use futures::future::try_join_all;
use heliax_ap_orchestrator_sdk::QueueClient;
use heliax_ap_orchestrator_sdk::{
    AggregateProofResult, BaseProofResult, GpuAggregationProofPayload, GpuComplianceProofPayload,
    GpuLogicProofPayload, ProofPayload, ProofType,
};
use pa_test_harness_core::witness::{ActionWitnesses, LogicWitness};
use risc0_zkvm::Digest;

use super::Transaction;
use super::queue;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum BaseJobKind {
    ConsumedLogic,
    CreatedLogic,
    Compliance,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct BaseJobKey {
    action_idx: usize,
    unit_idx: usize,
    kind: BaseJobKind,
}

#[derive(Clone, Debug)]
enum BaseJobPayload {
    Logic(ProofPayload),
    Compliance(ProofPayload),
}

#[derive(Clone, Debug)]
struct BaseJobSpec {
    key: BaseJobKey,
    payload: BaseJobPayload,
}

#[derive(Debug)]
struct SubmittedBaseJob {
    key: BaseJobKey,
    job_id: String,
}

#[derive(Debug)]
struct FetchedBaseJob {
    key: BaseJobKey,
    result: BaseProofResult,
}

pub(super) async fn prove_via_queue(
    queue: &QueueClient,
    action_witnesses: &[ActionWitnesses],
) -> anyhow::Result<Transaction> {
    let constrained = std::panic::catch_unwind(AssertUnwindSafe(|| {
        constrain_and_validate(action_witnesses)
    }))
    .unwrap_or_else(|cause| {
        if let Some(panic_msg) = cause.downcast_ref::<String>() {
            anyhow::bail!("proving failed: {panic_msg}");
        }
        if let Some(panic_msg) = cause.downcast_ref::<&'static str>() {
            anyhow::bail!("proving failed: {panic_msg}");
        }
        std::panic::resume_unwind(cause)
    })?;

    let (base_job_specs, rcvs) = build_base_job_specs(action_witnesses)?;

    let submitted_jobs = try_join_all(
        base_job_specs
            .into_iter()
            .map(|spec| submit_base_job(queue, spec)),
    )
    .await?;

    let base_results = try_join_all(
        submitted_jobs
            .into_iter()
            .map(|submitted| fetch_base_job(queue, submitted)),
    )
    .await?;

    let mut base_results_by_key: HashMap<BaseJobKey, BaseProofResult> =
        HashMap::with_capacity(base_results.len());
    for fetched in base_results {
        let replaced = base_results_by_key.insert(fetched.key, fetched.result);
        anyhow::ensure!(
            replaced.is_none(),
            "duplicate base proof result for action {} unit {} kind {:?}",
            fetched.key.action_idx,
            fetched.key.unit_idx,
            fetched.key.kind
        );
    }

    let mut actions = Vec::with_capacity(action_witnesses.len());

    for (action_idx, constrained_action) in constrained.into_iter().enumerate() {
        let mut compliance_units = Vec::with_capacity(constrained_action.compliance_units_len);
        let mut logic_verifier_inputs =
            Vec::with_capacity(constrained_action.logic_verifier_inputs.len());
        let mut constrained_logic_iter = constrained_action.logic_verifier_inputs.into_iter();

        for unit_idx in 0..constrained_action.compliance_units_len {
            let compliance_key = BaseJobKey {
                action_idx,
                unit_idx,
                kind: BaseJobKind::Compliance,
            };
            let compliance_result =
                base_results_by_key.remove(&compliance_key).ok_or_else(|| {
                    anyhow::anyhow!(
                        "missing compliance proof result for action {} unit {}",
                        action_idx,
                        unit_idx
                    )
                })?;

            compliance_units.push(ComplianceUnit {
                proof: Some(compliance_result.receipt),
                instance: compliance_result.instance,
            });

            let consumed_lvi = constrained_logic_iter.next().ok_or_else(|| {
                anyhow::anyhow!(
                    "missing constrained consumed logic input for action {} unit {}",
                    action_idx,
                    unit_idx
                )
            })?;
            let created_lvi = constrained_logic_iter.next().ok_or_else(|| {
                anyhow::anyhow!(
                    "missing constrained created logic input for action {} unit {}",
                    action_idx,
                    unit_idx
                )
            })?;

            let consumed_logic_result = base_results_by_key
                .remove(&BaseJobKey {
                    action_idx,
                    unit_idx,
                    kind: BaseJobKind::ConsumedLogic,
                })
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "missing consumed logic proof result for action {} unit {}",
                        action_idx,
                        unit_idx
                    )
                })?;
            let created_logic_result = base_results_by_key
                .remove(&BaseJobKey {
                    action_idx,
                    unit_idx,
                    kind: BaseJobKind::CreatedLogic,
                })
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "missing created logic proof result for action {} unit {}",
                        action_idx,
                        unit_idx
                    )
                })?;

            logic_verifier_inputs.push(logic_verifier_input_from_result(
                consumed_lvi,
                consumed_logic_result,
            )?);
            logic_verifier_inputs.push(logic_verifier_input_from_result(
                created_lvi,
                created_logic_result,
            )?);
        }

        anyhow::ensure!(
            constrained_logic_iter.next().is_none(),
            "extra constrained logic inputs found for action {}",
            action_idx
        );

        actions.push(Action {
            compliance_units,
            logic_verifier_inputs,
        });
    }

    anyhow::ensure!(
        base_results_by_key.is_empty(),
        "unused base proof results remaining after assembly: {}",
        base_results_by_key.len()
    );

    let delta = Delta::Witness(
        DeltaWitness::from_bytes_vec(&rcvs)
            .context("failed to construct delta witness from rcv values")?,
    );
    let arm_txn = ArmTxn::create(actions, delta)
        .generate_delta_proof()
        .context("failed to generate delta proof")?;

    let serialized =
        bincode::serialize(&arm_txn).context("failed to serialize transaction for aggregation")?;

    let agg_payload = GpuAggregationProofPayload {
        transaction: serialized,
    };
    let agg_job_id = queue
        .submit(agg_payload)
        .send()
        .await
        .context("failed to submit aggregation proof job")?;

    let agg_result: AggregateProofResult = queue::fetch_job_result(queue, &agg_job_id)
        .await
        .context("failed to fetch aggregation proof result")?;

    let aggregated: ArmTxn = bincode::deserialize(&agg_result.transaction)
        .context("failed to decode aggregated transaction")?;

    aggregated
        .clone()
        .verify()
        .context("aggregated transaction failed local verification")?;

    Ok(Transaction {
        arm_txn: aggregated,
    })
}

struct ConstrainedAction {
    compliance_units_len: usize,
    logic_verifier_inputs: Vec<ConstrainedLogicVerifierInput>,
}

struct ConstrainedLogicVerifierInput {
    tag: Digest,
    app_data: AppData,
}

fn logic_verifier_input_from_result(
    constrained: ConstrainedLogicVerifierInput,
    logic_result: BaseProofResult,
) -> anyhow::Result<LogicVerifierInputs> {
    anyhow::ensure!(
        logic_result.verifying_key.len() == 32,
        "verifying_key has invalid length {}, expected 32",
        logic_result.verifying_key.len()
    );
    let mut verifying_key_bytes = [0u8; 32];
    verifying_key_bytes.copy_from_slice(&logic_result.verifying_key);

    Ok(LogicVerifierInputs {
        tag: constrained.tag,
        verifying_key: Digest::from_bytes(verifying_key_bytes),
        app_data: constrained.app_data,
        proof: Some(logic_result.receipt),
    })
}

fn build_base_job_specs(
    action_witnesses: &[ActionWitnesses],
) -> anyhow::Result<(Vec<BaseJobSpec>, Vec<Vec<u8>>)> {
    let mut specs = Vec::new();
    let mut rcvs = Vec::new();

    for (action_idx, action_witness) in action_witnesses.iter().enumerate() {
        anyhow::ensure!(
            !action_witness.compliance_units.is_empty(),
            "action {action_idx} has no compliance units"
        );

        for (unit_idx, unit_witnesses) in action_witness.compliance_units.iter().enumerate() {
            specs.push(BaseJobSpec {
                key: BaseJobKey {
                    action_idx,
                    unit_idx,
                    kind: BaseJobKind::ConsumedLogic,
                },
                payload: BaseJobPayload::Logic(build_logic_proof_payload(
                    &unit_witnesses.consumed_logic_witness,
                )?),
            });
            specs.push(BaseJobSpec {
                key: BaseJobKey {
                    action_idx,
                    unit_idx,
                    kind: BaseJobKind::CreatedLogic,
                },
                payload: BaseJobPayload::Logic(build_logic_proof_payload(
                    &unit_witnesses.created_logic_witness,
                )?),
            });
            specs.push(BaseJobSpec {
                key: BaseJobKey {
                    action_idx,
                    unit_idx,
                    kind: BaseJobKind::Compliance,
                },
                payload: BaseJobPayload::Compliance(build_compliance_proof_payload(
                    &unit_witnesses.compliance_witness,
                )?),
            });

            rcvs.push(unit_witnesses.compliance_witness.rcv.clone());
        }
    }

    Ok((specs, rcvs))
}

async fn submit_base_job(
    queue: &QueueClient,
    spec: BaseJobSpec,
) -> anyhow::Result<SubmittedBaseJob> {
    let BaseJobSpec { key, payload } = spec;
    let job_id = match payload {
        BaseJobPayload::Logic(payload) => queue
            .submit(GpuLogicProofPayload(payload))
            .send()
            .await
            .with_context(|| {
                format!(
                    "failed to submit {:?} proof for action {} unit {}",
                    key.kind, key.action_idx, key.unit_idx
                )
            })?,
        BaseJobPayload::Compliance(payload) => queue
            .submit(GpuComplianceProofPayload(payload))
            .send()
            .await
            .with_context(|| {
                format!(
                    "failed to submit {:?} proof for action {} unit {}",
                    key.kind, key.action_idx, key.unit_idx
                )
            })?,
    };

    Ok(SubmittedBaseJob { key, job_id })
}

async fn fetch_base_job(
    queue: &QueueClient,
    submitted: SubmittedBaseJob,
) -> anyhow::Result<FetchedBaseJob> {
    let key = submitted.key;
    let result = queue::fetch_job_result::<BaseProofResult>(queue, &submitted.job_id)
        .await
        .with_context(|| {
            format!(
                "failed to fetch {:?} proof result for action {} unit {}",
                key.kind, key.action_idx, key.unit_idx
            )
        })?;

    Ok(FetchedBaseJob { key, result })
}

fn build_logic_proof_payload(logic_witness: &impl LogicWitness) -> anyhow::Result<ProofPayload> {
    Ok(ProofPayload {
        witness: logic_witness
            .witness_to_vec()
            .context("failed to serialize logic witness to risc0 words")?,
        proving_key: logic_witness.proving_key(),
        proof_type: ProofType::Succinct,
        verifying_key: logic_witness.verifying_key().as_bytes().to_vec(),
    })
}

fn build_compliance_proof_payload(
    compliance_witness: &anoma_rm_risc0::compliance::ComplianceWitness,
) -> anyhow::Result<ProofPayload> {
    let witness = risc0_zkvm::serde::to_vec(compliance_witness)
        .context("failed to serialize compliance witness to risc0 words")?;

    Ok(ProofPayload {
        witness,
        proving_key: COMPLIANCE_PK.to_vec(),
        proof_type: ProofType::Succinct,
        verifying_key: vec![0u8],
    })
}

fn constrain_and_validate(
    action_witnesses: &[ActionWitnesses],
) -> anyhow::Result<Vec<ConstrainedAction>> {
    let mut actions = Vec::with_capacity(action_witnesses.len());

    for (action_idx, action_witness) in action_witnesses.iter().enumerate() {
        anyhow::ensure!(
            !action_witness.compliance_units.is_empty(),
            "action {action_idx} has no compliance units"
        );

        let mut compliance_units_len = 0usize;
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

            let consumed_vk = unit_witnesses.consumed_logic_witness.verifying_key();
            let created_vk = unit_witnesses.created_logic_witness.verifying_key();

            anyhow::ensure!(
                consumed_vk == compliance_instance.consumed_logic_ref,
                "action {action_idx} unit {unit_idx} consumed verifying key mismatch"
            );
            anyhow::ensure!(
                created_vk == compliance_instance.created_logic_ref,
                "action {action_idx} unit {unit_idx} created verifying key mismatch"
            );

            compliance_units_len += 1;

            logic_verifier_inputs.push(ConstrainedLogicVerifierInput {
                tag: consumed_logic_instance.tag,
                app_data: consumed_logic_instance.app_data,
            });
            logic_verifier_inputs.push(ConstrainedLogicVerifierInput {
                tag: created_logic_instance.tag,
                app_data: created_logic_instance.app_data,
            });
        }

        anyhow::ensure!(
            logic_verifier_inputs.len() == compliance_units_len * 2,
            "action {action_idx} must have exactly 2 logic inputs per \
             compliance unit: logic={} compliance={}",
            logic_verifier_inputs.len(),
            compliance_units_len
        );

        actions.push(ConstrainedAction {
            compliance_units_len,
            logic_verifier_inputs,
        });
    }

    Ok(actions)
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
