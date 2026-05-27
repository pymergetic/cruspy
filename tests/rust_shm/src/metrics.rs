//! Usage metrics for registered mem slabs.

use std::fmt;

use crate::mem::kind::Kind;
use crate::registry::{Id, Locator};

/// Per-slab usage snapshot (capacity from mapping, used from registry high-water).
#[derive(Clone, Debug, PartialEq)]
pub struct Usage {
    pub id: Id,
    pub kind: Kind,
    pub locator: Locator,
    pub capacity: usize,
    pub used_len: usize,
}

impl Usage {
    pub fn free_len(&self) -> usize {
        self.capacity.saturating_sub(self.used_len)
    }

    /// Used bytes as a fraction of capacity in `[0.0, 1.0]`.
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            return 0.0;
        }
        (self.used_len as f64) / (self.capacity as f64)
    }

    pub fn utilization_pct(&self) -> f64 {
        self.utilization() * 100.0
    }

    /// ASCII bar: `█` = used, `·` = free.
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

/// Rolled-up totals across all registered mems.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RegistryTotals {
    pub slab_count: usize,
    pub total_capacity: usize,
    pub total_used: usize,
}

impl RegistryTotals {
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

/// Multi-line report for stdout / tests.
pub struct UsageReport {
    pub slabs: Vec<Usage>,
    pub totals: RegistryTotals,
}

impl UsageReport {
    pub fn format(&self, bar_width: usize) -> String {
        let mut out = String::new();
        out.push_str("Memory registry usage\n");
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
