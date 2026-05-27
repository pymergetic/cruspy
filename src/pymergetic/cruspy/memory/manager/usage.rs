//! Per-slab and rolled-up usage for [`super::Manager`].

use std::fmt;

use super::segment::SegmentId;
use super::Id;
use crate::pymergetic::cruspy::utils::url::Url;

/// Per-slab usage snapshot (arena capacity + manager high-water).
#[derive(Clone, Debug, PartialEq)]
pub struct Usage {
    pub id: Id,
    pub segment_id: SegmentId,
    pub scheme: String,
    pub locator: Url,
    pub capacity: usize,
    pub used_len: usize,
}

impl Usage {
    pub fn free_len(&self) -> usize {
        self.capacity.saturating_sub(self.used_len)
    }

    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            return 0.0;
        }
        (self.used_len as f64) / (self.capacity as f64)
    }

    pub fn utilization_pct(&self) -> f64 {
        self.utilization() * 100.0
    }

    pub fn bar(&self, width: usize) -> String {
        let width = width.max(1);
        let filled = ((self.utilization() * width as f64).round() as usize).min(width);
        let mut s = String::with_capacity(width);
        for i in 0..width {
            s.push(if i < filled { '█' } else { '·' });
        }
        s
    }

    pub fn format_line(&self, bar_width: usize) -> String {
        format!(
            "{:<24} [{}] {:>5} / {:>5} B ({:>5.1}%)  id={}",
            self.locator,
            self.bar(bar_width),
            self.used_len,
            self.capacity,
            self.utilization_pct(),
            self.id.0,
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UsageTotals {
    pub slab_count: usize,
    pub total_capacity: usize,
    pub total_used: usize,
}

impl UsageTotals {
    pub fn total_free(&self) -> usize {
        self.total_capacity.saturating_sub(self.total_used)
    }

    pub fn utilization(&self) -> f64 {
        if self.total_capacity == 0 {
            return 0.0;
        }
        (self.total_used as f64) / (self.total_capacity as f64)
    }

    pub fn utilization_pct(&self) -> f64 {
        self.utilization() * 100.0
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
        let bar_hdr = "USED / CAPACITY";
        out.push_str(&format!(
            "{:<24}   {:^bw$}   {:>5}   {:>5}   {:>7}\n",
            "LOCATOR",
            bar_hdr,
            "USED",
            "CAP",
            "UTIL%",
            bw = bar_width.max(bar_hdr.len()),
        ));
        out.push_str(&"-".repeat(72));
        out.push('\n');
        for u in &self.slabs {
            out.push_str(&u.format_line(bar_width));
            out.push('\n');
        }
        out.push_str(&"-".repeat(72));
        out.push('\n');
        let t = &self.totals;
        out.push_str(&format!(
            "TOTAL ({} slabs)     used {:>5} / {:>5} B ({:.1}%)\n",
            t.slab_count,
            t.total_used,
            t.total_capacity,
            t.utilization_pct(),
        ));
        out
    }
}

impl fmt::Display for UsageReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format(24))
    }
}
