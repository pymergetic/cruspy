//! POSIX shared memory segment.

use crate::pymergetic::cruspy::io::{HasAccess, HasInfo, HasMapping, HasResize, Info, OpenMode};
use crate::pymergetic::cruspy::utils::url::Url;

pub struct Shm {
    info: Info,
}

impl HasInfo for Shm {
    fn info(&self) -> &Info {
        &self.info
    }

    fn info_mut(&mut self) -> &mut Info {
        &mut self.info
    }
}

impl HasAccess for Shm {
    type Error = std::io::Error;

    fn open(_url: &Url, _mode: OpenMode, _capacity: Option<usize>) -> Result<Self, Self::Error> {
        todo!("shm::Shm::open")
    }

    fn close(&mut self) -> Result<(), Self::Error> {
        todo!("shm::Shm::close")
    }

    fn unlink(&mut self) -> Result<(), Self::Error> {
        todo!("shm::Shm::unlink")
    }
}

impl HasMapping for Shm {
    fn bytes(&self) -> &[u8] {
        &[]
    }

    fn bytes_mut(&mut self) -> &mut [u8] {
        &mut []
    }
}

impl HasResize for Shm {
    fn resize(&mut self, _new_capacity: usize) -> Result<(), Self::Error> {
        todo!("shm::Shm::resize")
    }
}
