//! Slab lifecycle: [`Open::open`] → use → [`Access::close`].

use std::any::Any;
use std::io;

use crate::registry::Locator;
use crate::registry::RegistryError;
use crate::utils::url::Url;

use super::address::Address;
use super::Write;

/// How storage was opened (same verbs on ram / shm / file).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OpenMode {
    /// Not set on a registration spec until `.create()` or `.attach()`.
    None,
    Create,
    Attach,
}

/// Opened slab: [`Address`] + mode + close (`dyn`‑safe).
pub trait Access: Address + Any {
    fn open_mode(&self) -> OpenMode;

    fn close(&mut self) -> io::Result<()>;
}

/// Open a concrete [`Storage`](crate::mem::device::ram::Storage) (not on `dyn Access`).
pub trait Open: Access + Write + Sized {
    type Error;

    fn open(mode: OpenMode, url: &Url, len: usize) -> Result<Self, Self::Error>;
}

/// Shared [`RegisterSpec::open`](crate::register::RegisterSpec::open) body.
pub fn open_backing<S, M>(
    mode: OpenMode,
    url: &Url,
    capacity: usize,
    map_err: M,
) -> Result<(Box<dyn Write>, Locator), RegistryError>
where
    S: Open,
    M: FnOnce(S::Error) -> RegistryError,
{
    if mode == OpenMode::None {
        return Err(RegistryError::OpenModeUnset);
    }
    let storage = S::open(mode, url, capacity).map_err(map_err)?;
    Ok((Box::new(storage), url.clone()))
}
