//! Central catalog of registered memory slabs across multiple [`Segment`]s.
//!
//! [`Manager`] is generic over [`ManagerData`] (default: [`DefaultData`]). URL scheme
//! selects the backend family (`ram` / `shm` / `file`); each scheme gets its own
//! homogeneous segment, and you may create additional segments explicitly.

mod data;
mod error;
mod usage;

pub use data::{Catalog, DefaultData, ManagerData, MemEntry};
pub use crate::pymergetic::cruspy::memory::segment::SegmentId;
pub use usage::{Usage, UsageReport, UsageTotals};

use std::fmt;

use crate::pymergetic::cruspy::io::OpenMode;
use crate::pymergetic::cruspy::memory::segment::Segment;
use crate::pymergetic::cruspy::utils::url::Url;

/// Opaque handle for a registered slab.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Id(pub u64);

/// External name for a slab (same as backing [`Url`]).
pub type Locator = Url;

/// Result of [`Manager::register`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Registered {
    pub id: Id,
    pub locator: Locator,
    pub segment_id: SegmentId,
    /// Slab index at registration time; use [`Manager::slab_index`] after closes.
    pub slab_index: usize,
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

#[derive(Debug)]
pub enum ManagerError {
    DuplicateLocator(String),
    UnknownLocator(String),
    UnknownId(Id),
    UnknownSegment(SegmentId),
    SlabNotInSegment,
    UnsupportedScheme(String),
    SchemeMismatch {
        url_scheme: String,
        kind: crate::pymergetic::cruspy::io::Kind,
    },
    Backend {
        scheme: String,
        message: String,
    },
    Layout {
        scheme: String,
        detail: String,
    },
}

impl fmt::Display for ManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateLocator(l) => write!(f, "locator already registered: {l}"),
            Self::UnknownLocator(l) => write!(f, "unknown locator: {l}"),
            Self::UnknownId(id) => write!(f, "unknown mem id: {}", id.0),
            Self::UnknownSegment(id) => write!(f, "unknown segment id: {}", id.0),
            Self::SlabNotInSegment => write!(f, "slab not found in segment"),
            Self::UnsupportedScheme(s) => write!(f, "unsupported url scheme: {s}"),
            Self::SchemeMismatch { url_scheme, kind } => write!(
                f,
                "url scheme {url_scheme} does not match storage kind {}",
                kind.scheme()
            ),
            Self::Backend { scheme, message } => {
                write!(f, "{scheme} backend error: {message}")
            }
            Self::Layout { scheme, detail } => write!(f, "{scheme} layout error: {detail}"),
        }
    }
}

impl std::error::Error for ManagerError {}

impl From<crate::pymergetic::cruspy::io::KindMismatch> for ManagerError {
    fn from(m: crate::pymergetic::cruspy::io::KindMismatch) -> Self {
        Self::SchemeMismatch {
            url_scheme: m.url_scheme,
            kind: m.kind,
        }
    }
}

/// Process-wide memory manager; storage and segment ops live in `D: [`ManagerData`].
pub struct Manager<D: ManagerData = DefaultData> {
    data: D,
}

impl Manager<DefaultData> {
    pub fn new() -> Self {
        Self::with_data(DefaultData::new())
    }
}

impl<D: ManagerData> Manager<D> {
    pub fn with_data(data: D) -> Self {
        Self { data }
    }

    pub fn data(&self) -> &D {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut D {
        &mut self.data
    }

    pub fn catalog(&self) -> &Catalog {
        self.data.catalog()
    }

    pub fn catalog_mut(&mut self) -> &mut Catalog {
        self.data.catalog_mut()
    }

    pub fn create_segment(&mut self, kind: crate::pymergetic::cruspy::io::Kind) -> SegmentId {
        self.data.create_segment(kind)
    }

    pub fn segment(&self, id: SegmentId) -> Option<&Segment> {
        self.data.segment(id)
    }

    pub fn segment_mut(&mut self, id: SegmentId) -> Option<&mut Segment> {
        self.data.segment_mut(id)
    }

    pub fn segment_ids(&self) -> impl Iterator<Item = SegmentId> + '_ {
        self.data.segment_ids()
    }

    /// Register on the default segment for `url.scheme()` (created on first use).
    pub fn register(
        &mut self,
        url: &Url,
        mode: OpenMode,
        capacity: Option<usize>,
    ) -> Result<Registered, ManagerError> {
        self.data.register(url, mode, capacity)
    }

    /// Register on a specific segment (scheme must match segment kind).
    pub fn register_on(
        &mut self,
        segment_id: SegmentId,
        url: &Url,
        mode: OpenMode,
        capacity: Option<usize>,
    ) -> Result<Registered, ManagerError> {
        self.data.register_on(segment_id, url, mode, capacity)
    }

    pub fn create(
        &mut self,
        url: &Url,
        capacity: Option<usize>,
    ) -> Result<Registered, ManagerError> {
        self.register(url, OpenMode::Create, capacity)
    }

    pub fn attach(
        &mut self,
        url: &Url,
        capacity: Option<usize>,
    ) -> Result<Registered, ManagerError> {
        self.register(url, OpenMode::Attach, capacity)
    }

    pub fn id<S: LocatorRef + ?Sized>(&self, locator: &S) -> Result<Id, ManagerError> {
        self.catalog()
            .by_locator
            .get(locator.locator_key())
            .copied()
            .ok_or_else(|| ManagerError::UnknownLocator(locator.locator_key().to_owned()))
    }

    pub fn locator(&self, id: Id) -> Result<&Locator, ManagerError> {
        Ok(&self.data.mem_entry(id)?.locator)
    }

    pub fn mem_entry(&self, id: Id) -> Result<&MemEntry, ManagerError> {
        self.data.mem_entry(id)
    }

    pub fn segment_id_for(&self, id: Id) -> Result<SegmentId, ManagerError> {
        Ok(self.data.mem_entry(id)?.segment_id)
    }

    pub fn entries(&self) -> impl Iterator<Item = (Id, &Locator)> + '_ {
        self.catalog()
            .by_mem
            .iter()
            .map(|(id, e)| (*id, &e.locator))
    }

    pub fn slab_index(&self, id: Id) -> Result<usize, ManagerError> {
        self.data.slab_index(id)
    }

    pub fn set_used_len(&mut self, id: Id, used_len: usize) -> Result<(), ManagerError> {
        self.data.mem_entry_mut(id)?.used_len = used_len;
        Ok(())
    }

    pub fn used_len(&self, id: Id) -> Result<usize, ManagerError> {
        Ok(self.data.mem_entry(id)?.used_len)
    }

    pub fn usage_report(&self) -> UsageReport {
        self.data.usage_report()
    }

    pub fn close(&mut self, id: Id) -> Result<(), ManagerError> {
        self.data.close_mem(id)
    }

    pub fn close_all(&mut self) -> Result<(), ManagerError> {
        self.data.close_all_mem()
    }

    pub fn unlink(&mut self, id: Id) -> Result<(), ManagerError> {
        self.data.unlink_mem(id)
    }

    pub fn unlink_all(&mut self) -> Result<(), ManagerError> {
        self.data.unlink_all_mem()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pymergetic::cruspy::io::Kind;
    use crate::pymergetic::cruspy::memory::backend::Ram;
    use crate::pymergetic::cruspy::memory::segment::HEADER_LEN;

    #[test]
    fn register_two_slabs_same_default_segment() {
        let mut mgr = Manager::new();
        let a = mgr
            .create(&Ram::build_url("a"), Some(4096))
            .expect("create a");
        let b = mgr
            .create(&Ram::build_url("b"), Some(8192))
            .expect("create b");
        assert_eq!(a.segment_id, b.segment_id);
        assert_ne!(a.id, b.id);
        mgr.set_used_len(a.id, 1024).unwrap();
        let report = mgr.usage_report();
        assert_eq!(report.totals.slab_count, 2);
        assert_eq!(report.totals.total_used, 1024);
        let arena_a = 4096 - HEADER_LEN;
        let arena_b = 8192 - HEADER_LEN;
        assert_eq!(report.totals.total_capacity, arena_a + arena_b);
        assert_eq!(mgr.segment_ids().count(), 1);
    }

    #[test]
    fn multiple_segments_explicit() {
        let mut mgr = Manager::new();
        let s0 = mgr.create_segment(Kind::Ram);
        let s1 = mgr.create_segment(Kind::Ram);
        assert_ne!(s0, s1);
        let a = mgr
            .register_on(s0, &Ram::build_url("a"), OpenMode::Create, Some(4096))
            .unwrap();
        let b = mgr
            .register_on(s1, &Ram::build_url("b"), OpenMode::Create, Some(8192))
            .unwrap();
        assert_ne!(a.segment_id, b.segment_id);
        assert_eq!(mgr.segment(s0).unwrap().backends().len(), 1);
        assert_eq!(mgr.segment(s1).unwrap().backends().len(), 1);
    }

    #[test]
    fn scheme_mismatch_rejected() {
        let mut mgr = Manager::new();
        let shm_seg = mgr.create_segment(Kind::Shm);
        let err = mgr
            .register_on(
                shm_seg,
                &Ram::build_url("wrong"),
                OpenMode::Create,
                Some(4096),
            )
            .unwrap_err();
        assert!(matches!(err, ManagerError::SchemeMismatch { .. }));
    }

    #[test]
    fn duplicate_locator_rejected() {
        let mut mgr = Manager::new();
        let url = Ram::build_url("dup");
        mgr.create(&url, Some(4096)).unwrap();
        assert!(matches!(
            mgr.create(&url, Some(4096)),
            Err(ManagerError::DuplicateLocator(_))
        ));
    }

    #[test]
    fn close_removes_registration() {
        let mut mgr = Manager::new();
        let reg = mgr.create(&Ram::build_url("x"), Some(4096)).unwrap();
        let seg = reg.segment_id;
        mgr.close(reg.id).unwrap();
        assert!(matches!(
            mgr.id(&reg.locator),
            Err(ManagerError::UnknownLocator(_))
        ));
        assert_eq!(mgr.segment(seg).unwrap().backends().len(), 0);
    }
}
