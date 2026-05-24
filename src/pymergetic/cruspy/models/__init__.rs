#[path = "document/__init__.rs"]
pub mod document;

pub fn register() {
    document::register();
}
