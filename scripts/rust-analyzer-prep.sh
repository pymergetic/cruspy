#!/usr/bin/env bash
# Build debug proc-macro dylibs for rust-analyzer (pyo3 #[pyclass], etc.).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"
cargo check -q
echo "rust-analyzer prep: debug proc-macros built under ${ROOT}/target/debug/deps"
