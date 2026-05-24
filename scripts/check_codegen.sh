#!/usr/bin/env bash
# EP-0021 codegen staleness check.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "${ROOT}"
uv run --with pyyaml --with jinja2 python tools/cruspy-gen/cruspy_gen.py --root src/pymergetic/cruspy --glob "models/**/*.openapi.yaml" --check
echo "codegen staleness check OK"
