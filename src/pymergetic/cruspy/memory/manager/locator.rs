use crate::pymergetic::cruspy::io::Kind;
use crate::pymergetic::cruspy::utils::url::Url;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process;

use super::{Id, Manager, ManagerError, Registered};

/// External name for a segment (base) or one of its slab backings.
///
/// - **Primary** slab: base URL/path with no `-N` suffix.
/// - **Heap extensions**: `-0`, `-1`, … on the host (ram/shm) or file stem (file).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Locator(Url);

impl Locator {
    /// Canonical segment key (primary locator) from any slab URL.
    pub fn segment_base_key(url: &str) -> String {
        let Ok(u) = Url::parse(url) else {
            return url.to_owned();
        };
        if u.is_file() {
            Self::strip_extension_suffix_path(u.path()).0
        } else {
            Self::strip_extension_suffix(u.host()).0
        }
    }

    pub fn base_key(&self) -> String {
        Self::segment_base_key(self.as_str())
    }

    /// Heap extension `n` (`0` → `…-0`, `1` → `…-1`). Primary is [`Self::is_segment_base`].
    pub fn extension(&self, n: u16) -> Self {
        let url = &self.0;
        if url.is_file() {
            let path = Self::extension_path(url.path(), n);
            return Self(Url::builder().scheme("file").path(path).build());
        }
        let (base_host, _) = Self::strip_extension_suffix(url.host());
        let host = format!("{base_host}-{n}");
        Self(
            Url::builder()
                .scheme(url.scheme())
                .host(&host)
                .path(url.path())
                .build(),
        )
    }

    /// Extension index from `-N` suffix, if any (`foo-2` → `Some(2)`).
    pub fn extension_index(&self) -> Option<u16> {
        if self.0.is_file() {
            let path = self.0.path();
            let file = Path::new(path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(path);
            let stem = Path::new(file)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(file);
            return Self::strip_extension_suffix(stem).1;
        }
        Self::strip_extension_suffix(self.0.host()).1
    }

    pub fn is_segment_base(&self) -> bool {
        self.extension_index().is_none()
    }

    fn strip_extension_suffix(part: &str) -> (String, Option<u16>) {
        if let Some(pos) = part.rfind('-') {
            let suffix = &part[pos + 1..];
            if !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()) {
                if let Ok(n) = suffix.parse::<u16>() {
                    return (part[..pos].to_owned(), Some(n));
                }
            }
        }
        (part.to_owned(), None)
    }

    fn strip_extension_suffix_path(path: &str) -> (String, Option<u16>) {
        let p = Path::new(path);
        let parent = p.parent();
        let file = p
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path);
        let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy()));
        let stem = Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(file);
        let (base_stem, idx) = Self::strip_extension_suffix(stem);
        let base_file = match ext.as_deref() {
            Some(e) => format!("{base_stem}{e}"),
            None => base_stem,
        };
        let base_path = match parent {
            Some(parent) if !parent.as_os_str().is_empty() => {
                PathBuf::from(parent).join(base_file)
            }
            _ => PathBuf::from(base_file),
        };
        (base_path.to_string_lossy().into_owned(), idx)
    }

    fn extension_path(path: &str, n: u16) -> String {
        let (base, _) = Self::strip_extension_suffix_path(path);
        let p = Path::new(&base);
        let parent = p.parent();
        let file = p
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(base.as_str());
        let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy()));
        let stem = Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(file);
        let new_file = match ext.as_deref() {
            Some(e) => format!("{stem}-{n}{e}"),
            None => format!("{stem}-{n}"),
        };
        match parent {
            Some(parent) if !parent.as_os_str().is_empty() => {
                PathBuf::from(parent).join(new_file).to_string_lossy().into_owned()
            }
            _ => new_file,
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pymergetic::cruspy::memory::backend::{ram::Ram, file::File, shm::Shm};

    #[test]
    fn ram_primary_and_extensions() {
        let base: Locator = Ram::build_url("my-app").into();
        assert!(base.is_segment_base());
        assert_eq!(base.extension(0).as_str(), "ram://my-app-0");
        assert_eq!(base.extension(1).as_str(), "ram://my-app-1");
        assert_eq!(
            Locator::segment_base_key("ram://my-app-0"),
            "my-app"
        );
    }

    #[test]
    fn shm_primary_and_extensions() {
        let base: Locator = Shm::build_url("core.slab").into();
        assert_eq!(base.extension(0).as_str(), "shm://core.slab-0");
        assert_eq!(
            Locator::segment_base_key("shm://core.slab-1"),
            "core.slab"
        );
    }

    #[test]
    fn file_primary_and_extensions() {
        let base: Locator = File::build_url("/tmp/my-seg.slab").into();
        assert!(base.is_segment_base());
        assert_eq!(
            base.extension(0).as_str(),
            "file:///tmp/my-seg-0.slab"
        );
        assert_eq!(
            base.extension(1).as_str(),
            "file:///tmp/my-seg-1.slab"
        );
        assert_eq!(
            Locator::segment_base_key("file:///tmp/my-seg-0.slab"),
            "/tmp/my-seg.slab"
        );
    }
}
