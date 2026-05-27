//! Which memory device backs a slab.

use std::fmt::{self, Display};
use std::str::FromStr;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Ram,
    PosixShm,
    File,
}

impl Kind {
    pub const ALL: [Self; 3] = [Self::Ram, Self::PosixShm, Self::File];

    pub fn iter() -> impl Iterator<Item = Self> {
        Self::ALL.into_iter()
    }

    /// Stable label for metrics, logs, and config (not `Debug` formatting).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ram => "ram",
            Self::PosixShm => "posix_shm",
            Self::File => "file",
        }
    }

    /// `ram | posix_shm | file` — built from [`Self::iter`].
    pub fn expected_list() -> String {
        Self::iter()
            .map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

impl Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Kind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::iter().find(|k| k.as_str() == s).ok_or_else(|| {
            format!("unknown mem kind {s:?}; expected: {}", Self::expected_list())
        })
    }
}
