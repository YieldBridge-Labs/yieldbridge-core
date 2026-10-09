### Description: Problem
1. In `contracts/vault_core/src/lib.rs:inject`, when an issuer deposits additional yield before the current streaming window terminates, the unvested token balance is calculated via `remaining_time * reward_rate / SCALE`. Because `reward_rate` is derived from an integer division over `stream_duration`, small round-down truncations occur on each consecutive injection. Over repeated injections, this accumulated truncation traps unallocated tokens inside the contract.
2. In `claim()`, sub-atomic remainder retainment stores `remainder = accrued % SCALE`. When dealing with tokens with 18 decimals (e.g. wrapped ERC-20 equivalents) combined with large investor share counts, the multiplication `shares * acc_per_share` approaches the upper boundary of `u128`, risking math panic in debug mode.
3. `period_finish` can be set into the past if `duration` is smaller than the current ledger timestamp difference during simulation, causing underflow when calculating `remaining_time = period_finish - current_time`.

### Solution
Refactor the rollover calculation in `inject()` to track unvested tokens using exact token amounts (`total_funded - total_streamed_at_checkpoint`) rather than multiplying the truncated rate back by remaining seconds. Use checked math operations (`checked_mul`, `checked_add`) with explicit error types instead of raw panics. Clamp `remaining_time` safely using `saturating_sub(current_time)`.

### Acceptance Criteria
- [ ] Replace rate-reconstruction arithmetic in `inject()` with exact funded-versus-streamed delta tracking.
- [ ] Guard all scaling multiplications against `u128` overflow using checked math.
- [ ] Prevent negative remaining duration via `period_finish.saturating_sub(now)`.
- [ ] Add unit test in `contracts/vault_core/src/test.rs` executing 20 consecutive mid-stream injections with 18-decimal token scale verifying that the vault balance reconciles to 0 unallocated dust.

### Note for Contributors
If you're assigned to this issue, write a clear and detailed description for your pull request. Explain what was changed, why it was needed, how it was implemented, and include any relevant testing or screenshots where applicable.
