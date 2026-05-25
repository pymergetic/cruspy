"""Hello cross-language demo model (EP-0021)."""

from __future__ import annotations

from pymergetic.cruspy.runtime import method_impl

from .hello_gen import Hello

__all__ = ["Hello"]


def hello_python(self) -> bytes:
    return f"Hello from Python — {self.field_string('message')}".encode("utf-8")


method_impl(Hello, "hello_python", hello_python)
