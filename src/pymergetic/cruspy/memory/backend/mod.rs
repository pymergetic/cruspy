//! Memory backends (ram / shm / file).

pub mod file;
pub mod ram;
pub mod shm;

pub use crate::pymergetic::cruspy::io::{HasAccess, HasInfo, HasMapping, Info, OpenMode, State};

/// Opened slab: [`HasInfo`] + [`HasAccess`] + [`HasMapping`].
pub trait Backend: HasInfo + HasAccess + HasMapping {}

impl Backend for ram::Ram {}
impl Backend for shm::Shm {}
impl Backend for file::File {}
