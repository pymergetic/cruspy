//! File-backed storage: [`File`] extends [`Write`] with a typed path.

use std::path::Path;

use super::write::Write;

/// File-backed storage — full [`Write`] plus the backing path on disk.
pub trait File: Write {
    fn path(&self) -> &Path;
}
