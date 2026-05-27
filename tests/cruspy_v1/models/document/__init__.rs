//! Hand-written Document model — Rust method bodies (EP-0021).

#[path = "metadata/__init__.rs"]
pub mod metadata;

include!("__init___gen.rs");

use crate::CRUSPY_REGISTER_METHOD;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::cruspy_root::runtime::kernel::MemoryHandle as KernelMemoryHandle;
use serde::Deserialize;

const SERIALIZE_MAGIC: [u8; 4] = *b"CD02";
const SERIALIZE_SIZE: usize = 29;

#[derive(Debug, Deserialize)]
struct DocumentJson {
    #[serde(default)]
    id: i32,
    #[serde(default)]
    score: f64,
    #[serde(default = "default_active")]
    active: bool,
    #[serde(default)]
    meta: MetaJson,
}

#[derive(Debug, Default, Deserialize)]
struct MetaJson {
    #[serde(default)]
    id: i32,
    #[serde(default)]
    created_at: i64,
}

fn default_active() -> bool {
    true
}

extern "C" {
    fn cruspy_create(fqn: *const c_char, domain_name: *const c_char, out: *mut KernelMemoryHandle) -> i32;
    fn cruspy_field_get_i32(handle: *const KernelMemoryHandle, field: *const c_char, out: *mut i32) -> i32;
    fn cruspy_field_set_i32(handle: *const KernelMemoryHandle, field: *const c_char, value: i32) -> i32;
    fn cruspy_field_get_i64(handle: *const KernelMemoryHandle, field: *const c_char, out: *mut i64) -> i32;
    fn cruspy_field_set_i64(handle: *const KernelMemoryHandle, field: *const c_char, value: i64) -> i32;
    fn cruspy_field_get_f64(handle: *const KernelMemoryHandle, field: *const c_char, out: *mut f64) -> i32;
    fn cruspy_field_set_f64(handle: *const KernelMemoryHandle, field: *const c_char, value: f64) -> i32;
    fn cruspy_field_get_bool(handle: *const KernelMemoryHandle, field: *const c_char, out: *mut i32) -> i32;
    fn cruspy_field_set_bool(handle: *const KernelMemoryHandle, field: *const c_char, value: i32) -> i32;
    fn cruspy_field_get_object(
        handle: *const KernelMemoryHandle,
        field: *const c_char,
        out: *mut KernelMemoryHandle,
    ) -> i32;
}

unsafe fn read_document_fields(handle: *const KernelMemoryHandle) -> Result<(i32, f64, bool, i32, i64), i32> {
    let mut id: i32 = 0;
    let mut score: f64 = 0.0;
    let mut active_raw: i32 = 0;
    let mut meta = KernelMemoryHandle {
        abi_version: 0,
        flags: 0,
        domain_id: crate::cruspy_root::runtime::kernel::DomainId { high: 0, low: 0 },
        offset: 0,
        byte_size: 0,
        schema_hash: 0,
        generation: 0,
        embedded_offset: 0,
        type_fqn: [0; 24],
    };
    let mut meta_id: i32 = 0;
    let mut meta_created_at: i64 = 0;

    let id_name = CString::new("id").map_err(|_| -1)?;
    let score_name = CString::new("score").map_err(|_| -1)?;
    let active_name = CString::new("active").map_err(|_| -1)?;
    let meta_name = CString::new("meta").map_err(|_| -1)?;

    if cruspy_field_get_i32(handle, id_name.as_ptr(), &mut id) != 0 {
        return Err(-1);
    }
    if cruspy_field_get_f64(handle, score_name.as_ptr(), &mut score) != 0 {
        return Err(-1);
    }
    if cruspy_field_get_bool(handle, active_name.as_ptr(), &mut active_raw) != 0 {
        return Err(-1);
    }
    if cruspy_field_get_object(handle, meta_name.as_ptr(), &mut meta) != 0 {
        return Err(-1);
    }
    if cruspy_field_get_i32(&meta, id_name.as_ptr(), &mut meta_id) != 0 {
        return Err(-1);
    }
    let created_at_name = CString::new("created_at").map_err(|_| -1)?;
    if cruspy_field_get_i64(&meta, created_at_name.as_ptr(), &mut meta_created_at) != 0 {
        return Err(-1);
    }

    Ok((id, score, active_raw != 0, meta_id, meta_created_at))
}

unsafe fn write_document_fields(
    handle: *const KernelMemoryHandle,
    id: i32,
    score: f64,
    active: bool,
    meta_id: i32,
    meta_created_at: i64,
) -> Result<(), i32> {
    let id_name = CString::new("id").map_err(|_| -1)?;
    let score_name = CString::new("score").map_err(|_| -1)?;
    let active_name = CString::new("active").map_err(|_| -1)?;
    let meta_name = CString::new("meta").map_err(|_| -1)?;
    let created_at_name = CString::new("created_at").map_err(|_| -1)?;

    if cruspy_field_set_i32(handle, id_name.as_ptr(), id) != 0 {
        return Err(-3);
    }
    if cruspy_field_set_f64(handle, score_name.as_ptr(), score) != 0 {
        return Err(-3);
    }
    if cruspy_field_set_bool(handle, active_name.as_ptr(), if active { 1 } else { 0 }) != 0 {
        return Err(-3);
    }

    let mut meta = KernelMemoryHandle {
        abi_version: 0,
        flags: 0,
        domain_id: crate::cruspy_root::runtime::kernel::DomainId { high: 0, low: 0 },
        offset: 0,
        byte_size: 0,
        schema_hash: 0,
        generation: 0,
        embedded_offset: 0,
        type_fqn: [0; 24],
    };
    if cruspy_field_get_object(handle, meta_name.as_ptr(), &mut meta) != 0 {
        return Err(-3);
    }
    if cruspy_field_set_i32(&meta, id_name.as_ptr(), meta_id) != 0 {
        return Err(-3);
    }
    if cruspy_field_set_i64(&meta, created_at_name.as_ptr(), meta_created_at) != 0 {
        return Err(-3);
    }
    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn document_serialize(
    handle: *const KernelMemoryHandle,
    out: *mut u8,
    capacity: usize,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    if capacity == 0 {
        return SERIALIZE_SIZE as i32;
    }
    if out.is_null() || capacity < SERIALIZE_SIZE {
        return -1;
    }

    let (id, score, active, meta_id, meta_created_at) = match read_document_fields(handle) {
        Ok(values) => values,
        Err(code) => return code,
    };

    std::ptr::copy_nonoverlapping(SERIALIZE_MAGIC.as_ptr(), out, SERIALIZE_MAGIC.len());
    std::ptr::copy_nonoverlapping(id.to_ne_bytes().as_ptr(), out.add(4), 4);
    std::ptr::copy_nonoverlapping(score.to_ne_bytes().as_ptr(), out.add(8), 8);
    out.add(16).write(if active { 1 } else { 0 });
    std::ptr::copy_nonoverlapping(meta_id.to_ne_bytes().as_ptr(), out.add(17), 4);
    std::ptr::copy_nonoverlapping(meta_created_at.to_ne_bytes().as_ptr(), out.add(21), 8);
    SERIALIZE_SIZE as i32
}

#[no_mangle]
pub unsafe extern "C" fn document_from_json(
    fqn: *const c_char,
    out: *mut KernelMemoryHandle,
    json: *const c_char,
    domain: *const c_char,
) -> i32 {
    if fqn.is_null() || out.is_null() || json.is_null() || domain.is_null() {
        return -1;
    }
    if cruspy_create(fqn, domain, out) != 0 {
        return -2;
    }

    let json_str = CStr::from_ptr(json).to_string_lossy();
    let parsed: DocumentJson = match serde_json::from_str(&json_str) {
        Ok(value) => value,
        Err(_) => return -4,
    };

    if write_document_fields(
        out,
        parsed.id,
        parsed.score,
        parsed.active,
        parsed.meta.id,
        parsed.meta.created_at,
    )
    .is_err()
    {
        return -3;
    }
    0
}

CRUSPY_REGISTER_METHOD!(Document, serialize, document_serialize);
CRUSPY_REGISTER_METHOD!(Document, from_json, document_from_json);
