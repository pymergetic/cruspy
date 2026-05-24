use std::sync::OnceLock;

use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::errors::{AllocationError, map_cxx_exception, ShmError};
use crate::module::register_submodule;

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
}

#[pyclass(name = "ShmArena", unsendable)]
pub struct ShmArena {
    name: String,
    capacity: usize,
    storage: std::sync::Mutex<Vec<u8>>,
}

impl ShmArena {
    fn write_bytes_impl(
        &self,
        type_fqn: String,
        schema_hash: u64,
        payload: &[u8],
    ) -> PyResult<ShmHandle> {
        if payload.len() > self.capacity {
            return Err(AllocationError::new_err("SHM slot exceeds arena capacity"));
        }
        let mut storage = self.storage.lock().expect("arena storage");
        storage[..payload.len()].copy_from_slice(payload);
        Ok(ShmHandle {
            segment: self.name.clone(),
            offset: 0,
            type_fqn,
            schema_hash,
            byte_size: payload.len() as u32,
        })
    }

    fn read_bytes_impl(&self, handle: &ShmHandle) -> PyResult<Vec<u8>> {
        if handle.segment != self.name {
            return Err(ShmError::new_err("handle segment mismatch"));
        }
        let storage = self.storage.lock().expect("arena storage");
        if handle.byte_size as usize > storage.len() {
            return Err(ShmError::new_err("handle out of bounds"));
        }
        Ok(storage[..handle.byte_size as usize].to_vec())
    }
}

#[pymethods]
impl ShmArena {
    #[new]
    fn new(name: String, size: usize) -> Self {
        Self {
            name,
            capacity: size,
            storage: std::sync::Mutex::new(vec![0; size]),
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
}

#[pymethods]
impl ShmView {
    fn __getattr__(&self, name: String) -> PyResult<Py<PyAny>> {
        Python::with_gil(|py| {
            let model_mod = PyModule::import(py, &self.model_module)?;
            let cls = model_mod.getattr(&self.model_name)?;
            let json_mod = PyModule::import(py, "json")?;
            let text = std::str::from_utf8(&self.payload).map_err(|err| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(err.to_string())
            })?;
            let data = json_mod.getattr("loads")?.call1((text,))?;
            let instance = cls.call_method1("model_validate", (data,))?;
            instance.getattr(name).map(|value| value.unbind())
        })
    }

    fn __setattr__(&self, _name: String, _value: Bound<'_, PyAny>) -> PyResult<()> {
        Err(ShmError::new_err("ShmView is read-only"))
    }

    fn materialize(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let model_mod = PyModule::import(py, &self.model_module)?;
        let cls = model_mod.getattr(&self.model_name)?;
        let json_mod = PyModule::import(py, "json")?;
        let text = std::str::from_utf8(&self.payload).map_err(|err| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(err.to_string())
        })?;
        let data = json_mod.getattr("loads")?.call1((text,))?;
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
    })
}

static TRANSFORM: OnceLock<Py<PyAny>> = OnceLock::new();

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
