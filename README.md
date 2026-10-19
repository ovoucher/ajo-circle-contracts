# Ajo Circle: Soroban contract

Collateral-backed rotating savings circles (ajo, susu, chama, tanda) as a single Soroban smart contract on Stellar. **Testnet only. Unaudited.**

This repo is self-contained. It is one of three Ajo Circle repos; the others (a TypeScript API and a React web app) talk to this contract only through the deployed contract's public interface described below.

## Problem

Rotating savings groups run on trust: a treasurer collects cash, pays the pot to one member per round, and hopes nobody stops paying. This contract holds the pot, records every payment publicly, and makes each member lock a deposit equal to everything they owe, so a default can never leave a recipient short.

## How a circle works

`n` members, contribution `c`. The recipient of round `r` is `members[r]` and does not pay in their own round, so each member owes `n-1` payments and locks a deposit of `c*(n-1)`.

1. `create_circle`: organizer fixes members (payout order), token, `c`, round length and grace.
2. `join`: each member locks the deposit. The last join starts the circle.
3. `contribute`: payers pay `c` once per round.
4. `settle_round`: anyone, once everyone paid or `deadline + grace` passed. Unpaid members lose `c` from their deposit; the recipient is credited the full pot.
5. `claim`: pull-based withdrawal of pots and, after completion, leftover deposits.

`cancel` (organizer only, before start) makes joined deposits claimable. There is no admin and no upgrade path. Full rules: [docs/CONTRACTS.md](docs/CONTRACTS.md).

## Layout

```
src/lib.rs          contract
src/test.rs         20 tests (unit, auth, boundary, state transitions, replay)
scripts/            deploy-testnet.sh, demo-testnet.sh, verify-deployment.sh
deployment/         testnet.json (contract id, wasm hash, real tx hashes)
docs/               CONTRACTS, SECURITY, DEPLOYMENT, DRIPS_WAVE_RESEARCH
```

## Build and test

Prereqs: Rust stable with the `wasm32v1-none` target; stellar-cli >= 25.2.0 (soroban-sdk 28).

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings
cargo test                 # 20 tests inside the Soroban host
stellar contract build     # target/wasm32v1-none/release/ajo_circle.wasm
```

## Deploy to Testnet

```bash
./scripts/deploy-testnet.sh        # tests, build, funded identity via Friendbot, deploy, writes deployment/testnet.json
./scripts/verify-deployment.sh     # on-chain wasm hash == local build; reads circle_count
./scripts/demo-testnet.sh          # runs a real 2-member circle with native XLM (~70 s)
```
Details and Testnet-reset steps: [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md). The scripts hardcode Testnet.

## Deployed on Testnet (verified)

- Contract: [`CBLJTYZCAF6F3VW4JSU57S6ODYOXGAK5O3I3W55W2OZVSUXGCZTREZT5`](https://stellar.expert/explorer/testnet/contract/CBLJTYZCAF6F3VW4JSU57S6ODYOXGAK5O3I3W55W2OZVSUXGCZTREZT5)
- Deploy tx: [`5ee4c410…a27301`](https://stellar.expert/explorer/testnet/tx/5ee4c41039834a30cdae0ec434e7a84c71813c880870d8eed9fa358a68a27301)
- A 2-member circle ran to `Completed`; every tx hash is in `deployment/testnet.json`. Testnet resets periodically, so this id may stop existing; redeploy with the script.

## Interface other services rely on (ABI v0.1)

Methods: `create_circle, join, cancel, contribute, settle_round, claim, get_circle, circle_count, deposit_of, claimable_of, has_paid, required_deposit`. Events: `circle_created, joined, started, contributed, slashed, round_settled, completed, cancelled, claimed`. Error codes 1-16 (table in docs/CONTRACTS.md). Consumers (the Ajo Circle API and web app) map these codes and names; changing them is a breaking change, so bump the major version and tell consumers.

## Results (last run)

`cargo test`: 20 passed. clippy `-D warnings` and fmt: clean. Coverage was not measured. Not audited. See [docs/SECURITY.md](docs/SECURITY.md).

## Ecosystem and Drips Wave

Research, competitive review, eligibility assessment and the acceptance checklist: [docs/DRIPS_WAVE_RESEARCH.md](docs/DRIPS_WAVE_RESEARCH.md). This project is not a Drips Wave participant.

## Contributing / License

Small, tested PRs welcome (`cargo test` must pass). MIT, see [LICENSE](LICENSE).
