//! pymergetic.cruspy — package root (Rust owner, EP-0010)

use pyo3::prelude::*;
use pyo3::types::PyModule;

#[path = "runtime/__init__.rs"]
pub mod runtime;

#[path = "models/__init__.rs"]
pub mod models;

#[path = "shm/__init__.rs"]
pub mod shm;

#[path = "testing/__init__.rs"]
pub mod testing;

pub fn init_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    runtime::register(m)?;
    models::register();
    shm::register(m)?;
    testing::register(m)?;
    Ok(())
}
