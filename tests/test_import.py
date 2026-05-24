import importlib

import pytest


def test_import_root() -> None:
    mod = importlib.import_module("pymergetic.cruspy")
    assert "polyglot" in (mod.__doc__ or "")
