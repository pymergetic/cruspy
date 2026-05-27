"""Hello cross-language demo model (EP-0021)."""

from __future__ import annotations

from pymergetic.cruspy.runtime import CRUSPY_REGISTER_METHOD

from .hello_gen import Hello

__all__ = ["Hello"]


def hello_python(self) -> bytes:
    return f"Hello from Python — {self.field_string('message')}".encode("utf-8")


CRUSPY_REGISTER_METHOD(Hello, hello_python)
