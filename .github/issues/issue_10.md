### Description: Problem
1. In `contracts/stream_factory/src/lib.rs`, callers cannot pre-calculate what a vault's contract address will be before executing `create_vault(salt, ...)`, making it impossible for frontends to show the predicted vault address to the issuer in a deployment confirmation modal.
2. If two different admins attempt to create vaults with simple or sequential salts (e.g. `salt = [0; 32]`), the second deployment panics because the factory uses its own contract address as the deployer namespace, creating a denial-of-service vector across different issuers.
3. The registry mapping `vault_for_salt(salt)` stores the deployed vault globally under the 32-byte salt alone, meaning salts are not scoped per issuer/admin.

### Solution
Update `contracts/stream_factory/src/lib.rs` to namespace the deployment salt by hashing the caller's address with the user-provided salt (`salt_hash = env.crypto().sha256(&(admin, salt).to_xdr(&env))`). Add a public read-only query function `predict_vault_address(admin: Address, salt: BytesN<32>) -> Address` that computes the deterministic address off-chain/on-chain without deploying the contract.

### Acceptance Criteria
- [ ] Namespace deployment salts using SHA-256 of `(caller_address, user_salt)` to prevent cross-admin salt collisions.
- [ ] Implement `predict_vault_address(admin: Address, salt: BytesN<32>) -> Address` using Stellar's address derivation algorithm.
- [ ] Expose TypeScript equivalent helper in the frontend SDK (`predictVaultAddress(factoryId, admin, salt)`).
- [ ] Add unit tests verifying that two different admins can safely use the exact same input salt without collision.

### Note for Contributors
If you're assigned to this issue, write a clear and detailed description for your pull request. Explain what was changed, why it was needed, how it was implemented, and include any relevant testing or screenshots where applicable.
