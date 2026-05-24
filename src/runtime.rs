use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::module::register_submodule;

#[pyfunction]
fn discover(py: Python<'_>) -> PyResult<Py<PyAny>> {
    let metadata = py.import("importlib.metadata")?;
    let kwargs = PyDict::new(py);
    kwargs.set_item("group", "pymergetic.cruspy.components")?;
    let selected = metadata.call_method("entry_points", (), Some(&kwargs))?;
    let loaded = PyList::empty(py);
    for ep in selected.try_iter()? {
        let ep = ep?;
        let module: String = ep.getattr("value")?.extract()?;
        py.import(&module)?;
        loaded.append(ep.getattr("name")?)?;
    }
    Ok(loaded.into())
}

pub fn register_runtime_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    register_submodule(
        py,
        parent,
        "pymergetic.cruspy.runtime",
        "runtime",
        |runtime| {
            runtime.add_function(wrap_pyfunction!(discover, runtime)?)?;
            Ok(())
        },
    )
}
