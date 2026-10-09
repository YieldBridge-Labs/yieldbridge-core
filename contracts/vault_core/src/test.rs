#![cfg(test)]

use super::*;
use soroban_sdk::{
    Address, Env, Vec,
    testutils::{Address as _, Ledger as _, storage::Persistent as _},
    token,
};

fn setup_vault(
    stream_duration: u64,
) -> (Env, Address, Address, Address, YieldVaultClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let vault_address = env.register(YieldVault, ());
    let client = YieldVaultClient::new(&env, &vault_address);
    client.initialize(&admin, &token_address, &stream_duration);
    (env, admin, token_address, vault_address, client)
}

#[test]
fn test_initialize_success_and_config() {
    let (_env, admin, token_address, _vault_address, client) = setup_vault(100);
    let (cfg_admin, cfg_token, cfg_dur) = client.configuration();
    assert_eq!(cfg_admin, admin);
    assert_eq!(cfg_token, token_address);
    assert_eq!(cfg_dur, 100);
    assert_eq!(client.total_shares(), 0);
    assert_eq!(client.total_funded(), 0);
    assert_eq!(client.total_claimed(), 0);
    assert_eq!(client.reward_rate(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_initialize_already_initialized() {
    let (_env, admin, token_address, _vault_address, client) = setup_vault(100);
    client.initialize(&admin, &token_address, &100);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_initialize_zero_duration() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let vault_address = env.register(YieldVault, ());
    let client = YieldVaultClient::new(&env, &vault_address);
    client.initialize(&admin, &token_address, &0);
}

#[test]
fn test_set_weights_batch_and_get_shares() {
    let (env, _admin, _token_address, _vault_address, client) = setup_vault(100);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let charlie = Address::generate(&env);

    let mut investors = Vec::new(&env);
    investors.push_back(alice.clone());
    investors.push_back(bob.clone());
    investors.push_back(charlie.clone());

    let mut weights = Vec::new(&env);
    weights.push_back(200);
    weights.push_back(300);
    weights.push_back(500);

    client.set_weights(&investors, &weights);

    assert_eq!(client.get_shares(&alice), 200);
    assert_eq!(client.get_shares(&bob), 300);
    assert_eq!(client.get_shares(&charlie), 500);
    assert_eq!(client.total_shares(), 1000);
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")]
fn test_set_weights_length_mismatch() {
    let (env, _admin, _token_address, _vault_address, client) = setup_vault(100);
    let alice = Address::generate(&env);

    let mut investors = Vec::new(&env);
    investors.push_back(alice);

    let mut weights = Vec::new(&env);
    weights.push_back(100);
    weights.push_back(200);

    client.set_weights(&investors, &weights);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn test_set_weights_duplicate_investor() {
    let (env, _admin, _token_address, _vault_address, client) = setup_vault(100);
    let alice = Address::generate(&env);

    let mut investors = Vec::new(&env);
    investors.push_back(alice.clone());
    investors.push_back(alice);

    let mut weights = Vec::new(&env);
    weights.push_back(100);
    weights.push_back(200);

    client.set_weights(&investors, &weights);
}

#[test]
fn test_single_set_shares_helper() {
    let (env, _admin, _token_address, _vault_address, client) = setup_vault(100);
    let alice = Address::generate(&env);
    client.set_shares(&alice, &50);
    assert_eq!(client.get_shares(&alice), 50);
    assert_eq!(client.total_shares(), 50);

    client.set_shares(&alice, &75);
    assert_eq!(client.get_shares(&alice), 75);
    assert_eq!(client.total_shares(), 75);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_set_shares_negative_rejected() {
    let (env, _admin, _token_address, _vault_address, client) = setup_vault(100);
    let alice = Address::generate(&env);
    client.set_shares(&alice, &-5);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_inject_zero_amount_rejected() {
    let (env, _admin, _token_address, _vault_address, client) = setup_vault(100);
    let alice = Address::generate(&env);
    client.set_shares(&alice, &100);
    client.inject(&0);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_inject_no_shares_rejected() {
    let (_env, _admin, _token_address, _vault_address, client) = setup_vault(100);
    client.inject(&1_000);
}

#[test]
fn test_linear_streaming_and_claim_proportionality() {
    let (env, admin, token_address, _vault_address, client) = setup_vault(10);
    let token_admin = token::StellarAssetClient::new(&env, &token_address);
    let token_client = token::Client::new(&env, &token_address);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    client.set_shares(&alice, &1);
    client.set_shares(&bob, &3);

    token_admin.mint(&admin, &1_000);
    client.inject(&1_000);

    assert_eq!(client.total_funded(), 1_000);
    assert_eq!(client.period_finish(), 10);

    // Halfway through stream (t = 5)
    env.ledger().set_timestamp(5);
    assert_eq!(client.claimable(&alice), 125);
    assert_eq!(client.claimable(&bob), 375);

    let claimed_alice = client.claim(&alice);
    let claimed_bob = client.claim(&bob);
    assert_eq!(claimed_alice, 125);
    assert_eq!(claimed_bob, 375);
    assert_eq!(token_client.balance(&alice), 125);
    assert_eq!(token_client.balance(&bob), 375);
    assert_eq!(client.total_claimed(), 500);

    // Full stream completion (t = 10)
    env.ledger().set_timestamp(10);
    assert_eq!(client.claimable(&alice), 125);
    assert_eq!(client.claimable(&bob), 375);

    assert_eq!(client.claim(&alice), 125);
    assert_eq!(client.claim(&bob), 375);
    assert_eq!(token_client.balance(&alice), 250);
    assert_eq!(token_client.balance(&bob), 750);
    assert_eq!(client.total_claimed(), 1_000);
}

#[test]
fn test_rolling_unvested_schedules_forward() {
    let (env, admin, token_address, _vault_address, client) = setup_vault(10);
    let token_admin = token::StellarAssetClient::new(&env, &token_address);
    let token_client = token::Client::new(&env, &token_address);

    let alice = Address::generate(&env);
    client.set_shares(&alice, &100);

    // First injection: 1_000 over 10 seconds (100 per sec)
    token_admin.mint(&admin, &3_000);
    client.inject(&1_000);
    assert_eq!(client.period_finish(), 10);

    // At t = 5, 500 vested, 500 unvested.
    env.ledger().set_timestamp(5);
    // Second injection of 1_000 at t = 5 extends duration by 10 (finish = 15).
    // Remaining unvested 500 + new 1000 = 1500 over 10s (150 per sec).
    client.inject(&1_000);
    assert_eq!(client.period_finish(), 15);
    assert_eq!(client.total_funded(), 2_000);

    // Claim at t = 5 should claim the 500 vested from schedule 1
    assert_eq!(client.claimable(&alice), 500);
    assert_eq!(client.claim(&alice), 500);
    assert_eq!(token_client.balance(&alice), 500);

    // At t = 10 (5s into schedule 2): 5 * 150 = 750 additional
    env.ledger().set_timestamp(10);
    assert_eq!(client.claimable(&alice), 750);
    assert_eq!(client.claim(&alice), 750);
    assert_eq!(token_client.balance(&alice), 1_250);

    // At t = 15 (end of schedule 2): remaining 750 vested
    env.ledger().set_timestamp(15);
    assert_eq!(client.claimable(&alice), 750);
    assert_eq!(client.claim(&alice), 750);
    assert_eq!(token_client.balance(&alice), 2_000);
    assert_eq!(client.total_claimed(), 2_000);
}

#[test]
fn test_settling_rewards_on_weight_modification_mid_stream() {
    let (env, admin, token_address, _vault_address, client) = setup_vault(10);
    let token_admin = token::StellarAssetClient::new(&env, &token_address);
    let token_client = token::Client::new(&env, &token_address);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    client.set_shares(&alice, &100);
    client.set_shares(&bob, &100);

    token_admin.mint(&admin, &1_000);
    client.inject(&1_000); // 100/sec, 50/sec each

    // At t = 4: each has earned 4 * 50 = 200.
    env.ledger().set_timestamp(4);

    // Now rebalance weights: Alice increases to 300, Bob stays 100 (total = 400).
    // Alice will now get 75/sec, Bob gets 25/sec for remaining 6 seconds.
    let mut investors = Vec::new(&env);
    investors.push_back(alice.clone());
    investors.push_back(bob.clone());
    let mut weights = Vec::new(&env);
    weights.push_back(300);
    weights.push_back(100);
    client.set_weights(&investors, &weights);

    // Advance to end (t = 10)
    env.ledger().set_timestamp(10);

    // Alice: 200 (first 4s) + 6 * 75 (last 6s) = 200 + 450 = 650
    // Bob: 200 (first 4s) + 6 * 25 (last 6s) = 200 + 150 = 350
    assert_eq!(client.claimable(&alice), 650);
    assert_eq!(client.claimable(&bob), 350);

    assert_eq!(client.claim(&alice), 650);
    assert_eq!(client.claim(&bob), 350);
    assert_eq!(token_client.balance(&alice), 650);
    assert_eq!(token_client.balance(&bob), 350);
}

#[test]
fn test_fractional_remainder_preserved_and_exact_whole_units() {
    let (env, admin, token_address, _vault_address, client) = setup_vault(4);
    let token_admin = token::StellarAssetClient::new(&env, &token_address);
    let token_client = token::Client::new(&env, &token_address);

    let alice = Address::generate(&env);
    client.set_shares(&alice, &100);

    // 10 tokens over 4 seconds = 2.5 tokens per second
    token_admin.mint(&admin, &10);
    client.inject(&10);

    // t = 1: 2.5 tokens vested -> whole unit claim is 2, 0.5 sub-unit remains
    env.ledger().set_timestamp(1);
    assert_eq!(client.claimable(&alice), 2);
    assert_eq!(client.claim(&alice), 2);
    assert_eq!(token_client.balance(&alice), 2);

    // t = 2: 2.5 newly accrued + 0.5 remainder = 3.0 claimable -> 3 whole units claimed
    env.ledger().set_timestamp(2);
    assert_eq!(client.claimable(&alice), 3);
    assert_eq!(client.claim(&alice), 3);
    assert_eq!(token_client.balance(&alice), 5);

    // t = 3: 2.5 newly accrued -> 2 claimable, 0.5 sub-unit remains
    env.ledger().set_timestamp(3);
    assert_eq!(client.claimable(&alice), 2);
    assert_eq!(client.claim(&alice), 2);
    assert_eq!(token_client.balance(&alice), 7);

    // t = 4: 2.5 newly accrued + 0.5 remainder = 3.0 claimable -> 3 whole units claimed
    env.ledger().set_timestamp(4);
    assert_eq!(client.claimable(&alice), 3);
    assert_eq!(client.claim(&alice), 3);
    assert_eq!(token_client.balance(&alice), 10);
    assert_eq!(client.total_claimed(), 10);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_claim_zero_amount_panics() {
    let (env, _admin, _token_address, _vault_address, client) = setup_vault(10);
    let alice = Address::generate(&env);
    client.set_shares(&alice, &100);
    // Nothing injected yet -> 0 claimable
    client.claim(&alice);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_cannot_remove_all_shares_during_active_stream() {
    let (env, admin, token_address, _vault_address, client) = setup_vault(10);
    token::StellarAssetClient::new(&env, &token_address).mint(&admin, &1_000);

    let alice = Address::generate(&env);
    client.set_shares(&alice, &100);
    client.inject(&1_000);

    // Active stream until t = 10; trying to set Alice to 0 shares panics
    client.set_shares(&alice, &0);
}

#[test]
fn test_can_remove_all_shares_after_stream_finishes() {
    let (env, admin, token_address, _vault_address, client) = setup_vault(10);
    token::StellarAssetClient::new(&env, &token_address).mint(&admin, &1_000);

    let alice = Address::generate(&env);
    client.set_shares(&alice, &100);
    client.inject(&1_000);

    env.ledger().set_timestamp(11);
    // After period finish, setting shares to 0 is permitted
    client.set_shares(&alice, &0);
    assert_eq!(client.total_shares(), 0);
    assert_eq!(client.claim(&alice), 1_000);
}

#[test]
fn test_ttl_extension_on_mutation_and_claim() {
    let (env, admin, token_address, vault_address, client) = setup_vault(10);
    let investor = Address::generate(&env);
    client.set_shares(&investor, &17);

    token::StellarAssetClient::new(&env, &token_address).mint(&admin, &1_000);
    client.inject(&1_000);
    env.ledger().set_timestamp(5);

    let investor_key = DataKey::Investor(investor.clone());
    let state_key = DataKey::State;

    let seq = env.ledger().sequence();
    env.ledger().set_sequence_number(seq + 450_000);
    client.claim(&investor);

    let (inv_ttl, state_ttl) = env.as_contract(&vault_address, || {
        (
            env.storage().persistent().get_ttl(&investor_key),
            env.storage().persistent().get_ttl(&state_key),
        )
    });
    assert!(inv_ttl >= PERSISTENT_TTL_EXTEND_TO);
    assert!(state_ttl >= PERSISTENT_TTL_EXTEND_TO);
}
