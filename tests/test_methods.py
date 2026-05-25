"""Cross-language method dispatch tests (EP-0021 Phase 5)."""

from __future__ import annotations

import json
import struct

from pymergetic.cruspy.models.document import Document
from pymergetic.cruspy.models.document.metadata import Metadata


def test_validate_cpp() -> None:
    doc = Document(id=50, score=0.5)
    assert doc.validate() is True

    bad = Document(id=200, score=0.5)
    assert bad.validate() is False


def test_normalize_cpp() -> None:
    doc = Document(id=-5, score=1.5)
    doc.normalize()
    assert doc.id == 0
    assert abs(doc.score - 1.0) < 1e-9


def test_score_text_python() -> None:
    doc = Document(id=1, score=0.0)
    score = doc.score_text("hello world")
    assert 0.0 < score <= 1.0
    assert doc.score_text("hello world", model_id="other") <= score


def test_score_text_callable_after_import() -> None:
    import pymergetic.cruspy  # noqa: F401

    from pymergetic.cruspy.models.document import Document as ImportedDocument

    doc = ImportedDocument(id=2, score=0.0)
    assert doc.score_text("hello") > 0.0


def test_serialize_rust() -> None:
    doc = Document(id=7, score=0.875, active=True, meta=Metadata(id=3, created_at=99))
    payload = doc.serialize()
    assert payload[:4] == b"CD02"
    assert len(payload) == 29
    id_val, score_val = struct.unpack("<id", payload[4:16])
    active = bool(payload[16])
    meta_id, meta_created_at = struct.unpack("<iq", payload[17:29])
    assert id_val == 7
    assert abs(score_val - 0.875) < 1e-9
    assert active is True
    assert meta_id == 3
    assert meta_created_at == 99


def test_call_bytes_capacity_probe() -> None:
    doc = Document(id=1, score=0.25)
    assert doc.handle.call_bytes_size("serialize") == 29


def test_from_json_rust() -> None:
    raw = '{"id": 3, "score": 0.25, "active": false, "meta": {"id": 8, "created_at": 1234}}'
    doc = Document.from_json(raw)
    assert doc.id == 3
    assert abs(doc.score - 0.25) < 1e-9
    assert doc.active is False
    assert doc.meta.id == 8
    assert doc.meta.created_at == 1234


def test_from_json_roundtrip_fields() -> None:
    original = Document(id=11, score=0.33, active=False, meta=Metadata(id=5, created_at=42))
    blob = original.serialize()
    id_val, score_val = struct.unpack("<id", blob[4:16])
    active = bool(blob[16])
    meta_id, meta_created_at = struct.unpack("<iq", blob[17:29])
    json_str = json.dumps(
        {
            "id": id_val,
            "score": score_val,
            "active": active,
            "meta": {"id": meta_id, "created_at": meta_created_at},
        }
    )
    restored = Document.from_json(json_str)
    assert restored.id == original.id
    assert abs(restored.score - original.score) < 1e-9
    assert restored.active == original.active
    assert restored.meta.id == original.meta.id
    assert restored.meta.created_at == original.meta.created_at


def test_default_domain_static() -> None:
    assert Document.default_domain() == "heap_default"


def test_schema_class_method() -> None:
    schema = json.loads(Document.schema())
    assert schema["fqn"] == "pymergetic.cruspy.models.document.Document"
    field_names = {f["name"] for f in schema["fields"]}
    assert "active" in field_names
