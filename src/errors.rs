use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use crate::module::register_submodule;

create_exception!(
    pymergetic.cruspy.errors,
    SchemaConflictError,
    PyException,
    "Raised when schema_hash mismatches across components."
);

pub fn map_cxx_exception(_py: Python<'_>, message: &str) -> PyErr {
    let msg = message.to_string();
    if msg.contains("ValidationError") || msg.contains("must be") {
        return PyErr::new::<pyo3::exceptions::PyValueError, _>(msg);
    }
    if msg.contains("ShmError") {
        return PyErr::new::<pyo3::exceptions::PyOSError, _>(msg);
    }
    if msg.contains("SchemaConflictError") {
        return SchemaConflictError::new_err(msg);
    }
    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(msg)
}

#[pyfunction]
fn cruspy_error_code(exc: Bound<'_, PyAny>) -> PyResult<Option<String>> {
    Ok(exc
        .getattr("_cruspy_error_code")
        .ok()
        .and_then(|value| value.extract().ok()))
}

pub fn register_errors_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    register_submodule(py, parent, "pymergetic.cruspy.errors", "errors", |errors| {
        errors.add("SchemaConflictError", py.get_type::<SchemaConflictError>())?;
        errors.add_function(wrap_pyfunction!(cruspy_error_code, errors)?)?;
        Ok(())
    })
}
