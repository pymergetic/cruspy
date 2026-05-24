use std::ffi::CString;
use std::mem::MaybeUninit;

use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::errors::{AllocationError, map_cxx_exception, ShmError};
use crate::memory::MemoryHandle;
use crate::module::register_submodule;
use crate::schema::{decode_all_fields, decode_field_value, FieldMetaEntry};

extern "C" {
    fn cruspy_process_arena_open(name: *const std::ffi::c_char, capacity: u64) -> i32;
    fn cruspy_process_arena_allocate(
        name: *const std::ffi::c_char,
        capacity: u64,
        type_fqn: *const std::ffi::c_char,
        schema_hash: u64,
        data: *const u8,
        byte_size: u32,
        out: *mut MemoryHandle,
    ) -> i32;
    fn cruspy_resolve(
        handle: *const MemoryHandle,
        out_data: *mut u8,
        out_capacity: u32,
        out_size: *mut u32,
    ) -> i32;
    fn cruspy_substrate_last_error() -> *const std::ffi::c_char;
}

#[pyclass(name = "ShmHandle")]
#[derive(Clone, Debug)]
pub struct ShmHandle {
    #[pyo3(get)]
    pub segment: String,
    #[pyo3(get)]
    pub offset: u64,
    #[pyo3(get)]
    pub type_fqn: String,
    #[pyo3(get)]
    pub schema_hash: u64,
    #[pyo3(get)]
    pub byte_size: u32,
    #[pyo3(get)]
    pub domain_id_high: u64,
    #[pyo3(get)]
    pub domain_id_low: u64,
    #[pyo3(get)]
    pub generation: u64,
    #[pyo3(get)]
    pub abi_version: u32,
    #[pyo3(get)]
    pub flags: u32,
    pub(crate) memory: MemoryHandle,
}

impl ShmHandle {
    pub(crate) fn from_memory(segment: String, type_fqn: String, memory: MemoryHandle) -> Self {
        Self {
            segment,
            offset: memory.offset,
            type_fqn,
            schema_hash: memory.schema_hash,
            byte_size: memory.byte_size as u32,
            domain_id_high: memory.domain_id.high,
            domain_id_low: memory.domain_id.low,
            generation: memory.generation,
            abi_version: memory.abi_version,
            flags: memory.flags,
            memory,
        }
    }
}

#[pyclass(name = "ShmArena", unsendable)]
pub struct ShmArena {
    name: String,
    capacity: usize,
}

impl ShmArena {
    pub(crate) fn write_bytes_impl(
        &self,
        type_fqn: String,
        schema_hash: u64,
        payload: &[u8],
    ) -> PyResult<ShmHandle> {
        let name = CString::new(self.name.as_str()).map_err(|_| {
            AllocationError::new_err("cruspy.allocation: arena name contains NUL")
        })?;
        let fqn = CString::new(type_fqn.as_str()).map_err(|_| {
            AllocationError::new_err("cruspy.allocation: type_fqn contains NUL")
        })?;
        let mut out = MaybeUninit::<MemoryHandle>::uninit();
        let status = unsafe {
            cruspy_process_arena_allocate(
                name.as_ptr(),
                self.capacity as u64,
                fqn.as_ptr(),
                schema_hash,
                payload.as_ptr(),
                payload.len() as u32,
                out.as_mut_ptr(),
            )
        };
        if status != 0 {
            let message = unsafe {
                std::ffi::CStr::from_ptr(cruspy_substrate_last_error())
                    .to_string_lossy()
                    .into_owned()
            };
            if message.contains("allocation") {
                return Err(AllocationError::new_err(message));
            }
            return Err(ShmError::new_err(message));
        }
        Ok(ShmHandle::from_memory(
            self.name.clone(),
            type_fqn,
            unsafe { out.assume_init() },
        ))
    }

    fn read_bytes_impl(&self, handle: &ShmHandle) -> PyResult<Vec<u8>> {
        if handle.segment != self.name {
            return Err(ShmError::new_err("cruspy.shm: handle segment mismatch"));
        }
        let mut out = vec![0u8; handle.byte_size as usize];
        let mut out_size = 0u32;
        let status = unsafe {
            cruspy_resolve(
                &handle.memory,
                out.as_mut_ptr(),
                out.len() as u32,
                &mut out_size,
            )
        };
        if status != 0 {
            let message = unsafe {
                std::ffi::CStr::from_ptr(cruspy_substrate_last_error())
                    .to_string_lossy()
                    .into_owned()
            };
            return Err(ShmError::new_err(message));
        }
        out.truncate(out_size as usize);
        Ok(out)
    }
}

#[pymethods]
impl ShmArena {
    #[new]
    fn new(name: String, size: usize) -> Self {
        if let Ok(name_c) = CString::new(name.as_str()) {
            unsafe {
                let _ = cruspy_process_arena_open(name_c.as_ptr(), size as u64);
            }
        }
        Self {
            name,
            capacity: size,
        }
    }

    fn __repr__(&self) -> String {
        format!("ShmArena(name={:?}, size={})", self.name, self.capacity)
    }

    fn write_bytes(
        &self,
        type_fqn: String,
        schema_hash: u64,
        payload: &[u8],
    ) -> PyResult<ShmHandle> {
        self.write_bytes_impl(type_fqn, schema_hash, payload)
    }

    fn read_bytes(&self, handle: &ShmHandle) -> PyResult<Vec<u8>> {
        self.read_bytes_impl(handle)
    }
}

#[pyclass(name = "ShmView")]
pub struct ShmView {
    payload: Vec<u8>,
    model_module: String,
    model_name: String,
    field_meta: Vec<FieldMetaEntry>,
}

impl ShmView {
    fn field_meta_static(&self) -> &[FieldMetaEntry] {
        self.field_meta.as_slice()
    }
}

#[pymethods]
impl ShmView {
    fn __getattr__(&self, name: String) -> PyResult<Py<PyAny>> {
        Python::with_gil(|py| {
            decode_field_value(py, &self.payload, &name, self.field_meta_static())
        })
    }

    fn __setattr__(&self, _name: String, _value: Bound<'_, PyAny>) -> PyResult<()> {
        Err(ShmError::new_err("ShmView is read-only"))
    }

    fn materialize(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let model_mod = PyModule::import(py, &self.model_module)?;
        let cls = model_mod.getattr(&self.model_name)?;
        let data = decode_all_fields(py, &self.payload, self.field_meta_static())?;
        cls.call_method1("model_validate", (data,)).map(|obj| obj.unbind())
    }
}

pub fn write_model_to_shm(
    arena: &Bound<'_, ShmArena>,
    model: Bound<'_, PyAny>,
    type_fqn: &str,
    schema_hash: u64,
) -> PyResult<ShmHandle> {
    let dumped = model.call_method0("model_dump_json")?;
    let payload: &str = dumped.extract()?;
    arena
        .borrow()
        .write_bytes_impl(type_fqn.to_string(), schema_hash, payload.as_bytes())
}

pub fn view_model_shm(
    py: Python<'_>,
    arena: &Bound<'_, ShmArena>,
    handle: &ShmHandle,
    expected_schema_hash: u64,
    model_module: &str,
    model_name: &str,
    field_meta: &[FieldMetaEntry],
) -> PyResult<ShmView> {
    if handle.schema_hash != expected_schema_hash {
        return Err(map_cxx_exception(
            py,
            "cruspy.schema_conflict:schema_hash mismatch",
        ));
    }
    let payload = arena.borrow().read_bytes_impl(handle)?;
    Ok(ShmView {
        payload,
        model_module: model_module.to_string(),
        model_name: model_name.to_string(),
        field_meta: field_meta.to_vec(),
    })
}

static TRANSFORM: std::sync::OnceLock<Py<PyAny>> = std::sync::OnceLock::new();

#[pyfunction]
fn register_transform(callable: Bound<'_, PyAny>) -> PyResult<()> {
    let _ = TRANSFORM.set(callable.unbind());
    Ok(())
}

#[pyfunction]
fn call_transform(value: f32) -> PyResult<f32> {
    Python::with_gil(|py| {
        let callable = TRANSFORM.get().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("transform slot is not registered")
        })?;
        let result = callable.call1(py, (value,))?;
        result.extract(py)
    })
}

#[pyfunction]
fn write_shm_async(
    py: Python<'_>,
    arena: Bound<'_, ShmArena>,
    type_fqn: String,
    schema_hash: u64,
    payload: String,
) -> PyResult<Py<PyAny>> {
    let handle = arena
        .borrow()
        .write_bytes_impl(type_fqn, schema_hash, payload.as_bytes())?;
    let handle_py = handle.into_pyobject(py)?.into_any().unbind();
    pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(handle_py) }).map(|bound| bound.unbind())
}

pub fn register_shm_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    register_submodule(py, parent, "pymergetic.cruspy.shm", "shm", |shm| {
        shm.add_class::<ShmArena>()?;
        shm.add_class::<ShmHandle>()?;
        shm.add_class::<ShmView>()?;
        shm.add_function(wrap_pyfunction!(write_shm_async, shm)?)?;
        Ok(())
    })?;
    register_submodule(
        py,
        parent,
        "pymergetic.cruspy.functions",
        "functions",
        |functions| {
            functions.add_function(wrap_pyfunction!(register_transform, functions)?)?;
            functions.add_function(wrap_pyfunction!(call_transform, functions)?)?;
            Ok(())
        },
    )?;
    Ok(())
}
