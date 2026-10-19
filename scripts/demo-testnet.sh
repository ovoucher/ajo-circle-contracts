#!/usr/bin/env bash
# Runs a real 2-member circle on Testnet using the native XLM token (SAC).
# Creates/funds throwaway identities ajo-alice and ajo-bob via Friendbot. Testnet only.
set -euo pipefail
cd "$(dirname "$0")/.."
N=testnet
ID=$(grep -Eo '"contractId": *"C[A-Z0-9]+"' deployment/testnet.json | grep -Eo 'C[A-Z0-9]{55}')
for u in ajo-alice ajo-bob; do
  stellar keys address $u >/dev/null 2>&1 || stellar keys generate $u --network $N --fund
done
A=$(stellar keys address ajo-alice); B=$(stellar keys address ajo-bob)
TOKEN=$(stellar contract id asset --asset native --network $N)
inv() { local src=$1; shift; stellar contract invoke --id "$ID" --source "$src" --network $N -- "$@"; }
echo "create";  CID=$(inv ajo-alice create_circle --organizer "$A" --token "$TOKEN" --members "[\"$A\",\"$B\"]" --contribution 10000000 --period 60 --grace 0 | tail -1)
echo "circle id: $CID"
inv ajo-alice join --circle_id "$CID" --member "$A"
inv ajo-bob   join --circle_id "$CID" --member "$B"
echo "round 0: bob pays alice"
inv ajo-bob contribute --circle_id "$CID" --member "$B"
inv ajo-bob settle_round --circle_id "$CID"
echo "waiting 60s for round 1 to open"; sleep 62
inv ajo-alice contribute --circle_id "$CID" --member "$A"
inv ajo-alice settle_round --circle_id "$CID"
inv ajo-alice claim --circle_id "$CID" --member "$A"
inv ajo-bob   claim --circle_id "$CID" --member "$B"
inv ajo-alice get_circle --circle_id "$CID"
