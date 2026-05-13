use alloy::primitives::Address;
use pa_test_harness_core::environment::Environment;
use pa_test_harness_core::environment::State;
use pa_test_harness_core::environment::StateBuilder;

use crate::keys::erc20_addr_key;

#[inline]
pub fn insert_erc20_address(builder: &mut StateBuilder, symbol: &str, address: Address) {
    builder.insert(erc20_addr_key(symbol), address);
}

#[inline]
pub fn erc20_address<E>(env: &E, symbol: &str) -> anyhow::Result<Address>
where
    E: Environment,
{
    erc20_address_in_state(env.state(), symbol)
}

#[inline]
pub fn erc20_address_in_state(state: &State, symbol: &str) -> anyhow::Result<Address> {
    state.get::<Address>(&erc20_addr_key(symbol)).copied()
}

#[cfg(test)]
mod tests {
    use pa_test_harness_core::mocks::MockEnvironment;

    use super::*;

    #[test]
    fn insert_and_resolve_erc20_address() {
        let expected = Address::from([0x11; 20]);

        let state = {
            let mut builder = StateBuilder::new();

            insert_erc20_address(&mut builder, "weth", expected);

            builder.finalize()
        };

        let mut env = MockEnvironment::new();
        env.expect_state().return_const(state);

        let resolved = erc20_address(&env, "weth").expect("must resolve stored token address");
        assert_eq!(resolved, expected);
    }
}
