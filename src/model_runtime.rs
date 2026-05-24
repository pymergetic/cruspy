use std::collections::HashMap;
use std::ffi::CString;
use std::sync::{OnceLock, RwLock};

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};

use crate::core::{FieldDescriptor, TypeDescriptor, register_model_type};
use crate::errors::map_cxx_exception;
use crate::module::register_submodule;
use crate::schema::{encode_model_bytes, register_model_field_meta, FieldMetaEntry};
use crate::shm::{view_model_shm, ShmArena, ShmHandle, ShmView};

static PYDANTIC_BY_MODULE: OnceLock<RwLock<HashMap<&'static str, Py<PyAny>>>> = OnceLock::new();

fn pydantic_registry() -> &'static RwLock<HashMap<&'static str, Py<PyAny>>> {
    PYDANTIC_BY_MODULE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub struct ModelSpec {
    pub name: &'static str,
    pub fqn: &'static str,
    pub schema_hash: u64,
    pub slab_size: u32,
    pub description: &'static str,
    pub field_descriptors: &'static [FieldDescriptor],
    pub field_meta: &'static [FieldMetaEntry],
    pub pydantic_index: usize,
}

pub struct ModuleSpec {
    pub python_module: &'static str,
    pub short_name: &'static str,
    pub pydantic_source: &'static str,
    pub models: &'static [&'static ModelSpec],
    pub primary: usize,
}

pub struct ModelBinding {
    pub fqn: &'static str,
    pub schema_hash: u64,
    pub field_meta: &'static [FieldMetaEntry],
}

pub fn validate_via_cxx(py: Python<'_>, validate: impl FnOnce() -> Result<(), String>) -> PyResult<()> {
    validate().map_err(|err| map_cxx_exception(py, &err))
}

pub fn type_descriptor(spec: &ModelSpec) -> TypeDescriptor {
    TypeDescriptor {
        fqn: spec.fqn,
        schema_hash: spec.schema_hash,
        slab_size: spec.slab_size,
        fields: spec.field_descriptors,
    }
}

pub fn type_descriptor_py(spec: &ModelSpec) -> PyResult<Py<PyAny>> {
    Python::with_gil(|py| {
        let dict = PyDict::new(py);
        let desc = type_descriptor(spec);
        dict.set_item("fqn", desc.fqn)?;
        dict.set_item("schema_hash", desc.schema_hash)?;
        dict.set_item("slab_size", desc.slab_size)?;
        dict.set_item("description", spec.description)?;
        let fields = PyDict::new(py);
        for field in desc.fields {
            let field_dict = PyDict::new(py);
            field_dict.set_item("kind", format!("{:?}", field.kind))?;
            field_dict.set_item("offset", field.offset)?;
            field_dict.set_item("size", field.size)?;
            field_dict.set_item("description", field.description)?;
            field_dict.set_item("optional", field.optional)?;
            fields.set_item(field.name, field_dict)?;
        }
        dict.set_item("fields", fields)?;
        Ok(dict.into())
    })
}

pub fn ensure_pydantic_models(py: Python<'_>, module_key: &'static str, source: &str) -> PyResult<()> {
    if pydantic_registry()
        .read()
        .expect("pydantic registry lock")
        .contains_key(module_key)
    {
        return Ok(());
    }
    let locals = PyDict::new(py);
    let code = CString::new(source).map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("invalid pydantic source")
    })?;
    py.run(&code, None, Some(&locals))?;
    let factory = locals
        .get_item("make_models")?
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("make_models missing"))?;
    let models = factory.call0()?.into();
    pydantic_registry()
        .write()
        .expect("pydantic registry lock")
        .insert(module_key, models);
    Ok(())
}

pub fn pydantic_class(py: Python<'_>, module_key: &'static str, index: usize) -> PyResult<Py<PyAny>> {
    let registry = pydantic_registry()
        .read()
        .expect("pydantic registry lock");
    let models = registry
        .get(module_key)
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("module models missing"))?
        .clone_ref(py);
    drop(registry);
    let tuple = models.bind(py).downcast::<PyTuple>()?;
    Ok(tuple.get_item(index)?.unbind())
}

pub fn write_model_to_shm(
    arena: &Bound<'_, ShmArena>,
    model: &Bound<'_, PyAny>,
    binding: ModelBinding,
) -> PyResult<ShmHandle> {
    let payload = encode_model_bytes(model, binding.field_meta)?;
    arena
        .borrow()
        .write_bytes_impl(binding.fqn.to_string(), binding.schema_hash, &payload)
}

pub fn view_model_from_shm(
    py: Python<'_>,
    arena: &Bound<'_, ShmArena>,
    handle: &Bound<'_, ShmHandle>,
    python_module: &str,
    spec: &ModelSpec,
) -> PyResult<ShmView> {
    view_model_shm(
        py,
        arena,
        &handle.borrow(),
        spec.schema_hash,
        python_module,
        spec.name,
        spec.field_meta,
    )
}

pub fn bind_shm_methods(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    cls: &Bound<'_, PyAny>,
    write_name: &str,
    view_name: &str,
) -> PyResult<()> {
    static BIND_SHM: OnceLock<Py<PyAny>> = OnceLock::new();
    if BIND_SHM.get().is_none() {
        let locals = PyDict::new(py);
        let code = CString::new(
            r#"
def bind_shm_methods(cls, write_fn, view_fn):
    def write_to_shm(self, arena):
        return write_fn(arena, self)

    @classmethod
    def view_shm(cls, arena, handle):
        return view_fn(arena, handle)

    cls.write_to_shm = write_to_shm
    cls.view_shm = view_shm
"#,
        )
        .expect("valid bind_shm source");
        py.run(&code, None, Some(&locals))?;
        let binder = locals
            .get_item("bind_shm_methods")?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("bind_shm missing"))?
            .unbind();
        let _ = BIND_SHM.set(binder);
    }
    let write = module.getattr(write_name)?;
    let view = module.getattr(view_name)?;
    BIND_SHM
        .get()
        .expect("bind_shm initialized")
        .call1(py, (cls, write, view))?;
    Ok(())
}

pub fn register_module(
    parent: &Bound<'_, PyModule>,
    spec: &ModuleSpec,
    register_extra: impl FnOnce(&Bound<'_, PyModule>, Python<'_>) -> PyResult<()>,
) -> PyResult<()> {
    let py = parent.py();
    ensure_pydantic_models(py, spec.python_module, spec.pydantic_source)?;
    register_submodule(
        py,
        parent,
        spec.python_module,
        spec.short_name,
        |module| {
            for model in spec.models {
                register_model_field_meta(model.fqn, model.field_meta);
                register_model_type(type_descriptor(model));
                let cls = pydantic_class(py, spec.python_module, model.pydantic_index)?;
                module.add(model.name, cls.bind(py).as_any())?;
                bind_shm_methods(
                    py,
                    module,
                    cls.bind(py).as_any(),
                    &format!("write_{}_to_shm", snake_case(model.name)),
                    &format!("view_{}_to_shm", snake_case(model.name)),
                )?;
            }
            let primary = spec.models[spec.primary];
            module.add("SCHEMA_HASH", primary.schema_hash)?;
            module.add("TYPE_FQN", primary.fqn)?;
            register_extra(module, py)?;
            Ok(())
        },
    )
}

fn snake_case(name: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && idx > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}
