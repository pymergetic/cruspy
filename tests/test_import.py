import importlib
import json
from importlib.metadata import version

import pytest
from pydantic import BaseModel, ValidationError


def test_import_root() -> None:
    mod = importlib.import_module("pymergetic.cruspy")
    assert "polyglot" in (mod.__doc__ or "")
    assert mod.ABI_VERSION == "1"
    assert mod.RUNTIME_VERSION == version("pymergetic-cruspy")


def test_import_document_model() -> None:
    from pymergetic.cruspy.models.document import Document, SCHEMA_HASH, TYPE_FQN

    doc = Document(id=1, text="hello", score=0.5, active=True)
    assert isinstance(doc, BaseModel)
    assert doc.model_dump() == {
        "id": 1,
        "text": "hello",
        "score": 0.5,
        "active": True,
    }
    assert TYPE_FQN == "pymergetic::cruspy::models::document::Document"
    assert isinstance(SCHEMA_HASH, int)
    assert SCHEMA_HASH > 0


def test_document_json_round_trip() -> None:
    from pymergetic.cruspy.models.document import Document

    doc = Document(id=42, text="round-trip", score=0.25, active=False)
    payload = doc.model_dump_json()
    restored = Document.model_validate_json(payload)
    assert restored.model_dump() == doc.model_dump()
    assert json.loads(payload) == {
        "id": 42,
        "text": "round-trip",
        "score": 0.25,
        "active": False,
    }


def test_document_validation() -> None:
    from pymergetic.cruspy.models.document import Document

    with pytest.raises(ValidationError):
        Document(id=0, text="x", score=0.5, active=True)
    with pytest.raises(ValidationError):
        Document(id=1, text="x", score=1.5, active=True)


def test_cpp_validate_document() -> None:
    from pymergetic.cruspy.models.document import validate_document

    validate_document(id=1, text="ok", score=0.5, active=True)
    with pytest.raises(ValueError, match="id must be >= 1"):
        validate_document(id=0, text="x", score=0.5, active=True)
    with pytest.raises(ValueError, match="score must be between"):
        validate_document(id=1, text="x", score=2.0, active=True)


def test_errors_module() -> None:
    from pymergetic.cruspy.errors import SchemaConflictError, cruspy_error_code

    exc = SchemaConflictError("schema mismatch")
    assert cruspy_error_code(exc) is None


def test_shm_and_functions_modules() -> None:
    from pymergetic.cruspy.functions import register_transform
    from pymergetic.cruspy.shm import ShmArena

    arena = ShmArena("test", 1024)
    assert "test" in repr(arena)
    register_transform(lambda x: x)
