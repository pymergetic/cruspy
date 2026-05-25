//! pymergetic.cruspy.runtime — PyO3 surface; C++ kernel via extern "C"

#[path = "kernel.rs"]
pub mod kernel;

use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Mutex, OnceLock};

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
#[derive(Clone, Copy, Debug)]
struct DomainId {
    high: u64,
    low: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MemoryHandle {
    abi_version: u32,
    flags: u32,
    domain_id: DomainId,
    offset: u64,
    byte_size: u64,
    schema_hash: u64,
    generation: u64,
    embedded_offset: u64,
    type_fqn: [u8; 24],
}

type PyMethod = Py<PyAny>;

static PYTHON_METHODS: OnceLock<Mutex<HashMap<(String, String), PyMethod>>> = OnceLock::new();

fn python_methods() -> &'static Mutex<HashMap<(String, String), PyMethod>> {
    PYTHON_METHODS.get_or_init(|| Mutex::new(HashMap::new()))
}

extern "C" {
    fn cruspy_bootstrap();
    fn cruspy_module_ensure(path: *const c_char);
    fn cruspy_allocator_stats_json(buffer: *mut c_char, capacity: usize) -> i32;
    fn cruspy_create(fqn: *const c_char, domain_name: *const c_char, out: *mut MemoryHandle) -> i32;
    fn cruspy_field_get_i32(handle: *const MemoryHandle, field: *const c_char, out: *mut i32) -> i32;
    fn cruspy_field_set_i32(handle: *const MemoryHandle, field: *const c_char, value: i32) -> i32;
    fn cruspy_field_get_i64(handle: *const MemoryHandle, field: *const c_char, out: *mut i64) -> i32;
    fn cruspy_field_set_i64(handle: *const MemoryHandle, field: *const c_char, value: i64) -> i32;
    fn cruspy_field_get_f64(handle: *const MemoryHandle, field: *const c_char, out: *mut f64) -> i32;
    fn cruspy_field_set_f64(handle: *const MemoryHandle, field: *const c_char, value: f64) -> i32;
    fn cruspy_field_get_bool(handle: *const MemoryHandle, field: *const c_char, out: *mut i32) -> i32;
    fn cruspy_field_set_bool(handle: *const MemoryHandle, field: *const c_char, value: i32) -> i32;
    fn cruspy_field_get_object(handle: *const MemoryHandle, field: *const c_char, out: *mut MemoryHandle) -> i32;
    fn cruspy_field_get_string(
        handle: *const MemoryHandle,
        field: *const c_char,
        out: *mut c_char,
        capacity: usize,
    ) -> i32;
    fn cruspy_field_set_string(
        handle: *const MemoryHandle,
        field: *const c_char,
        value: *const c_char,
        len: usize,
    ) -> i32;
    fn cruspy_registry_describe(fqn: *const c_char, buffer: *mut c_char, capacity: usize) -> i32;
    fn cruspy_call_bool(handle: *const MemoryHandle, method: *const c_char, out: *mut i32) -> i32;
    fn cruspy_call_void(handle: *mut MemoryHandle, method: *const c_char) -> i32;
    fn cruspy_call_f64(
        handle: *const MemoryHandle,
        method: *const c_char,
        arg0: *const c_char,
        arg1: *const c_char,
        out: *mut f64,
    ) -> i32;
    fn cruspy_call_bytes(
        handle: *const MemoryHandle,
        method: *const c_char,
        out: *mut u8,
        capacity: usize,
    ) -> i32;
    fn cruspy_call_constructor(
        fqn: *const c_char,
        method: *const c_char,
        arg0: *const c_char,
        arg1: *const c_char,
        out: *mut MemoryHandle,
    ) -> i32;
    fn cruspy_call_static_str(fqn: *const c_char, method: *const c_char, out: *mut c_char, capacity: usize)
        -> i32;
    fn cruspy_register_python_method(fqn: *const c_char, method: *const c_char) -> i32;
    fn cruspy_bind_python_method(fqn: *const c_char, method: *const c_char, py_fn: *mut std::ffi::c_void) -> i32;
    fn cruspy_foreach_python_method(
        callback: Option<unsafe extern "C" fn(*const c_char, *const c_char, *mut std::ffi::c_void)>,
        user: *mut std::ffi::c_void,
    );
    fn cruspy_resolve_handle_fqn(handle: *const MemoryHandle, out: *mut c_char, capacity: usize) -> i32;
    fn cruspy_test_patch_field_schema_hash(
        handle: *const MemoryHandle,
        field: *const c_char,
        schema_hash: u64,
    ) -> i32;
}

const PY_METHOD_BOUND: *mut std::ffi::c_void = 1 as *mut std::ffi::c_void;

unsafe extern "C" fn bind_pending_python_method(fqn: *const c_char, method: *const c_char, _user: *mut std::ffi::c_void) {
    if fqn.is_null() || method.is_null() {
        return;
    }
    let fqn_str = CStr::from_ptr(fqn).to_string_lossy();
    let method_str = CStr::from_ptr(method).to_string_lossy();
    let key = (fqn_str.into_owned(), method_str.into_owned());
    if let Ok(guard) = python_methods().lock() {
        if guard.contains_key(&key) {
            let _ = cruspy_bind_python_method(fqn, method, PY_METHOD_BOUND);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn cruspy_resolve_python_methods(_py_module: *mut pyo3::ffi::PyObject) -> i32 {
    cruspy_foreach_python_method(Some(bind_pending_python_method), std::ptr::null_mut());
    if let Ok(guard) = python_methods().lock() {
        for ((fqn, method), _) in guard.iter() {
            let cfqn = CString::new(fqn.as_str()).ok();
            let cmethod = CString::new(method.as_str()).ok();
            if let (Some(cfqn), Some(cmethod)) = (cfqn, cmethod) {
                let _ = cruspy_bind_python_method(cfqn.as_ptr(), cmethod.as_ptr(), PY_METHOD_BOUND);
            }
        }
    }
    0
}

fn fqn_from_handle(handle: &MemoryHandle) -> String {
    let mut buf = vec![0u8; 256];
    let rc = unsafe {
        cruspy_resolve_handle_fqn(
            handle,
            buf.as_mut_ptr() as *mut c_char,
            buf.len(),
        )
    };
    if rc >= 0 {
        let json = unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) };
        return json.to_string_lossy().into_owned();
    }
    let end = handle
        .type_fqn
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(handle.type_fqn.len());
    String::from_utf8_lossy(&handle.type_fqn[..end]).into_owned()
}

#[no_mangle]
pub unsafe extern "C" fn cruspy_dispatch_python_f64(
    handle: *const MemoryHandle,
    method: *const c_char,
    arg0: *const c_char,
    arg1: *const c_char,
    out: *mut f64,
) -> i32 {
    if handle.is_null() || method.is_null() || out.is_null() {
        return -1;
    }
    let method_name = match CStr::from_ptr(method).to_str() {
        Ok(name) => name.to_string(),
        Err(_) => return -1,
    };
    let arg0_str = if arg0.is_null() {
        String::new()
    } else {
        CStr::from_ptr(arg0).to_string_lossy().into_owned()
    };
    let arg1_str = if arg1.is_null() {
        String::new()
    } else {
        CStr::from_ptr(arg1).to_string_lossy().into_owned()
    };
    let result = Python::with_gil(|py| -> PyResult<f64> {
        let fqn = fqn_from_handle(&*handle);
        let key = (fqn, method_name);
        let guard = python_methods()
            .lock()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("python method registry lock poisoned"))?;
        let callable = guard.get(&key).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "no python method registered for {:?}",
                key
            ))
        })?;
        let wrapper = PyMemoryHandle { inner: *handle };
        callable.bind(py).call1((wrapper, arg0_str, arg1_str))?.extract()
    });
    match result {
        Ok(value) => {
            *out = value;
            0
        }
        Err(_) => -2,
    }
}

#[no_mangle]
pub unsafe extern "C" fn cruspy_dispatch_python_bytes(
    handle: *const MemoryHandle,
    method: *const c_char,
    out: *mut u8,
    capacity: usize,
) -> i32 {
    if handle.is_null() || method.is_null() {
        return -1;
    }
    let method_name = match CStr::from_ptr(method).to_str() {
        Ok(name) => name.to_string(),
        Err(_) => return -1,
    };
    let result = Python::with_gil(|py| -> PyResult<Vec<u8>> {
        let fqn = fqn_from_handle(&*handle);
        let key = (fqn, method_name);
        let guard = python_methods()
            .lock()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("python method registry lock poisoned"))?;
        let callable = guard.get(&key).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "no python method registered for {:?}",
                key
            ))
        })?;
        let wrapper = PyMemoryHandle { inner: *handle };
        let value = callable.bind(py).call1((wrapper,))?;
        if let Ok(blob) = value.extract::<Vec<u8>>() {
            Ok(blob)
        } else if let Ok(text) = value.extract::<String>() {
            Ok(text.into_bytes())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "python bytes method must return bytes or str",
            ))
        }
    });
    match result {
        Ok(blob) => {
            if capacity == 0 {
                return blob.len() as i32;
            }
            if out.is_null() || capacity < blob.len() {
                return -1;
            }
            std::ptr::copy_nonoverlapping(blob.as_ptr(), out, blob.len());
            blob.len() as i32
        }
        Err(_) => -2,
    }
}

#[pyclass(name = "MemoryHandle", module = "pymergetic.cruspy.runtime")]
#[derive(Clone, Copy)]
pub struct PyMemoryHandle {
    pub(crate) inner: MemoryHandle,
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

    fn set_field_i32(&mut self, name: &str, value: i32) -> PyResult<()> {
        let cname = CString::new(name).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid field"))?;
        let rc = unsafe { cruspy_field_set_i32(&self.inner, cname.as_ptr(), value) };
        if rc != 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "field_set_i32 failed ({rc})"
            )));
        }
        Ok(())
    }

    fn field_i64(&self, name: &str) -> PyResult<i64> {
        let cname = CString::new(name).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid field"))?;
        let mut value = 0i64;
        let rc = unsafe { cruspy_field_get_i64(&self.inner, cname.as_ptr(), &mut value) };
        if rc != 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "field_get_i64 failed ({rc})"
            )));
        }
        Ok(value)
    }

    fn set_field_i64(&mut self, name: &str, value: i64) -> PyResult<()> {
        let cname = CString::new(name).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid field"))?;
        let rc = unsafe { cruspy_field_set_i64(&self.inner, cname.as_ptr(), value) };
        if rc != 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "field_set_i64 failed ({rc})"
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

    fn set_field_f64(&mut self, name: &str, value: f64) -> PyResult<()> {
        let cname = CString::new(name).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid field"))?;
        let rc = unsafe { cruspy_field_set_f64(&self.inner, cname.as_ptr(), value) };
        if rc != 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "field_set_f64 failed ({rc})"
            )));
        }
        Ok(())
    }

    fn field_bool(&self, name: &str) -> PyResult<bool> {
        let cname = CString::new(name).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid field"))?;
        let mut value = 0i32;
        let rc = unsafe { cruspy_field_get_bool(&self.inner, cname.as_ptr(), &mut value) };
        if rc != 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "field_get_bool failed ({rc})"
            )));
        }
        Ok(value != 0)
    }

    fn set_field_bool(&mut self, name: &str, value: bool) -> PyResult<()> {
        let cname = CString::new(name).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid field"))?;
        let rc = unsafe { cruspy_field_set_bool(&self.inner, cname.as_ptr(), if value { 1 } else { 0 }) };
        if rc != 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "field_set_bool failed ({rc})"
            )));
        }
        Ok(())
    }

    fn field_string(&self, name: &str) -> PyResult<String> {
        let cname = CString::new(name).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid field"))?;
        let mut buf = vec![0u8; 256];
        let rc = unsafe {
            cruspy_field_get_string(
                &self.inner,
                cname.as_ptr(),
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
            )
        };
        if rc < 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "field_get_string failed ({rc})"
            )));
        }
        buf.truncate(rc as usize);
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }

    fn set_field_string(&mut self, name: &str, value: &str) -> PyResult<()> {
        let cname = CString::new(name).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid field"))?;
        let cvalue = CString::new(value).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid value"))?;
        let rc = unsafe {
            cruspy_field_set_string(
                &self.inner,
                cname.as_ptr(),
                cvalue.as_ptr(),
                cvalue.as_bytes().len(),
            )
        };
        if rc != 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "field_set_string failed ({rc})"
            )));
        }
        Ok(())
    }

    fn field_object(&self, name: &str) -> PyResult<PyMemoryHandle> {
        let cname = CString::new(name).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid field"))?;
        let mut out = MemoryHandle {
            abi_version: 0,
            flags: 0,
            domain_id: DomainId { high: 0, low: 0 },
            offset: 0,
            byte_size: 0,
            schema_hash: 0,
            generation: 0,
            embedded_offset: 0,
            type_fqn: [0; 24],
        };
        let rc = unsafe { cruspy_field_get_object(&self.inner, cname.as_ptr(), &mut out) };
        if rc != 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "field_get_object failed ({rc})"
            )));
        }
        Ok(PyMemoryHandle { inner: out })
    }

    fn call_bool(&self, method: &str) -> PyResult<bool> {
        let cmethod = CString::new(method).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid method"))?;
        let mut value = 0i32;
        let rc = unsafe { cruspy_call_bool(&self.inner, cmethod.as_ptr(), &mut value) };
        if rc != 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "call_bool failed ({rc})"
            )));
        }
        Ok(value != 0)
    }

    fn call_void(&mut self, method: &str) -> PyResult<()> {
        let cmethod = CString::new(method).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid method"))?;
        let rc = unsafe { cruspy_call_void(&mut self.inner, cmethod.as_ptr()) };
        if rc != 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "call_void failed ({rc})"
            )));
        }
        Ok(())
    }

    #[pyo3(signature = (method, arg0=None, arg1=None))]
    fn call_f64(&self, method: &str, arg0: Option<&str>, arg1: Option<&str>) -> PyResult<f64> {
        let cmethod = CString::new(method).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid method"))?;
        let c0 = CString::new(arg0.unwrap_or("")).unwrap();
        let c1 = CString::new(arg1.unwrap_or("")).unwrap();
        let mut value = 0f64;
        let rc = unsafe {
            cruspy_call_f64(
                &self.inner,
                cmethod.as_ptr(),
                c0.as_ptr(),
                c1.as_ptr(),
                &mut value,
            )
        };
        if rc != 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "call_f64 failed ({rc})"
            )));
        }
        Ok(value)
    }

    fn call_bytes(&self, method: &str) -> PyResult<Vec<u8>> {
        let cmethod = CString::new(method).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid method"))?;
        let mut buf = vec![0u8; 4096];
        let rc = unsafe { cruspy_call_bytes(&self.inner, cmethod.as_ptr(), buf.as_mut_ptr(), buf.len()) };
        if rc < 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "call_bytes failed ({rc})"
            )));
        }
        buf.truncate(rc as usize);
        Ok(buf)
    }

    fn call_bytes_size(&self, method: &str) -> PyResult<i32> {
        let cmethod = CString::new(method).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid method"))?;
        let rc = unsafe { cruspy_call_bytes(&self.inner, cmethod.as_ptr(), std::ptr::null_mut(), 0) };
        if rc < 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "call_bytes_size failed ({rc})"
            )));
        }
        Ok(rc)
    }

    #[classmethod]
    fn _call_constructor(_cls: &Bound<'_, PyType>, fqn: &str, method: &str, arg0: &str, arg1: &str) -> PyResult<Self> {
        let cfqn = CString::new(fqn).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid fqn"))?;
        let cmethod = CString::new(method).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid method"))?;
        let c0 = CString::new(arg0).unwrap();
        let c1 = CString::new(arg1).unwrap();
        let mut handle = MemoryHandle {
            abi_version: 0,
            flags: 0,
            domain_id: DomainId { high: 0, low: 0 },
            offset: 0,
            byte_size: 0,
            schema_hash: 0,
            generation: 0,
            embedded_offset: 0,
            type_fqn: [0; 24],
        };
        let rc = unsafe {
            cruspy_call_constructor(
                cfqn.as_ptr(),
                cmethod.as_ptr(),
                c0.as_ptr(),
                c1.as_ptr(),
                &mut handle,
            )
        };
        if rc != 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "call_constructor failed ({rc})"
            )));
        }
        Ok(PyMemoryHandle { inner: handle })
    }

    #[staticmethod]
    fn _call_static_str(fqn: &str, method: &str) -> PyResult<String> {
        let cfqn = CString::new(fqn).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid fqn"))?;
        let cmethod = CString::new(method).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid method"))?;
        let mut buf = vec![0u8; 4096];
        let rc = unsafe {
            cruspy_call_static_str(
                cfqn.as_ptr(),
                cmethod.as_ptr(),
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
            )
        };
        if rc < 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("call_static_str failed"));
        }
        let s = unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) };
        Ok(s.to_string_lossy().into_owned())
    }

    fn clone(&self) -> Self {
        PyMemoryHandle { inner: self.inner }
    }
}

#[pyfunction]
fn clone_handle(handle: &PyMemoryHandle) -> PyMemoryHandle {
    handle.clone()
}

#[pyfunction]
fn patch_field_schema_hash(handle: &PyMemoryHandle, field: &str, schema_hash: u64) -> PyResult<()> {
    let cfield =
        CString::new(field).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid field"))?;
    let rc = unsafe {
        cruspy_test_patch_field_schema_hash(&handle.inner, cfield.as_ptr(), schema_hash)
    };
    if rc != 0 {
        return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "patch_field_schema_hash failed ({rc})"
        )));
    }
    Ok(())
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
        embedded_offset: 0,
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

#[pyfunction]
fn method_impl(_py: Python<'_>, model_class: Bound<'_, PyAny>, method_name: &str, func: Bound<'_, PyAny>) -> PyResult<()> {
    let fqn: String = model_class.getattr("_FQN")?.extract()?;
    let cfqn = CString::new(fqn.clone()).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid fqn"))?;
    let cmethod =
        CString::new(method_name).map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid method"))?;
    let rc = unsafe { cruspy_register_python_method(cfqn.as_ptr(), cmethod.as_ptr()) };
    if rc != 0 {
        return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "register_python_method failed ({rc})"
        )));
    }
    let key = (fqn, method_name.to_string());
    if let Ok(mut guard) = python_methods().lock() {
        guard.insert(key.clone(), func.clone().unbind());
    }
    let rc_bind = unsafe { cruspy_bind_python_method(cfqn.as_ptr(), cmethod.as_ptr(), PY_METHOD_BOUND) };
    if rc_bind != 0 {
        return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "bind_python_method failed ({rc_bind})"
        )));
    }
    Ok(())
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    ModulePath::RUNTIME.ensure();
    unsafe {
        cruspy_bootstrap();
    };
    let py = parent.py();
    let m = PyModule::new(py, "pymergetic.cruspy.runtime")?;
    m.add_class::<PyMemoryHandle>()?;
    m.add_function(wrap_pyfunction!(bootstrap, &m)?)?;
    m.add_function(wrap_pyfunction!(domain_stats_json, &m)?)?;
    m.add_function(wrap_pyfunction!(create, &m)?)?;
    m.add_function(wrap_pyfunction!(describe, &m)?)?;
    m.add_function(wrap_pyfunction!(method_impl, &m)?)?;
    m.add_function(wrap_pyfunction!(clone_handle, &m)?)?;
    m.add_function(wrap_pyfunction!(patch_field_schema_hash, &m)?)?;
    parent.add_submodule(&m)?;
    py.import("sys")?
        .getattr("modules")?
        .set_item("pymergetic.cruspy.runtime", &m)?;
    Ok(())
}

pub fn finalize_python_methods(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let _ = py.import("pymergetic.cruspy.models.document")?;
    let _ = py.import("pymergetic.cruspy.models.hello")?;
    unsafe {
        cruspy_resolve_python_methods(parent.as_ptr());
    }
    Ok(())
}
