use pyo3::prelude::*;
use pyo3::types::PyList;

pub fn register_submodule<F>(
    py: Python<'_>,
    parent: &Bound<'_, PyModule>,
    qualified_name: &str,
    short_name: &str,
    init: F,
) -> PyResult<()>
where
    F: FnOnce(&Bound<'_, PyModule>) -> PyResult<()>,
{
    let module = PyModule::new(py, qualified_name)?;
    init(&module)?;
    ensure_package_path(&module)?;
    parent.add(short_name, &module)?;
    let sys = py.import("sys")?;
    sys.getattr("modules")?.set_item(qualified_name, &module)?;
    Ok(())
}

pub fn ensure_package_path(m: &Bound<'_, PyModule>) -> PyResult<()> {
    if !m.hasattr("__path__")? {
        m.add("__path__", PyList::empty(m.py()))?;
    }
    Ok(())
}
