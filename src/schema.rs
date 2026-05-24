use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub const DOCUMENT_FQN: &str = "pymergetic::cruspy::models::document::Document";

#[derive(Debug, Clone)]
pub struct TypeDescriptor {
    pub fqn: &'static str,
    pub schema_hash: u64,
    pub field_count: u32,
}

pub fn schema_hash(fqn: &str, fields: &[(&str, &str)]) -> u64 {
    let mut hasher = DefaultHasher::new();
    fqn.hash(&mut hasher);
    for (name, ty) in fields {
        name.hash(&mut hasher);
        ty.hash(&mut hasher);
    }
    hasher.finish()
}

pub fn document_descriptor() -> TypeDescriptor {
    let fields = [
        ("id", "int32"),
        ("text", "string"),
        ("score", "float64"),
        ("active", "bool"),
    ];
    TypeDescriptor {
        fqn: DOCUMENT_FQN,
        schema_hash: schema_hash(DOCUMENT_FQN, &fields),
        field_count: fields.len() as u32,
    }
}
