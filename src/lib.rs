use pyo3::prelude::*;

mod errors;
mod models;
mod module;
mod schema;
mod shm;

#[link(name = "cruspy-cpp", kind = "static")]
extern "C" {}

extern "C" {
    fn cruspy_runtime_version() -> *const std::ffi::c_char;
}

fn runtime_version() -> &'static str {
    unsafe {
        let ptr = cruspy_runtime_version();
        if ptr.is_null() {
            "unknown"
        } else {
            std::ffi::CStr::from_ptr(ptr).to_str().unwrap_or("unknown")
        }
    }
}

use module::ensure_package_path;
use errors::register_errors_module;
use models::document::register_document_module;
use shm::register_shm_module;

#[pymodule]
#[pyo3(name = "cruspy")]
fn cruspy(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__doc__", "cruspy — polyglot shared-memory runtime")?;
    ensure_package_path(m)?;
    m.add("ABI_VERSION", "1")?;
    m.add("RUNTIME_VERSION", runtime_version())?;
    register_errors_module(m)?;
    register_document_module(m)?;
    register_shm_module(m)?;
    Ok(())
}
