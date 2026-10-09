# YieldBridge Core Smart Contracts

> Non-custodial, share-weighted time-linear yield streaming engine and deterministic factory built natively on Stellar Soroban.

[![CI](https://github.com/YieldBridge-Labs/yieldbridge-core/actions/workflows/ci.yml/badge.svg)](https://github.com/YieldBridge-Labs/yieldbridge-core/actions/workflows/ci.yml)
[![Soroban SDK](https://img.shields.io/badge/Soroban%20SDK-v28.0.0-blue.svg)](https://stellar.org)
[![Rust Edition](https://img.shields.io/badge/Rust%20Edition-2024-orange.svg)](https://www.rust-lang.org)
[![Target](https://img.shields.io/badge/Target-wasm32v1--none-success.svg)](https://stellar.org)
[![Network](https://img.shields.io/badge/Stellar-Testnet-blueviolet.svg)](https://stellar.expert/explorer/testnet)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

---

## Overview

**YieldBridge** provides decentralized infrastructure for continuous, streaming yield distribution on the Stellar network. Built with **Soroban SDK v28** and **Rust Edition 2024**, YieldBridge eliminates manual claiming friction, batched gas costs, and distribution calculation errors for tokenized cash-flows, real-world assets (RWA), and protocol revenue sharing.

### Key Innovations
- **Discrete Checkpoint Accumulation:** Constant-time $O(1)$ reward settlement across arbitrary investor sets using scaled accumulator increments:
  $$\Delta S = \frac{\Delta t \cdot R}{\text{total\_shares}}$$
- **128-Bit Fixed-Point Math:** Math calculations operate on a $10^{12}$ scalar to prevent precision drift, while preserving sub-atomic fractional remainders between claims.
- **Dynamic Forward Rollover:** New yield injections seamlessly roll unvested tokens from active schedules into a refreshed uniform emission window.
- **Deterministic Deployer Factory:** Factory contract deploys child vaults using Soroban's native `deploy_contract` with 32-byte salts, maintaining an on-chain registry.
- **Automated Ledger Archival Prevention:** Persistent storage and instance state are refreshed to **535,680 ledgers** (~30 days) on every mutation and claim.

---

## On-Chain Testnet Deployments

YieldBridge Core contracts are deployed and verified on **Stellar Testnet**:

| Contract / Artifact | Address / Hash | Explorer Link |
| :--- | :--- | :--- |
| **Stream Factory** | `CDXFFV6Y5ZLIWECDCFDJAR3NVZCI6DK6IBQG2CCGBTC6Q5ER6ZKSW47V` | [View Contract](https://stellar.expert/explorer/testnet/contract/CDXFFV6Y5ZLIWECDCFDJAR3NVZCI6DK6IBQG2CCGBTC6Q5ER6ZKSW47V) |
| **Vault Core WASM Template** | `c6734622b3ba1b0f1a199fbf6e5b567fc4738fa3dcdc71a51c8cc37cfa144221` | [View Upload Tx](https://stellar.expert/explorer/testnet/tx/cfb6a827745377d7577813ba265f18a81a9e51982b87d79f46018996e219610a) |
| **Stream Factory WASM** | `dda1a2a80a34bf6ea47beca294651473260c117538336555a5afce8eafcd079d` | [View Upload Tx](https://stellar.expert/explorer/testnet/tx/e83f076a8d938a61e8cfc3e0f5519c36656e160105bf2465897101b5f28ca85d) |
| **Standalone Vault Instance** | `CATTLZBOWDCBHE3BSG5TKXYANDVLG6OLJUUT66PDUO3IUMYHZQOWGB7Z` | [View Contract](https://stellar.expert/explorer/testnet/contract/CATTLZBOWDCBHE3BSG5TKXYANDVLG6OLJUUT66PDUO3IUMYHZQOWGB7Z) |
| **Admin Deployer Account** | `GA23MAON7RAVLAZLWSF47WMEBY566IPUQL4MQVFSADCNEWCETXBV7TVR` | [View Account](https://stellar.expert/explorer/testnet/account/GA23MAON7RAVLAZLWSF47WMEBY566IPUQL4MQVFSADCNEWCETXBV7TVR) |

---

## Architecture

The system consists of two core smart contracts:

```
                      +-----------------------------+
                      |       Admin / Issuer        |
                      +--------------+--------------+
                                     |
               1. Upload WASM        | 2. Deploy Factory
                                     v
                      +-----------------------------+
                      |       stream_factory        |
                      +--------------+--------------+
                                     |
                         create_vault(salt, token)
                                     v
                      +-----------------------------+
                      |         vault_core          |
                      |  - Share weights registry   |
                      |  - Linear streaming engine  |
                      |  - Sub-atomic remainder bus |
                      +--------------+--------------+
                                     |
                     +---------------+---------------+
                     |                               |
                     v                               v
             inject(yield_tokens)            claim() via Freighter
            [Issuer / Distributor]              [Investor / RWA Holder]
```

### 1. `vault_core` (`contracts/vault_core/`)
- **State Storage:** Stores global streaming parameters and per-investor checkpoints in persistent storage.
- **Precision Accounting:**
  - $R = \frac{\text{unvested} + \text{injected}}{\text{duration}}$
  - Accumulator step: $\Delta S = \frac{(t_{\text{current}} - t_{\text{last}}) \cdot R \cdot 10^{12}}{\text{total\_shares}}$
  - Investor entitlement: $\text{accrued} = \frac{s_i \cdot (S - S_{\text{entry}, i})}{10^{12}} + \text{unclaimed}_i$
- **Atomic Transfers:** Whole token units are transferred via Stellar Asset Contract (`token::Client`). Fractional remainders ($< 1$ unit) are retained in storage for subsequent claims.

### 2. `stream_factory` (`contracts/stream_factory/`)
- **Deterministic Deployments:** Invokes `env.deployer().with_current_contract(salt).deploy_contract(ContractExecutable::Wasm(wasm_hash), ())`.
- **Registry & Collision Protection:** Verifies that salt entries are unique and caches deployed vault addresses.

---

## Contract Interfaces

### `YieldVault` (`vault_core`)

| Method | Parameters | Access | Description |
|---|---|---|---|
| `initialize` | `admin: Address`, `token: Address`, `duration: u64` | `admin` | Initializes vault parameters and emission asset. |
| `set_weights` | `investors: Vec<Address>`, `weights: Vec<u128>` | `admin` | Batch updates investor weights after checkpointing prior rewards. |
| `set_shares` | `investor: Address`, `shares: i128` | `admin` | Single-investor share allocation helper. |
| `inject` | `amount: i128` | `admin` | Deposits tokens and restarts linear schedule with rollover. |
| `claim` | `investor: Address` -> `i128` | `investor` | Transfers accrued whole tokens and preserves remainders. |
| `claimable` | `investor: Address` -> `i128` | View | Simulates pending claimable tokens without state mutation. |
| `configuration` | `()` -> `(Address, Address, u64)` | View | Returns `(admin, token, stream_duration)`. |
| `get_shares` | `investor: Address` -> `i128` | View | Returns investor's current share balance. |
| `total_shares` | `()` -> `i128` | View | Returns aggregate shares across all investors. |
| `total_funded` | `()` -> `i128` | View | Returns cumulative yield injected. |
| `total_claimed` | `()` -> `i128` | View | Returns cumulative yield withdrawn. |
| `reward_rate` | `()` -> `i128` | View | Returns current emission rate per second ($10^{12}$ scaled). |
| `period_finish` | `()` -> `u64` | View | Returns timestamp when active stream terminates. |

### `StreamFactory` (`stream_factory`)

| Method | Parameters | Access | Description |
|---|---|---|---|
| `initialize` | `admin: Address`, `vault_wasm_hash: BytesN<32>` | `admin` | Configures factory administrator and vault template hash. |
| `set_vault_wasm_hash` | `vault_wasm_hash: BytesN<32>` | `admin` | Updates default template hash for future vault deployments. |
| `create_vault` | `salt: BytesN<32>`, `token: Address`, `duration: u64` -> `Address` | `admin` | Deterministically deploys and initializes a child vault. |
| `deploy` | `salt: BytesN<32>`, `admin: Address`, `token: Address`, `duration: u64` -> `Address` | `admin` | Deploys child vault with a custom admin address. |
| `vault_for_salt` | `salt: BytesN<32>` -> `Option<Address>` | View | Resolves salt to deployed vault address. |
| `configuration` | `()` -> `(Address, BytesN<32>)` | View | Returns `(admin, vault_wasm_hash)`. |

---

## Quick Start & Verification

### 1. Smart Contract Workspace (Rust)

```bash
# 1. Format check
cargo fmt --all -- --check

# 2. Run static linter
cargo clippy --all-targets -- -D warnings

# 3. Run all 30 unit & integration tests
cargo test --workspace --all-targets

# 4. Compile optimized WASM binaries
stellar contract build --package vault_core
stellar contract build --package stream_factory
```

### 2. Automated Testnet Deployment

Execute the automated deployment script (available in Bash and PowerShell):

```bash
# Bash (Linux / macOS / Git Bash)
./scripts/deploy.sh

# PowerShell (Windows)
./scripts/deploy.ps1
```

### 3. Frontend & TypeScript SDK Integration

Copy deployed contract identifiers into your frontend environment (`.env.local`):

```env
NEXT_PUBLIC_STREAM_FACTORY_ID=CDXFFV6Y5ZLIWECDCFDJAR3NVZCI6DK6IBQG2CCGBTC6Q5ER6ZKSW47V
NEXT_PUBLIC_VAULT_TEMPLATE_HASH=c6734622b3ba1b0f1a199fbf6e5b567fc4738fa3dcdc71a51c8cc37cfa144221
NEXT_PUBLIC_STELLAR_NETWORK=testnet
NEXT_PUBLIC_RPC_URL=https://soroban-testnet.stellar.org
```

Install SDK dependencies and run the dashboard:

```bash
# Install dependencies
pnpm install

# Start local Next.js development server
pnpm dev
```

---

## Storage & Archival Policy (TTL)

To prevent contract state from archiving during extended streaming durations, all persistent state entries and instance storage are refreshed on every write or claim:

```rust
const PERSISTENT_TTL_THRESHOLD: u32 = 17_280;   // ~1 day
const PERSISTENT_TTL_EXTEND_TO: u32 = 535_680;  // ~30 days

env.storage().persistent().extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
env.storage().instance().extend_ttl(PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
```

---

## Maintainers & Community

| Role | Name | GitHub / Contact |
| :--- | :--- | :--- |
| **Lead Protocol Architect** | Mutech939 | [@Mutech939](https://github.com/Mutech939) |
| **Security Coordination** | YieldBridge Security | `security@yieldbridge.io` |
| **Organization** | YieldBridge Labs | [GitHub Org](https://github.com/YieldBridge-Labs) |

---

## License

This project is licensed under the [MIT License](LICENSE).
