use anoma_rm_risc0::action_tree::MerkleTree as ArmTree;
use anoma_rm_risc0::compliance::ComplianceWitness;
use anoma_rm_risc0::logic_instance::LogicInstance;
use anoma_rm_risc0::logic_proof::LogicProver;
use anoma_rm_risc0::resource::Resource;
use anoma_rm_risc0::resource_logic::LogicCircuit;
use anyhow::Context;
use generic_call_library::GenericCall;
use generic_call_library::GenericCallLogic;
use generic_call_witness::GenericCallWitness;
use pa_test_harness_core::witness::ActionWitnesses;
use pa_test_harness_core::witness::ComplianceUnitWitnesses;
use pa_test_harness_core::witness::LogicWitness;

use crate::resource;
use crate::resource::GenericCallActionOverrides;

struct GenericCallLogicWitness {
    inner: GenericCallWitness,
}

impl GenericCallLogicWitness {
    #[inline]
    fn new(inner: GenericCallWitness) -> Self {
        Self { inner }
    }
}

impl LogicWitness for GenericCallLogicWitness {
    fn verifying_key(&self) -> anoma_rm_risc0::Digest {
        resource::generic_call_vk()
    }

    fn constrain(&self) -> anyhow::Result<LogicInstance> {
        LogicCircuit::constrain(&self.inner)
            .map_err(anyhow::Error::from)
            .context("invalid generic call logic witness")
    }

    fn witness_to_vec(&self) -> anyhow::Result<Vec<u32>> {
        risc0_zkvm::serde::to_vec(&self.inner)
            .context("failed to serialize generic call logic witness to risc0 words")
    }

    fn proving_key(&self) -> Vec<u8> {
        GenericCallLogic::proving_key().to_vec()
    }
}

pub struct GenericCallActionParts {
    pub action: ActionWitnesses,
    pub consumed_resource: Resource,
    pub created_resource: Resource,
}

pub fn build_generic_call_action(
    seed: u8,
    forwarder_addr: Vec<u8>,
    calls: Vec<GenericCall>,
) -> anyhow::Result<ActionWitnesses> {
    Ok(build_generic_call_action_with_parts(
        seed,
        forwarder_addr,
        calls,
        GenericCallActionOverrides::default(),
    )?
    .action)
}

pub fn build_generic_call_action_with_overrides(
    seed: u8,
    forwarder_addr: Vec<u8>,
    calls: Vec<GenericCall>,
    overrides: GenericCallActionOverrides,
) -> anyhow::Result<ActionWitnesses> {
    Ok(build_generic_call_action_with_parts(seed, forwarder_addr, calls, overrides)?.action)
}

pub fn build_generic_call_action_with_parts(
    seed: u8,
    forwarder_addr: Vec<u8>,
    calls: Vec<GenericCall>,
    overrides: GenericCallActionOverrides,
) -> anyhow::Result<GenericCallActionParts> {
    let nf_key = resource::nullifier_key(seed);
    let nk_commitment = nf_key.commit();

    let consumed_resource =
        resource::consumed_resource(seed, nk_commitment, &forwarder_addr, &calls, &overrides)?;
    let consumed_nullifier = consumed_resource
        .nullifier(&nf_key)
        .context("failed to compute consumed nullifier")?;
    let created_resource = resource::created_resource(
        seed,
        nk_commitment,
        consumed_nullifier,
        &forwarder_addr,
        &calls,
        &overrides,
    )?;

    let compliance_witness = ComplianceWitness::from_resources(
        consumed_resource,
        *anoma_rm_risc0::compliance::INITIAL_ROOT,
        nf_key.clone(),
        created_resource,
    );

    let action_tree_root = ArmTree::new(vec![consumed_nullifier, created_resource.commitment()])
        .root()
        .context("failed to compute action tree root")?;

    let consumed_logic_witness = GenericCallLogic::consumed_ephemeral_resource_logic(
        consumed_resource,
        action_tree_root,
        nf_key,
        forwarder_addr.clone(),
        calls.clone(),
    )
    .witness;

    let created_logic_witness = GenericCallLogic::created_ephemeral_resource_logic(
        created_resource,
        action_tree_root,
        forwarder_addr,
        calls,
    )
    .witness;

    let action = ActionWitnesses {
        compliance_units: vec![ComplianceUnitWitnesses {
            compliance_witness: Box::new(compliance_witness),
            consumed_logic_witness: Box::new(GenericCallLogicWitness::new(consumed_logic_witness)),
            created_logic_witness: Box::new(GenericCallLogicWitness::new(created_logic_witness)),
        }],
    };

    Ok(GenericCallActionParts {
        action,
        consumed_resource,
        created_resource,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_calls() -> Vec<GenericCall> {
        vec![GenericCall {
            to: vec![0x11; 20],
            value: 7,
            data: vec![0xaa, 0xbb],
        }]
    }

    #[test]
    fn build_generic_call_action_happy_path() {
        let action =
            build_generic_call_action(1, vec![0x22; 20], sample_calls()).expect("must build");

        assert_eq!(action.compliance_units.len(), 1);
    }

    #[test]
    fn invalid_consumed_non_ephemeral_still_builds_for_negative_tests() {
        let action = build_generic_call_action_with_overrides(
            2,
            vec![0x33; 20],
            sample_calls(),
            GenericCallActionOverrides::invalid_consumed_non_ephemeral(),
        )
        .expect("must build invalid action");

        assert_eq!(action.compliance_units.len(), 1);
    }

    #[test]
    fn invalid_consumed_label_ref_still_builds_for_negative_tests() {
        let action = build_generic_call_action_with_overrides(
            3,
            vec![0x44; 20],
            sample_calls(),
            GenericCallActionOverrides::invalid_consumed_label_ref(),
        )
        .expect("must build invalid action");

        assert_eq!(action.compliance_units.len(), 1);
    }
}
