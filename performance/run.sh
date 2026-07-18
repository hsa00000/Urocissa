#!/usr/bin/env bash
set -euo pipefail

command_name="${1:-smoke}"
shift || true
performance_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ ! -d "$performance_root/node_modules" ]]; then
  npm ci --prefix "$performance_root"
  npm run install-browser --prefix "$performance_root"
fi

node "$performance_root/run.mjs" "$command_name" "$@"
