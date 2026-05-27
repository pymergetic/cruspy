//! [`HasSlab`] — object-safe runtime backend (metadata, open, map, resize, kind).

use std::any::Any;
use std::fmt;

use super::{HasAccess, HasInfo, HasKind, HasMapping, HasResize, Info, Kind, OpenMode};
use crate::pymergetic::cruspy::utils::url::Url;

/// Erased backend error from [`HasSlab::open`] / teardown / resize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlabError(pub String);

impl SlabError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl fmt::Display for SlabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for SlabError {}

impl From<std::io::Error> for SlabError {
    fn from(e: std::io::Error) -> Self {
        Self(e.to_string())
    }
}

/// Runtime slab (segment backends, [`Kind::create`](crate::pymergetic::cruspy::memory::backend::factory::Kind::create)).
///
/// Implemented automatically for any type that implements [`HasKind`] + the [`HasAccess`] stack
/// (see blanket impl below). Runtime kind: [`kind`](Self::kind); static: [`HasKind::KIND`].
pub trait HasSlab: Send + Any {
    fn kind(&self) -> Kind;

    fn info(&self) -> &Info;
    fn info_mut(&mut self) -> &mut Info;

    fn open(
        &mut self,
        url: &Url,
        mode: OpenMode,
        capacity: Option<usize>,
    ) -> Result<(), SlabError>;

    fn close(&mut self) -> Result<(), SlabError>;
    fn unlink(&mut self) -> Result<(), SlabError>;

    fn bytes(&self) -> &[u8];
    fn bytes_mut(&mut self) -> &mut [u8];
    fn resize(&mut self, new_capacity: usize) -> Result<(), SlabError>;

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> HasSlab for T
where
    T: Send + Any + HasKind + HasInfo + HasAccess + HasMapping + HasResize,
    T::Error: fmt::Display,
{
    fn kind(&self) -> Kind {
        <Self as HasKind>::KIND
    }

    fn info(&self) -> &Info {
        HasInfo::info(self)
    }

    fn info_mut(&mut self) -> &mut Info {
        HasInfo::info_mut(self)
    }

    fn open(
        &mut self,
        url: &Url,
        mode: OpenMode,
        capacity: Option<usize>,
    ) -> Result<(), SlabError> {
        *self = <Self as HasAccess>::open(url, mode, capacity).map_err(|e| SlabError(e.to_string()))?;
        Ok(())
    }

    fn close(&mut self) -> Result<(), SlabError> {
        HasAccess::close(self).map_err(|e| SlabError(e.to_string()))
    }

    fn unlink(&mut self) -> Result<(), SlabError> {
        HasAccess::unlink(self).map_err(|e| SlabError(e.to_string()))
    }

    fn bytes(&self) -> &[u8] {
        HasMapping::bytes(self)
    }

    fn bytes_mut(&mut self) -> &mut [u8] {
        HasMapping::bytes_mut(self)
    }

    fn resize(&mut self, new_capacity: usize) -> Result<(), SlabError> {
        HasResize::resize(self, new_capacity).map_err(|e| SlabError(e.to_string()))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
