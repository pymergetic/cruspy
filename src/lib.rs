//! Cruspy native extension.

pub mod pymergetic;

use pyo3::prelude::*;

#[pymodule]
fn pymergetic_cruspy(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let _ = m;
    Ok(())
}
