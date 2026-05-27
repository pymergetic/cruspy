//! Type-erased [`Segment`]s for the memory manager (`Box<dyn SegmentOps>`).

use std::any::Any;
use std::fmt;

use crate::pymergetic::cruspy::io::{HasKind, Kind, OpenMode};
use crate::pymergetic::cruspy::memory::backend::file::File;
use crate::pymergetic::cruspy::memory::backend::ram::Ram;
use crate::pymergetic::cruspy::memory::backend::shm::Shm;
use crate::pymergetic::cruspy::memory::segment::{Segment, SegmentOpenError, SegmentTeardownError};
use crate::pymergetic::cruspy::utils::url::Url;

use super::ManagerError;

/// Opaque id for a [`Segment`] instance inside the manager.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SegmentId(pub u64);

/// Operations the manager needs on any homogeneous [`Segment<B>`].
pub trait SegmentOps: Any {
    fn kind(&self) -> Kind;

    fn open(
        &mut self,
        url: &Url,
        mode: OpenMode,
        capacity: Option<usize>,
    ) -> Result<usize, ManagerError>;

    fn locate_slab(&self, locator: &Url) -> Option<usize>;

    fn slab_arena_len(&self, slab_index: usize) -> Option<usize>;

    fn close_slab(&mut self, slab_index: usize) -> Result<(), ManagerError>;

    fn unlink_slab(&mut self, slab_index: usize) -> Result<(), ManagerError>;

    fn close_all(&mut self) -> Result<(), ManagerError>;

    fn unlink_all(&mut self) -> Result<(), ManagerError>;

    fn slab_count(&self) -> usize;

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<B> SegmentOps for Segment<B>
where
    B: HasKind + 'static,
    B::Error: fmt::Display,
{
    fn kind(&self) -> Kind {
        B::KIND
    }

    fn open(
        &mut self,
        url: &Url,
        mode: OpenMode,
        capacity: Option<usize>,
    ) -> Result<usize, ManagerError> {
        self.open(url, mode, capacity)
            .map_err(|e| map_open_err(B::KIND, e))
    }

    fn locate_slab(&self, locator: &Url) -> Option<usize> {
        self.backends()
            .iter()
            .position(|b| b.info().url == *locator)
    }

    fn slab_arena_len(&self, slab_index: usize) -> Option<usize> {
        self.size(slab_index)
    }

    fn close_slab(&mut self, slab_index: usize) -> Result<(), ManagerError> {
        self.close(slab_index)
            .map_err(|e| map_teardown_err(B::KIND, e))
    }

    fn unlink_slab(&mut self, slab_index: usize) -> Result<(), ManagerError> {
        self.unlink(slab_index)
            .map_err(|e| map_teardown_err(B::KIND, e))
    }

    fn close_all(&mut self) -> Result<(), ManagerError> {
        Segment::close_all(self).map_err(|e| backend_err(B::KIND, e))
    }

    fn unlink_all(&mut self) -> Result<(), ManagerError> {
        Segment::unlink_all(self).map_err(|e| backend_err(B::KIND, e))
    }

    fn slab_count(&self) -> usize {
        self.backends().len()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Erased segment — [`Kind`] picks the concrete `Segment<B>` at construction.
pub struct AnySegment(Box<dyn SegmentOps>);

impl Kind {
    pub fn new_any_segment(self) -> AnySegment {
        AnySegment(match self {
            Kind::Ram => Box::new(Segment::<Ram>::new()),
            Kind::Shm => Box::new(Segment::<Shm>::new()),
            Kind::File => Box::new(Segment::<File>::new()),
        })
    }
}

impl AnySegment {
    pub fn new(kind: Kind) -> Self {
        kind.new_any_segment()
    }

    pub fn downcast_ref<B: HasKind + 'static>(&self) -> Option<&Segment<B>> {
        self.0.as_any().downcast_ref()
    }

    pub fn downcast_mut<B: HasKind + 'static>(&mut self) -> Option<&mut Segment<B>> {
        self.0.as_any_mut().downcast_mut()
    }

    pub fn as_ram(&self) -> Option<&Segment<Ram>> {
        self.downcast_ref()
    }

    pub fn as_ram_mut(&mut self) -> Option<&mut Segment<Ram>> {
        self.downcast_mut()
    }

    pub fn as_shm(&self) -> Option<&Segment<Shm>> {
        self.downcast_ref()
    }

    pub fn as_shm_mut(&mut self) -> Option<&mut Segment<Shm>> {
        self.downcast_mut()
    }

    pub fn as_file(&self) -> Option<&Segment<File>> {
        self.downcast_ref()
    }

    pub fn as_file_mut(&mut self) -> Option<&mut Segment<File>> {
        self.downcast_mut()
    }
}

impl std::ops::Deref for AnySegment {
    type Target = dyn SegmentOps;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl std::ops::DerefMut for AnySegment {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

fn map_open_err<E: fmt::Display>(kind: Kind, err: SegmentOpenError<E>) -> ManagerError {
    match err {
        SegmentOpenError::Backend(e) => ManagerError::Backend {
            scheme: kind.scheme().into(),
            message: e.to_string(),
        },
        SegmentOpenError::Layout(e) => ManagerError::Layout {
            scheme: kind.scheme().into(),
            detail: format!("{e:?}"),
        },
    }
}

fn map_teardown_err<E: fmt::Display>(kind: Kind, err: SegmentTeardownError<E>) -> ManagerError {
    match err {
        SegmentTeardownError::BadIndex => ManagerError::SlabNotInSegment,
        SegmentTeardownError::Backend(e) => ManagerError::Backend {
            scheme: kind.scheme().into(),
            message: e.to_string(),
        },
    }
}

fn backend_err<E: fmt::Display>(kind: Kind, err: E) -> ManagerError {
    ManagerError::Backend {
        scheme: kind.scheme().into(),
        message: err.to_string(),
    }
}

/// URL scheme must match the segment's [`Kind`].
pub fn ensure_url_matches(url: &Url, kind: Kind) -> Result<(), ManagerError> {
    kind.ensure_url(url).map_err(|m| ManagerError::SchemeMismatch {
        url_scheme: m.url_scheme,
        kind: m.kind,
    })
}
