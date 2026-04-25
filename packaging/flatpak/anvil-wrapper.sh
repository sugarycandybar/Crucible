#!/usr/bin/env sh
set -eu
export PATH="/app/bin:$PATH"
cd /app/share
exec python3 /app/share/anvil.py "$@"
