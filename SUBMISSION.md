# Drips Stellar Wave Program — Submission Package Manifest

**Project Name:** YieldBridge  
**Program:** Drips Stellar Wave  
**Target Platform:** Stellar Soroban (Smart Contracts) & Web3 Dashboard  
**Date of Submission:** October 2026  

---

## 1. Project Summary

YieldBridge is a non-custodial, share-weighted time-linear yield streaming engine and deterministic factory built natively on Stellar Soroban. In traditional DeFi and tokenized Real World Assets (RWA), yield distribution relies on batch airdrops or periodic multi-send transactions that introduce custodial friction, gas spikes, and payout delays. YieldBridge solves this by implementing discrete, constant-time $O(1)$ checkpoint accumulation ($\Delta S = \frac{\Delta t \cdot R}{\text{total\_shares}}$) with 128-bit fixed-point precision ($10^{12}$ scale). Capital providers and issuers inject yield tokens into self-sustaining vaults that continuously stream rewards to investors based on their dynamic share weights. The system automatically rolls forward unvested funds upon subsequent injections, preserves sub-atomic fractional remainders across claims, prevents archival via automated 535,680-ledger (~30-day) persistent TTL extensions, and deploys deterministic vaults via a factory pattern using Stellar's native `deploy_contract`.

---

## 2. Repository & Governance Links

- **Smart Contracts Repository:** [https://github.com/YieldBridge-Labs/yieldbridge-core](https://github.com/YieldBridge-Labs/yieldbridge-core)
- **Application & Dashboard Monorepo:** [https://github.com/YieldBridge-Labs/yieldbridge-app](https://github.com/YieldBridge-Labs/yieldbridge-app)
- **CI / Automated Test Pipeline:** [GitHub Actions CI](https://github.com/YieldBridge-Labs/yieldbridge-core/actions/workflows/ci.yml)
- **Contribution Guidelines:** [CONTRIBUTING.md](CONTRIBUTING.md)
- **Security Policy & Disclosures:** [SECURITY.md](SECURITY.md)
- **License:** [MIT License](LICENSE)

---

## 3. On-Chain Testnet Deployment Artifacts

All smart contracts have been compiled with Soroban SDK v28.0.0 and deployed to **Stellar Testnet**:

| Component | Identifier / Address / Hash | Network & Explorer Link |
| :--- | :--- | :--- |
| **Stream Factory Contract** | `CDXFFV6Y5ZLIWECDCFDJAR3NVZCI6DK6IBQG2CCGBTC6Q5ER6ZKSW47V` | [Stellar Expert Contract Explorer](https://stellar.expert/explorer/testnet/contract/CDXFFV6Y5ZLIWECDCFDJAR3NVZCI6DK6IBQG2CCGBTC6Q5ER6ZKSW47V) |
| **Vault Core WASM Template** | `c6734622b3ba1b0f1a199fbf6e5b567fc4738fa3dcdc71a51c8cc37cfa144221` | [WASM Upload Tx Explorer](https://stellar.expert/explorer/testnet/tx/cfb6a827745377d7577813ba265f18a81a9e51982b87d79f46018996e219610a) |
| **Stream Factory WASM** | `dda1a2a80a34bf6ea47beca294651473260c117538336555a5afce8eafcd079d` | [WASM Upload Tx Explorer](https://stellar.expert/explorer/testnet/tx/e83f076a8d938a61e8cfc3e0f5519c36656e160105bf2465897101b5f28ca85d) |
| **Standalone Vault Instance** | `CATTLZBOWDCBHE3BSG5TKXYANDVLG6OLJUUT66PDUO3IUMYHZQOWGB7Z` | [Stellar Expert Contract Explorer](https://stellar.expert/explorer/testnet/contract/CATTLZBOWDCBHE3BSG5TKXYANDVLG6OLJUUT66PDUO3IUMYHZQOWGB7Z) |
| **Admin Deployer Account** | `GA23MAON7RAVLAZLWSF47WMEBY566IPUQL4MQVFSADCNEWCETXBV7TVR` | [Stellar Expert Account Explorer](https://stellar.expert/explorer/testnet/account/GA23MAON7RAVLAZLWSF47WMEBY566IPUQL4MQVFSADCNEWCETXBV7TVR) |

### Key Deployment Transactions
- **Stream Factory Deployment:** [`bc1737cdce611f804ec4d9fca491aab9b2189be6bf46bf7f511c680623fe7346`](https://stellar.expert/explorer/testnet/tx/bc1737cdce611f804ec4d9fca491aab9b2189be6bf46bf7f511c680623fe7346)
- **Stream Factory Initialization:** [`c186afd0fb989ac84eb7cab15001e8d05b16fa9400304f0b763befc43bfba829`](https://stellar.expert/explorer/testnet/tx/c186afd0fb989ac84eb7cab15001e8d05b16fa9400304f0b763befc43bfba829)
  - Emitted Event: `FactoryInitialized` linking admin address to vault template hash.
- **Standalone Vault Core Deployment:** [`7ce8880ff2ab445a49dcfe26e8feb409dede44b70685c639a142a4024dcff15e`](https://stellar.expert/explorer/testnet/tx/7ce8880ff2ab445a49dcfe26e8feb409dede44b70685c639a142a4024dcff15e)

---

## 4. Technical Specifications & Verification

- **Rust Edition:** 2024
- **Soroban SDK:** `28.0.0`
- **Compilation Target:** `wasm32v1-none`
- **Optimized Binary Sizes:**
  - `vault_core.wasm`: 13,039 bytes (14 exported contract endpoints)
  - `stream_factory.wasm`: 4,704 bytes (6 exported contract endpoints)
- **Test Suite Results:**
  - **30 / 30 tests passing** (18 unit in `vault_core`, 8 unit in `stream_factory`, 4 integration tests).
  - Clippy check: 0 warnings with `-D warnings`.
  - Format check: 100% compliant with standard `rustfmt`.

---

## 5. Demo Video Checklist (60-Second Walkthrough)

The 60-second end-to-end demonstration video showcases the complete lifecycle of YieldBridge on Stellar Testnet:

| Segment | Timing | Action & Screen Description |
| :--- | :--- | :--- |
| **1. Factory Deployment** | `0:00 - 0:12` | Admin connects Freighter wallet to the YieldBridge dashboard and creates a new streaming vault via `StreamFactory.create_vault(salt, token, duration)`. Freighter approves transaction; newly deployed contract address appears in real time. |
| **2. Share Configuration** | `0:12 - 0:25` | Admin sets investor share weights via `set_weights` (e.g., Investor A: 70%, Investor B: 30%). Dashboard reflects updated proportional entitlement. |
| **3. Yield Injection** | `0:25 - 0:38` | Admin deposits yield reward tokens (e.g., 10,000 USDC / XLM) via `inject(amount)`. Linear streaming schedule begins immediately; real-time counters begin ticking on screen. |
| **4. Investor Claim** | `0:38 - 0:52` | Investor switches Freighter account, views live claimable balance incrementing every second, and executes `claim()`. Tokens transfer atomically into investor wallet. |
| **5. On-Chain Proof** | `0:52 - 1:00` | Display Stellar Expert transaction hash confirming successful payout, updated vault balance, and preserved remainder state. |

---

## 6. Environment Configuration for Integrators

```env
NEXT_PUBLIC_STREAM_FACTORY_ID=CDXFFV6Y5ZLIWECDCFDJAR3NVZCI6DK6IBQG2CCGBTC6Q5ER6ZKSW47V
NEXT_PUBLIC_VAULT_TEMPLATE_HASH=c6734622b3ba1b0f1a199fbf6e5b567fc4738fa3dcdc71a51c8cc37cfa144221
NEXT_PUBLIC_STELLAR_NETWORK=testnet
NEXT_PUBLIC_RPC_URL=https://soroban-testnet.stellar.org
```
