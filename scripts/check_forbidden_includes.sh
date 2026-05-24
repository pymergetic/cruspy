#!/usr/bin/env bash
# EP-0011 — forbidden bindings (delegates to check_unified_tree.py rule 8).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
python3 "${ROOT}/scripts/check_unified_tree.py" >/dev/null
echo "forbidden binding check OK"
