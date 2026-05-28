//! Map [`segment`](crate::pymergetic::cruspy::memory::segment) errors into [`ManagerError`](super::ManagerError).

use thiserror::Error;

use crate::pymergetic::cruspy::io::{Kind, SlabError};
use crate::pymergetic::cruspy::memory::segment::{SegmentOpenError, SegmentTeardownError};

use super::{Id, SegmentId};

#[derive(Error, Debug)]
pub enum ManagerError {
    #[error("locator already registered: {0}")]
    DuplicateLocator(String),

    #[error("segment already open: {0}")]
    DuplicateSegment(String),

    #[error("unknown segment base: {0}")]
    UnknownSegmentBase(String),

    #[error("unknown locator: {0}")]
    UnknownLocator(String),

    #[error("unknown mem id: {}", .0.0)]
    UnknownId(Id),

    #[error("unknown segment id: {}", .0.0)]
    UnknownSegment(SegmentId),

    #[error("slab not found in segment")]
    SlabNotInSegment,

    #[error("unsupported url scheme: {0}")]
    UnsupportedScheme(String),

    #[error("url scheme {url_scheme} does not match storage kind {}", kind.scheme())]
    SchemeMismatch {
        url_scheme: String,
        kind: Kind,
    },

    #[error("{scheme} backend error: {message}")]
    Backend {
        scheme: String,
        message: String,
    },

    #[error("{scheme} layout error: {detail}")]
    Layout {
        scheme: String,
        detail: String,
    },
}

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
