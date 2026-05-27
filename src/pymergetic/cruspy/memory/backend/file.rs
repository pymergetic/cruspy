//! File-mapped byte slab.

use crate::pymergetic::cruspy::io::{HasAccess, HasInfo, HasMapping, HasResize, Info, OpenMode};
use crate::pymergetic::cruspy::utils::url::Url;

pub struct File {
    info: Info,
}

impl HasInfo for File {
    fn info(&self) -> &Info {
        &self.info
    }

    fn info_mut(&mut self) -> &mut Info {
        &mut self.info
    }
}

impl HasAccess for File {
    type Error = std::io::Error;

    fn open(_url: &Url, _mode: OpenMode, _capacity: Option<usize>) -> Result<Self, Self::Error> {
        todo!("file::File::open")
    }

    fn close(&mut self) -> Result<(), Self::Error> {
        todo!("file::File::close")
    }

    fn unlink(&mut self) -> Result<(), Self::Error> {
        todo!("file::File::unlink")
    }
}

impl HasMapping for File {
    fn bytes(&self) -> &[u8] {
        &[]
    }

    fn bytes_mut(&mut self) -> &mut [u8] {
        &mut []
    }
}

impl HasResize for File {
    fn resize(&mut self, _new_capacity: usize) -> Result<(), Self::Error> {
        todo!("file::File::resize")
    }
}
