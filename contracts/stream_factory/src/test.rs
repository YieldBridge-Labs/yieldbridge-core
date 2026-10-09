#![cfg(test)]

use super::*;
use soroban_sdk::{
    Address, BytesN, Env,
    testutils::{Address as _, Events as _, Ledger as _, storage::Persistent as _},
};

mod vault_wasm {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/vault_core.wasm");
}

fn setup_factory() -> (
    Env,
    Address,
    BytesN<32>,
    Address,
    StreamFactoryClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let admin = Address::generate(&env);
    let wasm_hash = env.deployer().upload_contract_wasm(vault_wasm::WASM);
    let factory_address = env.register(StreamFactory, ());
    let client = StreamFactoryClient::new(&env, &factory_address);
    client.initialize(&admin, &wasm_hash);
    (env, admin, wasm_hash, factory_address, client)
}

#[test]
fn test_factory_initialize_and_configuration() {
    let (_env, admin, wasm_hash, _factory_address, client) = setup_factory();
    let (cfg_admin, cfg_hash) = client.configuration();
    assert_eq!(cfg_admin, admin);
    assert_eq!(cfg_hash, wasm_hash);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_factory_double_initialize_rejected() {
    let (_env, admin, wasm_hash, _factory_address, client) = setup_factory();
    client.initialize(&admin, &wasm_hash);
}

#[test]
fn test_set_vault_wasm_hash() {
    let (env, admin, old_hash, _factory_address, client) = setup_factory();
    let new_hash = BytesN::from_array(&env, &[9u8; 32]);
    assert_ne!(old_hash, new_hash);

    client.set_vault_wasm_hash(&new_hash);
    let (cfg_admin, cfg_hash) = client.configuration();
    assert_eq!(cfg_admin, admin);
    assert_eq!(cfg_hash, new_hash);
}

#[test]
fn test_create_vault_deterministic_deployment_and_registry() {
    let (env, admin, _wasm_hash, _factory_address, client) = setup_factory();
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let salt = BytesN::from_array(&env, &[42u8; 32]);

    assert_eq!(client.vault_for_salt(&salt), None);

    let vault_address = client.create_vault(&salt, &token_address, &3600);

    // Verify event emission from the create_vault invocation
    let events = env.events().all();
    assert!(!events.events().is_empty());

    // Verify registry record
    assert_eq!(client.vault_for_salt(&salt), Some(vault_address.clone()));

    // Verify deployed vault is initialized with factory admin, token, duration
    let vault_client = vault_wasm::Client::new(&env, &vault_address);
    let (v_admin, v_token, v_dur) = vault_client.configuration();
    assert_eq!(v_admin, admin);
    assert_eq!(v_token, token_address);
    assert_eq!(v_dur, 3600);
}

#[test]
fn test_deploy_with_custom_admin() {
    let (env, _factory_admin, _wasm_hash, _factory_address, client) = setup_factory();
    let custom_admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let salt = BytesN::from_array(&env, &[77u8; 32]);
    let vault_address = client.deploy(&salt, &custom_admin, &token_address, &7200);

    assert_eq!(client.vault_for_salt(&salt), Some(vault_address.clone()));

    let vault_client = vault_wasm::Client::new(&env, &vault_address);
    let (v_admin, v_token, v_dur) = vault_client.configuration();
    assert_eq!(v_admin, custom_admin);
    assert_eq!(v_token, token_address);
    assert_eq!(v_dur, 7200);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_duplicate_salt_deployment_rejected() {
    let (env, _admin, _wasm_hash, _factory_address, client) = setup_factory();
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let salt = BytesN::from_array(&env, &[1u8; 32]);
    client.create_vault(&salt, &token_address, &3600);
    // Deploying with same salt panics with AlreadyDeployed
    client.create_vault(&salt, &token_address, &3600);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_deploy_zero_duration_rejected() {
    let (env, _admin, _wasm_hash, _factory_address, client) = setup_factory();
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let salt = BytesN::from_array(&env, &[2u8; 32]);
    client.create_vault(&salt, &token_address, &0);
}

#[test]
fn test_factory_registry_ttl_extension() {
    let (env, _admin, _wasm_hash, factory_address, client) = setup_factory();
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let salt = BytesN::from_array(&env, &[55u8; 32]);
    let vault_addr = client.create_vault(&salt, &token_address, &1000);

    let vault_key = DataKey::Vault(salt.clone());
    let seq = env.ledger().sequence();
    env.ledger().set_sequence_number(seq + 450_000);

    // Calling vault_for_salt refreshes TTL
    let resolved = client.vault_for_salt(&salt);
    assert_eq!(resolved, Some(vault_addr));

    let ttl = env.as_contract(&factory_address, || {
        env.storage().persistent().get_ttl(&vault_key)
    });
    assert!(ttl >= PERSISTENT_TTL_EXTEND_TO);
}
