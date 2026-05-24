use pyo3::prelude::*;

use crate::module::register_submodule;

extern "C" {
    fn cruspy_registered_type_count() -> u32;
}

#[pyclass(name = "RegistryStats")]
#[derive(Clone, Copy, Debug)]
pub struct RegistryStats {
    #[pyo3(get)]
    pub registered_count: u32,
}

#[pyfunction]
fn stats() -> RegistryStats {
    RegistryStats {
        registered_count: unsafe { cruspy_registered_type_count() },
    }
}

pub fn register_allocator_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    register_submodule(
        py,
        parent,
        "pymergetic.cruspy.allocator",
        "allocator",
        |allocator| {
            allocator.add_class::<RegistryStats>()?;
            allocator.add_function(wrap_pyfunction!(stats, allocator)?)?;
            Ok(())
        },
    )
}
