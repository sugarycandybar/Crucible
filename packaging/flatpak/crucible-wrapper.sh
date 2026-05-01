#!/usr/bin/env sh
set -eu
export PATH="/app/bin:$PATH"
export PYTHONPATH="/app/share:${PYTHONPATH:-}"

exec python3 /app/share/crucible/main.py "$@"
