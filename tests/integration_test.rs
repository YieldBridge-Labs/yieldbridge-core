use soroban_sdk::{
    Address, BytesN, Env, Vec,
    testutils::{Address as _, Ledger as _, storage::Persistent as _},
    token,
};
use stream_factory::{StreamFactory, StreamFactoryClient};
use vault_core::{DataKey, PERSISTENT_TTL_EXTEND_TO, YieldVault, YieldVaultClient};

mod vault_wasm {
    soroban_sdk::contractimport!(file = "./test_fixtures/vault_core.wasm");
}

fn setup() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let vault_address = env.register(YieldVault, ());
    YieldVaultClient::new(&env, &vault_address).initialize(&admin, &token_address, &10);
    (env, admin, token_address, vault_address)
}

#[test]
fn streams_funding_proportionally_to_share_weights() {
    let (env, admin, token_address, vault_address) = setup();
    let vault = YieldVaultClient::new(&env, &vault_address);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let token_admin = token::StellarAssetClient::new(&env, &token_address);
    let token_client = token::Client::new(&env, &token_address);

    let mut investors = Vec::new(&env);
    investors.push_back(alice.clone());
    investors.push_back(bob.clone());
    let mut weights = Vec::new(&env);
    weights.push_back(1);
    weights.push_back(3);

    vault.set_weights(&investors, &weights);
    token_admin.mint(&admin, &1_000);
    vault.inject(&1_000);

    env.ledger().set_timestamp(5);
    assert_eq!(vault.claimable(&alice), 125);
    assert_eq!(vault.claimable(&bob), 375);
    assert_eq!(vault.claim(&alice), 125);
    assert_eq!(vault.claim(&bob), 375);
    assert_eq!(token_client.balance(&alice), 125);
    assert_eq!(token_client.balance(&bob), 375);

    env.ledger().set_timestamp(10);
    assert_eq!(vault.claimable(&alice), 125);
    assert_eq!(vault.claimable(&bob), 375);
    assert_eq!(vault.claim(&alice), 125);
    assert_eq!(vault.claim(&bob), 375);
    assert_eq!(token_client.balance(&alice), 250);
    assert_eq!(token_client.balance(&bob), 750);
}

#[test]
fn investor_entries_receive_the_configured_persistent_ttl() {
    let (env, admin, token_address, vault_address) = setup();
    let vault = YieldVaultClient::new(&env, &vault_address);
    let investor = Address::generate(&env);
    vault.set_shares(&investor, &17);
    token::StellarAssetClient::new(&env, &token_address).mint(&admin, &1_000);
    vault.inject(&1_000);
    env.ledger().set_timestamp(5);

    let investor_key = DataKey::Investor(investor.clone());
    let state_key = DataKey::State;
    let seq = env.ledger().sequence();
    env.ledger().set_sequence_number(seq + 450_000);
    vault.claim(&investor);

    let ttl = env.as_contract(&vault_address, || {
        (
            env.storage().persistent().get_ttl(&investor_key),
            env.storage().persistent().get_ttl(&state_key),
        )
    });
    assert!(ttl.0 >= PERSISTENT_TTL_EXTEND_TO);
    assert!(ttl.1 >= PERSISTENT_TTL_EXTEND_TO);
}

#[test]
fn factory_initialization_persists_admin_and_vault_code_hash() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let admin = Address::generate(&env);
    let wasm_hash = BytesN::from_array(&env, &[7; 32]);
    let factory_address = env.register(StreamFactory, ());
    let factory = StreamFactoryClient::new(&env, &factory_address);

    factory.initialize(&admin, &wasm_hash);

    assert_eq!(factory.configuration(), (admin, wasm_hash));
}

#[test]
fn factory_deployed_vault_end_to_end_streaming() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let admin = Address::generate(&env);
    let wasm_hash = env.deployer().upload_contract_wasm(vault_wasm::WASM);
    let factory_address = env.register(StreamFactory, ());
    let factory = StreamFactoryClient::new(&env, &factory_address);
    factory.initialize(&admin, &wasm_hash);

    let token_admin = token::StellarAssetClient::new(
        &env,
        &env.register_stellar_asset_contract_v2(admin.clone())
            .address(),
    );
    let token_address = token_admin.address.clone();
    let token_client = token::Client::new(&env, &token_address);

    let salt = BytesN::from_array(&env, &[99u8; 32]);
    let vault_address = factory.create_vault(&salt, &token_address, &20);
    assert_eq!(factory.vault_for_salt(&salt), Some(vault_address.clone()));

    let vault = vault_wasm::Client::new(&env, &vault_address);
    let alice = Address::generate(&env);
    vault.set_shares(&alice, &100);

    token_admin.mint(&admin, &2_000);
    vault.inject(&2_000);

    env.ledger().set_timestamp(10);
    assert_eq!(vault.claimable(&alice), 1_000);
    assert_eq!(vault.claim(&alice), 1_000);
    assert_eq!(token_client.balance(&alice), 1_000);

    env.ledger().set_timestamp(20);
    assert_eq!(vault.claimable(&alice), 1_000);
    assert_eq!(vault.claim(&alice), 1_000);
    assert_eq!(token_client.balance(&alice), 2_000);
}
