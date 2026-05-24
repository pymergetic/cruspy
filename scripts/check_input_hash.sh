#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HASH_FILE="${ROOT}/target/.input_hash"
CURRENT="$(
  find "${ROOT}/src/pymergetic/cruspy" -name '*.hpp' -o -name '*.cpp' \
    | sort \
    | xargs sha256sum \
    | sha256sum \
    | awk '{print $1}'
)"

if [[ -f "${HASH_FILE}" ]] && [[ "$(cat "${HASH_FILE}")" == "${CURRENT}" ]]; then
  echo "input hash unchanged: ${CURRENT}"
  exit 0
fi

echo "${CURRENT}" > "${HASH_FILE}"
echo "input hash updated: ${CURRENT}"
