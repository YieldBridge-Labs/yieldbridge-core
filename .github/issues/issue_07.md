### Description: Problem
1. While `vault_core` and `stream_factory` call `extend_ttl(...)` to 535,680 ledgers on every write, existing tests in `tests/integration_test.rs` only simulate short time steps (10 to 100 seconds) and never advance the mock ledger past the `PERSISTENT_TTL_THRESHOLD` (17,280 ledgers).
2. There are no tests verifying that an inactive investor who leaves their yield unclaimed for 25 days (~360,000 ledgers) can still claim their yield without contract state read errors.
3. Tests do not assert that instance storage and persistent storage TTLs remain synchronized over multi-week streaming periods.

### Solution
Add an extensive integration test suite `tests/ttl_lifecycle_test.rs`. Using Soroban test environment's `env.ledger().set(...)` and `env.as_contract(...)`, simulate ledger sequences progressing through 100,000, 300,000, and 500,000 ledgers. Verify that calls to `claim()`, `inject()`, and `vault_for_salt()` consistently restore TTL values back to `PERSISTENT_TTL_EXTEND_TO` (535,680 ledgers) without storage eviction.

### Acceptance Criteria
- [ ] Create `tests/ttl_lifecycle_test.rs` simulating ledger advances past 17,280 and 400,000 ledgers.
- [ ] Assert that persistent storage entries for both vault state and investor keys remain alive and readable.
- [ ] Verify that `factory.vault_for_salt(salt)` successfully refreshes the salt registry entry TTL on read.
- [ ] Verify that `client.claim()` executed near period expiration properly renews both persistent and instance storage TTLs.

### Note for Contributors
If you're assigned to this issue, write a clear and detailed description for your pull request. Explain what was changed, why it was needed, how it was implemented, and include any relevant testing or screenshots where applicable.
