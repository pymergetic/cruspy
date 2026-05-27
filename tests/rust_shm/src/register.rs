//! Registration trait (kept separate to avoid `registry` ↔ `mem` cycle).

use crate::registry::{Locator, RegistryError};
use crate::mem::io::Write;

/// Open storage described by [`InfoData`](crate::mem::device::info::InfoData).
pub trait RegisterSpec {
    fn locator(&self) -> &Locator;
    fn open(&self) -> Result<(Box<dyn Write>, Locator), RegistryError>;
}
