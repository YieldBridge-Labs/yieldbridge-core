### Description: Problem
1. The frontend application currently lacks an activity feed or transaction history tab showing recent protocol interactions.
2. Users who submit a `claim()` or `inject()` transaction receive no on-screen confirmation other than the Freighter popup, without real-time audit trail logs of previous claims.
3. Manually querying `getEvents` from the Soroban RPC endpoint requires parsing raw XDR contract topics and payloads, which leads to duplicated, untyped decoding logic scattered across frontend components.

### Solution
Create a centralized, type-safe event parsing module `src/services/eventService.ts` using `@stellar/stellar-sdk` and `sorobanClient.getEvents()`. Expose a React hook `useVaultEvents(vaultAddress)` that decodes `YieldInjected`, `YieldClaimed`, and `SharesUpdated` into strongly typed JavaScript objects and maintains an auto-refreshing activity timeline.

### Acceptance Criteria
- [ ] Implement `decodeVaultEvent(rawEvent)` supporting all `#[contractevent]` schemas from `vault_core`.
- [ ] Build `useVaultEvents` hook supporting pagination with `cursor` and configurable polling interval (default 10s).
- [ ] Create a responsive `<ActivityFeed />` UI component showing timestamp, investor address, token amount, and link to Stellar Expert.
- [ ] Include graceful error handling for RPC rate limits (HTTP 429) with exponential backoff.

### Note for Contributors
If you're assigned to this issue, write a clear and detailed description for your pull request. Explain what was changed, why it was needed, how it was implemented, and include any relevant testing or screenshots where applicable.
