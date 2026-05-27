//! Hello 3×3 dispatch matrix — Rust caller row (EP-0021).
//!
//! ```text
//!   rust_calls_cpp     → Hello::hello_cpp()    → C++ impl
//!   rust_calls_rust    → Hello::hello_rust()   → Rust impl
//!   rust_calls_python  → Hello::hello_python() → Python impl
//! ```

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::cruspy_root::models::hello::{Hello, HelloInit};
use crate::cruspy_root::runtime::kernel::CruspyError;

const MESSAGE: &str = "cruspy";

extern "C" {
    fn cruspy_test_cpp_calls_cpp() -> i32;
    fn cruspy_test_cpp_calls_rust() -> i32;
    fn cruspy_test_cpp_calls_python() -> i32;
}

fn expected(lang: &str) -> Vec<u8> {
    format!("Hello from {lang} — {MESSAGE}").into_bytes()
}

fn check_greeting(result: Result<Vec<u8>, CruspyError>, lang: &str) -> bool {
    result.map(|bytes| bytes == expected(lang)).unwrap_or(false)
}

fn hello_with_message() -> PyResult<Hello> {
    Hello::new(
        "heap_default",
        HelloInit {
            message: MESSAGE.to_string(),
        },
    )
    .map_err(|err| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.message()))
}

fn rust_calls_cpp() -> PyResult<bool> {
    let hello = hello_with_message()?;
    Ok(check_greeting(hello.hello_cpp(), "C++"))
}

fn rust_calls_rust() -> PyResult<bool> {
    let hello = hello_with_message()?;
    Ok(check_greeting(hello.hello_rust(), "Rust"))
}

fn rust_calls_python() -> PyResult<bool> {
    let hello = hello_with_message()?;
    Ok(check_greeting(hello.hello_python(), "Python"))
}

#[pyfunction]
pub fn run_hello_crosslang_tests(py: Python<'_>) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);

    dict.set_item("rust_calls_cpp", rust_calls_cpp()?)?;
    dict.set_item("rust_calls_rust", rust_calls_rust()?)?;
    dict.set_item("rust_calls_python", rust_calls_python()?)?;

    dict.set_item(
        "cpp_calls_cpp",
        unsafe { cruspy_test_cpp_calls_cpp() == 1 },
    )?;
    dict.set_item(
        "cpp_calls_rust",
        unsafe { cruspy_test_cpp_calls_rust() == 1 },
    )?;
    dict.set_item(
        "cpp_calls_python",
        unsafe { cruspy_test_cpp_calls_python() == 1 },
    )?;

    Ok(dict.into())
}
