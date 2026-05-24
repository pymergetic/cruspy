//! Handle-based registry helpers for generated model wrappers (EP-0021).

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DomainId {
    pub high: u64,
    pub low: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MemoryHandle {
    pub abi_version: u32,
    pub flags: u32,
    pub domain_id: DomainId,
    pub offset: u64,
    pub byte_size: u64,
    pub schema_hash: u64,
    pub generation: u64,
    pub embedded_offset: u64,
    pub type_fqn: [u8; 24],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CruspyError(pub i32);

impl CruspyError {
    pub fn message(self) -> &'static str {
        match self.0 {
            -1 => "invalid argument",
            -2 => "registry operation failed",
            _ => "unknown cruspy error",
        }
    }
}

extern "C" {
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
    fn cruspy_call_static_str(fqn: *const c_char, method: *const c_char, out: *mut c_char, capacity: usize) -> i32;
}

fn field_name(name: &str) -> Result<CString, CruspyError> {
    CString::new(name).map_err(|_| CruspyError(-1))
}

pub fn create_object(fqn: &str, domain: &str) -> Result<MemoryHandle, CruspyError> {
    let cfqn = CString::new(fqn).map_err(|_| CruspyError(-1))?;
    let cdomain = CString::new(domain).map_err(|_| CruspyError(-1))?;
    let mut handle = zero_handle();
    let rc = unsafe { cruspy_create(cfqn.as_ptr(), cdomain.as_ptr(), &mut handle) };
    if rc != 0 {
        return Err(CruspyError(rc));
    }
    Ok(handle)
}

pub fn field_get_i32(handle: &MemoryHandle, field: &str) -> Result<i32, CruspyError> {
    let cfield = field_name(field)?;
    let mut value = 0i32;
    let rc = unsafe { cruspy_field_get_i32(handle, cfield.as_ptr(), &mut value) };
    if rc != 0 {
        return Err(CruspyError(rc));
    }
    Ok(value)
}

pub fn field_set_i32(handle: &MemoryHandle, field: &str, value: i32) -> Result<(), CruspyError> {
    let cfield = field_name(field)?;
    let rc = unsafe { cruspy_field_set_i32(handle, cfield.as_ptr(), value) };
    if rc != 0 {
        return Err(CruspyError(rc));
    }
    Ok(())
}

pub fn field_get_i64(handle: &MemoryHandle, field: &str) -> Result<i64, CruspyError> {
    let cfield = field_name(field)?;
    let mut value = 0i64;
    let rc = unsafe { cruspy_field_get_i64(handle, cfield.as_ptr(), &mut value) };
    if rc != 0 {
        return Err(CruspyError(rc));
    }
    Ok(value)
}

pub fn field_set_i64(handle: &MemoryHandle, field: &str, value: i64) -> Result<(), CruspyError> {
    let cfield = field_name(field)?;
    let rc = unsafe { cruspy_field_set_i64(handle, cfield.as_ptr(), value) };
    if rc != 0 {
        return Err(CruspyError(rc));
    }
    Ok(())
}

pub fn field_get_f64(handle: &MemoryHandle, field: &str) -> Result<f64, CruspyError> {
    let cfield = field_name(field)?;
    let mut value = 0f64;
    let rc = unsafe { cruspy_field_get_f64(handle, cfield.as_ptr(), &mut value) };
    if rc != 0 {
        return Err(CruspyError(rc));
    }
    Ok(value)
}

pub fn field_set_f64(handle: &MemoryHandle, field: &str, value: f64) -> Result<(), CruspyError> {
    let cfield = field_name(field)?;
    let rc = unsafe { cruspy_field_set_f64(handle, cfield.as_ptr(), value) };
    if rc != 0 {
        return Err(CruspyError(rc));
    }
    Ok(())
}

pub fn field_get_bool(handle: &MemoryHandle, field: &str) -> Result<bool, CruspyError> {
    let cfield = field_name(field)?;
    let mut value = 0i32;
    let rc = unsafe { cruspy_field_get_bool(handle, cfield.as_ptr(), &mut value) };
    if rc != 0 {
        return Err(CruspyError(rc));
    }
    Ok(value != 0)
}

pub fn field_set_bool(handle: &MemoryHandle, field: &str, value: bool) -> Result<(), CruspyError> {
    let cfield = field_name(field)?;
    let rc = unsafe { cruspy_field_set_bool(handle, cfield.as_ptr(), if value { 1 } else { 0 }) };
    if rc != 0 {
        return Err(CruspyError(rc));
    }
    Ok(())
}

pub fn field_get_object(handle: &MemoryHandle, field: &str) -> Result<MemoryHandle, CruspyError> {
    let cfield = field_name(field)?;
    let mut out = zero_handle();
    let rc = unsafe { cruspy_field_get_object(handle, cfield.as_ptr(), &mut out) };
    if rc != 0 {
        return Err(CruspyError(rc));
    }
    Ok(out)
}

pub fn describe(fqn: &str) -> Result<String, CruspyError> {
    let cfqn = CString::new(fqn).map_err(|_| CruspyError(-1))?;
    let mut buf = vec![0u8; 8192];
    let rc = unsafe { cruspy_registry_describe(cfqn.as_ptr(), buf.as_mut_ptr() as *mut c_char, buf.len()) };
    if rc < 0 {
        return Err(CruspyError(rc));
    }
    let json = unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) };
    Ok(json.to_string_lossy().into_owned())
}

pub fn call_bool(handle: &MemoryHandle, method: &str) -> Result<bool, CruspyError> {
    let cmethod = CString::new(method).map_err(|_| CruspyError(-1))?;
    let mut value = 0i32;
    let rc = unsafe { cruspy_call_bool(handle, cmethod.as_ptr(), &mut value) };
    if rc != 0 {
        return Err(CruspyError(rc));
    }
    Ok(value != 0)
}

pub fn call_void(handle: &mut MemoryHandle, method: &str) -> Result<(), CruspyError> {
    let cmethod = CString::new(method).map_err(|_| CruspyError(-1))?;
    let rc = unsafe { cruspy_call_void(handle, cmethod.as_ptr()) };
    if rc != 0 {
        return Err(CruspyError(rc));
    }
    Ok(())
}

pub fn call_f64(handle: &MemoryHandle, method: &str, arg0: &str, arg1: &str) -> Result<f64, CruspyError> {
    let cmethod = CString::new(method).map_err(|_| CruspyError(-1))?;
    let c0 = CString::new(arg0).map_err(|_| CruspyError(-1))?;
    let c1 = CString::new(arg1).map_err(|_| CruspyError(-1))?;
    let mut value = 0f64;
    let rc = unsafe {
        cruspy_call_f64(
            handle,
            cmethod.as_ptr(),
            c0.as_ptr(),
            c1.as_ptr(),
            &mut value,
        )
    };
    if rc != 0 {
        return Err(CruspyError(rc));
    }
    Ok(value)
}

pub fn call_bytes(handle: &MemoryHandle, method: &str) -> Result<Vec<u8>, CruspyError> {
    let cmethod = CString::new(method).map_err(|_| CruspyError(-1))?;
    let mut buf = vec![0u8; 4096];
    let rc = unsafe { cruspy_call_bytes(handle, cmethod.as_ptr(), buf.as_mut_ptr(), buf.len()) };
    if rc < 0 {
        return Err(CruspyError(rc));
    }
    buf.truncate(rc as usize);
    Ok(buf)
}

pub fn call_constructor(fqn: &str, method: &str, arg0: &str, arg1: &str) -> Result<MemoryHandle, CruspyError> {
    let cfqn = CString::new(fqn).map_err(|_| CruspyError(-1))?;
    let cmethod = CString::new(method).map_err(|_| CruspyError(-1))?;
    let c0 = CString::new(arg0).map_err(|_| CruspyError(-1))?;
    let c1 = CString::new(arg1).map_err(|_| CruspyError(-1))?;
    let mut handle = zero_handle();
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
        return Err(CruspyError(rc));
    }
    Ok(handle)
}

pub fn call_static_str(fqn: &str, method: &str) -> Result<String, CruspyError> {
    let cfqn = CString::new(fqn).map_err(|_| CruspyError(-1))?;
    let cmethod = CString::new(method).map_err(|_| CruspyError(-1))?;
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
        return Err(CruspyError(rc));
    }
    let s = unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) };
    Ok(s.to_string_lossy().into_owned())
}

fn zero_handle() -> MemoryHandle {
    MemoryHandle {
        abi_version: 0,
        flags: 0,
        domain_id: DomainId { high: 0, low: 0 },
        offset: 0,
        byte_size: 0,
        schema_hash: 0,
        generation: 0,
        embedded_offset: 0,
        type_fqn: [0; 24],
    }
}
