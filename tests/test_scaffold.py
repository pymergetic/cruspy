"""Smoke tests for EP-0010 unified tree scaffold."""

from __future__ import annotations

import pymergetic.cruspy as cruspy


def test_extension_imports() -> None:
    assert cruspy.__name__ == "pymergetic.cruspy"


def test_version_exposed() -> None:
    assert isinstance(cruspy.__version__, str)
    assert cruspy.__version__
