//! Write access to an opened slab.

use super::read::Read;

/// Mutable storage: [`Read`] plus persist hook (file `fsync`, etc.).
pub trait Write: Read {
    fn flush(&self) -> std::io::Result<()> {
        Ok(())
    }
}
