//! Central catalog of registered memory slabs across multiple [`Segment`]s.

mod data;
mod error;
mod usage;

pub use data::MemEntry;
pub use crate::pymergetic::cruspy::memory::segment::SegmentId;
pub use usage::{Usage, UsageReport, UsageTotals};

use std::collections::HashMap;
use std::fmt;

use crate::pymergetic::cruspy::io::{Kind, OpenMode};
use crate::pymergetic::cruspy::memory::segment::Segment;
use crate::pymergetic::cruspy::utils::url::Url;

use error::{map_open_err, map_slab_err, map_teardown_err};

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
        kind: Kind,
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

/// Process-wide memory registry: locators, segments, and usage.
pub struct Manager {
    next_mem_id: u64,
    next_segment_id: u64,
    by_locator: HashMap<String, Id>,
    by_mem: HashMap<Id, MemEntry>,
    segments: HashMap<SegmentId, Segment>,
    /// First segment created per scheme (auto-routing for [`Self::register`]).
    default_segment: HashMap<String, SegmentId>,
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}

impl Manager {
    pub fn new() -> Self {
        Self {
            next_mem_id: 0,
            next_segment_id: 0,
            by_locator: HashMap::new(),
            by_mem: HashMap::new(),
            segments: HashMap::new(),
            default_segment: HashMap::new(),
        }
    }

    fn alloc_mem_id(&mut self) -> Id {
        let id = Id(self.next_mem_id);
        self.next_mem_id += 1;
        id
    }

    fn alloc_segment_id(&mut self) -> SegmentId {
        let id = SegmentId(self.next_segment_id);
        self.next_segment_id += 1;
        id
    }

    pub fn create_segment(&mut self, kind: Kind) -> SegmentId {
        let id = self.alloc_segment_id();
        self.default_segment
            .entry(kind.scheme().to_owned())
            .or_insert(id);
        self.segments.insert(id, Segment::new(kind));
        id
    }

    pub fn segment(&self, id: SegmentId) -> Option<&Segment> {
        self.segments.get(&id)
    }

    pub fn segment_mut(&mut self, id: SegmentId) -> Option<&mut Segment> {
        self.segments.get_mut(&id)
    }

    pub fn segment_ids(&self) -> impl Iterator<Item = SegmentId> + '_ {
        self.segments.keys().copied()
    }

    fn default_segment_for_scheme(&self, scheme: &str) -> Option<SegmentId> {
        self.default_segment.get(scheme).copied()
    }

    fn ensure_default_segment(&mut self, scheme: &str) -> Result<SegmentId, ManagerError> {
        if let Some(id) = self.default_segment_for_scheme(scheme) {
            return Ok(id);
        }
        let kind = Kind::from_scheme(scheme)
            .ok_or_else(|| ManagerError::UnsupportedScheme(scheme.to_owned()))?;
        Ok(self.create_segment(kind))
    }

    /// Register on the default segment for `url.scheme()` (created on first use).
    pub fn register(
        &mut self,
        url: &Url,
        mode: OpenMode,
        capacity: Option<usize>,
    ) -> Result<Registered, ManagerError> {
        let segment_id = self.ensure_default_segment(url.scheme())?;
        self.register_on(segment_id, url, mode, capacity)
    }

    /// Register on a specific segment (scheme must match segment kind).
    pub fn register_on(
        &mut self,
        segment_id: SegmentId,
        url: &Url,
        mode: OpenMode,
        capacity: Option<usize>,
    ) -> Result<Registered, ManagerError> {
        let key = url.as_str();
        if self.by_locator.contains_key(key) {
            return Err(ManagerError::DuplicateLocator(key.to_owned()));
        }

        let segment = self
            .segment_mut(segment_id)
            .ok_or(ManagerError::UnknownSegment(segment_id))?;

        let kind = segment.kind();
        kind.compare_url(url)?;

        let slab_index = segment
            .open(url, mode, capacity)
            .map_err(|e| map_open_err(kind, e))?;
        let id = self.alloc_mem_id();
        self.by_locator.insert(key.to_owned(), id);
        self.by_mem.insert(
            id,
            MemEntry {
                locator: url.clone(),
                segment_id,
                used_len: 0,
            },
        );

        Ok(Registered {
            id,
            locator: url.clone(),
            segment_id,
            slab_index,
        })
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
        self.by_locator
            .get(locator.locator_key())
            .copied()
            .ok_or_else(|| ManagerError::UnknownLocator(locator.locator_key().to_owned()))
    }

    pub fn locator(&self, id: Id) -> Result<&Locator, ManagerError> {
        Ok(&self.mem_entry(id)?.locator)
    }

    pub fn mem_entry(&self, id: Id) -> Result<&MemEntry, ManagerError> {
        self.by_mem
            .get(&id)
            .ok_or(ManagerError::UnknownId(id))
    }

    fn mem_entry_mut(&mut self, id: Id) -> Result<&mut MemEntry, ManagerError> {
        self.by_mem
            .get_mut(&id)
            .ok_or(ManagerError::UnknownId(id))
    }

    pub fn segment_id_for(&self, id: Id) -> Result<SegmentId, ManagerError> {
        Ok(self.mem_entry(id)?.segment_id)
    }

    pub fn entries(&self) -> impl Iterator<Item = (Id, &Locator)> + '_ {
        self.by_mem.iter().map(|(id, e)| (*id, &e.locator))
    }

    pub fn slab_index(&self, id: Id) -> Result<usize, ManagerError> {
        let entry = self.mem_entry(id)?;
        let segment = self
            .segment(entry.segment_id)
            .ok_or(ManagerError::UnknownSegment(entry.segment_id))?;
        segment
            .locate_slab(&entry.locator)
            .ok_or(ManagerError::SlabNotInSegment)
    }

    pub fn set_used_len(&mut self, id: Id, used_len: usize) -> Result<(), ManagerError> {
        self.mem_entry_mut(id)?.used_len = used_len;
        Ok(())
    }

    pub fn used_len(&self, id: Id) -> Result<usize, ManagerError> {
        Ok(self.mem_entry(id)?.used_len)
    }

    pub fn usage_report(&self) -> UsageReport {
        let mut slabs = Vec::with_capacity(self.by_mem.len());
        let mut totals = UsageTotals::default();

        for (&id, entry) in &self.by_mem {
            let capacity = self
                .segment(entry.segment_id)
                .and_then(|s| {
                    s.locate_slab(&entry.locator).and_then(|i| s.size(i))
                })
                .unwrap_or(0);
            totals.slab_count += 1;
            totals.total_capacity += capacity;
            totals.total_used += entry.used_len;
            slabs.push(Usage {
                id,
                segment_id: entry.segment_id,
                scheme: entry.locator.scheme().to_owned(),
                locator: entry.locator.clone(),
                capacity,
                used_len: entry.used_len,
            });
        }

        slabs.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        UsageReport { slabs, totals }
    }

    pub fn close(&mut self, id: Id) -> Result<(), ManagerError> {
        let entry = self.mem_entry(id)?.clone();
        let key = entry.locator.as_str().to_owned();
        let idx = self.slab_index(id)?;
        let kind = self
            .segment(entry.segment_id)
            .ok_or(ManagerError::UnknownSegment(entry.segment_id))?
            .kind();
        self.segment_mut(entry.segment_id)
            .ok_or(ManagerError::UnknownSegment(entry.segment_id))?
            .close(idx)
            .map_err(|e| map_teardown_err(kind, e))?;
        self.by_locator.remove(&key);
        self.by_mem.remove(&id);
        Ok(())
    }

    pub fn close_all(&mut self) -> Result<(), ManagerError> {
        for segment in self.segments.values_mut() {
            let kind = segment.kind();
            segment.close_all().map_err(|e| map_slab_err(kind, e))?;
        }
        self.by_locator.clear();
        self.by_mem.clear();
        Ok(())
    }

    pub fn unlink(&mut self, id: Id) -> Result<(), ManagerError> {
        let entry = self.mem_entry(id)?.clone();
        let key = entry.locator.as_str().to_owned();
        let idx = self.slab_index(id)?;
        let kind = self
            .segment(entry.segment_id)
            .ok_or(ManagerError::UnknownSegment(entry.segment_id))?
            .kind();
        self.segment_mut(entry.segment_id)
            .ok_or(ManagerError::UnknownSegment(entry.segment_id))?
            .unlink(idx)
            .map_err(|e| map_teardown_err(kind, e))?;
        self.by_locator.remove(&key);
        self.by_mem.remove(&id);
        Ok(())
    }

    pub fn unlink_all(&mut self) -> Result<(), ManagerError> {
        for segment in self.segments.values_mut() {
            let kind = segment.kind();
            segment.unlink_all().map_err(|e| map_slab_err(kind, e))?;
        }
        self.by_locator.clear();
        self.by_mem.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pymergetic::cruspy::io::State;
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
    fn close_removes_registration_but_keeps_mapping_for_talc() {
        let mut mgr = Manager::new();
        let reg = mgr.create(&Ram::build_url("x"), Some(4096)).unwrap();
        let seg = reg.segment_id;
        mgr.close(reg.id).unwrap();
        assert!(matches!(
            mgr.id(&reg.locator),
            Err(ManagerError::UnknownLocator(_))
        ));
        let slab = mgr.segment(seg).unwrap().backend(0).unwrap();
        assert_eq!(slab.info().state, State::Closed);
        assert_eq!(mgr.segment(seg).unwrap().backends().len(), 1);
        assert_eq!(mgr.segment(seg).unwrap().size(0).unwrap(), 4096 - HEADER_LEN);
    }
}
