use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::module::register_submodule;

#[pyclass(name = "ShmArena")]
pub struct ShmArena {
    name: String,
    size: usize,
}

#[pymethods]
impl ShmArena {
    #[new]
    fn new(name: String, size: usize) -> Self {
        Self { name, size }
    }

    fn __repr__(&self) -> String {
        format!("ShmArena(name={:?}, size={})", self.name, self.size)
    }
}

#[pyfunction]
fn register_transform(_callable: Bound<'_, PyAny>) -> PyResult<()> {
    Ok(())
}

pub fn register_shm_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    register_submodule(py, parent, "pymergetic.cruspy.shm", "shm", |shm| {
        shm.add_class::<ShmArena>()?;
        Ok(())
    })?;
    register_submodule(
        py,
        parent,
        "pymergetic.cruspy.functions",
        "functions",
        |functions| {
            functions.add_function(wrap_pyfunction!(register_transform, functions)?)?;
            Ok(())
        },
    )
}
