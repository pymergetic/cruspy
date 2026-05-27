//! [`Kind`] → concrete backend value ([`Ram`], [`Shm`], [`File`]).

use std::fmt;

use crate::pymergetic::cruspy::io::{HasSlab, Kind};
use crate::pymergetic::cruspy::utils::url::Url;

use super::{File, Ram, Shm};

/// URL scheme is not `ram`, `shm`, or `file`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownScheme(pub String);

impl fmt::Display for UnknownScheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported url scheme: {}", self.0)
    }
}

impl std::error::Error for UnknownScheme {}

impl Kind {
    /// Unopened backend for this kind; call [`HasSlab::open`] to bind a [`Url`].
    pub fn create(self) -> Box<dyn HasSlab> {
        match self {
            Kind::Ram => Box::new(Ram::new()),
            Kind::Shm => Box::new(Shm::new()),
            Kind::File => Box::new(File::new()),
        }
    }

    /// Unopened slab for `scheme` (`ram` / `shm` / `file`).
    pub fn create_from_scheme(scheme: &str) -> Result<Box<dyn HasSlab>, UnknownScheme> {
        Self::from_scheme(scheme)
            .ok_or_else(|| UnknownScheme(scheme.into()))
            .map(|kind| kind.create())
    }

    /// Unopened slab for the URL's scheme (same as [`create_from_scheme`](Self::create_from_scheme) on `url.scheme()`).
    pub fn create_from_url(url: &Url) -> Result<Box<dyn HasSlab>, UnknownScheme> {
        Self::create_from_scheme(url.scheme())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pymergetic::cruspy::io::OpenMode;
    use crate::pymergetic::cruspy::memory::backend::ram::Ram;

    #[test]
    fn create_from_scheme_ram() {
        let slab = Kind::create_from_scheme("ram").unwrap();
        assert_eq!(slab.kind(), Kind::Ram);
    }

    #[test]
    fn create_from_scheme_rejects_unknown() {
        assert!(Kind::create_from_scheme("ftp").is_err());
    }

    #[test]
    fn create_from_url_opens() {
        let url = Ram::build_url("heap");
        let mut slab = Kind::create_from_url(&url).unwrap();
        assert_eq!(slab.kind(), Kind::Ram);
        slab.open(&url, OpenMode::Create, Some(4096)).unwrap();
        assert_eq!(slab.info().capacity, 4096);
    }
}
