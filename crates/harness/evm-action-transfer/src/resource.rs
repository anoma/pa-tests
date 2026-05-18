use alloy::primitives::Address;
use alloy::primitives::U256;
use anoma_rm_risc0::Digest;
use anoma_rm_risc0::resource::Resource;
use transfer_witness::ValueInfo;
use transfer_witness::calculate_label_ref;
use transfer_witness::calculate_persistent_value_ref;
use transfer_witness::calculate_value_ref_from_ethereum_account_addr;

use crate::fixtures::TransferKeychain;

pub const TOKEN_TRANSFER_VK: Digest = Digest::from_bytes([
    0xbc, 0x12, 0x32, 0x36, 0x68, 0xc3, 0x7c, 0x3d, 0x38, 0x1c, 0xa7, 0x98, 0xf1, 0x11, 0x16, 0xf3,
    0x5f, 0xb1, 0x63, 0x9d, 0x12, 0x23, 0x9b, 0x29, 0xda, 0x78, 0x10, 0xdf, 0x39, 0x85, 0xe7, 0xad,
]);

#[derive(Clone, Debug, Default)]
pub struct WrapActionOverrides {
    pub quantity: Option<u128>,
    pub consumed_value_ref: Option<Digest>,
    pub consumed_label_ref: Option<Digest>,
    pub consumed_is_ephemeral: Option<bool>,
    pub permit_signature: Option<Vec<u8>>,
}

impl WrapActionOverrides {
    pub fn invalid_non_ephemeral_consumed() -> Self {
        Self {
            consumed_is_ephemeral: Some(false),
            ..Self::default()
        }
    }

    pub fn invalid_label_ref() -> Self {
        Self {
            consumed_label_ref: Some(Digest::default()),
            ..Self::default()
        }
    }

    pub fn invalid_permit_signature_length() -> Self {
        Self {
            permit_signature: Some(vec![7u8; 64]),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct TransferActionOverrides {
    pub quantity: Option<u128>,
    pub consumed_label_ref: Option<Digest>,
    pub consumed_value_ref: Option<Digest>,
    pub auth_signature: Option<anoma_rm_risc0_gadgets::authority::AuthoritySignature>,
}

impl TransferActionOverrides {
    pub fn invalid_label_ref() -> Self {
        Self {
            consumed_label_ref: Some(Digest::default()),
            ..Self::default()
        }
    }

    pub fn invalid_value_ref() -> Self {
        Self {
            consumed_value_ref: Some(Digest::default()),
            ..Self::default()
        }
    }

    pub fn invalid_auth_signature() -> Self {
        let fake = anoma_rm_risc0_gadgets::authority::AuthoritySigningKey::from_bytes(&[9u8; 32])
            .expect("valid deterministic auth key bytes")
            .sign(transfer_witness::AUTH_SIGNATURE_DOMAIN, &[0u8; 32]);
        Self {
            auth_signature: Some(fake),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct UnwrapActionOverrides {
    pub quantity: Option<u128>,
    pub created_value_ref: Option<Digest>,
    pub created_label_ref: Option<Digest>,
    pub created_is_ephemeral: Option<bool>,
    pub auth_signature: Option<anoma_rm_risc0_gadgets::authority::AuthoritySignature>,
}

impl UnwrapActionOverrides {
    pub fn invalid_created_non_ephemeral() -> Self {
        Self {
            created_is_ephemeral: Some(false),
            ..Self::default()
        }
    }

    pub fn invalid_value_ref() -> Self {
        Self {
            created_value_ref: Some(Digest::default()),
            ..Self::default()
        }
    }
}

pub fn wrap_consumed_resource(
    sender: &TransferKeychain,
    forwarder: Address,
    token: Address,
    quantity: u128,
    seed: u8,
    overrides: &WrapActionOverrides,
) -> Resource {
    let label_ref = overrides
        .consumed_label_ref
        .unwrap_or(calculate_label_ref(forwarder.as_ref(), token.as_ref()));
    let value_ref =
        overrides
            .consumed_value_ref
            .unwrap_or(calculate_value_ref_from_ethereum_account_addr(
                sender.ethereum_addr.as_ref(),
            ));

    Resource {
        logic_ref: TOKEN_TRANSFER_VK,
        label_ref,
        quantity: overrides.quantity.unwrap_or(quantity),
        value_ref,
        is_ephemeral: overrides.consumed_is_ephemeral.unwrap_or(true),
        nonce: [seed; 32],
        nk_commitment: sender.nf_key.commit(),
        rand_seed: [seed.wrapping_add(17); 32],
    }
}

pub fn wrap_created_resource(
    sender: &TransferKeychain,
    consumed_nullifier: Digest,
    forwarder: Address,
    token: Address,
    quantity: u128,
    seed: u8,
) -> anyhow::Result<Resource> {
    let nonce: [u8; 32] = consumed_nullifier
        .as_bytes()
        .try_into()
        .map_err(|_| anyhow::anyhow!("nullifier must be 32 bytes"))?;

    Ok(Resource {
        logic_ref: TOKEN_TRANSFER_VK,
        label_ref: calculate_label_ref(forwarder.as_ref(), token.as_ref()),
        quantity,
        value_ref: calculate_persistent_value_ref(&ValueInfo {
            auth_pk: sender.auth_verifying_key(),
            encryption_pk: sender.encryption_pk,
        }),
        is_ephemeral: false,
        nonce,
        nk_commitment: sender.nf_key.commit(),
        rand_seed: [seed.wrapping_add(33); 32],
    })
}

pub fn transfer_created_resource(
    receiver: &TransferKeychain,
    consumed_nullifier: Digest,
    forwarder: Address,
    token: Address,
    quantity: u128,
    seed: u8,
) -> anyhow::Result<Resource> {
    let nonce: [u8; 32] = consumed_nullifier
        .as_bytes()
        .try_into()
        .map_err(|_| anyhow::anyhow!("nullifier must be 32 bytes"))?;

    Ok(Resource {
        logic_ref: TOKEN_TRANSFER_VK,
        label_ref: calculate_label_ref(forwarder.as_ref(), token.as_ref()),
        quantity,
        value_ref: calculate_persistent_value_ref(&ValueInfo {
            auth_pk: receiver.auth_verifying_key(),
            encryption_pk: receiver.encryption_pk,
        }),
        is_ephemeral: false,
        nonce,
        nk_commitment: receiver.nf_key.commit(),
        rand_seed: [seed.wrapping_add(51); 32],
    })
}

pub fn unwrap_created_resource(
    owner: &TransferKeychain,
    consumed_nullifier: Digest,
    forwarder: Address,
    token: Address,
    quantity: u128,
    seed: u8,
    overrides: &UnwrapActionOverrides,
) -> anyhow::Result<Resource> {
    let nonce: [u8; 32] = consumed_nullifier
        .as_bytes()
        .try_into()
        .map_err(|_| anyhow::anyhow!("nullifier must be 32 bytes"))?;

    let label_ref = overrides
        .created_label_ref
        .unwrap_or(calculate_label_ref(forwarder.as_ref(), token.as_ref()));
    let value_ref =
        overrides
            .created_value_ref
            .unwrap_or(calculate_value_ref_from_ethereum_account_addr(
                owner.ethereum_addr.as_ref(),
            ));

    Ok(Resource {
        logic_ref: TOKEN_TRANSFER_VK,
        label_ref,
        quantity: overrides.quantity.unwrap_or(quantity),
        value_ref,
        is_ephemeral: overrides.created_is_ephemeral.unwrap_or(true),
        nonce,
        nk_commitment: owner.nf_key.commit(),
        rand_seed: [seed.wrapping_add(71); 32],
    })
}

pub fn random_permit_nonce(seed: u8) -> U256 {
    U256::from(seed as u64 + 1)
}
