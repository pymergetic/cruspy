#[cxx::bridge(namespace = "pymergetic::cruspy::models::document")]
pub mod ffi {
    struct Document {
        id: i32,
        text: String,
        score: f64,
        active: bool,
    }

    unsafe extern "C++" {
        include!("models/document/mod.hpp");

        fn validate_document(doc: &Document) -> Result<()>;
    }
}

use std::ffi::CString;
use std::sync::OnceLock;

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::errors::map_cxx_exception;
use crate::module::register_submodule;
use crate::schema::{document_descriptor, DOCUMENT_FQN};

static DOCUMENT_CLASS: OnceLock<Py<PyAny>> = OnceLock::new();

fn document_class(py: Python<'_>) -> PyResult<Py<PyAny>> {
    if let Some(cls) = DOCUMENT_CLASS.get() {
        return Ok(cls.clone_ref(py));
    }
    let locals = PyDict::new(py);
    let code = CString::new(
        r#"
def make_document_model():
    from pydantic import BaseModel, Field, create_model
    return create_model(
        "Document",
        __base__=BaseModel,
        id=(int, Field(ge=1)),
        text=(str, Field(max_length=512)),
        score=(float, Field(ge=0.0, le=1.0)),
        active=(bool, False),
    )
"#,
    )
    .expect("valid python source");
    py.run(&code, None, Some(&locals))?;
    let model = locals
        .get_item("make_document_model")?
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("factory missing"))?
        .call0()?;
    DOCUMENT_CLASS
        .set(model.unbind())
        .map_err(|_| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("model init race"))?;
    Ok(DOCUMENT_CLASS.get().unwrap().clone_ref(py))
}

pub fn validate_document_fields(doc: &ffi::Document) -> Result<(), String> {
    ffi::validate_document(doc).map_err(|err| err.to_string())
}

#[pyfunction]
fn validate_document(py: Python<'_>, id: i32, text: String, score: f64, active: bool) -> PyResult<()> {
    let doc = ffi::Document {
        id,
        text,
        score,
        active,
    };
    validate_document_fields(&doc).map_err(|err| map_cxx_exception(py, &err))
}

pub fn register_document_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    register_submodule(py, parent, "pymergetic.cruspy.models", "models", |models| {
        register_submodule(
            py,
            models,
            "pymergetic.cruspy.models.document",
            "document",
            |document| {
                let cls = document_class(py)?;
                let cls_bound = cls.bind(py);
                document.add("Document", cls_bound.as_any())?;
                document.add("SCHEMA_HASH", document_descriptor().schema_hash)?;
                document.add("TYPE_FQN", DOCUMENT_FQN)?;
                document.add_function(wrap_pyfunction!(validate_document, document)?)?;
                Ok(())
            },
        )
    })
}
