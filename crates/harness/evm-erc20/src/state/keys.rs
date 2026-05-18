pub const KEY_PREFIX_ERC20_ADDR: &str = "evm.erc20.addr";

#[inline]
pub fn erc20_addr_key(symbol: &str) -> String {
    format!("{KEY_PREFIX_ERC20_ADDR}.{symbol}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erc20_addr_key_format() {
        assert_eq!(erc20_addr_key("weth"), "evm.erc20.addr.weth");
    }
}
