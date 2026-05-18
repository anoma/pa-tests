pub const KEY_PREFIX_ERC20_FORWARDER_V1: &str = "evm.forwarder.erc20.v1";

#[inline]
pub fn erc20_forwarder_v1_addr_key() -> &'static str {
    KEY_PREFIX_ERC20_FORWARDER_V1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erc20_forwarder_v1_key_format() {
        assert_eq!(erc20_forwarder_v1_addr_key(), "evm.forwarder.erc20.v1");
    }
}
