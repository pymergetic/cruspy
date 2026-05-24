#!/usr/bin/env bash
# Build libcruspy_core.so for EP-0013 plugin packaging experiments.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${ROOT}/python/pymergetic"
INCLUDE="${ROOT}/src/pymergetic/cruspy"
VERSION="$(python3 -c 'import tomllib; print(tomllib.load(open("'"${ROOT}"'/pyproject.toml","rb"))["project"]["version"])')"
SOURCES=(
  src/pymergetic/cruspy/core/mod.cpp
  src/pymergetic/cruspy/core/registry.cpp
  src/pymergetic/cruspy/runtime/mod.cpp
  src/pymergetic/cruspy/allocator/mod.cpp
  src/pymergetic/cruspy/shm/mod.cpp
  src/pymergetic/cruspy/functions/mod.cpp
)

mkdir -p "${OUT}"
OBJECTS=()
for src in "${SOURCES[@]}"; do
  obj="${OUT}/$(echo "${src}" | tr '/' '_').o"
  c++ -std=c++20 -fPIC -I"${INCLUDE}" -DCRUSPY_PKG_VERSION=\"${VERSION}\" -c "${ROOT}/${src}" -o "${obj}"
  OBJECTS+=("${obj}")
done
c++ -shared -o "${OUT}/libcruspy_core.so" "${OBJECTS[@]}"
echo "built ${OUT}/libcruspy_core.so"
