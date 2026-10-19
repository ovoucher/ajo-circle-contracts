# Contract: `ajo_circle`

soroban-sdk 28.0.0, built with `stellar contract build` (wasm32v1-none). One contract hosts many circles keyed by `u32` id.

**Trust assumptions.** No admin/upgrade path. The organizer can only create and cancel a *forming* circle. Funds move only via token `transfer` inside `join`, `contribute` (member -> contract) and `claim` (contract -> member). The token contract is trusted (use a SAC/standard token; a malicious token can misbehave).

## Storage
Instance: `Count`. Persistent (TTL extended on access, threshold 30 d / extend to 120 d): `Circle(id)`, `Joined(id,addr)`, `Deposit(id,addr)`, `Paid(id,round,addr)`, `Claimable(id,addr)`. Members <= 20 keep `Circle` small. Entries that go unread > 120 days can expire; extend by calling any method.

## Methods
| Method | Auth | Rule |
|---|---|---|
| `create_circle(organizer, token, members, contribution, period, grace) -> id` | organizer | 2-20 unique members, contribution > 0, 60 s <= period <= 366 d, grace <= period, no overflow of `c*(n-1)*n` |
| `join(id, member)` | member | circle Forming, member listed, once; locks `c*(n-1)`; last join -> Active, `start_time = now` |
| `cancel(id)` | organizer | Forming only; deposits -> claimable |
| `contribute(id, member)` | member | Active, listed, round open (`now >= start + r*period`), not the recipient, once per round |
| `settle_round(id)` | none (permissionless) | all `n-1` paid, or `now >= start+(r+1)*period+grace`; slashes `c` per unpaid payer; credits pot `c*(n-1)` to recipient; last round -> Completed and leftover deposits credited |
| `claim(id, member) -> amount` | member | pays out `Claimable`, zeroed before transfer |
| views | none | `get_circle`, `circle_count`, `deposit_of`, `claimable_of`, `has_paid`, `required_deposit` |

## Invariants
- Deposit `c*(n-1)` equals the maximum a member can ever owe, so a slash never underflows and the pot is always fully funded.
- Contract token balance = sum(deposits) + sum(claimable) + payments in the current round.
- Each (round, member) pays at most once; rounds advance only through `settle_round`.

## Events
`circle_created, joined, started, contributed, slashed, round_settled, completed, cancelled, claimed` (topics: circle id, plus round where relevant).

## Errors
1 CircleNotFound, 2 InvalidMembers, 3 InvalidAmount, 4 InvalidPeriod, 5 InvalidGrace, 6 NotMember, 7 AlreadyJoined, 8 NotForming, 9 NotActive, 10 NotOrganizer (reserved), 11 RoundNotOpen, 12 AlreadyPaid, 13 RecipientDoesNotPay, 14 RoundNotSettleable, 15 NothingToClaim, 16 Overflow.

## Failure scenarios
- Member never joins: organizer cancels, joined members claim deposits back. There is no timeout: until the organizer cancels, joined deposits stay locked (known limitation).
- Member stops paying: slashed `c` each settled round; their deposit funds the pot.
- Nobody calls `settle_round`: round and pot stall until anyone calls it (permissionless).
- Member stops paying after receiving their pot: their deposit was sized for exactly the remaining obligations, so slashing covers it and later recipients are still paid in full.
