#![no_std]

//! Administrator-controlled deterministic deployment and registry for YieldVault instances.

#[cfg(test)]
mod test;

use soroban_sdk::{
    Address, BytesN, ContractExecutable, Env, contract, contractclient, contracterror,
    contractevent, contractimpl, contracttype,
};

#[contractclient(name = "YieldVaultClient")]
pub trait YieldVaultInterface {
    fn initialize(env: Env, admin: Address, token: Address, duration: u64);
}

pub const PERSISTENT_TTL_THRESHOLD: u32 = 100_000;
pub const PERSISTENT_TTL_EXTEND_TO: u32 = 535_680;

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum FactoryError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    AlreadyDeployed = 4,
    InvalidDuration = 5,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactoryInitialized {
    pub admin: Address,
    pub vault_wasm_hash: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultWasmHashUpdated {
    pub vault_wasm_hash: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultDeployed {
    pub vault: Address,
    pub salt: BytesN<32>,
    pub wasm_hash: BytesN<32>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    VaultWasmHash,
    Vault(BytesN<32>),
}

#[contract]
pub struct StreamFactory;

#[contractimpl]
impl StreamFactory {
    /// Initializes the factory with administrator and pre-uploaded vault WASM hash.
    pub fn initialize(env: Env, admin: Address, vault_wasm_hash: BytesN<32>) {
        let admin_key = DataKey::Admin;
        if env.storage().persistent().has(&admin_key) {
            soroban_sdk::panic_with_error!(&env, FactoryError::AlreadyInitialized);
        }
        admin.require_auth();
        persist(&env, &admin_key, &admin);
        persist(&env, &DataKey::VaultWasmHash, &vault_wasm_hash);

        FactoryInitialized {
            admin,
            vault_wasm_hash,
        }
        .publish(&env);
    }

    /// Replaces the code hash used for subsequent vault deployments.
    pub fn set_vault_wasm_hash(env: Env, vault_wasm_hash: BytesN<32>) {
        let admin: Address = read(&env, &DataKey::Admin)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, FactoryError::NotInitialized));
        admin.require_auth();
        persist(&env, &DataKey::VaultWasmHash, &vault_wasm_hash);

        VaultWasmHashUpdated { vault_wasm_hash }.publish(&env);
    }

    /// Deploys and initializes a vault at the deterministic address derived
    /// from this factory and caller-provided 32-byte salt, using factory admin as vault admin.
    pub fn create_vault(
        env: Env,
        salt: BytesN<32>,
        token: Address,
        stream_duration: u64,
    ) -> Address {
        let admin: Address = read(&env, &DataKey::Admin)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, FactoryError::NotInitialized));
        admin.require_auth();
        Self::deploy_internal(&env, salt, admin, token, stream_duration)
    }

    /// Deploys and initializes a vault at the deterministic address derived
    /// from this factory and caller-provided 32-byte salt, with explicit admin.
    pub fn deploy(
        env: Env,
        salt: BytesN<32>,
        admin: Address,
        token: Address,
        duration: u64,
    ) -> Address {
        let factory_admin: Address = read(&env, &DataKey::Admin)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, FactoryError::NotInitialized));
        factory_admin.require_auth();
        Self::deploy_internal(&env, salt, admin, token, duration)
    }

    fn deploy_internal(
        env: &Env,
        salt: BytesN<32>,
        admin: Address,
        token: Address,
        duration: u64,
    ) -> Address {
        if duration == 0 {
            soroban_sdk::panic_with_error!(env, FactoryError::InvalidDuration);
        }
        let vault_key = DataKey::Vault(salt.clone());
        if env.storage().persistent().has(&vault_key) {
            soroban_sdk::panic_with_error!(env, FactoryError::AlreadyDeployed);
        }

        let wasm_hash: BytesN<32> = read(env, &DataKey::VaultWasmHash)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, FactoryError::NotInitialized));

        let vault_address = env
            .deployer()
            .with_current_contract(salt.clone())
            .deploy_contract(ContractExecutable::Wasm(wasm_hash.clone()), ());

        YieldVaultClient::new(env, &vault_address).initialize(&admin, &token, &duration);
        persist(env, &vault_key, &vault_address);

        VaultDeployed {
            vault: vault_address.clone(),
            salt,
            wasm_hash,
        }
        .publish(env);

        vault_address
    }

    /// Resolves a salt to its deployed vault address, refreshing entry TTL.
    pub fn vault_for_salt(env: Env, salt: BytesN<32>) -> Option<Address> {
        let key = DataKey::Vault(salt);
        let address = read(&env, &key);
        if address.is_some() {
            env.storage().persistent().extend_ttl(
                &key,
                PERSISTENT_TTL_THRESHOLD,
                PERSISTENT_TTL_EXTEND_TO,
            );
        }
        address
    }

    /// Returns the factory administrator and currently configured vault code hash.
    pub fn configuration(env: Env) -> (Address, BytesN<32>) {
        let admin = read(&env, &DataKey::Admin)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, FactoryError::NotInitialized));
        let wasm_hash = read(&env, &DataKey::VaultWasmHash)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, FactoryError::NotInitialized));
        (admin, wasm_hash)
    }
}

pub fn read<T>(env: &Env, key: &DataKey) -> Option<T>
where
    T: soroban_sdk::IntoVal<Env, soroban_sdk::Val> + soroban_sdk::TryFromVal<Env, soroban_sdk::Val>,
{
    env.storage().persistent().get(key)
}

pub fn persist<T>(env: &Env, key: &DataKey, value: &T)
where
    T: soroban_sdk::IntoVal<Env, soroban_sdk::Val>,
{
    env.storage().persistent().set(key, value);
    env.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
    env.storage()
        .instance()
        .extend_ttl(PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}
