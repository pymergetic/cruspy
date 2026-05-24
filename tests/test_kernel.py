"""Phases 1–4: substrate, allocator, registry, models + runtime."""

from __future__ import annotations

import json

from pymergetic.cruspy.runtime import create, describe, domain_stats_json

METADATA_FQN = "pymergetic.cruspy.models.document.metadata.Metadata"
DOCUMENT_FQN = "pymergetic.cruspy.models.document.Document"


def test_heap_domain_stats() -> None:
    stats = json.loads(domain_stats_json())
    names = {entry["name"] for entry in stats["domains"]}
    assert "heap_default" in names


def test_metadata_create_and_field_roundtrip() -> None:
    handle = create(METADATA_FQN, domain="heap_default")
    assert handle.schema_hash > 0
    assert handle.byte_size > 0
    handle.set_field_i32("id", 42)
    assert handle.field_i32("id") == 42


def test_document_create_and_fields() -> None:
    handle = create(DOCUMENT_FQN, domain="heap_default")
    handle.set_field_i32("id", 7)
    handle.set_field_f64("score", 0.875)
    assert handle.field_i32("id") == 7
    assert abs(handle.field_f64("score") - 0.875) < 1e-9


def test_describe_document() -> None:
    spec = json.loads(describe(DOCUMENT_FQN))
    assert spec["fqn"] == DOCUMENT_FQN
    fields = {f["name"]: f for f in spec["fields"]}
    assert set(fields) == {"id", "score", "meta"}
    assert fields["id"]["desc"] == "Primary identifier"
    assert fields["id"]["default"] == 0
    assert fields["id"]["min"] == 0
    assert fields["id"]["max"] == 100
    assert fields["score"]["desc"] == "Relevance score"
    assert spec["schema_hash"] > 0
