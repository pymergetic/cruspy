"""Cruspy schema mixin for generated Pydantic models."""

from __future__ import annotations

from typing import Any


class CruspyModel:
    """Mixin attached to codegen Pydantic models."""

    @classmethod
    def type_descriptor_py(cls) -> dict[str, Any]:
        from importlib import import_module

        module = import_module(cls.__module__)
        fn = getattr(module, "type_descriptor_py", None)
        if fn is None:
            raise TypeError(f"{cls.__module__} does not expose type_descriptor_py()")
        return fn()
