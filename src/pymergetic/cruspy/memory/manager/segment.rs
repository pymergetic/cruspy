//! Manager segment id and URL / kind checks.

use crate::pymergetic::cruspy::io::Kind;
use crate::pymergetic::cruspy::memory::segment::{SegmentOpenError, SegmentTeardownError};

use super::ManagerError;

/// Opaque id for a [`Segment`](crate::pymergetic::cruspy::memory::segment::Segment) in the manager.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SegmentId(pub u64);

pub(crate) fn map_open_err(kind: Kind, err: SegmentOpenError) -> ManagerError {
    match err {
        SegmentOpenError::Backend(e) => ManagerError::Backend {
            scheme: kind.scheme().into(),
            message: e.to_string(),
        },
        SegmentOpenError::Layout(e) => ManagerError::Layout {
            scheme: kind.scheme().into(),
            detail: format!("{e:?}"),
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

pub(crate) fn map_slab_err(kind: Kind, err: crate::pymergetic::cruspy::io::SlabError) -> ManagerError {
    ManagerError::Backend {
        scheme: kind.scheme().into(),
        message: err.to_string(),
    }
}
