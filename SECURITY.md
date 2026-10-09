# Security Policy

YieldBridge Labs takes the security of our smart contracts, streaming math, and developer tooling seriously. This policy outlines our vulnerability disclosure process, supported scopes, and protocol security considerations.

---

## Supported Versions

Only the latest release tag on the `main` branch is actively supported with security updates.

| Component | Target Version | Supported |
| :--- | :--- | :--- |
| `vault_core` | `v0.1.0` (Soroban SDK 28) | :white_check_mark: |
| `stream_factory` | `v0.1.0` (Soroban SDK 28) | :white_check_mark: |
| Deployment Scripts | Latest `main` | :white_check_mark: |

---

## Reporting a Security Vulnerability

If you discover a security vulnerability, integer overflow risk, unauthorized state mutation vector, or denial-of-service issue in YieldBridge Core contracts:

1. **Do NOT open a public GitHub issue.**
2. Send an encrypted email or report directly to our security coordination team at:
   - **Email:** `security@yieldbridge.io`
   - **Alternative / Lead Maintainer:** `mutech939@users.noreply.github.com`
3. Include detailed steps to reproduce the vulnerability:
   - Contract name (`vault_core` or `stream_factory`) and function name
   - Proof of Concept (PoC) Rust unit test demonstrating the exploit
   - Impact assessment (loss of funds, state bricking, precision leak, reentrancy)
   - Proposed remediation (if available)

We acknowledge receipt of reports within **24 hours** and aim to provide an initial assessment and mitigation plan within **72 hours**.

---

## Security Scope & In-Scope Targets

### In-Scope
- `contracts/vault_core/`:
  - Token custody, reentrancy vulnerabilities, math precision leaks, and remainder truncation.
  - Streaming rate calculations and unvested rollover edge cases.
  - Share weight checkpoint manipulation and front-running risks.
  - Persistent storage expiration due to inadequate TTL extension logic.
- `contracts/stream_factory/`:
  - Deterministic salt collision vulnerabilities and unauthorized deployments.
  - WASM hash registry corruption or unauthorized upgrades.
- `scripts/`:
  - Deployment and initialization authentication flaws.

### Out-of-Scope
- Stellar network consensus or core protocol outages.
- Soroban host environment or Rust compiler vulnerabilities.
- Phishing, social engineering, or physical compromise of developer credentials.

---

## Unaudited Code & Testnet Disclaimer

> [!WARNING]
> The YieldBridge smart contracts currently deployed to Stellar Testnet are part of the **Drips Stellar Wave program** implementation and are actively undergoing testing. 
> 
> While the contracts feature comprehensive unit tests, fixed-point math tests, and property checks, **they have not yet completed an independent third-party cryptographic security audit**. 
> 
> Do NOT deploy these contracts to Stellar Mainnet with real financial assets until a formal security audit report has been published.
