//! [`Kind`] — storage family (`ram` / `shm` / `file`): URL scheme + slab type association.

use super::HasSlab;
use crate::pymergetic::cruspy::utils::url::Url;

/// Backing family for a slab or homogeneous segment.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Ram,
    Shm,
    File,
}

impl Kind {
    pub const ALL: [Self; 3] = [Self::Ram, Self::Shm, Self::File];

    pub fn scheme(self) -> &'static str {
        match self {
            Self::Ram => "ram",
            Self::Shm => "shm",
            Self::File => "file",
        }
    }

    pub fn from_scheme(scheme: &str) -> Option<Self> {
        match scheme {
            "ram" => Some(Self::Ram),
            "shm" => Some(Self::Shm),
            "file" => Some(Self::File),
            _ => None,
        }
    }

    pub fn matches_url(&self, url: &Url) -> bool {
        url.scheme() == self.scheme()
    }

    pub fn ensure_url(&self, url: &Url) -> Result<(), KindMismatch> {
        if self.matches_url(url) {
            Ok(())
        } else {
            Err(KindMismatch {
                url_scheme: url.scheme().into(),
                kind: *self,
            })
        }
    }
}

/// Links a concrete slab type to its [`Kind`] (scheme + erasure key).
pub trait HasKind: HasSlab {
    const KIND: Kind;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindMismatch {
    pub url_scheme: String,
    pub kind: Kind,
}

impl std::fmt::Display for KindMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "url scheme {} does not match storage kind {}",
            self.url_scheme,
            self.kind.scheme()
        )
    }
}

impl std::error::Error for KindMismatch {}
