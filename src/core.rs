use std::sync::OnceLock;

use pyo3::prelude::*;

use crate::module::register_submodule;

#[derive(Clone, Copy, Debug)]
pub enum FieldKind {
    Int32,
    Int64,
    Float64,
    Bool,
    String,
}

#[derive(Clone, Debug)]
pub struct FieldDescriptor {
    pub name: &'static str,
    pub kind: FieldKind,
    pub offset: u32,
    pub size: u32,
}

#[derive(Clone, Debug)]
pub struct TypeDescriptor {
    pub fqn: &'static str,
    pub schema_hash: u64,
    pub slab_size: u32,
    pub fields: &'static [FieldDescriptor],
}

extern "C" {
    fn cruspy_abi_version() -> u32;
    fn cruspy_registered_type_count() -> u32;
    fn cruspy_register_type_simple(fqn: *const std::ffi::c_char, schema_hash: u64, slab_size: u32)
        -> u64;
}

static REGISTERED: OnceLock<()> = OnceLock::new();

pub fn register_model_type(descriptor: TypeDescriptor) -> u64 {
    let c_fqn = std::ffi::CString::new(descriptor.fqn).expect("valid fqn");
    let hash = unsafe {
        cruspy_register_type_simple(c_fqn.as_ptr(), descriptor.schema_hash, descriptor.slab_size)
    };
    let _ = REGISTERED.set(());
    hash
}

#[pyfunction]
fn abi_version() -> u32 {
    unsafe { cruspy_abi_version() }
}

#[pyfunction]
fn registered_type_count() -> u32 {
    unsafe { cruspy_registered_type_count() }
}

pub fn register_core_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    register_submodule(py, parent, "pymergetic.cruspy.core", "core", |core| {
        core.add_function(wrap_pyfunction!(abi_version, core)?)?;
        core.add_function(wrap_pyfunction!(registered_type_count, core)?)?;
        Ok(())
    })
}
