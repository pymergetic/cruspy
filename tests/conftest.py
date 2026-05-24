"""Pytest bootstrap — ensure pymergetic.cruspy loads via installed extension (EP-0010)."""

from __future__ import annotations

import importlib


def pytest_configure() -> None:
    # Force extension init so PyO3 submodules win over on-disk namespace dirs.
    importlib.import_module("pymergetic.cruspy")
