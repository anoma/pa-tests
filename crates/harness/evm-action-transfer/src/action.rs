use alloy::primitives::{Address, B256, U256};
use anoma_rm_risc0::action_tree::MerkleTree as ArmTree;
use anoma_rm_risc0::compliance::ComplianceWitness;
use anoma_rm_risc0::logic_instance::LogicInstance;
use anoma_rm_risc0::logic_proof::LogicProver;
use anoma_rm_risc0::merkle_path::MerklePath;
use anoma_rm_risc0::resource::Resource;
use anoma_rm_risc0::resource_logic::LogicCircuit;
use anoma_rm_risc0_gadgets::authority::AuthoritySignature;
use anyhow::Context;
use pa_test_harness_core::witness::ActionWitnesses;
use pa_test_harness_core::witness::ComplianceUnitWitnesses;
use pa_test_harness_core::witness::LogicWitness;
use transfer_library::TransferLogic;
use transfer_witness::AUTH_SIGNATURE_DOMAIN;
use transfer_witness::EncryptionInfo;
use transfer_witness::ForwarderInfo;
use transfer_witness::LabelInfo;
use transfer_witness::PermitInfo;
use transfer_witness::TokenTransferWitness;
use transfer_witness::ValueInfo;
use transfer_witness::call_type::CallType;

use crate::fixtures::TransferKeychain;
use crate::fixtures::receiver_keychain;
use crate::fixtures::sender_keychain;
use crate::permit2::Permit2Data;
use crate::permit2::permit_witness_transfer_from_signature;
use crate::resource;
use crate::resource::TransferActionOverrides;
use crate::resource::UnwrapActionOverrides;
use crate::resource::WrapActionOverrides;

struct TransferLogicWitness {
    inner: TokenTransferWitness,
}

impl TransferLogicWitness {
    #[inline]
    fn new(inner: TokenTransferWitness) -> Self {
        Self { inner }
    }
}

impl LogicWitness for TransferLogicWitness {
    fn verifying_key(&self) -> anoma_rm_risc0::Digest {
        resource::token_transfer_vk()
    }

    fn constrain(&self) -> anyhow::Result<LogicInstance> {
        LogicCircuit::constrain(&self.inner)
            .map_err(anyhow::Error::from)
            .context("invalid transfer logic witness")
    }

    fn witness_to_vec(&self) -> anyhow::Result<Vec<u32>> {
        risc0_zkvm::serde::to_vec(&self.inner)
            .context("failed to serialize transfer logic witness to risc0 words")
    }

    fn proving_key(&self) -> Vec<u8> {
        TransferLogic::proving_key().to_vec()
    }
}

pub struct WrapActionParts {
    pub action: ActionWitnesses,
    pub created_persistent: Resource,
    pub sender: TransferKeychain,
}

pub struct TransferActionParts {
    pub action: ActionWitnesses,
    pub consumed_persistent: Resource,
    pub created_persistent: Resource,
    pub sender: TransferKeychain,
    pub receiver: TransferKeychain,
}

pub struct UnwrapActionParts {
    pub action: ActionWitnesses,
    pub consumed_persistent: Resource,
    pub created_ephemeral: Resource,
    pub owner: TransferKeychain,
}

pub async fn build_wrap_action(
    chain_id: u64,
    forwarder: Address,
    token: Address,
    quantity: u128,
    seed: u8,
) -> anyhow::Result<ActionWitnesses> {
    Ok(build_wrap_action_with_overrides(
        chain_id,
        forwarder,
        token,
        quantity,
        seed,
        WrapActionOverrides::default(),
    )
    .await?
    .action)
}

pub async fn build_wrap_action_with_overrides(
    chain_id: u64,
    forwarder: Address,
    token: Address,
    quantity: u128,
    seed: u8,
    overrides: WrapActionOverrides,
) -> anyhow::Result<WrapActionParts> {
    let sender = sender_keychain()?;

    let consumed =
        resource::wrap_consumed_resource(&sender, forwarder, token, quantity, seed, &overrides);
    let consumed_nf = consumed.nullifier(&sender.nf_key)?;
    let created = resource::wrap_created_resource(
        &sender,
        consumed_nf,
        forwarder,
        token,
        overrides.quantity.unwrap_or(quantity),
        seed,
    )?;

    let action_tree_root = ArmTree::new(vec![consumed_nf, created.commitment()]).root()?;

    let permit_nonce = resource::random_permit_nonce(seed);
    let permit_deadline = U256::from(4_294_967_295u64);
    let permit_sig = match overrides.permit_signature {
        Some(bytes) => bytes,
        None => permit_witness_transfer_from_signature(
            &sender.ethereum_signer,
            Permit2Data {
                chain_id,
                token,
                amount: U256::from(overrides.quantity.unwrap_or(quantity)),
                nonce: permit_nonce,
                deadline: permit_deadline,
                spender: forwarder,
                action_tree_root: B256::from_slice(action_tree_root.as_bytes()),
            },
        )
        .await?
        .into(),
    };

    let consumed_logic = TokenTransferWitness::new(
        consumed,
        true,
        action_tree_root,
        Some(sender.nf_key.clone()),
        None,
        None,
        Some(ForwarderInfo {
            call_type: CallType::Wrap,
            ethereum_account_addr: sender.ethereum_addr.to_vec(),
            permit_info: Some(PermitInfo {
                permit_nonce: permit_nonce.to_be_bytes_vec(),
                permit_deadline: permit_deadline.to_be_bytes_vec(),
                permit_sig,
            }),
        }),
        Some(LabelInfo {
            forwarder_addr: forwarder.to_vec(),
            erc20_token_addr: token.to_vec(),
        }),
        None,
    );

    let created_logic = TokenTransferWitness::new(
        created,
        false,
        action_tree_root,
        None,
        None,
        Some(EncryptionInfo::new(&sender.discovery_pk)),
        None,
        Some(LabelInfo {
            forwarder_addr: forwarder.to_vec(),
            erc20_token_addr: token.to_vec(),
        }),
        Some(ValueInfo {
            auth_pk: sender.auth_verifying_key(),
            encryption_pk: sender.encryption_pk,
        }),
    );

    let compliance = ComplianceWitness::from_resources(
        consumed,
        *anoma_rm_risc0::compliance::INITIAL_ROOT,
        sender.nf_key.clone(),
        created,
    );

    Ok(WrapActionParts {
        action: ActionWitnesses {
            compliance_units: vec![ComplianceUnitWitnesses {
                compliance_witness: Box::new(compliance),
                consumed_logic_witness: Box::new(TransferLogicWitness::new(consumed_logic)),
                created_logic_witness: Box::new(TransferLogicWitness::new(created_logic)),
            }],
        },
        created_persistent: created,
        sender,
    })
}

pub fn build_transfer_action(
    resource_to_transfer: Resource,
    forwarder: Address,
    token: Address,
    seed: u8,
) -> anyhow::Result<ActionWitnesses> {
    Ok(build_transfer_action_with_overrides(
        resource_to_transfer,
        forwarder,
        token,
        seed,
        TransferActionOverrides::default(),
    )?
    .action)
}

pub fn build_transfer_action_with_overrides(
    resource_to_transfer: Resource,
    forwarder: Address,
    token: Address,
    seed: u8,
    overrides: TransferActionOverrides,
) -> anyhow::Result<TransferActionParts> {
    build_transfer_action_with_overrides_and_path(
        resource_to_transfer,
        forwarder,
        token,
        seed,
        overrides,
        None,
    )
}

pub fn build_transfer_action_with_overrides_and_path(
    resource_to_transfer: Resource,
    forwarder: Address,
    token: Address,
    seed: u8,
    overrides: TransferActionOverrides,
    merkle_path: Option<MerklePath>,
) -> anyhow::Result<TransferActionParts> {
    let sender = sender_keychain()?;
    let receiver = receiver_keychain()?;

    let mut consumed = resource_to_transfer;
    if let Some(label_ref) = overrides.consumed_label_ref {
        consumed.label_ref = label_ref;
    }
    if let Some(value_ref) = overrides.consumed_value_ref {
        consumed.value_ref = value_ref;
    }
    if let Some(quantity) = overrides.quantity {
        consumed.quantity = quantity;
    }

    let consumed_nf = consumed.nullifier(&sender.nf_key)?;
    let created = resource::transfer_created_resource(
        &receiver,
        consumed_nf,
        forwarder,
        token,
        consumed.quantity,
        seed,
    )?;

    let action_tree_root = ArmTree::new(vec![consumed_nf, created.commitment()]).root()?;

    let auth_sig = match overrides.auth_signature {
        Some(sig) => sig,
        None => sender
            .auth_signing_key
            .sign(AUTH_SIGNATURE_DOMAIN, action_tree_root.as_bytes()),
    };

    let consumed_logic = TokenTransferWitness::new(
        consumed,
        true,
        action_tree_root,
        Some(sender.nf_key.clone()),
        Some(auth_sig),
        None,
        None,
        None,
        Some(ValueInfo {
            auth_pk: sender.auth_verifying_key(),
            encryption_pk: sender.encryption_pk,
        }),
    );

    let created_logic = TokenTransferWitness::new(
        created,
        false,
        action_tree_root,
        None,
        None,
        Some(EncryptionInfo::new(&receiver.discovery_pk)),
        None,
        Some(LabelInfo {
            forwarder_addr: forwarder.to_vec(),
            erc20_token_addr: token.to_vec(),
        }),
        Some(ValueInfo {
            auth_pk: receiver.auth_verifying_key(),
            encryption_pk: receiver.encryption_pk,
        }),
    );

    let compliance = match merkle_path {
        Some(path) => ComplianceWitness::from_resources_with_path(
            consumed,
            sender.nf_key.clone(),
            path,
            created,
        ),
        None => ComplianceWitness::from_resources(
            consumed,
            *anoma_rm_risc0::compliance::INITIAL_ROOT,
            sender.nf_key.clone(),
            created,
        ),
    };

    Ok(TransferActionParts {
        action: ActionWitnesses {
            compliance_units: vec![ComplianceUnitWitnesses {
                compliance_witness: Box::new(compliance),
                consumed_logic_witness: Box::new(TransferLogicWitness::new(consumed_logic)),
                created_logic_witness: Box::new(TransferLogicWitness::new(created_logic)),
            }],
        },
        consumed_persistent: consumed,
        created_persistent: created,
        sender,
        receiver,
    })
}

pub fn build_unwrap_action(
    resource_to_unwrap: Resource,
    forwarder: Address,
    token: Address,
    seed: u8,
) -> anyhow::Result<ActionWitnesses> {
    Ok(build_unwrap_action_with_overrides(
        resource_to_unwrap,
        forwarder,
        token,
        seed,
        UnwrapActionOverrides::default(),
    )?
    .action)
}

pub fn build_unwrap_action_with_overrides(
    resource_to_unwrap: Resource,
    forwarder: Address,
    token: Address,
    seed: u8,
    overrides: UnwrapActionOverrides,
) -> anyhow::Result<UnwrapActionParts> {
    build_unwrap_action_with_overrides_and_path(
        resource_to_unwrap,
        forwarder,
        token,
        seed,
        overrides,
        None,
    )
}

pub fn build_unwrap_action_with_overrides_and_path(
    resource_to_unwrap: Resource,
    forwarder: Address,
    token: Address,
    seed: u8,
    overrides: UnwrapActionOverrides,
    merkle_path: Option<MerklePath>,
) -> anyhow::Result<UnwrapActionParts> {
    let owner = receiver_keychain()?;

    let mut consumed = resource_to_unwrap;
    if let Some(quantity) = overrides.quantity {
        consumed.quantity = quantity;
    }

    let consumed_nf = consumed.nullifier(&owner.nf_key)?;
    let created = resource::unwrap_created_resource(
        &owner,
        consumed_nf,
        forwarder,
        token,
        consumed.quantity,
        seed,
        &overrides,
    )?;

    let action_tree_root = ArmTree::new(vec![consumed_nf, created.commitment()]).root()?;

    let auth_sig: AuthoritySignature = match overrides.auth_signature {
        Some(sig) => sig,
        None => owner
            .auth_signing_key
            .sign(AUTH_SIGNATURE_DOMAIN, action_tree_root.as_bytes()),
    };

    let consumed_logic = TokenTransferWitness::new(
        consumed,
        true,
        action_tree_root,
        Some(owner.nf_key.clone()),
        Some(auth_sig),
        None,
        None,
        None,
        Some(ValueInfo {
            auth_pk: owner.auth_verifying_key(),
            encryption_pk: owner.encryption_pk,
        }),
    );

    let created_logic = TokenTransferWitness::new(
        created,
        false,
        action_tree_root,
        None,
        None,
        None,
        Some(ForwarderInfo {
            call_type: CallType::Unwrap,
            ethereum_account_addr: owner.ethereum_addr.to_vec(),
            permit_info: None,
        }),
        Some(LabelInfo {
            forwarder_addr: forwarder.to_vec(),
            erc20_token_addr: token.to_vec(),
        }),
        None,
    );

    let compliance = match merkle_path {
        Some(path) => ComplianceWitness::from_resources_with_path(
            consumed,
            owner.nf_key.clone(),
            path,
            created,
        ),
        None => ComplianceWitness::from_resources(
            consumed,
            *anoma_rm_risc0::compliance::INITIAL_ROOT,
            owner.nf_key.clone(),
            created,
        ),
    };

    Ok(UnwrapActionParts {
        action: ActionWitnesses {
            compliance_units: vec![ComplianceUnitWitnesses {
                compliance_witness: Box::new(compliance),
                consumed_logic_witness: Box::new(TransferLogicWitness::new(consumed_logic)),
                created_logic_witness: Box::new(TransferLogicWitness::new(created_logic)),
            }],
        },
        consumed_persistent: consumed,
        created_ephemeral: created,
        owner,
    })
}
