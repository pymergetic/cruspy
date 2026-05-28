//! Map [`segment`](crate::pymergetic::cruspy::memory::segment) errors into [`ManagerError`](super::ManagerError).

use std::fmt;

use crate::pymergetic::cruspy::io::{Kind, SlabError};
use crate::pymergetic::cruspy::memory::segment::{SegmentOpenError, SegmentTeardownError};

use super::{Id, SegmentId};

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

pub(crate) fn map_open_err(kind: Kind, err: SegmentOpenError) -> ManagerError {
    match err {
        SegmentOpenError::Backend(e) => ManagerError::Backend {
            scheme: kind.scheme().into(),
            message: e.to_string(),
        },
        SegmentOpenError::Layout(e) => ManagerError::Layout {
            scheme: kind.scheme().into(),
            detail: e.to_string(),
        },
        SegmentOpenError::UnsupportedScheme(s) => ManagerError::UnsupportedScheme(s),
    }
}

pub(crate) fn map_teardown_err(kind: Kind, err: SegmentTeardownError) -> ManagerError {
    match err {
        SegmentTeardownError::BadIndex => ManagerError::SlabNotInSegment,
        SegmentTeardownError::Backend(e) => ManagerError::Backend {
            scheme: kind.scheme().into(),
            message: e.to_string(),
        },
    }
}

pub(crate) fn map_slab_err(kind: Kind, err: SlabError) -> ManagerError {
    ManagerError::Backend {
        scheme: kind.scheme().into(),
        message: err.to_string(),
    }
}
