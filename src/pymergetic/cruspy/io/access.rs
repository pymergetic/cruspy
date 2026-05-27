//! [`HasAccess`] — open resources with [`OpenMode`].

use crate::pymergetic::cruspy::utils::url::Url;

/// How storage is opened (same verbs on ram / shm / file).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OpenMode {
    /// Not set until opened.
    None,
    Create,
    Attach,
}

/// Types opened from a [`Url`] with [`OpenMode::Create`] or [`OpenMode::Attach`].
pub trait HasAccess: Sized {
    type Error;

    fn create(url: &Url, capacity: Option<usize>) -> Result<Self, Self::Error> {
        Self::open(url, OpenMode::Create, capacity)
    }

    fn attach(url: &Url, capacity: Option<usize>) -> Result<Self, Self::Error> {
        Self::open(url, OpenMode::Attach, capacity)
    }

    fn open(url: &Url, mode: OpenMode, capacity: Option<usize>) -> Result<Self, Self::Error>;

    fn close(&mut self) -> Result<(), Self::Error>;

    fn unlink(&mut self) -> Result<(), Self::Error>;
}
