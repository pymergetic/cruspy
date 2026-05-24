import pytest
from pydantic import BaseModel


def test_type_descriptor_and_registry() -> None:
    from pymergetic.cruspy.core import registered_type_count
    from pymergetic.cruspy.models.document import type_descriptor_py

    desc = type_descriptor_py()
    assert "Document" in desc["fqn"]
    assert desc["schema_hash"] > 0
    assert desc["slab_size"] > 0
    assert desc["description"] == "Indexed text document"
    assert "id" in desc["fields"]
    assert desc["fields"]["id"]["description"] == "Primary key"
    assert registered_type_count() >= 1


def test_document_shm_round_trip() -> None:
    from pymergetic.cruspy.models.document import Document, SCHEMA_HASH
    from pymergetic.cruspy.shm import ShmArena

    doc = Document(id=1, text="shm", score=0.5, active=True, revision=3)
    arena = ShmArena("docs", 4096)
    handle = doc.write_to_shm(arena)
    assert handle.schema_hash == SCHEMA_HASH
    assert handle.domain_id_low > 0
    assert handle.generation > 0
    assert handle.abi_version == 1
    view = Document.view_shm(arena, handle)
    assert view.score == pytest.approx(0.5)
    assert view.revision == 3
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
    from pymergetic.cruspy.models.document import Document
    from pymergetic.cruspy.shm import ShmArena

    arena = ShmArena("stats-docs", 4096)
    doc = Document(id=1, text="stats", score=0.3, active=True)
    handle = doc.write_to_shm(arena)
    result = stats()
    assert result.registered_count >= 1
    assert result.domain_count >= 1
    assert result.bytes_total >= handle.byte_size
    assert result.bytes_used >= handle.byte_size
    assert result.object_count >= 1


def test_allocator_resolve_and_stale_handle() -> None:
    from pymergetic.cruspy.allocator import resolve
    from pymergetic.cruspy.errors import ShmError
    from pymergetic.cruspy.models.document import Document
    from pymergetic.cruspy.shm import ShmArena

    doc = Document(id=1, text="resolve", score=0.4, active=True)
    arena = ShmArena("resolve-docs", 4096)
    handle = doc.write_to_shm(arena)
    payload = resolve(handle)
    assert len(payload) == handle.byte_size

    arena.write_bytes(handle.type_fqn, handle.schema_hash, payload)
    with pytest.raises(ShmError, match="stale"):
        resolve(handle)


def test_allocator_migrate() -> None:
    from pymergetic.cruspy.allocator import domain_stats, migrate, resolve
    from pymergetic.cruspy.models.document import Document
    from pymergetic.cruspy.shm import ShmArena

    source = ShmArena("migrate-src", 4096)
    target = ShmArena("migrate-dst", 4096)
    doc = Document(id=1, text="migrate", score=0.7, active=True)
    handle = doc.write_to_shm(source)
    target_stats = None
    for domain in __import__("pymergetic.cruspy.allocator", fromlist=["list_domain_stats"]).list_domain_stats():
        if domain.name == "migrate-dst":
            target_stats = domain
            break
    assert target_stats is not None
    migrated = migrate(handle, target_stats.domain_id_high, target_stats.domain_id_low)
    assert migrated.domain_id_low == target_stats.domain_id_low
    assert migrated.generation >= 1
    payload = resolve(migrated)
    assert len(payload) == migrated.byte_size


def test_runtime_discover() -> None:
    from pymergetic.cruspy.runtime import discover

    loaded = discover()
    assert loaded == ["document"]


def test_function_transform() -> None:
    from pymergetic.cruspy.functions import call_transform, register_transform

    register_transform(lambda x: x * 2.0)
    assert call_transform(1.5) == pytest.approx(3.0)


@pytest.mark.asyncio
async def test_write_shm_async() -> None:
    from pymergetic.cruspy.models.document import Document, SCHEMA_HASH
    from pymergetic.cruspy.shm import ShmArena, write_shm_async

    doc = Document(id=1, text="async", score=0.2, active=True, revision=1)
    arena = ShmArena("async-docs", 4096)
    handle = await write_shm_async(
        arena,
        "pymergetic::cruspy::models::document::Document",
        SCHEMA_HASH,
        doc.model_dump_json(),
    )
    assert handle.byte_size > 0
    assert handle.domain_id_low > 0


def test_cruspy_error_code_on_validation() -> None:
    from pymergetic.cruspy.errors import cruspy_error_code
    from pymergetic.cruspy.models.document import validate_document

    with pytest.raises(ValueError):
        validate_document(id=0, text="x", score=0.5, active=True, revision=None)
    try:
        validate_document(id=0, text="x", score=0.5, active=True, revision=None)
    except ValueError as exc:
        assert cruspy_error_code(exc) == "cruspy.validation"
