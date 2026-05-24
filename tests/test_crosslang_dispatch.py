"""Cross-language dispatch from non-Python callers (EP-0021)."""

from __future__ import annotations

import importlib

import pytest

from pymergetic.cruspy.models.document import Document
from pymergetic.cruspy.runtime import create

DOCUMENT_FQN = "pymergetic.cruspy.models.document.Document"


def _native():
    return importlib.import_module("pymergetic.cruspy._native")


def test_score_text_goes_through_registry_vtable() -> None:
    """Python impl methods must dispatch via registry, not a patched class method."""
    doc = Document(id=1, score=0.0)
    via_model = doc.score_text("hello world")
    via_handle = doc.handle.call_f64("score_text", "hello world", "default")
    assert via_model == via_handle


@pytest.mark.parametrize(
    "key",
    [
        "rust_validate_cpp",
        "rust_serialize_rust",
        "rust_from_json_rust",
        "cpp_validate_cpp",
        "cpp_normalize_cpp",
        "cpp_serialize_rust",
        "cpp_from_json_rust",
        "cpp_score_text_python",
    ],
)
def test_native_crosslang_dispatch(key: str) -> None:
    results = _native().run_crosslang_dispatch_tests()
    assert results[key] is True, f"{key} failed in native harness"


def test_registry_fallback_from_raw_handle() -> None:
    """Raw MemoryHandle callers use the same vtable as generated wrappers."""
    handle = create(DOCUMENT_FQN, domain="heap_default")
    handle.set_field_i32("id", 50)
    handle.set_field_f64("score", 0.5)
    assert handle.call_bool("validate") is True
    blob = handle.call_bytes("serialize")
    assert blob[:4] == b"CD02"
    assert len(blob) == 29
