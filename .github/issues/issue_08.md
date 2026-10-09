### Description: Problem
1. The admin dashboard only allows manual entry of investors one at a time via form inputs, making onboarding dozens or hundreds of tokenized RWA investors tedious and prone to manual input errors.
2. If an admin inputs duplicate addresses or invalid Stellar public keys (`G...`), the contract call to `set_weights` fails on-chain, consuming testnet gas and displaying a generic transaction failure without identifying which row caused the failure.
3. There is no client-side normalization of weights (e.g. converting user percentages like `33.33%` into integer share weights), forcing users to manually calculate big integers.

### Solution
Build a drag-and-drop CSV/JSON upload component `<BatchWeightImporter />` in the admin dashboard. The component should parse files in the browser, validate Stellar public key checksums using `@stellar/stellar-sdk` (`StrKey.isValidEd25519PublicKey`), detect duplicates, normalize percentages into 128-bit integer weights, and batch the inputs into chunks matching Soroban transaction footprint limits.

### Acceptance Criteria
- [ ] Support drag-and-drop CSV and JSON file parsing with preview table.
- [ ] Validate each address format client-side and highlight invalid rows with clear error tooltips.
- [ ] Detect and block duplicate addresses prior to transaction submission.
- [ ] Provide auto-normalize toggle converting percentage columns (summing to 100%) into integer weights.
- [ ] Export template CSV and JSON files for admin reference.

### Note for Contributors
If you're assigned to this issue, write a clear and detailed description for your pull request. Explain what was changed, why it was needed, how it was implemented, and include any relevant testing or screenshots where applicable.
