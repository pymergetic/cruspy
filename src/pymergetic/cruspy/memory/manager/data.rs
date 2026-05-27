//! [`ManagerData`] — catalog + segments storage behind [`super::Manager`].

use std::collections::HashMap;

use crate::pymergetic::cruspy::io::OpenMode;
use crate::pymergetic::cruspy::utils::url::Url;

use crate::pymergetic::cruspy::io::Kind;

use super::segment::{ensure_url_matches, AnySegment, SegmentId};
use super::usage::{Usage, UsageReport, UsageTotals};
use super::{Id, Locator, ManagerError, Registered};

/// Registered slab metadata (catalog only).
#[derive(Clone, Debug)]
pub struct MemEntry {
    pub locator: Locator,
    pub segment_id: SegmentId,
    pub used_len: usize,
}

/// Locator index + segment table.
#[derive(Default)]
pub struct Catalog {
    pub next_mem_id: u64,
    pub next_segment_id: u64,
    pub by_locator: HashMap<String, Id>,
    pub by_mem: HashMap<Id, MemEntry>,
    pub segments: HashMap<SegmentId, AnySegment>,
    /// First segment created per scheme (auto-routing for [`Manager::register`]).
    pub default_segment: HashMap<String, SegmentId>,
}

impl Catalog {
    pub fn alloc_mem_id(&mut self) -> Id {
        let id = Id(self.next_mem_id);
        self.next_mem_id += 1;
        id
    }

    pub fn alloc_segment_id(&mut self) -> SegmentId {
        let id = SegmentId(self.next_segment_id);
        self.next_segment_id += 1;
        id
    }
}

/// Storage and segment operations for [`super::Manager`].
pub trait ManagerData {
    fn catalog(&self) -> &Catalog;
    fn catalog_mut(&mut self) -> &mut Catalog;

    /// Create an empty segment for `kind` and return its id.
    fn create_segment(&mut self, kind: Kind) -> SegmentId {
        let id = self.catalog_mut().alloc_segment_id();
        let segment = AnySegment::new(kind);
        self.catalog_mut()
            .default_segment
            .entry(kind.scheme().to_owned())
            .or_insert(id);
        self.catalog_mut().segments.insert(id, segment);
        id
    }

    fn segment(&self, id: SegmentId) -> Option<&AnySegment> {
        self.catalog().segments.get(&id)
    }

    fn segment_mut(&mut self, id: SegmentId) -> Option<&mut AnySegment> {
        self.catalog_mut().segments.get_mut(&id)
    }

    fn segment_ids(&self) -> impl Iterator<Item = SegmentId> + '_ {
        self.catalog().segments.keys().copied()
    }

    fn default_segment_for_scheme(&self, scheme: &str) -> Option<SegmentId> {
        self.catalog().default_segment.get(scheme).copied()
    }

    /// Segment for `scheme`, creating one if this data layer has none yet.
    fn ensure_default_segment(&mut self, scheme: &str) -> Result<SegmentId, ManagerError> {
        if let Some(id) = self.default_segment_for_scheme(scheme) {
            return Ok(id);
        }
        let kind = Kind::from_scheme(scheme).ok_or_else(|| {
            ManagerError::UnsupportedScheme(scheme.to_owned())
        })?;
        Ok(self.create_segment(kind))
    }

    fn register_on(
        &mut self,
        segment_id: SegmentId,
        url: &Url,
        mode: OpenMode,
        capacity: Option<usize>,
    ) -> Result<Registered, ManagerError> {
        let key = url.as_str();
        if self.catalog().by_locator.contains_key(key) {
            return Err(ManagerError::DuplicateLocator(key.to_owned()));
        }

        let segment = self
            .segment_mut(segment_id)
            .ok_or(ManagerError::UnknownSegment(segment_id))?;

        ensure_url_matches(url, segment.kind())?;

        let slab_index = segment.open(url, mode, capacity)?;
        let id = self.catalog_mut().alloc_mem_id();
        self.catalog_mut().by_locator.insert(key.to_owned(), id);
        self.catalog_mut().by_mem.insert(
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

    fn register(
        &mut self,
        url: &Url,
        mode: OpenMode,
        capacity: Option<usize>,
    ) -> Result<Registered, ManagerError> {
        let segment_id = self.ensure_default_segment(url.scheme())?;
        self.register_on(segment_id, url, mode, capacity)
    }

    fn mem_entry(&self, id: Id) -> Result<&MemEntry, ManagerError> {
        self.catalog()
            .by_mem
            .get(&id)
            .ok_or(ManagerError::UnknownId(id))
    }

    fn mem_entry_mut(&mut self, id: Id) -> Result<&mut MemEntry, ManagerError> {
        self.catalog_mut()
            .by_mem
            .get_mut(&id)
            .ok_or(ManagerError::UnknownId(id))
    }

    fn slab_index(&self, id: Id) -> Result<usize, ManagerError> {
        let entry = self.mem_entry(id)?;
        let segment = self
            .segment(entry.segment_id)
            .ok_or(ManagerError::UnknownSegment(entry.segment_id))?;
        segment
            .locate_slab(&entry.locator)
            .ok_or(ManagerError::SlabNotInSegment)
    }

    fn usage_report(&self) -> UsageReport {
        let mut slabs = Vec::with_capacity(self.catalog().by_mem.len());
        let mut totals = UsageTotals::default();

        for (&id, entry) in &self.catalog().by_mem {
            let capacity = self
                .segment(entry.segment_id)
                .and_then(|s| {
                    s.locate_slab(&entry.locator)
                        .and_then(|i| s.slab_arena_len(i))
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

    fn close_mem(&mut self, id: Id) -> Result<(), ManagerError> {
        let entry = self.mem_entry(id)?.clone();
        let key = entry.locator.as_str().to_owned();
        let idx = self.slab_index(id)?;
        self.segment_mut(entry.segment_id)
            .ok_or(ManagerError::UnknownSegment(entry.segment_id))?
            .close_slab(idx)?;
        self.catalog_mut().by_locator.remove(&key);
        self.catalog_mut().by_mem.remove(&id);
        Ok(())
    }

    fn unlink_mem(&mut self, id: Id) -> Result<(), ManagerError> {
        let entry = self.mem_entry(id)?.clone();
        let key = entry.locator.as_str().to_owned();
        let idx = self.slab_index(id)?;
        self.segment_mut(entry.segment_id)
            .ok_or(ManagerError::UnknownSegment(entry.segment_id))?
            .unlink_slab(idx)?;
        self.catalog_mut().by_locator.remove(&key);
        self.catalog_mut().by_mem.remove(&id);
        Ok(())
    }

    fn close_all_mem(&mut self) -> Result<(), ManagerError> {
        for segment in self.catalog_mut().segments.values_mut() {
            segment.close_all()?;
        }
        self.catalog_mut().by_locator.clear();
        self.catalog_mut().by_mem.clear();
        Ok(())
    }

    fn unlink_all_mem(&mut self) -> Result<(), ManagerError> {
        for segment in self.catalog_mut().segments.values_mut() {
            segment.unlink_all()?;
        }
        self.catalog_mut().by_locator.clear();
        self.catalog_mut().by_mem.clear();
        Ok(())
    }
}

/// Default [`ManagerData`] — [`Catalog`] only.
#[derive(Default)]
pub struct DefaultData {
    catalog: Catalog,
}

impl DefaultData {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ManagerData for DefaultData {
    fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    fn catalog_mut(&mut self) -> &mut Catalog {
        &mut self.catalog
    }
}
