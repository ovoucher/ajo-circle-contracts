# Drips Wave research and participation assessment

Researched 2026-10-18. Sources: [Drips Stellar Wave](https://www.drips.network/wave/stellar), [Wave docs](https://docs.drips.network/wave/), maintainer guide ("Participating in a Wave"), Wave 7 changelog and Terms (local copies in `../repos/guide/`). The docs page `.../wave/maintainers/applying-with-a-repo` returned 404, and the exact criteria organizers use to approve repos are **not published** in what I read.

## Program facts (verified in those pages)
- Waves are ~7-day cycles roughly monthly. Wave 10: Oct 15 13:00 to Oct 22 13:00, $75,000 pool. Rewards split by share of Points.
- Maintainers sign in with GitHub, install the Drips Wave GitHub App, sync **public** repos and apply them to a Wave Program; **organizers must approve**. Only then can issues be added (by dashboard or the `Stellar Wave` label).
- Points per issue: Trivial 100, Medium 150, High 200 (set by the maintainer). Issues must be resolved before the Wave ends.
- Rejected repos cannot re-apply; appeal in-app, first after 2 weeks, 3 max, only with substantive change. Terms give organizers sole discretion over eligibility.

## Eligibility assessment for this project
- **Can this repo join the Stellar Wave now?** Unknown and not automatic. It must be public, have the GitHub App installed, be applied for in the Drips app and be approved by the Stellar Wave organizers. Deployment alone does not qualify it. I have not applied and make no claim of approval.
- **Timing:** Wave 10 ends in 4 days; a new application is unlikely to produce points this cycle. Target the next Wave.
- **Risk:** a single-author fresh repo with no history and no users may be rejected, and a rejection blocks re-application. Build history (issues, commits, README, tests, real usage) first.

## Better direct route: contribute to approved repos
`gh search issues --label "Stellar Wave" --state open` found issues in approved repos, e.g. [Tx-wat/stellar-txwatch-web](https://github.com/Tx-wat/stellar-txwatch-web/issues) (#442-#449: README screenshots, CODEOWNERS, issue templates, CI cache, env validation). These look like small docs/CI tasks. I did not verify they are in the active Wave or unassigned. Check before starting:
1. The issue shows the Drips Wave bot comment/label and is listed in the Wave app for the active Wave.
2. It is unassigned on GitHub (assigned issues cannot be in a Program).
3. Apply through the Wave app and wait for assignment; do not open a PR first.
4. Read CONTRIBUTING; scope the PR to the issue; note a maintainer has 72 h before auto-assessment.

Soroban expertise suits Stellar-ecosystem repos with contract issues; search `label:"Stellar Wave" language:Rust` weekly.

## Plan for this repo
1. Push the three repos to public GitHub, enable CI in each, add 8-12 small, well-specified issues with acceptance criteria (examples: organizer-name signing UI via SEP-53; join timeout in contract; mobile layout audit; Playwright E2E with a mocked Freighter; backend Dockerfile; stablecoin presets; event-based push updates; docs translations).
2. Get a few real Testnet users/circles and publish the demo.
3. Install the Drips Wave GitHub App, apply to the Stellar Wave Program early in the next scoping phase, and describe it accurately (unaudited Testnet dApp for savings groups, test coverage numbers from CI).
4. If rejected, wait 2 weeks, make substantive changes, appeal in-app.
5. Application text: *"Ajo Circle is an MIT-licensed set of three repos (Soroban contract, TypeScript API, React web app) for collateral-backed rotating savings circles on Stellar Testnet. It has a tested contract (20 tests), API (16 tests) and web app (tests in its own repo), a deployed Testnet contract and CI in each repo. We are applying so contributors can take scoped issues listed in the repo. It is unaudited and has no production users."*

## Competitive review
Orbit ([ChrisQuint0/Orbit](https://github.com/ChrisQuint0/Orbit), MIT) is the closest Soroban ROSCA: hackathon-style, 0 stars, 0 issues, last push 2026-07-27. Other Stellar projects found are different problems: Grainlify (GitHub-webhook payouts), DevAsign (PR-merge bounties), Boundless (milestone grant pools), EscrowFlow (freelance escrow), RemitWise (remittance split/savings goals), Kitty Ledger (campaign funds with M-of-N payouts). Differentiators here: deposit sized to exactly cover obligations, recipient-exempt rounds, permissionless settle, pull-based claims, no admin.
This was a web/gh search, not an exhaustive survey; I did not read Orbit's code.

## Acceptance checklist
| Requirement | Status |
|---|---|
| Research + ideas comparison + sources | Verified (above; Wave criteria for new repos unpublished) |
| Soroban contract + tests | Verified: 20 tests pass, clippy/fmt clean |
| Backend + tests (own repo) | See that repo's README for its latest results |
| Frontend + tests + build (own repo) | See that repo's README for its latest results |
| Testnet deployment, tx hashes, explorer links | Verified: `deployment/testnet.json` |
| Real contract workflow on Testnet | Verified via CLI demo (2-member circle completed) |
| Frontend -> Freighter -> contract on Testnet | **Incomplete**: needs manual browser session with Freighter |
| E2E browser tests | **Incomplete** |
| Coverage measurement | **Not measured** |
| CI | Written per repo, **not yet run** on GitHub |
| Security review | Self-review in SECURITY.md; **no audit** |
| Wave eligibility | **Unknown / not applied** |
