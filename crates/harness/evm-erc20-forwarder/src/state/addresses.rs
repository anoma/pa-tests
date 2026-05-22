use alloy::primitives::Address;
use anyhow::Context;
use pa_test_harness_core::environment::Environment;
use pa_test_harness_core::environment::State;
use pa_test_harness_core::environment::StateBuilder;

use crate::state::keys::erc20_forwarder_v1_addr_key;

#[inline]
pub fn insert_erc20_forwarder_v1_address(builder: &mut StateBuilder, address: Address) {
    builder.insert(erc20_forwarder_v1_addr_key(), address);
}

#[inline]
pub fn erc20_forwarder_v1_address<E>(env: &E) -> anyhow::Result<Address>
where
    E: Environment,
{
    erc20_forwarder_v1_address_in_state(env.state())
}

#[inline]
pub fn erc20_forwarder_v1_address_in_state(state: &State) -> anyhow::Result<Address> {
    state
        .get::<Address>(erc20_forwarder_v1_addr_key())
        .copied()
        .context("failed to retrieve ERC20 forwarder v1 address from env")
}

#[cfg(test)]
mod tests {
    use pa_test_harness_core::mocks::MockEnvironment;

    use super::*;

    #[test]
    fn insert_and_resolve_erc20_forwarder_v1_address() {
        let expected = Address::from([0x22; 20]);

        let state = {
            let mut builder = StateBuilder::new();
            insert_erc20_forwarder_v1_address(&mut builder, expected);
            builder.finalize()
        };

        let mut env = MockEnvironment::new();
        env.expect_state().return_const(state);

        let resolved =
            erc20_forwarder_v1_address(&env).expect("must resolve stored forwarder address");
        assert_eq!(resolved, expected);
    }

    #[test]
    fn missing_erc20_forwarder_v1_address_errors() {
        let state = StateBuilder::new().finalize();

        let mut env = MockEnvironment::new();
        env.expect_state().return_const(state);

        let err = erc20_forwarder_v1_address(&env)
            .expect_err("must fail on missing forwarder address key");
        assert!(
            err.to_string()
                .contains("failed to retrieve ERC20 forwarder v1 address from env"),
            "error should mention missing forwarder address, got: {err}"
        );
    }
}
