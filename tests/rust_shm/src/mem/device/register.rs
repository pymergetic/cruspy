//! Open [`InfoData`] by URL scheme (avoids per-backend wrapper types).

use crate::mem::backing;
use crate::mem::device::{file, ram, shm};
use crate::mem::io::open_backing;
use crate::mem::kind::Kind;
use crate::mem::device::info::InfoData;
use crate::register::RegisterSpec;
use crate::registry::{Locator, RegistryError};
use crate::mem::io::Write;

impl RegisterSpec for InfoData {
    fn locator(&self) -> &Locator {
        &self.url
    }

    fn open(&self) -> Result<(Box<dyn Write>, Locator), RegistryError> {
        match backing::kind(&self.url) {
            Some(Kind::Ram) => open_backing::<ram::Storage, _>(
                self.open_mode,
                &self.url,
                self.capacity,
                |e| match e {},
            ),
            Some(Kind::PosixShm) => open_backing::<shm::Storage, _>(
                self.open_mode,
                &self.url,
                self.capacity,
                RegistryError::Backend,
            ),
            Some(Kind::File) => open_backing::<file::Storage, _>(
                self.open_mode,
                &self.url,
                self.capacity,
                RegistryError::Io,
            ),
            None => Err(RegistryError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "unknown URL scheme for registration",
            ))),
        }
    }
}
