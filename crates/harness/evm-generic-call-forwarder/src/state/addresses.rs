use alloy::primitives::Address;
use anyhow::Context;
use pa_test_harness_core::environment::Environment;
use pa_test_harness_core::environment::State;
use pa_test_harness_core::environment::StateBuilder;

use crate::state::keys::generic_call_forwarder_v1_addr_key;

#[inline]
pub fn insert_generic_call_forwarder_v1_address(builder: &mut StateBuilder, address: Address) {
    builder.insert(generic_call_forwarder_v1_addr_key(), address);
}

#[inline]
pub fn generic_call_forwarder_v1_address<E>(env: &E) -> anyhow::Result<Address>
where
    E: Environment,
{
    generic_call_forwarder_v1_address_in_state(env.state())
}

#[inline]
pub fn generic_call_forwarder_v1_address_in_state(state: &State) -> anyhow::Result<Address> {
    state
        .get::<Address>(generic_call_forwarder_v1_addr_key())
        .copied()
        .context("failed to retrieve generic call forwarder v1 address from env")
}

#[cfg(test)]
mod tests {
    use pa_test_harness_core::mocks::MockEnvironment;

    use super::*;

    #[test]
    fn insert_and_resolve_generic_call_forwarder_v1_address() {
        let expected = Address::from([0x41; 20]);

        let state = {
            let mut builder = StateBuilder::new();
            insert_generic_call_forwarder_v1_address(&mut builder, expected);
            builder.finalize()
        };

        let mut env = MockEnvironment::new();
        env.expect_state().return_const(state);

        let resolved = generic_call_forwarder_v1_address(&env)
            .expect("must resolve stored generic call forwarder address");
        assert_eq!(resolved, expected);
    }

    #[test]
    fn missing_generic_call_forwarder_v1_address_errors() {
        let state = StateBuilder::new().finalize();

        let mut env = MockEnvironment::new();
        env.expect_state().return_const(state);

        let err = generic_call_forwarder_v1_address(&env)
            .expect_err("must fail on missing generic call forwarder address key");
        assert!(
            err.to_string()
                .contains("failed to retrieve generic call forwarder v1 address from env"),
            "error should mention missing generic call forwarder address, got: {err}"
        );
    }
}
