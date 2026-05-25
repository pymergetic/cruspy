mod macros;

#[path = "pymergetic/cruspy/__init__.rs"]
mod cruspy_root;

use pyo3::prelude::*;

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    cruspy_root::init_module(m)
}
