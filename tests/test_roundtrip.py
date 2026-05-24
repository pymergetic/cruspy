"""Full cross-language roundtrip from Python (EP-0021).

Exercises one Document flowing through method implementations owned by
C++ (validate, normalize, default_domain, schema), Python (score_text),
and Rust (serialize, from_json) on the same shared-memory handle.
"""

from __future__ import annotations

import json
import struct

from pymergetic.cruspy.models.document import Document
from pymergetic.cruspy.models.document.metadata import Metadata

SERIALIZE_MAGIC = b"CD02"
SERIALIZE_SIZE = 29


def unpack_document_blob(payload: bytes) -> tuple[int, float, bool, int, int]:
    assert len(payload) == SERIALIZE_SIZE
    assert payload[:4] == SERIALIZE_MAGIC
    id_val, score_val = struct.unpack("<id", payload[4:16])
    active = bool(payload[16])
    meta_id, meta_created_at = struct.unpack("<iq", payload[17:29])
    return id_val, score_val, active, meta_id, meta_created_at


def test_cross_language_roundtrip_from_python() -> None:
    """Single flow: Python ctor → C++ → Python → Rust → C++ → Python."""
    meta = Metadata(id=99, created_at=1_716_500_000)
    doc = Document(id=7, score=1.5, active=True, meta=meta)

    # C++ validate + normalize (score out of range)
    assert doc.validate() is False
    doc.normalize()
    assert doc.validate() is True
    assert doc.id == 7
    assert abs(doc.score - 1.0) < 1e-9

    # C++ static helpers
    assert Document.default_domain() == "heap_default"
    schema = json.loads(Document.schema())
    assert schema["fqn"] == "pymergetic.cruspy.models.document.Document"
    assert {f["name"] for f in schema["fields"]} == {"id", "score", "active", "meta"}

    # Python method_impl body
    score = doc.score_text("hello cruspy", model_id="default")
    assert 0.0 < score <= 1.0

    # Rust serialize — binary v2 includes nested meta
    blob = doc.serialize()
    id_v, score_v, active_v, meta_id_v, meta_created_v = unpack_document_blob(blob)
    assert id_v == doc.id
    assert abs(score_v - doc.score) < 1e-9
    assert active_v is doc.active
    assert meta_id_v == doc.meta.id
    assert meta_created_v == doc.meta.created_at

    # Rust from_json — full field roundtrip including nested meta
    payload = json.dumps(
        {
            "id": doc.id,
            "score": doc.score,
            "active": doc.active,
            "meta": {"id": doc.meta.id, "created_at": doc.meta.created_at},
        }
    )
    restored = Document.from_json(payload)
    assert restored.id == doc.id
    assert abs(restored.score - doc.score) < 1e-9
    assert restored.active == doc.active
    assert restored.meta.id == doc.meta.id
    assert restored.meta.created_at == doc.meta.created_at

    # C++ validate on restored object still works
    assert restored.validate() is True


def test_from_json_nested_meta_only() -> None:
    raw = '{"id": 3, "score": 0.25, "active": false, "meta": {"id": 1, "created_at": 99}}'
    doc = Document.from_json(raw)
    assert doc.id == 3
    assert abs(doc.score - 0.25) < 1e-9
    assert doc.active is False
    assert doc.meta.id == 1
    assert doc.meta.created_at == 99


def test_serialize_from_json_binary_roundtrip() -> None:
    original = Document(id=11, score=0.33, active=False, meta=Metadata(id=5, created_at=42))
    blob = original.serialize()
    id_v, score_v, active_v, meta_id_v, meta_created_v = unpack_document_blob(blob)
    restored = Document.from_json(
        json.dumps(
            {
                "id": id_v,
                "score": score_v,
                "active": active_v,
                "meta": {"id": meta_id_v, "created_at": meta_created_v},
            }
        )
    )
    assert restored.id == original.id
    assert abs(restored.score - original.score) < 1e-9
    assert restored.active == original.active
    assert restored.meta.id == original.meta.id
    assert restored.meta.created_at == original.meta.created_at
