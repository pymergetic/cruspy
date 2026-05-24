#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE="${ROOT}/src/pymergetic/cruspy"
OUTPUT="${SOURCE}"
NAMESPACE="pymergetic::cruspy"

if [[ -x "${ROOT}/../../.venv/bin/python" ]]; then
  PYTHON="${ROOT}/../../.venv/bin/python"
elif [[ -n "${VIRTUAL_ENV:-}" && -x "${VIRTUAL_ENV}/bin/python" ]]; then
  PYTHON="${VIRTUAL_ENV}/bin/python"
else
  PYTHON="${PYTHON:-python3}"
fi

exec "${PYTHON}" -m pymergetic.easybind.rust_codegen.cli \
  --source "${SOURCE}" \
  --namespace "${NAMESPACE}" \
  --output "${OUTPUT}" \
  "$@"
