//! Map [`segment`](crate::pymergetic::cruspy::memory::segment) errors into [`ManagerError`](super::ManagerError).

use crate::pymergetic::cruspy::io::{Kind, SlabError};
use crate::pymergetic::cruspy::memory::segment::{SegmentOpenError, SegmentTeardownError};

use super::ManagerError;

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
