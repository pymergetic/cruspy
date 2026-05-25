"""Hello 3×3 dispatch matrix: caller language × impl language (EP-0021).

Each cell checks that a caller in language X can invoke hello_{impl} and receive
bytes from the native implementation in language Y:

    caller \\ impl   cpp          rust         python
    python           .hello_cpp() .hello_rust() .hello_python()
    cpp              native       native        native
    rust             native       native        native
"""

from __future__ import annotations

import importlib
from typing import Literal

import pytest

from pymergetic.cruspy.models.hello import Hello

Caller = Literal["python", "cpp", "rust"]
Impl = Literal["cpp", "rust", "python"]

MESSAGE = "cruspy"

IMPL_LANG: dict[Impl, str] = {
    "cpp": "C++",
    "rust": "Rust",
    "python": "Python",
}

IMPL_METHOD: dict[Impl, str] = {
    "cpp": "hello_cpp",
    "rust": "hello_rust",
    "python": "hello_python",
}


def _native():
    return importlib.import_module("pymergetic.cruspy._native")


def _expected(impl: Impl) -> bytes:
    return f"Hello from {IMPL_LANG[impl]} — {MESSAGE}".encode()


def _matrix_id(caller: Caller, impl: Impl) -> str:
    return f"{caller}_calls_{impl}"


def _call_from_python(impl: Impl) -> bytes:
    hello = Hello(message=MESSAGE)
    return getattr(hello, IMPL_METHOD[impl])()


def _call_from_native(caller: Caller, impl: Impl) -> bool:
    key = _matrix_id(caller, impl)
    results = _native().run_hello_crosslang_tests()
    return bool(results[key])


@pytest.mark.parametrize(
    ("caller", "impl"),
    [
        pytest.param("python", "cpp", id="python_calls_cpp"),
        pytest.param("python", "rust", id="python_calls_rust"),
        pytest.param("python", "python", id="python_calls_python"),
        pytest.param("cpp", "cpp", id="cpp_calls_cpp"),
        pytest.param("cpp", "rust", id="cpp_calls_rust"),
        pytest.param("cpp", "python", id="cpp_calls_python"),
        pytest.param("rust", "cpp", id="rust_calls_cpp"),
        pytest.param("rust", "rust", id="rust_calls_rust"),
        pytest.param("rust", "python", id="rust_calls_python"),
    ],
)
def test_dispatch_matrix(caller: Caller, impl: Impl) -> None:
    if caller == "python":
        assert _call_from_python(impl) == _expected(impl)
    else:
        assert _call_from_native(caller, impl), (
            f"{caller} caller failed to dispatch to {impl} impl "
            f"(expected {_expected(impl)!r})"
        )


def test_all_impls_read_same_message_field_from_python() -> None:
    hello = Hello(message="shared")
    for method in IMPL_METHOD.values():
        assert b"shared" in getattr(hello, method)()
