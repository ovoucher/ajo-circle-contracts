# Deployment (Testnet)

```bash
stellar --version                       # tested 25.2.0 (soroban-sdk 28 needs >= 25.2.0)
./scripts/deploy-testnet.sh             # tests, builds, creates funded identity ajo-deployer via Friendbot, deploys, writes deployment/testnet.json
./scripts/verify-deployment.sh          # wasm hash + circle_count read
./scripts/demo-testnet.sh               # full 2-member circle with native XLM (takes ~70 s); funds ajo-alice / ajo-bob
```
Give the resulting `contractId` from `deployment/testnet.json` to whoever runs the Ajo Circle API (its `AJO_CONTRACT_ID`). The native-XLM token id for Testnet is `stellar contract id asset --asset native --network testnet`.

**After a Testnet reset**: identities and contracts disappear. Delete `~/.config/stellar` test identities if needed, rerun the three scripts and hand the new contract id to the API operator (who should also reset the API's database).

**Mainnet**: intentionally unsupported. The scripts hardcode `testnet`; do not change that before an audit.

## Troubleshooting
- `Error(Contract, #N)`: see the error table in `docs/CONTRACTS.md`.
