use anoma_rm_risc0::action_tree::MerkleTree as ArmTree;
use anoma_rm_risc0::compliance::ComplianceWitness;
use anoma_rm_risc0::resource::Resource;
use anoma_rm_risc0::resource_logic::TrivialLogicWitness;
use anyhow::Context;
use pa_test_harness_core::witness::ActionWitnesses;
use pa_test_harness_core::witness::ComplianceUnitWitnesses;

use crate::resource;
use crate::resource::TrivialActionOverrides;

pub struct TrivialActionParts {
    pub action: ActionWitnesses,
    pub consumed_resource: Resource,
    pub created_resource: Resource,
}

pub fn build_action(seed: u8) -> anyhow::Result<ActionWitnesses> {
    Ok(build_action_with_parts(seed, TrivialActionOverrides::default())?.action)
}

pub fn build_actions(count: usize, seed_start: u8) -> anyhow::Result<Vec<ActionWitnesses>> {
    (0..count)
        .map(|idx| {
            let seed = seed_start.wrapping_add(idx as u8);
            build_action(seed)
        })
        .collect()
}

pub fn build_action_with_overrides(
    seed: u8,
    overrides: TrivialActionOverrides,
) -> anyhow::Result<ActionWitnesses> {
    Ok(build_action_with_parts(seed, overrides)?.action)
}

pub fn build_action_with_parts(
    seed: u8,
    overrides: TrivialActionOverrides,
) -> anyhow::Result<TrivialActionParts> {
    let nf_key = resource::nullifier_key(seed);
    let nk_commitment = nf_key.commit();

    let consumed_resource = resource::consumed_resource(seed, nk_commitment, &overrides);
    let consumed_nullifier = consumed_resource
        .nullifier(&nf_key)
        .context("failed to compute consumed nullifier")?;
    let created_resource =
        resource::created_resource(seed, nk_commitment, consumed_nullifier, &overrides)?;

    let compliance_witness = ComplianceWitness::from_resources(
        consumed_resource,
        *anoma_rm_risc0::compliance::INITIAL_ROOT,
        nf_key.clone(),
        created_resource,
    );

    let action_tree_root = ArmTree::new(vec![consumed_nullifier, created_resource.commitment()])
        .root()
        .context("failed to compute action tree root")?;

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

    let action = ActionWitnesses {
        compliance_units: vec![ComplianceUnitWitnesses {
            compliance_witness: Box::new(compliance_witness),
            consumed_logic_witness: Box::new(consumed_logic_witness),
            created_logic_witness: Box::new(created_logic_witness),
        }],
    };

    Ok(TrivialActionParts {
        action,
        consumed_resource,
        created_resource,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_action_valid_trivial() {
        let action = build_action(1).expect("valid trivial action must build");
        assert_eq!(action.compliance_units.len(), 1);
    }

    #[test]
    fn invalid_non_ephemeral_consumed_still_builds_action_for_negative_tests() {
        let action = build_action_with_overrides(
            2,
            TrivialActionOverrides::invalid_consumed_non_ephemeral(),
        )
        .expect("action construction should succeed");

        assert_eq!(action.compliance_units.len(), 1);
    }

    #[test]
    fn invalid_nonzero_quantity_still_builds_action_for_negative_tests() {
        let action =
            build_action_with_overrides(3, TrivialActionOverrides::invalid_nonzero_quantity())
                .expect("action construction should succeed");

        assert_eq!(action.compliance_units.len(), 1);
    }
}
