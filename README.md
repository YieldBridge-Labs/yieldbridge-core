# YieldBridge Core Smart Contracts

> Non-custodial, share-weighted time-linear yield streaming engine and deterministic factory built natively on Stellar Soroban.

[![Soroban SDK](https://img.shields.io/badge/Soroban%20SDK-v28.0.0-blue.svg)](https://stellar.org)
[![Rust Edition](https://img.shields.io/badge/Rust%20Edition-2024-orange.svg)](https://www.rust-lang.org)
[![Target](https://img.shields.io/badge/Target-wasm32v1--none-success.svg)](https://stellar.org)

---

## Architecture Overview

YieldBridge Core consists of two interconnected, production-grade Soroban smart contracts:

1. **`vault_core` (`contracts/vault_core/`)**:
   - Manages non-custodial share weights across investors.
   - Computes real-time linear reward emission rates with 128-bit fixed-point precision ($10^{12}$ scale).
   - Rolls forward unvested rewards dynamically upon new yield injections.
   - Implements discrete checkpoint accumulation ($\Delta S = \frac{\Delta t \cdot R}{\text{total\_shares}}$).
   - Enforces reentrancy safeguards, whole-token transfers, and sub-atomic remainder preservation.
   - Enforces automatic 535,680-ledger TTL extensions on persistent and instance state for all mutations and claims.

2. **`stream_factory` (`contracts/stream_factory/`)**:
   - Provides deterministic contract deployment using Stellar Soroban's native deployer (`deploy_contract`) via pre-uploaded WASM bytecode hashes and 32-byte salts (`BytesN<32>`).
   - Maintains an on-chain salt-to-vault registry with automatic TTL renewal.
   - Emits structured indexer events (`#[contractevent]`) for deployment tracking.

---

## Contract Interfaces

### 1. `vault_core` (`YieldVault`)

| Function | Parameters | Authorization | Description |
|---|---|---|---|
| `initialize` | `admin: Address`, `token: Address`, `duration: u64` | `admin.require_auth()` | Initializes the vault with administrator, reward asset address, and stream duration in seconds. |
| `set_weights` | `investors: Vec<Address>`, `weights: Vec<u128>` | `admin.require_auth()` | Batch configures investor share weights after settling prior accrued rewards. Validates vector lengths and address uniqueness. |
| `set_shares` | `investor_address: Address`, `shares: i128` | `admin.require_auth()` | Single-investor helper to configure share weight. |
| `inject` | `amount: i128` | `admin.require_auth()` | Transfers reward tokens from admin into the vault, checkpoints accrued rewards, and rolls unvested funding into a new linear streaming schedule. |
| `claim` | `investor_address: Address` | `investor.require_auth()` | Settles accrued rewards, transfers whole token units to claimant, retains sub-atomic fractional remainders, and renews persistent TTL. |
| `claimable` | `investor_address: Address` -> `i128` | None (View) | Read-only projection of vested token units available to claim. |
| `configuration` | `()` -> `(Address, Address, u64)` | None (View) | Returns `(admin, token, stream_duration)`. |
| `get_shares` | `investor_address: Address` -> `i128` | None (View) | Returns current shares assigned to an investor. |
| `total_shares` | `()` -> `i128` | None (View) | Returns total aggregate shares across all investors. |
| `total_funded` | `()` -> `i128` | None (View) | Returns cumulative tokens injected into the vault. |
| `total_claimed` | `()` -> `i128` | None (View) | Returns cumulative tokens claimed by investors. |
| `reward_rate` | `()` -> `i128` | None (View) | Returns current scaled reward rate per second. |
| `period_finish` | `()` -> `u64` | None (View) | Returns ledger timestamp when active stream concludes. |

### 2. `stream_factory` (`StreamFactory`)

| Function | Parameters | Authorization | Description |
|---|---|---|---|
| `initialize` | `admin: Address`, `vault_wasm_hash: BytesN<32>` | `admin.require_auth()` | Initializes factory administrator and configured `vault_core` WASM bytecode hash. |
| `set_vault_wasm_hash` | `vault_wasm_hash: BytesN<32>` | `admin.require_auth()` | Updates WASM hash for future vault deployments. |
| `create_vault` | `salt: BytesN<32>`, `token: Address`, `stream_duration: u64` -> `Address` | `admin.require_auth()` | Deterministically deploys and initializes a vault using the factory administrator as the vault administrator. |
| `deploy` | `salt: BytesN<32>`, `admin: Address`, `token: Address`, `duration: u64` -> `Address` | `admin.require_auth()` | Deterministically deploys and initializes a vault with a custom administrator address. |
| `vault_for_salt` | `salt: BytesN<32>` -> `Option<Address>` | None (View) | Resolves 32-byte salt to deployed contract address and renews storage entry TTL. |
| `configuration` | `()` -> `(Address, BytesN<32>)` | None (View) | Returns `(admin, vault_wasm_hash)`. |

---

## Storage & TTL Management

All state entries are stored in Soroban persistent storage under `DataKey`:
- `DataKey::State` and `DataKey::Investor(Address)` in `vault_core`.
- `DataKey::Admin`, `DataKey::VaultWasmHash`, and `DataKey::Vault(BytesN<32>)` in `stream_factory`.

Every state mutation and claim explicitly executes TTL extensions to **535,680 ledgers** (~30 days) via:
```rust
env.storage().persistent().extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
env.storage().instance().extend_ttl(PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
```

---

## Workspace Verification Commands

Verify formatting, run full unit and integration test suites, and compile release WASM binaries:

```bash
# 1. Code formatting check
cargo fmt --all -- --check

# 2. Comprehensive unit and integration tests (30 tests across workspace)
cargo test --workspace --all-targets

# 3. Compile optimized release WASM for vault_core
stellar contract build --package vault_core --target wasm32v1-none

# 4. Compile optimized release WASM for stream_factory
stellar contract build --package stream_factory --target wasm32v1-none
```
