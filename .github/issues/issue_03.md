### Description: Problem
1. The existing test suite in `contracts/vault_core/src/test.rs` only tests share weight updates with static values (e.g., 2 investors with fixed 70/30 or 50/50 splits) and does not test chaotic multi-investor rebalancing scenarios.
2. Current tests do not assert the conservation of funds invariant (sum of claimed + pending + unvested == total_funded) under pseudo-random continuous ledger time jumps and random investor weight reallocations.
3. Edge cases involving dynamic removal of investors (`weight = 0`), followed by re-addition with high weights mid-stream, lack automated coverage.

### Solution
Implement a property-based testing suite utilizing `proptest` within a new test module `contracts/vault_core/src/fuzz_test.rs`. The fuzz harness should generate pseudo-random action sequences: arbitrary investor sets (1 to 20 addresses), random share weight assignments, intermittent token injections, pseudo-random time progressions, and random claims. Assert global zero-sum balance invariants after every step.

### Acceptance Criteria
- [ ] Add `proptest = "1.6"` to workspace dev-dependencies.
- [ ] Create `contracts/vault_core/src/fuzz_test.rs` with properties evaluating at least 1,000 randomized permutations.
- [ ] Formally assert that the vault's total token balance strictly matches `total_funded - total_claimed` with zero invariant deviation.
- [ ] Ensure the fuzz test runs cleanly in CI under `cargo test --workspace`.

### Note for Contributors
If you're assigned to this issue, write a clear and detailed description for your pull request. Explain what was changed, why it was needed, how it was implemented, and include any relevant testing or screenshots where applicable.
