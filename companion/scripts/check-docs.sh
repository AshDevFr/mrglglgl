#!/usr/bin/env bash
# Checks the user documentation: every guide exists, and the app guide's privacy section quotes the
# app's own endpoint disclosure word for word, so the two cannot drift apart.
#
# Usage, from anywhere: companion/scripts/check-docs.sh
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=0
fail() { echo "docs: $*" >&2; failures=$((failures + 1)); }

for doc in README.md CONTRIBUTING.md LICENSE docs/app-guide.md docs/narrators.md docs/releasing.md \
  docs/schema.md docs/forever-api.md addon/README.md companion/README.md; do
  [[ -s "$root/$doc" ]] || fail "$doc is missing or empty"
done

# The disclosure as the app shows it, with the address placeholder as the guide words it.
disclosure="$(node -e '
  const en = require(process.argv[1]);
  process.stdout.write(en.settings.disclosure.replace("{{host}}", "that address"));
' "$root/companion/ui/src/i18n/locales/en.json")"
grep -qF "> $disclosure" "$root/docs/app-guide.md" \
  || fail "docs/app-guide.md does not quote the app's endpoint disclosure word for word"

if (( failures > 0 )); then exit 1; fi
echo "docs: ok"
