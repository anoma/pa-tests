use alloy_chains::NamedChain;

pub const KEY_PREFIX_CHAIN_ID: &str = "evm.chain.id";

#[inline]
pub fn chain_id_key(chain: NamedChain) -> String {
    format!("{KEY_PREFIX_CHAIN_ID}.{}", chain.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_id_key_uses_named_chain_slug() {
        assert_eq!(chain_id_key(NamedChain::Sepolia), "evm.chain.id.sepolia");
        assert_eq!(chain_id_key(NamedChain::Mainnet), "evm.chain.id.mainnet");
    }
}
