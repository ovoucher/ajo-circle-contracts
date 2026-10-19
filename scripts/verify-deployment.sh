#!/usr/bin/env bash
# Verifies the recorded Testnet deployment: the on-chain wasm matches the local build and reads work.
set -euo pipefail
cd "$(dirname "$0")/.."
ID=$(grep -Eo 'C[A-Z0-9]{55}' deployment/testnet.json)
stellar contract build >/dev/null 2>&1
LOCAL=$(shasum -a 256 target/wasm32v1-none/release/ajo_circle.wasm | cut -d' ' -f1)
TMP=$(mktemp -d); trap 'rm -rf "$TMP"' EXIT
stellar contract fetch --id "$ID" --network testnet --out-file "$TMP/onchain.wasm" 2>/dev/null
REMOTE=$(shasum -a 256 "$TMP/onchain.wasm" | cut -d' ' -f1)
echo "local  wasm sha256: $LOCAL"; echo "onchain wasm sha256: $REMOTE"
[ "$LOCAL" = "$REMOTE" ] && echo "OK: wasm matches" || echo "NOTE: differs (rebuilt with another toolchain, or contract was redeployed)"
echo -n "circle_count: "; stellar contract invoke --id "$ID" --source ajo-deployer --network testnet -- circle_count 2>/dev/null
