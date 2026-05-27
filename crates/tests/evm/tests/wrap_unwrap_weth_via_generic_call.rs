use alloy::primitives::U256;
use alloy::providers::Provider;
use alloy::sol_types::SolCall;
use anyhow::Context;
#[cfg(feature = "e2e")]
use pa_evm_tests::setup_transfer_generic_call_e2e_env;
use pa_evm_tests::{
    commitment_root, execute_tx, generic_call_forwarder_address, prove_actions,
    setup_transfer_generic_call_integration_env, transfer_chain_id, transfer_forwarder_address,
    transfer_weth_token_address,
};
use pa_test_harness_core::environment::CommitmentTree;
use pa_test_harness_core::environment::Environment;
use pa_test_harness_core::environment::ProtocolAdapter;
use pa_test_harness_evm::state::actors::default_signer;
use pa_test_harness_evm_action_generic_call::build_generic_call_action;
use pa_test_harness_evm_action_transfer::{
    TransferActionOverrides, UnwrapActionOverrides, WrapActionOverrides,
    build_transfer_action_with_overrides_and_path, build_unwrap_action_with_overrides_and_path,
    build_wrap_action_with_overrides, receiver_keychain, sender_keychain,
};
use pa_test_harness_evm_erc20::ierc20_bindings::ierc20;
use pa_test_harness_evm_erc20::weth_bindings::WETH9;
use rstest::*;

use generic_call_witness::GenericCall;

#[rstest]
#[case::integration_test(setup_transfer_generic_call_integration_env())]
#[cfg_attr(feature = "e2e", case::e2e_test(setup_transfer_generic_call_e2e_env()))]
#[tokio::test]
async fn wrap_unwrap_weth_via_generic_call<Env: Environment>(
    #[future(awt)]
    #[case]
    env_with_setup: anyhow::Result<Env>,
) -> anyhow::Result<()> {
    let mut env = env_with_setup.context("env setup failed")?;
    let chain_id = transfer_chain_id(&env).await?;
    let erc20_forwarder = transfer_forwarder_address(&env)?;
    let generic_forwarder = generic_call_forwarder_address(&env)?;
    let weth = transfer_weth_token_address(&env)?;
    let provider = default_signer(&env).context("failed to retrieve default signer")?;

    let amount = 1u128;
    let amount_u256 = U256::from(amount);
    let sender = sender_keychain().context("failed to build sender keychain")?;
    let recipient = receiver_keychain().context("failed to build recipient keychain")?;

    let before_root = commitment_root(&env)?;

    let sender_weth_before = ierc20(weth, provider.clone())
        .balanceOf(sender.ethereum_addr)
        .call()
        .await
        .context("failed to read sender WETH balance before wrap")?;
    let erc20_forwarder_weth_before_wrap = ierc20(weth, provider.clone())
        .balanceOf(erc20_forwarder)
        .call()
        .await
        .context("failed to read ERC20 forwarder WETH balance before wrap")?;
    let generic_forwarder_weth_before_unwrap = ierc20(weth, provider.clone())
        .balanceOf(generic_forwarder)
        .call()
        .await
        .context("failed to read generic call forwarder WETH balance before unwrap")?;
    let rand_seed = 11;

    let wrapped = build_wrap_action_with_overrides(
        chain_id,
        erc20_forwarder,
        weth,
        amount,
        rand_seed,
        WrapActionOverrides::default(),
    )
    .await
    .context("failed to build wrap action")?;
    let tx = prove_actions(&env, &[wrapped.action])
        .await
        .context("failed to prove wrap action")?;
    execute_tx(&mut env, tx)
        .await
        .context("failed to execute wrap action")?;

    let sender_weth_after_wrap = ierc20(weth, provider.clone())
        .balanceOf(sender.ethereum_addr)
        .call()
        .await
        .context("failed to read sender WETH balance after wrap")?;
    anyhow::ensure!(
        sender_weth_before - sender_weth_after_wrap == amount_u256,
        "sender WETH must decrease by wrapped amount"
    );

    let erc20_forwarder_weth_after_wrap = ierc20(weth, provider.clone())
        .balanceOf(erc20_forwarder)
        .call()
        .await
        .context("failed to read ERC20 forwarder WETH balance after wrap")?;
    anyhow::ensure!(
        erc20_forwarder_weth_after_wrap - erc20_forwarder_weth_before_wrap == amount_u256,
        "ERC20 forwarder WETH must equal wrapped amount"
    );

    let transfer_merkle_path = env
        .protocol_adapter()
        .commitment_tree()
        .path_to(wrapped.created_persistent.commitment())
        .context("failed to generate transfer merkle path")?;

    let rand_seed = 17;

    let transferred = build_transfer_action_with_overrides_and_path(
        wrapped.created_persistent,
        erc20_forwarder,
        weth,
        rand_seed,
        TransferActionOverrides::default(),
        Some(transfer_merkle_path),
    )
    .context("failed to build transfer action")?;
    let tx = prove_actions(&env, &[transferred.action])
        .await
        .context("failed to prove transfer action")?;
    execute_tx(&mut env, tx)
        .await
        .context("failed to execute transfer action")?;

    let unwrap_merkle_path = env
        .protocol_adapter()
        .commitment_tree()
        .path_to(transferred.created_persistent.commitment())
        .context("failed to generate unwrap merkle path")?;

    let rand_seed = 21;

    let unwrapped = build_unwrap_action_with_overrides_and_path(
        transferred.created_persistent,
        erc20_forwarder,
        weth,
        rand_seed,
        UnwrapActionOverrides {
            unwrap_ethereum_account_addr: Some(generic_forwarder.to_vec()),
            ..UnwrapActionOverrides::default()
        },
        Some(unwrap_merkle_path),
    )
    .context("failed to build unwrap action")?;

    let calls = vec![
        GenericCall {
            to: weth.to_vec(),
            value: 0,
            data: WETH9::withdrawCall { wad: amount_u256 }.abi_encode(),
        },
        GenericCall {
            to: recipient.ethereum_addr.to_vec(),
            value: amount,
            data: Vec::new(),
        },
    ];

    let rand_seed = 31;

    let generic_call_action =
        build_generic_call_action(rand_seed, generic_forwarder.to_vec(), calls)
            .context("failed to build generic call action")?;

    let tx = prove_actions(&env, &[unwrapped.action])
        .await
        .context("failed to prove unwrap action")?;
    execute_tx(&mut env, tx)
        .await
        .context("failed to execute unwrap action")?;

    let generic_forwarder_weth_after_unwrap = ierc20(weth, provider.clone())
        .balanceOf(generic_forwarder)
        .call()
        .await
        .context("failed to read generic call forwarder WETH balance after unwrap")?;
    anyhow::ensure!(
        generic_forwarder_weth_after_unwrap - generic_forwarder_weth_before_unwrap == amount_u256,
        "generic call forwarder WETH must equal unwrap amount before generic call"
    );

    let recipient_eth_before = provider
        .get_balance(recipient.ethereum_addr)
        .await
        .context("failed to read recipient ETH balance before")?;

    let tx = prove_actions(&env, &[generic_call_action])
        .await
        .context("failed to prove generic call action")?;
    execute_tx(&mut env, tx)
        .await
        .context("failed to execute generic call action")?;

    let erc20_forwarder_weth_after = ierc20(weth, provider.clone())
        .balanceOf(erc20_forwarder)
        .call()
        .await
        .context("failed to read ERC20 forwarder WETH balance after generic call")?;
    anyhow::ensure!(
        erc20_forwarder_weth_after == U256::ZERO,
        "ERC20 forwarder WETH should be zero after unwrap"
    );

    let generic_forwarder_weth_after = ierc20(weth, provider.clone())
        .balanceOf(generic_forwarder)
        .call()
        .await
        .context("failed to read generic call forwarder WETH balance after generic call")?;
    anyhow::ensure!(
        generic_forwarder_weth_after == U256::ZERO,
        "generic call forwarder WETH should be zero after withdraw"
    );

    let generic_forwarder_eth_after = provider
        .get_balance(generic_forwarder)
        .await
        .context("failed to read generic call forwarder ETH balance after generic call")?;
    anyhow::ensure!(
        generic_forwarder_eth_after == U256::ZERO,
        "generic call forwarder ETH should be zero after forwarding"
    );

    let recipient_eth_after = provider
        .get_balance(recipient.ethereum_addr)
        .await
        .context("failed to read recipient ETH balance after")?;
    anyhow::ensure!(
        recipient_eth_after - recipient_eth_before == amount_u256,
        "recipient ETH must increase by transfer amount"
    );

    let after_root = commitment_root(&env)?;
    anyhow::ensure!(
        before_root != after_root,
        "commitment tree root must change"
    );

    Ok(())
}
