# Security (contract)

**Status: unaudited, Testnet only.** Passing tests does not make this production-secure.

## Threat model and mitigations
| Threat | Mitigation |
|---|---|
| Unauthorized calls | `require_auth` on creator/member for every state change; tests with auth reset assert failures; `settle_round` is permissionless by design and cannot redirect funds |
| Double pay / replay | `Paid(id,round,member)` flag; claimable zeroed before transfer; duplicate joins rejected |
| Recipient shortfall from default | deposit = max obligation; slash `c` per missed round |
| Stalled payouts from a bad trustline | pull-based `claim` |
| Arithmetic overflow | checked math; creation rejects amounts whose totals could overflow; release profile keeps `overflow-checks` |
| Resource exhaustion | members <= 20; no unbounded loops over user data |
| Reentrancy | Soroban's host forbids re-entering a contract during a call; `claim` also zeroes the balance before transferring |
| Supply chain | `Cargo.lock` committed; single dependency (soroban-sdk 28.0.0) |
| Mainnet misuse | deploy scripts hardcode Testnet |

## Known limitations
- Not audited; no formal verification or fuzzing.
- Default is economically neutral for the defaulter: a slashed deposit replaces the payment they owed, so there is no extra penalty. The group's pots are fully funded either way. Penalties beyond the deposit are future work.
- Organizer fixes order and members; no on-chain identity.
- A forming circle has no join timeout; only the organizer can cancel, until then joined deposits stay locked.
- Token trust: a non-standard or freezable token (e.g. issuer clawback) can break guarantees.
- Persistent entries expire after TTL if untouched for ~120 days.
- Round timing depends on ledger time, which validators set approximately.
