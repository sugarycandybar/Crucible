#!/usr/bin/env sh
set -eu
cd /app/share
exec python3 /app/share/anvil.py "$@"
