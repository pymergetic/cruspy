//! PyO3 root for `pymergetic.cruspy` (EP-0010 / EP-0012).
use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "cruspy")]
fn cruspy(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__doc__", "cruspy — polyglot shared-memory runtime")?;
    Ok(())
}
