//! Central catalog of registered memory slabs across multiple [`Segment`]s.

mod data;
mod error;
mod locator;
mod usage;

pub use data::{MemEntry, Registered};
pub use error::ManagerError;
pub use locator::{Locator, LocatorRef};
pub use crate::pymergetic::cruspy::memory::segment::SegmentId;
pub use usage::{format_talc_counters, Usage, UsageReport, UsageTotals};

use std::collections::HashMap;

use crate::pymergetic::cruspy::io::{Kind, OpenMode};
use crate::pymergetic::cruspy::memory::segment::Segment;
use crate::pymergetic::cruspy::utils::url::Url;

use error::{map_open_err, map_slab_err, map_teardown_err};

/// Opaque handle for a registered slab.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Id(pub u64);

/// Process-wide memory registry: locators, segments, and usage.
pub struct Manager {
    next_mem_id: u64,
    next_segment_id: u64,
    by_locator: HashMap<String, Id>,
    by_mem: HashMap<Id, MemEntry>,
    segments: HashMap<SegmentId, Segment>,
    /// Base locator key → segment ([`Locator::segment_base_key`]).
    by_segment_base: HashMap<String, SegmentId>,
    /// First segment created per scheme (auto-routing for [`Self::register`]).
    default_segment: HashMap<Locator, SegmentId>,
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
            by_segment_base: HashMap::new(),
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
            .entry(Locator::default_for_kind(kind))
            .or_insert(id);
        self.segments.insert(id, Segment::new(kind));
        id
    }

    /// Open a named segment: primary slab at `base` locator, type catalog on primary arena.
    pub fn open_segment(
        &mut self,
        base: Locator,
        capacity: Option<usize>,
    ) -> Result<SegmentId, ManagerError> {
        let key = base.base_key();
        if self.by_segment_base.contains_key(&key) {
            return Err(ManagerError::DuplicateSegment(key));
        }
        let kind = Kind::from_scheme(base.scheme())
            .ok_or_else(|| ManagerError::UnsupportedScheme(base.scheme().to_owned()))?;
        let id = self.alloc_segment_id();
        let mut segment = Segment::with_base(kind, base.clone());
        segment
            .create_primary(capacity)
            .map_err(|e| map_open_err(kind, e))?;
        self.segments.insert(id, segment);
        self.by_segment_base.insert(key, id);
        Ok(id)
    }

    pub fn segment_id_for_base<S: LocatorRef + ?Sized>(
        &self,
        locator: &S,
    ) -> Result<SegmentId, ManagerError> {
        let key = Locator::segment_base_key(locator.locator_key());
        self.by_segment_base
            .get(&key)
            .copied()
            .ok_or_else(|| ManagerError::UnknownSegmentBase(key))
    }

    /// Add heap extension `n` (`0` → `base-0`, …) to an open segment.
    pub fn add_extension(
        &mut self,
        base: Locator,
        n: u16,
        capacity: Option<usize>,
    ) -> Result<usize, ManagerError> {
        let seg_id = self.segment_id_for_base(&base)?;
        let kind = self
            .segment(seg_id)
            .ok_or(ManagerError::UnknownSegment(seg_id))?
            .kind();
        self.segment_mut(seg_id)
            .ok_or(ManagerError::UnknownSegment(seg_id))?
            .add_extension(n, capacity)
            .map_err(|e| map_open_err(kind, e))
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

    fn ensure_default_segment(&mut self, scheme: &str) -> Result<SegmentId, ManagerError> {
        if let Some(key) = Locator::default_for_scheme(scheme) {
            if let Some(id) = self.default_segment.get(&key).copied() {
                return Ok(id);
            }
        }
        let kind = Kind::from_scheme(scheme)
            .ok_or_else(|| ManagerError::UnsupportedScheme(scheme.to_owned()))?;
        if let Some(id) = self
            .default_segment
            .get(&Locator::default_for_kind(kind))
            .copied()
        {
            return Ok(id);
        }
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
                locator: Locator::from(url.clone()),
                segment_id,
            },
        );

        Ok(Registered {
            id,
            locator: Locator::from(url.clone()),
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

    pub fn try_id<S: LocatorRef + ?Sized>(&self, locator: &S) -> Option<Id> {
        self.by_locator.get(locator.locator_key()).copied()
    }

    pub fn contains_locator<S: LocatorRef + ?Sized>(&self, locator: &S) -> bool {
        self.by_locator.contains_key(locator.locator_key())
    }

    pub fn locator(&self, id: Id) -> Result<&Locator, ManagerError> {
        Ok(&self.mem_entry(id)?.locator)
    }

    pub fn mem_entry(&self, id: Id) -> Result<&MemEntry, ManagerError> {
        self.by_mem
            .get(&id)
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
            .locate_slab(entry.locator.as_url())
            .ok_or(ManagerError::SlabNotInSegment)
    }

    pub fn usage_report(&self) -> UsageReport {
        let mut slabs = Vec::with_capacity(self.by_mem.len());
        let mut totals = UsageTotals::default();

        for (&id, entry) in &self.by_mem {
            let (raw_len, arena_len) = self
                .segment(entry.segment_id)
                .and_then(|s| s.locate_slab(entry.locator.as_url()).map(|i| (s.size_raw(i), s.size(i))))
                .map(|(raw, arena)| (raw.unwrap_or(0), arena.unwrap_or(0)))
                .unwrap_or((0, 0));
            let header_len = raw_len.saturating_sub(arena_len);
            totals.slab_count += 1;
            totals.total_raw_len += raw_len;
            totals.total_arena_len += arena_len;
            totals.total_header_len += header_len;
            slabs.push(Usage {
                id,
                segment_id: entry.segment_id,
                scheme: entry.locator.scheme().to_owned(),
                locator: entry.locator.clone(),
                raw_len,
                header_len,
                arena_len,
            });
        }

        for segment in self.segments.values() {
            let c = segment.talc().counters();
            totals.talc.allocation_count += c.allocation_count;
            totals.talc.total_allocation_count += c.total_allocation_count;
            totals.talc.allocated_bytes += c.allocated_bytes;
            totals.talc.total_allocated_bytes += c.total_allocated_bytes;
            totals.talc.available_bytes += c.available_bytes;
            totals.talc.fragment_count += c.fragment_count;
            totals.talc.heap_count += c.heap_count;
            totals.talc.total_heap_count += c.total_heap_count;
            totals.talc.claimed_bytes += c.claimed_bytes;
            totals.talc.total_claimed_bytes += c.total_claimed_bytes;
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
    use crate::pymergetic::cruspy::memory::defaults::MIN_SLAB_CAPACITY;
    use crate::pymergetic::cruspy::memory::segment::{
        DEFAULT_METATYPE_CATALOG_CAPACITY, DEFAULT_OBJECT_CATALOG_CAPACITY,
    };
    use crate::pymergetic::cruspy::memory::segment::{
        HEADER_LEN, MAGIC, SLAB_ROLE_HEAP_EXT, VERSION,
    };

    #[test]
    fn register_two_slabs_same_default_segment() {
        let mut mgr = Manager::new();
        let a = mgr
            .create(&Ram::build_url("a"), Some(MIN_SLAB_CAPACITY))
            .expect("create a");
        let b = mgr
            .create(&Ram::build_url("b"), Some(MIN_SLAB_CAPACITY))
            .expect("create b");
        assert_eq!(a.segment_id, b.segment_id);
        assert_ne!(a.id, b.id);
        let report = mgr.usage_report();
        assert_eq!(report.totals.slab_count, 2);
        let arena_a = MIN_SLAB_CAPACITY - HEADER_LEN;
        let arena_b = MIN_SLAB_CAPACITY - HEADER_LEN;
        assert_eq!(report.totals.total_arena_len, arena_a + arena_b);
        assert_eq!(mgr.segment_ids().count(), 1);
    }

    #[test]
    fn multiple_segments_explicit() {
        let mut mgr = Manager::new();
        let s0 = mgr.create_segment(Kind::Ram);
        let s1 = mgr.create_segment(Kind::Ram);
        assert_ne!(s0, s1);
        let a = mgr
            .register_on(
                s0,
                &Ram::build_url("a"),
                OpenMode::Create,
                Some(MIN_SLAB_CAPACITY),
            )
            .unwrap();
        let b = mgr
            .register_on(s1, &Ram::build_url("b"), OpenMode::Create, Some(MIN_SLAB_CAPACITY))
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
                Some(MIN_SLAB_CAPACITY),
            )
            .unwrap_err();
        assert!(matches!(err, ManagerError::SchemeMismatch { .. }));
    }

    #[test]
    fn duplicate_locator_rejected() {
        let mut mgr = Manager::new();
        let url = Ram::build_url("dup");
        mgr.create(&url, Some(MIN_SLAB_CAPACITY)).unwrap();
        assert!(matches!(
            mgr.create(&url, Some(MIN_SLAB_CAPACITY)),
            Err(ManagerError::DuplicateLocator(_))
        ));
    }

    #[test]
    fn close_removes_registration_but_keeps_mapping_for_talc() {
        let mut mgr = Manager::new();
        let reg = mgr
            .create(&Ram::build_url("x"), Some(MIN_SLAB_CAPACITY))
            .unwrap();
        let seg = reg.segment_id;
        mgr.close(reg.id).unwrap();
        assert!(matches!(
            mgr.id(&reg.locator),
            Err(ManagerError::UnknownLocator(_))
        ));
        let slab = mgr.segment(seg).unwrap().backend(0).unwrap();
        assert_eq!(slab.info().state, State::Closed);
        assert_eq!(mgr.segment(seg).unwrap().backends().len(), 1);
        assert_eq!(
            mgr.segment(seg).unwrap().size(0).unwrap(),
            MIN_SLAB_CAPACITY - HEADER_LEN
        );
    }

    #[test]
    fn slab_below_minimum_rejected() {
        let mut mgr = Manager::new();
        let err = mgr
            .create(&Ram::build_url("tiny"), Some(4096))
            .unwrap_err();
        assert!(matches!(
            err,
            ManagerError::Layout {
                scheme,
                detail
            } if scheme == "ram" && detail.contains("capacity required")
        ));
    }

    #[test]
    fn open_segment_base_locator_and_extension() {
        let mut mgr = Manager::new();
        let base: Locator = Ram::build_url("seg-core").into();
        let seg_id = mgr.open_segment(base.clone(), Some(MIN_SLAB_CAPACITY)).unwrap();
        assert_eq!(mgr.segment_id_for_base(&base).unwrap(), seg_id);
        let seg = mgr.segment(seg_id).unwrap();
        assert_eq!(seg.slab_count(), 1);
        let cat = seg.metatype_catalog().unwrap();
        use crate::pymergetic::cruspy::memory::segment::{
            MetaTypeCatalog, METATYPE_CATALOG_SELF_INDEX,
        };
        use crate::pymergetic::cruspy::memory::types::MetaType;
        assert_eq!(cat.metatypes().len(), 1);
        assert_eq!(cat.capacity(), DEFAULT_METATYPE_CATALOG_CAPACITY);
        assert_eq!(
            cat.metatypes()[METATYPE_CATALOG_SELF_INDEX as usize],
            MetaType::from_type::<MetaTypeCatalog>().to_header()
        );
        let obj = seg.object_catalog().unwrap();
        assert_eq!(obj.capacity(), DEFAULT_OBJECT_CATALOG_CAPACITY);
        let ext_idx = mgr.add_extension(base.clone(), 0, Some(MIN_SLAB_CAPACITY)).unwrap();
        assert_eq!(ext_idx, 1);
        let seg = mgr.segment(seg_id).unwrap();
        assert_eq!(seg.slab_count(), 2);
        let primary = seg.header(0).unwrap();
        assert!(primary.is_primary());
        assert_eq!(primary.extension_count, 2);
        let ext = base.extension(0);
        assert_eq!(seg.locate_slab(ext.as_url()), Some(1));
        assert_eq!(seg.header(1).unwrap().slab_role, SLAB_ROLE_HEAP_EXT);
        assert!(seg.header(0).unwrap().is_mounted());
        assert!(seg.header(1).unwrap().is_mounted());
    }

    #[test]
    fn manager_end_to_end_catalog_segment_slab_flow() {
        let mut mgr = Manager::new();

        // Layer 1: locator defaults are deterministic per storage kind.
        assert_eq!(Locator::default().scheme(), "ram");
        assert_eq!(Locator::default_for_kind(Kind::Shm).scheme(), "shm");
        assert_eq!(Locator::default_for_kind(Kind::File).scheme(), "file");

        // Layer 2 + 3: manager registration creates/uses one segment and installs slab headers.
        let a = mgr
            .create(&Ram::build_url("flow-a"), Some(MIN_SLAB_CAPACITY))
            .unwrap();
        let b = mgr.create(&Ram::build_url("flow-b"), Some(MIN_SLAB_CAPACITY)).unwrap();
        assert_eq!(a.segment_id, b.segment_id);

        // Catalog round-trips through id/locator with fast existence checks.
        assert_eq!(mgr.id(&a.locator).unwrap(), a.id);
        assert_eq!(mgr.try_id(&a.locator), Some(a.id));
        assert!(mgr.contains_locator(&a.locator));
        assert_eq!(mgr.locator(a.id).unwrap().as_str(), a.locator.as_str());

        // Resolve to the segment/slab layer and verify slab layout metadata.
        let a_index = mgr.slab_index(a.id).unwrap();
        let seg = mgr.segment(a.segment_id).unwrap();
        assert_eq!(seg.backends().len(), 2);
        let hdr = seg.header(a_index).unwrap();
        assert_eq!(hdr.magic, MAGIC);
        assert_eq!(hdr.version, VERSION);
        assert_eq!(hdr.len as usize, MIN_SLAB_CAPACITY - HEADER_LEN);

        // Logical close removes registration, but leaves slab mapping reachable for talc safety.
        mgr.close(a.id).unwrap();
        assert_eq!(mgr.try_id(&a.locator), None);
        let seg = mgr.segment(a.segment_id).unwrap();
        let slab = seg.backend(a_index).unwrap();
        assert_eq!(slab.info().state, State::Closed);
        assert_eq!(
            seg.size(a_index).unwrap(),
            MIN_SLAB_CAPACITY - HEADER_LEN
        );
    }
}
