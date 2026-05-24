#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${ROOT}/src/pymergetic/cruspy"
FAIL=0

if rg -n '#include\s+<Python\.h>' "${SRC}"; then
  echo "forbidden: Python.h in cruspy C++ sources" >&2
  FAIL=1
fi

if rg -n 'nanobind|#include\s+<nanobind' "${SRC}"; then
  echo "forbidden: nanobind in cruspy C++ sources" >&2
  FAIL=1
fi

if [[ "${FAIL}" -ne 0 ]]; then
  exit 1
fi

echo "forbidden include check passed"
