//! Rolled-up segment memory: slab layout, talc, and catalog chains.

use crate::pymergetic::cruspy::memory::segment::{Segment, SegmentError};

use super::chain::load_chain;
use super::metatype::MetaTypeCatalogKind;
use super::objects::ObjectCatalogKind;
use super::primary::primary_header;

/// Per-catalog-kind totals across all chain chunks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogKindStats {
    pub tag: &'static str,
    pub chunks: usize,
    pub rows: usize,
    pub slot_capacity: u32,
    pub reserved_bytes: usize,
    pub used_wire_bytes: usize,
}

/// Slab + talc + catalog breakdown for one segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentMemoryOverview {
    pub slabs: usize,
    pub raw_bytes: usize,
    pub header_bytes: usize,
    pub arena_bytes: usize,
    pub talc_claimed: usize,
    pub talc_allocated: usize,
    pub talc_overhead: usize,
    pub talc_available: usize,
    pub unclaimed_arena: usize,
    pub catalogs: [CatalogKindStats; 2],
}

impl SegmentMemoryOverview {
    pub fn catalog_reserved_total(&self) -> usize {
        self.catalogs.iter().map(|c| c.reserved_bytes).sum()
    }

    pub fn catalog_used_wire_total(&self) -> usize {
        self.catalogs.iter().map(|c| c.used_wire_bytes).sum()
    }

    pub fn heap_reserved_in_talc(&self) -> usize {
        self.talc_allocated.saturating_sub(self.catalog_reserved_total())
    }

    pub fn pct(part: usize, whole: usize) -> f64 {
        if whole == 0 {
            0.0
        } else {
            (part as f64) * 100.0 / (whole as f64)
        }
    }
}

fn chain_stats<K: super::wire::CatalogKind>(
    segment: &Segment,
    offset: u32,
    len: u32,
    tag: &'static str,
) -> Result<CatalogKindStats, SegmentError>
where
    K::Row: super::wire::CatalogRow,
{
    let chunks = load_chain::<K>(segment, offset, len)?;
    let mut stats = CatalogKindStats {
        tag,
        chunks: chunks.len(),
        rows: 0,
        slot_capacity: 0,
        reserved_bytes: 0,
        used_wire_bytes: 0,
    };
    for chunk in &chunks {
        stats.rows += chunk.catalog.count();
        stats.slot_capacity += chunk.catalog.capacity;
        stats.reserved_bytes += chunk.catalog.allocated_len();
        stats.used_wire_bytes += chunk.catalog.used_len();
    }
    Ok(stats)
}

impl Segment {
    pub fn memory_overview(&self) -> Result<SegmentMemoryOverview, SegmentError> {
        let h = primary_header(self)?;
        let raw_bytes = self.size_raw_all();
        let arena_bytes = self.size_all();
        let header_bytes = raw_bytes.saturating_sub(arena_bytes);
        let c = self.talc().counters();
        let talc_claimed = c.claimed_bytes;
        let catalogs = [
            chain_stats::<MetaTypeCatalogKind>(
                self,
                h.metatype_catalog_offset,
                h.metatype_catalog_len,
                "CTLG",
            )?,
            chain_stats::<ObjectCatalogKind>(
                self,
                h.object_catalog_offset,
                h.object_catalog_len,
                "COBJ",
            )?,
        ];
        Ok(SegmentMemoryOverview {
            slabs: self.backends().len(),
            raw_bytes,
            header_bytes,
            arena_bytes,
            talc_claimed,
            talc_allocated: c.allocated_bytes,
            talc_overhead: c.overhead_bytes(),
            talc_available: c.available_bytes,
            unclaimed_arena: arena_bytes.saturating_sub(talc_claimed),
            catalogs,
        })
    }
}

pub fn format_memory_overview(o: &SegmentMemoryOverview) -> String {
    use SegmentMemoryOverview as O;
    let mut out = String::new();
    out.push_str("Memory overview (segment)\n");
    out.push_str(&format!(
        "  DEVICE raw={} B\n",
        o.raw_bytes
    ));
    out.push_str(&format!(
        "    slab header (CRUS×{}): {} B ({:.1}% of raw)\n",
        o.slabs,
        o.header_bytes,
        O::pct(o.header_bytes, o.raw_bytes)
    ));
    out.push_str(&format!(
        "    arena body:            {} B ({:.1}% of raw)\n",
        o.arena_bytes,
        O::pct(o.arena_bytes, o.raw_bytes)
    ));
    if o.unclaimed_arena > 0 {
        out.push_str(&format!(
            "      unclaimed by talc:   {} B ({:.1}% of arena)\n",
            o.unclaimed_arena,
            O::pct(o.unclaimed_arena, o.arena_bytes)
        ));
    }
    out.push_str(&format!(
        "  TALC claimed:            {} B ({:.1}% of arena)\n",
        o.talc_claimed,
        O::pct(o.talc_claimed, o.arena_bytes)
    ));
    out.push_str(&format!(
        "    allocated (in use):    {} B ({:.1}% of claimed)\n",
        o.talc_allocated,
        O::pct(o.talc_allocated, o.talc_claimed)
    ));
    out.push_str(&format!(
        "    allocator overhead:    {} B ({:.1}% of claimed)\n",
        o.talc_overhead,
        O::pct(o.talc_overhead, o.talc_claimed)
    ));
    out.push_str(&format!(
        "    available (free heap): {} B ({:.1}% of claimed)\n",
        o.talc_available,
        O::pct(o.talc_available, o.talc_claimed)
    ));
    let heap_other = o.heap_reserved_in_talc();
    out.push_str("  TALC allocated breakdown:\n");
    for cat in &o.catalogs {
        out.push_str(&format!(
            "    {}: {} chunk(s) rows={}/{} slots reserved={} B used_wire={} B ({:.1}% of allocated)\n",
            cat.tag,
            cat.chunks,
            cat.rows,
            cat.slot_capacity,
            cat.reserved_bytes,
            cat.used_wire_bytes,
            O::pct(cat.reserved_bytes, o.talc_allocated),
        ));
    }
    out.push_str(&format!(
        "    heap (non-catalog):    {} B ({:.1}% of allocated)\n",
        heap_other,
        O::pct(heap_other, o.talc_allocated)
    ));
    out.push_str(&format!(
        "  Summary: used={} B free_in_talc={} B overhead={} B (of claimed {} B)\n",
        o.talc_allocated,
        o.talc_available,
        o.talc_overhead,
        o.talc_claimed
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pymergetic::cruspy::io::Kind;
    use crate::pymergetic::cruspy::memory::backend::ram::Ram;
    use crate::pymergetic::cruspy::memory::defaults::MIN_SLAB_CAPACITY;

    #[test]
    fn memory_overview_after_mount() {
        let mut seg = Segment::new(Kind::Ram);
        seg.create(&Ram::build_url("stats"), Some(MIN_SLAB_CAPACITY))
            .unwrap();
        let o = seg.memory_overview().unwrap();
        assert_eq!(o.slabs, 1);
        assert_eq!(o.header_bytes, HEADER_LEN);
        assert_eq!(o.catalogs[0].tag, "CTLG");
        assert_eq!(o.catalogs[0].rows, 1);
        assert_eq!(o.catalogs[1].tag, "COBJ");
        assert_eq!(o.catalogs[1].rows, 0);
        assert!(o.talc_allocated >= o.catalog_reserved_total());
    }
}
