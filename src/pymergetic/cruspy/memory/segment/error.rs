//! Segment operation errors.

use std::fmt;

use crate::pymergetic::cruspy::io::SlabError;

#[derive(Debug)]
pub enum SegmentError {
    CapacityRequired,
    ArenaClaim,
    BadIndex,
    BadHeader,
    UnsupportedScheme(String),
    NoBaseLocator,
    SegmentUuidMismatch,
    CatalogAlloc,
    NotMounted,
}

impl fmt::Display for SegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityRequired => write!(f, "capacity required"),
            Self::ArenaClaim => write!(f, "arena talc claim failed"),
            Self::BadIndex => write!(f, "slab index out of range"),
            Self::BadHeader => write!(f, "invalid or missing segment header"),
            Self::UnsupportedScheme(s) => write!(f, "unsupported url scheme: {s}"),
            Self::NoBaseLocator => write!(f, "segment has no base locator"),
            Self::SegmentUuidMismatch => write!(f, "slab segment_uuid mismatch"),
            Self::CatalogAlloc => write!(f, "talc catalog allocation failed"),
            Self::NotMounted => write!(f, "slab arena not mounted"),
        }
    }
}

impl std::error::Error for SegmentError {}

/// [`Segment::close`] / [`Segment::unlink`] failed on a backend, or slab index out of range.
#[derive(Debug)]
pub enum SegmentTeardownError {
    BadIndex,
    Backend(SlabError),
}

impl fmt::Display for SegmentTeardownError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadIndex => write!(f, "slab index out of range"),
            Self::Backend(e) => write!(f, "backend teardown: {e}"),
        }
    }
}

impl std::error::Error for SegmentTeardownError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BadIndex => None,
            Self::Backend(e) => Some(e),
        }
    }
}

/// Backend open failed, or segment layout / talc claim failed after open.
#[derive(Debug)]
pub enum SegmentOpenError {
    Backend(SlabError),
    Layout(SegmentError),
    UnsupportedScheme(String),
}

impl fmt::Display for SegmentOpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(e) => write!(f, "backend open: {e}"),
            Self::Layout(e) => write!(f, "segment layout: {e}"),
            Self::UnsupportedScheme(s) => write!(f, "unsupported url scheme: {s}"),
        }
    }
}

impl std::error::Error for SegmentOpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Backend(e) => Some(e),
            Self::Layout(e) => Some(e),
            Self::UnsupportedScheme(_) => None,
        }
    }
}
