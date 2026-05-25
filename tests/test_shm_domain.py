"""POSIX SHM domain tests (EP-0019)."""

from __future__ import annotations

import json

from pymergetic.cruspy.runtime import clone_handle, create, domain_stats_json

METADATA_FQN = "pymergetic.cruspy.models.document.metadata.Metadata"


def test_shm_default_domain_registered() -> None:
    stats = json.loads(domain_stats_json())
    names = {entry["name"] for entry in stats["domains"]}
    assert "shm_default" in names


def test_shm_domain_field_roundtrip() -> None:
    handle = create(METADATA_FQN, domain="shm_default")
    handle.set_field_i32("id", 42)
    assert handle.field_i32("id") == 42


def test_shm_domain_zero_copy_reattach() -> None:
    handle = create(METADATA_FQN, domain="shm_default")
    handle.set_field_i32("id", 7)
    handle.set_field_i64("created_at", 123456789)

    reattached = clone_handle(handle)
    assert reattached.field_i32("id") == 7
    assert reattached.field_i64("created_at") == 123456789

    reattached.set_field_i32("id", 99)
    assert handle.field_i32("id") == 99
