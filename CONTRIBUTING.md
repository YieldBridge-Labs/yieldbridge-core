# Contributing to YieldBridge Core

Thank you for your interest in contributing to **YieldBridge Core**! We are committed to building robust, verifiable, non-custodial yield streaming infrastructure on Stellar Soroban.

---

## Code of Conduct

We expect all contributors to adhere to open-source etiquette, respect fellow collaborators, and maintain professional communication across discussions, issues, and pull requests.

---

## Development Environment Setup

### Prerequisites
- **Rust Toolchain:** Version 1.85+ (Edition 2024 support)
- **Target Architecture:** `wasm32v1-none`
- **Stellar CLI:** Version 28.0.0+ (`stellar --version`)
- **Git**

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/YieldBridge-Labs/yieldbridge-core.git
   cd yieldbridge-core
   ```

2. Add the WebAssembly compilation target:
   ```bash
   rustup target add wasm32v1-none
   ```

3. Install Stellar CLI (if not already installed):
   ```bash
   cargo install --locked stellar-cli
   # Or on Windows via winget:
   winget install Stellar.StellarCLI
   ```

4. Verify compilation and test suite:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test --workspace --all-targets
   ```

---

## Branching & Pull Request Workflow

1. **Fork and Branch:** Create a feature or fix branch from `main`:
   ```bash
   git checkout -b feat/linear-stream-optimization
   ```

2. **Run Local Checks:** Ensure all tests pass and code is formatted prior to pushing:
   ```bash
   cargo fmt --all -- --check
   cargo test --workspace --all-targets
   stellar contract build --package vault_core
   stellar contract build --package stream_factory
   ```

3. **Open a Pull Request:**
   - Target the `main` branch.
   - Fill in the PR description with clear rationale, problem statement, and testing steps.
   - Reference any relevant GitHub issues (`Closes #123`).

4. **CI & Review:**
   - All automated CI checks (Format, Test Suite, Clippy, Contract Build) must pass.
   - At least one code review approval is required prior to merge.

---

## Commit Message Conventions

We strictly enforce the **[Conventional Commits](https://www.conventionalcommits.org/)** specification. Every commit message must follow this format:

```
<type>(<scope>): <short description>

[optional body explaining rationale]

[optional footer(s)]
```

### Allowed Types
- `feat`: A new feature or contract capability.
- `fix`: A bug fix or math correction.
- `test`: Adding or refactoring unit/integration tests.
- `docs`: Documentation updates, specs, or guides.
- `refactor`: Code changes that neither fix a bug nor add a feature.
- `perf`: A code change that optimizes gas, memory, or CPU footprint.
- `ci`: Changes to CI configuration or build scripts.
- `chore`: Maintenance tasks, dependency updates, or repository hygiene.

### Examples
- `feat(vault): add sub-atomic fractional remainder retention to claim mechanism`
- `test(factory): add deterministic salt collision unit test`
- `docs(readme): update testnet deployed contract addresses`

---

## Smart Contract Design Guidelines

When submitting changes to `vault_core` or `stream_factory`:
- **Soroban SDK:** Target `soroban-sdk = "28.0.0"`.
- **Zero Panics in User Paths:** Use proper checks, structured errors, or graceful returns. Panic only on unrecoverable authorization failures.
- **Fixed-Point Precision:** All rate calculations must use 128-bit integer scaling ($10^{12}$) without precision drift.
- **State Archival & TTL:** Always include explicit TTL extension calls to `535,680` ledgers on persistent and instance keys upon every state mutation.
- **Event Logging:** Emit structured events using `#[contractevent]`.
