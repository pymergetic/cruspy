//! Per-slab and rolled-up usage for [`super::Manager`].

use std::fmt;

use super::{Id, Locator};
use crate::pymergetic::cruspy::memory::segment::SegmentId;
use talc::base::Counters;

/// Per-slab usage snapshot (raw mapping, fixed header, and allocator-visible arena).
#[derive(Clone, Debug, PartialEq)]
pub struct Usage {
    pub id: Id,
    pub segment_id: SegmentId,
    pub scheme: String,
    pub locator: Locator,
    /// Full mapped length for this slab.
    pub raw_len: usize,
    /// Fixed segment header bytes (raw - arena).
    pub header_len: usize,
    /// Arena bytes made available to talc.
    pub arena_len: usize,
}

impl Usage {
    pub fn header_pct_raw(&self) -> f64 {
        if self.raw_len == 0 {
            return 0.0;
        }
        (self.header_len as f64) * 100.0 / (self.raw_len as f64)
    }

    pub fn arena_pct_raw(&self) -> f64 {
        if self.raw_len == 0 {
            return 0.0;
        }
        (self.arena_len as f64) * 100.0 / (self.raw_len as f64)
    }

    pub fn format_line(&self, bar_width: usize) -> String {
        let bar = {
            let width = bar_width.max(1);
            let filled = ((self.arena_pct_raw() / 100.0) * width as f64).round() as usize;
            let mut s = String::with_capacity(width);
            for i in 0..width {
                s.push(if i < filled.min(width) { '█' } else { '·' });
            }
            s
        };
        format!(
            "{:<24} [{}] raw={:>5} arena={:>5} header={:>4} ({:>5.1}%/{:>5.1}%) id={}",
            self.locator,
            bar,
            self.raw_len,
            self.arena_len,
            self.header_len,
            self.arena_pct_raw(),
            self.header_pct_raw(),
            self.id.0,
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UsageTotals {
    pub slab_count: usize,
    pub total_raw_len: usize,
    pub total_header_len: usize,
    pub total_arena_len: usize,
    /// Aggregated allocator counters across all segments.
    pub talc: Counters,
}

impl UsageTotals {
    pub fn header_pct_raw(&self) -> f64 {
        if self.total_raw_len == 0 {
            return 0.0;
        }
        (self.total_header_len as f64) * 100.0 / (self.total_raw_len as f64)
    }

    pub fn arena_pct_raw(&self) -> f64 {
        if self.total_raw_len == 0 {
            return 0.0;
        }
        (self.total_arena_len as f64) * 100.0 / (self.total_raw_len as f64)
    }

    pub fn talc_allocated_pct_claimed(&self) -> f64 {
        if self.talc.claimed_bytes == 0 {
            return 0.0;
        }
        (self.talc.allocated_bytes as f64) * 100.0 / (self.talc.claimed_bytes as f64)
    }

    pub fn talc_overhead_pct_claimed(&self) -> f64 {
        if self.talc.claimed_bytes == 0 {
            return 0.0;
        }
        (self.talc.overhead_bytes() as f64) * 100.0 / (self.talc.claimed_bytes as f64)
    }
}

pub struct UsageReport {
    pub slabs: Vec<Usage>,
    pub totals: UsageTotals,
}

impl UsageReport {
    pub fn format(&self, bar_width: usize) -> String {
        let mut out = String::new();
        out.push_str("Memory manager usage\n");
        let bar_hdr = "ARENA / RAW";
        out.push_str(&format!(
            "{:<24}   {:^bw$}   {:>5}   {:>5}   {:>6}   {:>11}\n",
            "LOCATOR",
            bar_hdr,
            "RAW",
            "ARENA",
            "HEADER",
            "ARENA/HEADER",
            bw = bar_width.max(bar_hdr.len()),
        ));
        out.push_str(&"-".repeat(102));
        out.push('\n');
        for u in &self.slabs {
            out.push_str(&u.format_line(bar_width));
            out.push('\n');
        }
        out.push_str(&"-".repeat(102));
        out.push('\n');
        let t = &self.totals;
        out.push_str(&format!(
            "TOTAL ({} slabs): raw={} arena={} header={} (arena={:.1}% header={:.1}%)\n",
            t.slab_count,
            t.total_raw_len,
            t.total_arena_len,
            t.total_header_len,
            t.arena_pct_raw(),
            t.header_pct_raw(),
        ));
        out.push_str(&format!(
            "TALC: claimed={} available={} allocated={} overhead={} fragments={} (alloc={:.1}% overhead={:.1}% of claimed)\n",
            t.talc.claimed_bytes,
            t.talc.available_bytes,
            t.talc.allocated_bytes,
            t.talc.overhead_bytes(),
            t.talc.fragment_count,
            t.talc_allocated_pct_claimed(),
            t.talc_overhead_pct_claimed(),
        ));
        out
    }
}

impl fmt::Display for UsageReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format(24))
    }
}

/// Human-readable talc allocator snapshot (shared [`talc::base::Counters`] shape).
pub fn format_talc_counters(prefix: &str, c: &Counters) -> String {
    let alloc_pct = if c.claimed_bytes == 0 {
        0.0
    } else {
        (c.allocated_bytes as f64) * 100.0 / (c.claimed_bytes as f64)
    };
    let overhead_pct = if c.claimed_bytes == 0 {
        0.0
    } else {
        (c.overhead_bytes() as f64) * 100.0 / (c.claimed_bytes as f64)
    };
    let avail_pct = if c.claimed_bytes == 0 {
        0.0
    } else {
        (c.available_bytes as f64) * 100.0 / (c.claimed_bytes as f64)
    };
    format!(
        "{prefix}talc heap (one allocator per segment):\n\
         {prefix}  claimed={} B  available={} B ({avail_pct:.1}% of claimed)\n\
         {prefix}  allocated={} B ({alloc_pct:.1}% of claimed)  overhead={} B ({overhead_pct:.1}%)\n\
         {prefix}  live_allocs={} (total ever={})  fragments={}  heaps={} (total ever={})",
        c.claimed_bytes,
        c.available_bytes,
        c.allocated_bytes,
        c.overhead_bytes(),
        c.allocation_count,
        c.total_allocation_count,
        c.fragment_count,
        c.heap_count,
        c.total_heap_count,
    )
}
