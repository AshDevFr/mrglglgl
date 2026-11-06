#!/usr/bin/env bash
# Exits 0 only if the latest run of a GitHub Actions workflow on main finished successfully.
# Usage: companion/scripts/check-ci.sh <workflow file name>, e.g. companion.yml
set -euo pipefail

workflow="${1:?usage: $0 <workflow file name>}"
run="$(gh run list --workflow "$workflow" --branch main --limit 1 \
  --json status,conclusion,headSha,url \
  --jq '.[0] // empty | "\(.status) \(.conclusion) \(.headSha[0:7]) \(.url)"')"

if [[ -z $run ]]; then
  echo "no run of $workflow on main yet" >&2
  exit 1
fi

read -r status conclusion sha url <<<"$run"
echo "$workflow on main at $sha: $status ${conclusion:-pending} ($url)"
[[ $status == completed && $conclusion == success ]]
