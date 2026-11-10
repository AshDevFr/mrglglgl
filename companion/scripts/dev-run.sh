#!/bin/sh
# Runs the app binary `make dev` just built, signed first with a stable identity when one is
# given. macOS ties a keychain item's "Always Allow" to the program asking: to its code hash for
# an unsigned build, which every rebuild changes, so the endpoint key is asked for again after
# each change; to the certificate and identifier for a signed one, which rebuilds keep.
set -e
binary="$1"
if [ -n "$DEV_SIGN" ] && [ "$(uname)" = Darwin ]; then
  codesign --force --sign "$DEV_SIGN" --identifier io.github.ashdevfr.mrglglgl "$binary" \
    2>/dev/null || echo "dev-run: could not sign with $DEV_SIGN; the keychain may ask again" >&2
fi
exec "$@"
