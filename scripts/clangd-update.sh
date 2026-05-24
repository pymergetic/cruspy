#!/usr/bin/env bash
# Regenerate build_clangd/compile_commands.json for clangd (stable include paths).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${ROOT}/build_clangd"
INCLUDE_ROOT="${ROOT}/src/pymergetic/cruspy"
CXXBRIDGE_LINK="${BUILD_DIR}/cxxbridge-include"

cd "${ROOT}"
cargo build -q 2>/dev/null || cargo build -q

CXXBRIDGE="$(find "${ROOT}/target/debug/build" -path '*/out/cxxbridge/include' -type d 2>/dev/null | head -1)"
if [[ -z "${CXXBRIDGE}" ]]; then
  CXXBRIDGE="$(find "${ROOT}/target/release/build" -path '*/out/cxxbridge/include' -type d 2>/dev/null | head -1)"
fi
if [[ -z "${CXXBRIDGE}" ]]; then
  echo "error: cxxbridge include dir not found; run cargo build first" >&2
  exit 1
fi

mkdir -p "${BUILD_DIR}"
ln -sfn "$(realpath "${CXXBRIDGE}")" "${CXXBRIDGE_LINK}"

# Per-model headers are indexed directly under models/<module>/.
REFLECT_CPP="${ROOT}/target/deps/reflect-cpp/include"
if [[ ! -f "${REFLECT_CPP}/rfl/Field.hpp" ]]; then
  cargo build -q
fi
if [[ ! -f "${REFLECT_CPP}/rfl/Field.hpp" ]]; then
  echo "error: reflect-cpp headers not found at ${REFLECT_CPP}; run cargo build first" >&2
  exit 1
fi

COMMON_FLAGS=(-std=c++20 "-I${INCLUDE_ROOT}" "-I${CXXBRIDGE_LINK}" "-I${REFLECT_CPP}")

python3 - "${ROOT}" "${BUILD_DIR}" "${COMMON_FLAGS[@]}" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
build_dir = Path(sys.argv[2])
common = sys.argv[3:]

cpp_root = root / "src" / "pymergetic" / "cruspy"
entries = []

def add_entry(path: Path, *, header: bool) -> None:
    rel = path.relative_to(root)
    flags = list(common)
    if header:
        flags.extend(["-x", "c++-header"])
    flags.extend(["-c", str(path)])
    entries.append(
        {
            "directory": str(root),
            "file": str(path),
            "command": "c++ " + " ".join(flags),
        }
    )

for cpp in sorted(cpp_root.rglob("*.cpp")):
    add_entry(cpp, header=False)

for hpp in sorted(cpp_root.rglob("*.hpp")):
    add_entry(hpp, header=True)

out = build_dir / "compile_commands.json"
out.write_text(json.dumps(entries, indent=2) + "\n", encoding="utf-8")
print(f"wrote {out} ({len(entries)} entries)")
PY

echo "cxxbridge include: ${CXXBRIDGE}"
