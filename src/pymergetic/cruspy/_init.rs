//! pymergetic.cruspy — package root (Rust owner, EP-0010)

use pyo3::prelude::*;
use pyo3::types::PyModule;

#[path = "runtime/_init.rs"]
pub mod runtime;

#[path = "shm/_init.rs"]
pub mod shm;

pub fn init_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    runtime::register(m)?;
    shm::register(m)?;
    Ok(())
}
