"""Nested object schema-hash validation (EP-0021)."""

from __future__ import annotations

import pytest

from pymergetic.cruspy.runtime import create, patch_field_schema_hash

DOCUMENT_FQN = "pymergetic.cruspy.models.document.Document"


def test_field_get_object_rejects_corrupted_schema_hash() -> None:
    handle = create(DOCUMENT_FQN, domain="heap_default")
    patch_field_schema_hash(handle, "meta", 0xDEADBEEF)
    with pytest.raises(RuntimeError, match="field_get_object failed"):
        handle.field_object("meta")
