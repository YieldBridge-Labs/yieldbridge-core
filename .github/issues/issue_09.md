### Description: Problem
1. Developers working on the frontend dashboard currently depend entirely on the public Stellar Testnet RPC (`https://soroban-testnet.stellar.org`), which occasionally suffers from friendbot rate limits, testnet resets, or network latency during local UI development.
2. There is no automated local script to deploy a standard Stellar Asset Contract (SAC) test token and fund multiple simulated investor accounts with test balances.
3. Running integration tests against a mock network requires developers to manually construct and fund test keypairs, creating friction for new open-source contributors.

### Solution
Create a `docker-compose.yml` configuration launching a local standalone Stellar/Soroban container (`stellar/quickstart`). Add a developer utility script `scripts/seed_local_env.sh` (and `seed_local_env.ps1`) that generates admin and investor test accounts, deploys a mock SAC token, installs contract WASMs, deploys a test vault via `stream_factory`, and generates a populated `.env.local` file ready for instant frontend development.

### Acceptance Criteria
- [ ] Add `docker-compose.yml` configuring `stellar/quickstart:testing` with local RPC port 8000.
- [ ] Implement `scripts/seed_local_env.sh` and `scripts/seed_local_env.ps1` that automates end-to-end setup.
- [ ] Mint test reward tokens to 3 predefined demo accounts (Admin, Investor 1, Investor 2).
- [ ] Output a ready-to-use `.env.local` containing all generated local contract IDs and keys.

### Note for Contributors
If you're assigned to this issue, write a clear and detailed description for your pull request. Explain what was changed, why it was needed, how it was implemented, and include any relevant testing or screenshots where applicable.
