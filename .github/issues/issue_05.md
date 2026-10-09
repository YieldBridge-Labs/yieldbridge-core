### Description: Problem
1. In `contracts/vault_core/src/lib.rs`, the `admin` address initialized during `initialize()` cannot be updated. If the admin private key is compromised or needs migration to a multisig (e.g., Soroban multisig contract), the vault must be completely abandoned.
2. Single-step admin updates risk permanently locking vault management if a typo or wrong key format is passed as the new address.
3. If an upstream reward asset contract experiences an exploit or suspension, vault administrators have no `pause()` mechanism to halt claims and injections, leaving unvested funds vulnerable to automated drain scripts.

### Solution
Implement a two-step admin transfer pattern (`propose_admin(new_admin)` followed by `accept_admin()`) in `vault_core`. Add an emergency circuit breaker flag (`is_paused`) managed exclusively by the admin, guarding `inject()`, `claim()`, and `set_weights()` while leaving read-only view endpoints (`claimable`, `configuration`, `get_shares`) operational.

### Acceptance Criteria
- [ ] Add `propose_admin(new_admin: Address)` and `accept_admin()` with persistent storage keys and TTL renewals.
- [ ] Add `pause()` and `unpause()` admin endpoints emitting structured `VaultPaused` and `VaultUnpaused` contract events.
- [ ] Guard `inject`, `claim`, and `set_weights` with pause status checks returning `Error::ContractPaused`.
- [ ] Add unit tests covering unauthorized pause attempts, successful pause/unpause lifecycle, and two-step ownership handoff.

### Note for Contributors
If you're assigned to this issue, write a clear and detailed description for your pull request. Explain what was changed, why it was needed, how it was implemented, and include any relevant testing or screenshots where applicable.
