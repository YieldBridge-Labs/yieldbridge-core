### Description: Problem
1. When a user changes their active account in the Freighter browser extension, `src/hooks/useFreighter.ts` does not receive the change event, leaving the application displaying the previous account's share weights and claimable balances.
2. If the user's Freighter extension is connected to Stellar `PUBLIC` (Mainnet) or a custom standalone network, the dashboard attempts to simulate Soroban contract calls against Testnet contract IDs, causing cryptic `SimulationFailed: HostError` toast errors with no actionable explanation for the user.
3. Disconnecting or locking Freighter leaves stale signing sessions active in React state, causing transaction submission modals to hang indefinitely.

### Solution
Integrate `@stellar/freighter-api` event listeners (`watchAccountChange`, `getNetworkDetails`) to synchronize wallet state with React context. Add an automatic network verification banner that detects when Freighter is not set to `TESTNET` (passphrase `Test SDF Network ; September 2015`), disabling write operations and displaying a prominent "Switch to Testnet" alert.

### Acceptance Criteria
- [ ] Subscribe to Freighter account change events and automatically refresh investor balances on change.
- [ ] Validate connected network passphrase against `NEXT_PUBLIC_STELLAR_NETWORK`; display modal/banner if mismatched.
- [ ] Disable "Claim Yield", "Inject Yield", and "Deploy Vault" buttons when on wrong network or disconnected.
- [ ] Reset all local vault state upon wallet lock or explicit disconnect.

### Note for Contributors
If you're assigned to this issue, write a clear and detailed description for your pull request. Explain what was changed, why it was needed, how it was implemented, and include any relevant testing or screenshots where applicable.
