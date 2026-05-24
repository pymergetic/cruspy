use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use pyo3::IntoPyObjectExt;

use crate::core::{field_kind_from_schema, FieldKind};
use crate::errors::ShmError;

pub type FieldMetaEntry = (
    &'static str,
    &'static str,
    &'static str,
    bool,
    &'static [(&'static str, &'static str)],
);

static FIELD_META_REGISTRY: OnceLock<RwLock<HashMap<&'static str, &'static [FieldMetaEntry]>>> =
    OnceLock::new();

fn registry() -> &'static RwLock<HashMap<&'static str, &'static [FieldMetaEntry]>> {
    FIELD_META_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn register_model_field_meta(fqn: &'static str, meta: &'static [FieldMetaEntry]) {
    registry()
        .write()
        .expect("field meta registry lock")
        .insert(fqn, meta);
}

fn field_meta_for(schema_type: &str) -> PyResult<&'static [FieldMetaEntry]> {
    let fqn = schema_type
        .strip_prefix("model:")
        .ok_or_else(|| ShmError::new_err(format!("cruspy.shm: invalid model schema type {schema_type}")))?;
    registry()
        .read()
        .expect("field meta registry lock")
        .get(fqn)
        .copied()
        .ok_or_else(|| ShmError::new_err(format!("cruspy.shm: unregistered model field meta {fqn}")))
}

fn append_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> PyResult<u32> {
    if *offset + 4 > bytes.len() {
        return Err(ShmError::new_err("cruspy.shm: schema decode underflow"));
    }
    let value = u32::from_le_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
    *offset += 4;
    Ok(value)
}

pub fn encode_model_bytes(model: &Bound<'_, PyAny>, fields: &[FieldMetaEntry]) -> PyResult<Vec<u8>> {
    let mut out = Vec::new();
    for (name, schema_type, _, _, _) in fields {
        let value = model.getattr(*name)?;
        encode_value(&mut out, &value, schema_type)?;
    }
    Ok(out)
}

fn encode_value(out: &mut Vec<u8>, value: &Bound<'_, PyAny>, schema_type: &str) -> PyResult<()> {
    if schema_type == "optional_int32" {
        if value.is_none() {
            out.push(0);
            append_u32(out, 0);
        } else {
            out.push(1);
            let number: i32 = value.extract()?;
            append_u32(out, number as u32);
        }
        return Ok(());
    }
    if schema_type.starts_with("model:") {
        let nested_meta = field_meta_for(schema_type)?;
        let nested_bytes = encode_model_bytes(value, nested_meta)?;
        append_u32(out, nested_bytes.len() as u32);
        out.extend_from_slice(&nested_bytes);
        return Ok(());
    }

    match field_kind_from_schema(schema_type) {
        FieldKind::Int32 => {
            let number: i32 = value.extract()?;
            append_u32(out, number as u32);
        }
        FieldKind::Int64 => {
            return Err(ShmError::new_err(format!(
                "cruspy.shm: unsupported schema type {schema_type}"
            )));
        }
        FieldKind::Float64 => {
            let number: f64 = value.extract()?;
            out.extend_from_slice(&number.to_le_bytes());
        }
        FieldKind::Bool => {
            let flag: bool = value.extract()?;
            out.push(u8::from(flag));
        }
        FieldKind::String => {
            let text: String = value.extract()?;
            append_u32(out, text.len() as u32);
            out.extend_from_slice(text.as_bytes());
        }
        FieldKind::Model => {
            return Err(ShmError::new_err(format!(
                "cruspy.shm: unsupported schema type {schema_type}"
            )));
        }
    }
    Ok(())
}

pub fn decode_field_value(
    py: Python<'_>,
    bytes: &[u8],
    field_name: &str,
    fields: &[FieldMetaEntry],
) -> PyResult<Py<PyAny>> {
    let mut offset = 0usize;
    for (name, schema_type, _, _, _) in fields {
        let value = read_value(py, bytes, schema_type, &mut offset)?;
        if *name == field_name {
            return Ok(value);
        }
    }
    Err(ShmError::new_err(format!(
        "cruspy.shm: unknown field {field_name}"
    )))
}

pub fn decode_all_fields<'py>(
    py: Python<'py>,
    bytes: &[u8],
    fields: &[FieldMetaEntry],
) -> PyResult<Bound<'py, PyDict>> {
    let mut offset = 0usize;
    let dict = PyDict::new(py);
    for (name, schema_type, _, _, _) in fields {
        let value = read_value(py, bytes, schema_type, &mut offset)?;
        dict.set_item(*name, value)?;
    }
    Ok(dict)
}

fn read_value(
    py: Python<'_>,
    bytes: &[u8],
    schema_type: &str,
    offset: &mut usize,
) -> PyResult<Py<PyAny>> {
    if schema_type == "optional_int32" {
        if *offset >= bytes.len() {
            return Err(ShmError::new_err("cruspy.shm: schema decode underflow"));
        }
        let present = bytes[*offset] != 0;
        *offset += 1;
        let number = read_u32(bytes, offset)? as i32;
        return if present {
            Ok(number.into_pyobject(py)?.into_any().unbind())
        } else {
            Ok(py.None().into())
        };
    }
    if schema_type.starts_with("model:") {
        let len = read_u32(bytes, offset)? as usize;
        if *offset + len > bytes.len() {
            return Err(ShmError::new_err("cruspy.shm: schema decode underflow"));
        }
        let nested_bytes = &bytes[*offset..*offset + len];
        *offset += len;
        let nested_meta = field_meta_for(schema_type)?;
        let data = decode_all_fields(py, nested_bytes, nested_meta)?;
        let (module_path, class_name) = fqn_to_import(schema_type.strip_prefix("model:").unwrap())?;
        let model_mod = PyModule::import(py, &module_path)?;
        let cls = model_mod.getattr(&class_name)?;
        return Ok(cls
            .call_method1("model_validate", (data,))?
            .into_pyobject(py)?
            .into_any()
            .unbind());
    }

    match field_kind_from_schema(schema_type) {
        FieldKind::Int32 => Ok((read_u32(bytes, offset)? as i32).into_pyobject(py)?.into_any().unbind()),
        FieldKind::Int64 => Err(ShmError::new_err(format!(
            "cruspy.shm: unsupported schema type {schema_type}"
        ))),
        FieldKind::Float64 => {
            if *offset + 8 > bytes.len() {
                return Err(ShmError::new_err("cruspy.shm: schema decode underflow"));
            }
            let value = f64::from_le_bytes(bytes[*offset..*offset + 8].try_into().unwrap());
            *offset += 8;
            Ok(value.into_pyobject(py)?.into_any().unbind())
        }
        FieldKind::Bool => {
            if *offset >= bytes.len() {
                return Err(ShmError::new_err("cruspy.shm: schema decode underflow"));
            }
            let value = bytes[*offset] != 0;
            *offset += 1;
            Ok(value.into_bound_py_any(py)?.unbind())
        }
        FieldKind::String => {
            let len = read_u32(bytes, offset)? as usize;
            if *offset + len > bytes.len() {
                return Err(ShmError::new_err("cruspy.shm: schema decode underflow"));
            }
            let text = std::str::from_utf8(&bytes[*offset..*offset + len])
                .map_err(|err| ShmError::new_err(err.to_string()))?;
            *offset += len;
            Ok(text.into_pyobject(py)?.into_any().unbind())
        }
        FieldKind::Model => Err(ShmError::new_err(format!(
            "cruspy.shm: unsupported schema type {schema_type}"
        ))),
    }
}

fn fqn_to_import(fqn: &str) -> PyResult<(String, String)> {
    let (namespace, class_name) = fqn
        .rsplit_once("::")
        .ok_or_else(|| ShmError::new_err(format!("cruspy.shm: invalid model fqn {fqn}")))?;
    Ok((namespace.replace("::", "."), class_name.to_string()))
}
