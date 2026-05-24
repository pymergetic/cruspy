#!/usr/bin/env bash
set -euo pipefail

# Regenerate build_clangd/compile_commands.json for clangd (stable local paths).
#
# Run after dependency or C++ source changes, or when IDE squiggles return.
# From repo root: bash scripts/clangd-update.sh

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${ROOT_DIR}/build_clangd"

cmake -S "${ROOT_DIR}" \
  -B "${BUILD_DIR}" \
  -DCMAKE_EXPORT_COMPILE_COMMANDS=ON

cmake --build "${BUILD_DIR}" --target cruspy_clangd -j"$(nproc)"
