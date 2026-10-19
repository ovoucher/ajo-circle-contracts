#!/usr/bin/env bash
# Build, test and deploy the Ajo Circle contract to Stellar Testnet.
# Testnet only: refuses any other network. Secrets stay in the local stellar CLI keystore.
set -euo pipefail
cd "$(dirname "$0")/.."

NETWORK=testnet
IDENTITY="${AJO_DEPLOYER:-ajo-deployer}"
OUT=deployment/testnet.json

command -v stellar >/dev/null || { echo "stellar CLI not found" >&2; exit 1; }
stellar --version | head -1

stellar network ls | grep -qx "$NETWORK" || stellar network add "$NETWORK" \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"

if ! stellar keys address "$IDENTITY" >/dev/null 2>&1; then
  stellar keys generate "$IDENTITY" --network "$NETWORK" --fund
fi
echo "deployer: $(stellar keys address "$IDENTITY")"

cargo test --quiet
stellar contract build
WASM=target/wasm32v1-none/release/ajo_circle.wasm

DEPLOY_LOG=$(stellar contract deploy --wasm "$WASM" --source "$IDENTITY" --network "$NETWORK" 2>&1 | tee /dev/stderr)
CONTRACT_ID=$(echo "$DEPLOY_LOG" | grep -Eo 'C[A-Z2-7]{55}' | tail -1)
[ -n "$CONTRACT_ID" ] || { echo "could not read contract id" >&2; exit 1; }

mkdir -p deployment
printf '{\n  "network": "%s",\n  "contractId": "%s",\n  "wasmSha256": "%s",\n  "deployer": "%s"\n}\n' \
  "$NETWORK" "$CONTRACT_ID" "$(shasum -a 256 "$WASM" | cut -d' ' -f1)" "$(stellar keys address "$IDENTITY")" > "$OUT"
echo "contract: $CONTRACT_ID  (saved to $OUT)"
echo "explorer: https://stellar.expert/explorer/testnet/contract/$CONTRACT_ID"
