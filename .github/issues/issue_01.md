### Description: Problem
1. In the investor dashboard (`src/components/VaultDashboard.tsx`), claimable balances currently only update upon manual page reload or when an explicit RPC query finishes, leading to a static and unresponsive user experience.
2. The current implementation polls the Soroban RPC endpoint `claimable(investor)` on a fixed 30-second interval, which exceeds user expectations for a "real-time streaming" protocol and causes unnecessary RPC rate-limiting on public testnet infrastructure.
3. When an active streaming schedule reaches its `period_finish`, the UI does not clamp the progress bar or live counter to 100%, causing the calculated claimable projection to mathematically overshoot the vault's total funded reward ceiling.

### Solution
Implement a client-side high-frequency interpolation hook (`useStreamingBalance`) using `requestAnimationFrame` or a 100ms ticking interval. The hook should compute the local delta based on contract `reward_rate`, investor `get_shares`, `total_shares`, and local elapsed clock time since the last verified RPC checkpoint. Clamp the balance and visual progress bar at `period_finish` to guarantee the displayed claimable amount never exceeds the unvested ceiling.

### Acceptance Criteria
- [ ] Implement `useStreamingBalance` hook that smoothly increments claimable tokens client-side at ~60fps without extra RPC requests.
- [ ] Add SVG/Tailwind linear progress bar component reflecting percentage elapsed `(now - start) / (period_finish - start)`.
- [ ] Hard-clamp calculations when `now >= period_finish` to match the exact on-chain `claimable` ceiling.
- [ ] Gracefully handle browser tab hibernation/backgrounding using `document.visibilityState` to re-synchronize state with the Soroban RPC on resume.

### Note for Contributors
If you're assigned to this issue, write a clear and detailed description for your pull request. Explain what was changed, why it was needed, how it was implemented, and include any relevant testing or screenshots where applicable.
