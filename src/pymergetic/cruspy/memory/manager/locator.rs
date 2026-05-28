use crate::pymergetic::cruspy::utils::url::Url;
use crate::pymergetic::cruspy::io::Kind;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::process;

use super::{Id, Manager, ManagerError, Registered};

/// External name for a slab.
///
/// Newtype around [`Url`] so manager-specific behavior can be added over time
/// without mutating URL semantics globally.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Locator(Url);

impl Locator {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn scheme(&self) -> &str {
        self.0.scheme()
    }

    pub fn as_url(&self) -> &Url {
        &self.0
    }

    pub fn into_url(self) -> Url {
        self.0
    }

    pub fn resolve_id(&self, manager: &Manager) -> Result<Id, ManagerError> {
        manager.id(self)
    }

    pub fn default_for_kind(kind: Kind) -> Self {
        let stem = Self::default_stem(kind);
        let url = match kind {
            Kind::Ram => Url::builder().scheme("ram").host(&stem).build(),
            Kind::Shm => Url::builder().scheme("shm").host(&stem).build(),
            Kind::File => Url::builder()
                .scheme("file")
                .path(format!("/tmp/{stem}"))
                .build(),
        };
        url.into()
    }

    pub fn default_for_scheme(scheme: &str) -> Option<Self> {
        Some(Self::default_for_kind(Kind::from_scheme(scheme)?))
    }

    fn default_stem(kind: Kind) -> String {
        let base = format!("cruspy-{}-default", process::id());
        match kind {
            Kind::Ram => base,
            Kind::Shm => format!("{base}.slab"),
            Kind::File => format!("{base}.slab"),
        }
    }
}

impl From<Url> for Locator {
    fn from(value: Url) -> Self {
        Self(value)
    }
}

impl From<Locator> for Url {
    fn from(value: Locator) -> Self {
        value.0
    }
}

impl Default for Locator {
    fn default() -> Self {
        Self::default_for_kind(Kind::Ram)
    }
}

impl fmt::Display for Locator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Hash for Locator {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

pub trait LocatorRef {
    fn locator_key(&self) -> &str;
}

impl LocatorRef for str {
    fn locator_key(&self) -> &str {
        self
    }
}

impl LocatorRef for Url {
    fn locator_key(&self) -> &str {
        self.as_str()
    }
}

impl LocatorRef for Locator {
    fn locator_key(&self) -> &str {
        self.as_str()
    }
}

impl LocatorRef for Registered {
    fn locator_key(&self) -> &str {
        self.locator.as_str()
    }
}

impl<T: LocatorRef + ?Sized> LocatorRef for &T {
    fn locator_key(&self) -> &str {
        T::locator_key(self)
    }
}
