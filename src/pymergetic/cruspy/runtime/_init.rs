//! pymergetic.cruspy.runtime — PyO3 surface; C++ kernel via extern "C"

use pyo3::prelude::*;
use pyo3::types::PyModule;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Harmonized module path — mirrors ``pymergetic::cruspy::module::kPackageRoot`` (C++).
pub struct ModulePath(&'static str);

impl ModulePath {
    pub const ROOT: Self = Self("pymergetic.cruspy");
    pub const RUNTIME: Self = Self("pymergetic.cruspy.runtime");

    pub const fn new(path: &'static str) -> Self {
        Self(path)
    }

    pub fn as_str(self) -> &'static str {
        self.0
    }

    pub fn ensure(self) {
        let path = CString::new(self.0).expect("module path contains NUL");
        unsafe { cruspy_module_ensure(path.as_ptr()) };
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DomainId {
    high: u64,
    low: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MemoryHandle {
    abi_version: u32,
    flags: u32,
    domain_id: DomainId,
    offset: u64,
    byte_size: u64,
    schema_hash: u64,
    generation: u64,
    type_fqn: [u8; 24],
}

extern "C" {
    fn cruspy_bootstrap();
    fn cruspy_module_ensure(path: *const c_char);
    fn cruspy_allocator_stats_json(buffer: *mut c_char, capacity: usize) -> i32;
    fn cruspy_create(
        fqn: *const c_char,
        domain_name: *const c_char,
        out: *mut MemoryHandle,
    ) -> i32;
    fn cruspy_field_get_i32(handle: *const MemoryHandle, field: *const c_char, out: *mut i32) -> i32;
    fn cruspy_field_set_i32(handle: *const MemoryHandle, field: *const c_char, value: i32) -> i32;
    fn cruspy_field_get_f64(handle: *const MemoryHandle, field: *const c_char, out: *mut f64) -> i32;
    fn cruspy_field_set_f64(handle: *const MemoryHandle, field: *const c_char, value: f64) -> i32;
    fn cruspy_registry_describe(fqn: *const c_char, buffer: *mut c_char, capacity: usize) -> i32;
}

#[pyclass(name = "MemoryHandle", module = "pymergetic.cruspy.runtime")]
#[derive(Clone, Copy)]
struct PyMemoryHandle {
    inner: MemoryHandle,
}

#[pymethods]
impl PyMemoryHandle {
    #[getter]
    fn schema_hash(&self) -> u64 {
        self.inner.schema_hash
    }

    #[getter]
    fn byte_size(&self) -> u64 {
        self.inner.byte_size
    }

    #[getter]
    fn type_fqn(&self) -> String {
        let end = self
            .inner
            .type_fqn
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.inner.type_fqn.len());
        String::from_utf8_lossy(&self.inner.type_fqn[..end]).into_owned()
    }

    fn field_i32(&self, name: &str) -> PyResult<i32> {
        let cname = CString::new(name).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid field"))?;
        let mut value = 0i32;
        let rc = unsafe { cruspy_field_get_i32(&self.inner, cname.as_ptr(), &mut value) };
        if rc != 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "field_get_i32 failed ({rc})"
            )));
        }
        Ok(value)
    }

    fn set_field_i32(&self, name: &str, value: i32) -> PyResult<()> {
        let cname = CString::new(name).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid field"))?;
        let rc = unsafe { cruspy_field_set_i32(&self.inner, cname.as_ptr(), value) };
        if rc != 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "field_set_i32 failed ({rc})"
            )));
        }
        Ok(())
    }

    fn field_f64(&self, name: &str) -> PyResult<f64> {
        let cname = CString::new(name).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid field"))?;
        let mut value = 0f64;
        let rc = unsafe { cruspy_field_get_f64(&self.inner, cname.as_ptr(), &mut value) };
        if rc != 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "field_get_f64 failed ({rc})"
            )));
        }
        Ok(value)
    }

    fn set_field_f64(&self, name: &str, value: f64) -> PyResult<()> {
        let cname = CString::new(name).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid field"))?;
        let rc = unsafe { cruspy_field_set_f64(&self.inner, cname.as_ptr(), value) };
        if rc != 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "field_set_f64 failed ({rc})"
            )));
        }
        Ok(())
    }
}

#[pyfunction]
fn bootstrap() -> PyResult<()> {
    unsafe { cruspy_bootstrap() };
    Ok(())
}

#[pyfunction]
fn domain_stats_json() -> PyResult<String> {
    let mut buf = vec![0u8; 4096];
    let rc = unsafe { cruspy_allocator_stats_json(buf.as_mut_ptr() as *mut c_char, buf.len()) };
    if rc < 0 {
        return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("domain stats failed"));
    }
    let json = unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) };
    Ok(json.to_string_lossy().into_owned())
}

#[pyfunction]
#[pyo3(signature = (fqn, domain=None))]
fn create(fqn: &str, domain: Option<&str>) -> PyResult<PyMemoryHandle> {
    let domain_name = domain.unwrap_or("heap_default");
    let cfqn = CString::new(fqn).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid fqn"))?;
    let cdomain =
        CString::new(domain_name).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid domain"))?;
    let mut handle = MemoryHandle {
        abi_version: 0,
        flags: 0,
        domain_id: DomainId { high: 0, low: 0 },
        offset: 0,
        byte_size: 0,
        schema_hash: 0,
        generation: 0,
        type_fqn: [0; 24],
    };
    let rc = unsafe { cruspy_create(cfqn.as_ptr(), cdomain.as_ptr(), &mut handle) };
    if rc != 0 {
        return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("create failed ({rc})")));
    }
    Ok(PyMemoryHandle { inner: handle })
}

#[pyfunction]
fn describe(fqn: &str) -> PyResult<String> {
    let cfqn = CString::new(fqn).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid fqn"))?;
    let mut buf = vec![0u8; 8192];
    let rc = unsafe { cruspy_registry_describe(cfqn.as_ptr(), buf.as_mut_ptr() as *mut c_char, buf.len()) };
    if rc < 0 {
        return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("describe failed"));
    }
    let json = unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) };
    Ok(json.to_string_lossy().into_owned())
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    ModulePath::RUNTIME.ensure();
    unsafe { cruspy_bootstrap() };
    let py = parent.py();
    let m = PyModule::new(py, "pymergetic.cruspy.runtime")?;
    m.add_class::<PyMemoryHandle>()?;
    m.add_function(wrap_pyfunction!(bootstrap, &m)?)?;
    m.add_function(wrap_pyfunction!(domain_stats_json, &m)?)?;
    m.add_function(wrap_pyfunction!(create, &m)?)?;
    m.add_function(wrap_pyfunction!(describe, &m)?)?;
    parent.add_submodule(&m)?;
    py.import("sys")?
        .getattr("modules")?
        .set_item("pymergetic.cruspy.runtime", &m)?;
    Ok(())
}
