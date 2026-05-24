import importlib

import pytest
from pydantic import BaseModel


def test_type_descriptor_and_registry() -> None:
    from pymergetic.cruspy.core import registered_type_count
    from pymergetic.cruspy.models.document import type_descriptor_py

    desc = type_descriptor_py()
    assert "Document" in desc["fqn"]
    assert desc["schema_hash"] > 0
    assert desc["slab_size"] > 0
    assert registered_type_count() >= 2


def test_document_shm_round_trip() -> None:
    from pymergetic.cruspy.models.document import Document, SCHEMA_HASH
    from pymergetic.cruspy.shm import ShmArena

    doc = Document(id=1, text="shm", score=0.5, active=True)
    arena = ShmArena("docs", 4096)
    handle = doc.write_to_shm(arena)
    assert handle.schema_hash == SCHEMA_HASH
    view = Document.view_shm(arena, handle)
    assert view.score == pytest.approx(0.5)
    restored = view.materialize()
    assert isinstance(restored, BaseModel)
    assert restored.model_dump() == doc.model_dump()


def test_shm_view_is_read_only() -> None:
    from pymergetic.cruspy.errors import ShmError
    from pymergetic.cruspy.models.document import Document
    from pymergetic.cruspy.shm import ShmArena

    doc = Document(id=1, text="x", score=0.1, active=True)
    arena = ShmArena("readonly", 4096)
    view = Document.view_shm(arena, doc.write_to_shm(arena))
    with pytest.raises(ShmError, match="read-only"):
        view.score = 0.9


def test_allocator_stats() -> None:
    from pymergetic.cruspy.allocator import stats

    result = stats()
    assert result.registered_count >= 2


def test_runtime_discover() -> None:
    from pymergetic.cruspy.runtime import discover

    loaded = discover()
    assert "document" in loaded
    assert "token" in loaded


def test_function_transform() -> None:
    from pymergetic.cruspy.functions import call_transform, register_transform

    register_transform(lambda x: x * 2.0)
    assert call_transform(1.5) == pytest.approx(3.0)


@pytest.mark.asyncio
async def test_write_shm_async() -> None:
    from pymergetic.cruspy.models.document import Document, SCHEMA_HASH
    from pymergetic.cruspy.shm import ShmArena, write_shm_async

    doc = Document(id=1, text="async", score=0.2, active=True)
    arena = ShmArena("async-docs", 4096)
    handle = await write_shm_async(
        arena,
        "pymergetic::cruspy::models::document::Document",
        SCHEMA_HASH,
        doc.model_dump_json(),
    )
    assert handle.byte_size > 0


def test_cruspy_error_code_on_validation() -> None:
    from pymergetic.cruspy.errors import cruspy_error_code
    from pymergetic.cruspy.models.document import validate_document

    with pytest.raises(ValueError):
        validate_document(id=0, text="x", score=0.5, active=True)
    try:
        validate_document(id=0, text="x", score=0.5, active=True)
    except ValueError as exc:
        assert cruspy_error_code(exc) == "cruspy.validation"
