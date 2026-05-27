pub const KEY_GENERIC_CALL_FORWARDER_V1_ADDRESS: &str = "evm.forwarder.generic-call.v1";

#[inline]
pub fn generic_call_forwarder_v1_addr_key() -> &'static str {
    KEY_GENERIC_CALL_FORWARDER_V1_ADDRESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_call_forwarder_v1_key_format() {
        assert_eq!(
            generic_call_forwarder_v1_addr_key(),
            "evm.forwarder.generic-call.v1"
        );
    }
}
