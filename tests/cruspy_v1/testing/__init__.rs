//! Native cross-language dispatch checks exposed to pytest (EP-0021).

use pyo3::prelude::*;
use pyo3::types::PyDict;

#[path = "hello/__init__.rs"]
mod hello;

use crate::cruspy_root::models::document::{Document, DocumentInit};

extern "C" {
    fn cruspy_test_cpp_validate() -> i32;
    fn cruspy_test_cpp_normalize() -> i32;
    fn cruspy_test_cpp_serialize_rust() -> i32;
    fn cruspy_test_cpp_from_json_rust() -> i32;
    fn cruspy_test_cpp_score_text_python() -> f64;
}

fn rust_validate_calls_cpp() -> PyResult<bool> {
    let doc = Document::new(
        "heap_default",
        DocumentInit {
            id: 50,
            score: 0.5,
            active: true,
            meta: None,
        },
    )
    .map_err(|err| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.message()))?;
    doc.validate()
        .map_err(|err| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.message()))
}

fn rust_serialize_local() -> PyResult<usize> {
    let doc = Document::new(
        "heap_default",
        DocumentInit {
            id: 7,
            score: 0.875,
            active: true,
            meta: None,
        },
    )
    .map_err(|err| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.message()))?;
    let bytes = doc
        .serialize()
        .map_err(|err| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.message()))?;
    Ok(bytes.len())
}

fn rust_from_json_constructor() -> PyResult<i32> {
    let doc = Document::from_json(
        r#"{"id":3,"score":0.25,"active":false,"meta":{"id":8,"created_at":1234}}"#,
        "heap_default",
    )
    .map_err(|err| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.message()))?;
    Ok(doc
        .id()
        .map_err(|err| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.message()))?)
}

#[pyfunction]
fn run_crosslang_dispatch_tests(py: Python<'_>) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);

    dict.set_item("rust_validate_cpp", rust_validate_calls_cpp()?)?;
    dict.set_item("rust_serialize_rust", rust_serialize_local()? == 29)?;

    let rust_id = rust_from_json_constructor()?;
    dict.set_item("rust_from_json_rust", rust_id == 3)?;

    let cpp_validate = unsafe { cruspy_test_cpp_validate() };
    dict.set_item("cpp_validate_cpp", cpp_validate == 1)?;

    let cpp_normalize = unsafe { cruspy_test_cpp_normalize() };
    dict.set_item("cpp_normalize_cpp", cpp_normalize == 1)?;

    let cpp_serialize = unsafe { cruspy_test_cpp_serialize_rust() };
    dict.set_item("cpp_serialize_rust", cpp_serialize == 29)?;

    let cpp_from_json = unsafe { cruspy_test_cpp_from_json_rust() };
    dict.set_item("cpp_from_json_rust", cpp_from_json == 0)?;

    let cpp_score = unsafe { cruspy_test_cpp_score_text_python() };
    dict.set_item("cpp_score_text_python", cpp_score > 0.0 && cpp_score <= 1.0)?;

    Ok(dict.into())
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_function(wrap_pyfunction!(run_crosslang_dispatch_tests, parent)?)?;
    parent.add_function(wrap_pyfunction!(hello::run_hello_crosslang_tests, parent)?)?;
    Ok(())
}
