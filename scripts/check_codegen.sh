#!/usr/bin/env bash
# EP-0012 codegen staleness — skipped until exporter is wired (EP-0020 optional freeze).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if compgen -G "${ROOT}/src/pymergetic/cruspy/**/__init__.rs" > /dev/null 2>&1; then
  echo "TODO: compare __init__.rs / __init__.pyi against registry snapshot"
  exit 0
fi
echo "codegen staleness check: no generated __init.rs files yet — skipped"
