//! Process-local byte slab (`Vec<u8>`).

use std::fmt;

use crate::pymergetic::cruspy::io::{
    HasAccess, HasInfo, HasKind, HasMapping, HasResize, Info, Kind, OpenMode, State,
};
use crate::pymergetic::cruspy::utils::url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RamError {
    WrongScheme,
    HostRequired,
    ModeRequired,
    CapacityRequired,
    NotOpen,
}

impl fmt::Display for RamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RamError::WrongScheme => write!(f, "url scheme must be ram"),
            RamError::HostRequired => write!(f, "ram url requires a host"),
            RamError::ModeRequired => write!(f, "open mode must be create or attach"),
            RamError::CapacityRequired => write!(f, "capacity required and must be > 0"),
            RamError::NotOpen => write!(f, "ram backend is not open"),
        }
    }
}

impl std::error::Error for RamError {}

pub struct Ram {
    info: Info,
    buf: Vec<u8>,
}

impl Ram {
    /// Unopened slab; use [`HasAccess::open`] or [`HasAccess::create`].
    pub fn new() -> Self {
        Self {
            info: Info::empty(Self::build_url("_")),
            buf: Vec::new(),
        }
    }

    /// Build `ram://<host>`.
    pub fn build_url(host: impl AsRef<str>) -> Url {
        Url::builder()
            .scheme("ram")
            .host(host.as_ref())
            .build()
    }
}

impl HasKind for Ram {
    const KIND: Kind = Kind::Ram;
}

impl HasInfo for Ram {
    fn info(&self) -> &Info {
        &self.info
    }

    fn info_mut(&mut self) -> &mut Info {
        &mut self.info
    }
}

impl HasAccess for Ram {
    type Error = RamError;

    fn open(url: &Url, mode: OpenMode, capacity: Option<usize>) -> Result<Self, Self::Error> {
        if url.scheme() != "ram" {
            return Err(RamError::WrongScheme);
        }
        if url.host().is_empty() {
            return Err(RamError::HostRequired);
        }
        if mode == OpenMode::None {
            return Err(RamError::ModeRequired);
        }
        let capacity = capacity.filter(|&n| n > 0).ok_or(RamError::CapacityRequired)?;

        Ok(Self {
            info: Info {
                url: url.clone(),
                capacity,
                open_mode: mode,
                state: State::Open,
            },
            buf: vec![0u8; capacity],
        })
    }

    fn close(&mut self) -> Result<(), Self::Error> {
        self.info.state = State::Closed;
        self.buf.clear();
        self.buf.shrink_to_fit();
        Ok(())
    }

    fn unlink(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl HasMapping for Ram {
    fn bytes(&self) -> &[u8] {
        &self.buf
    }

    fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.buf
    }
}

impl HasResize for Ram {
    fn resize(&mut self, new_capacity: usize) -> Result<(), Self::Error> {
        if self.info.state != State::Open {
            return Err(RamError::NotOpen);
        }
        if new_capacity == 0 {
            return Err(RamError::CapacityRequired);
        }
        self.buf.resize(new_capacity, 0);
        self.info.capacity = new_capacity;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pymergetic::cruspy::io::Kind;
    use crate::pymergetic::cruspy::memory::segment::{Segment, HEADER_LEN, MAGIC, VERSION};

    #[test]
    fn open_create_and_segment_layout() {
        let url = Ram::build_url("heap");
        let mut seg = Segment::new(Kind::Ram);
        seg.create(&url, Some(4096)).unwrap();
        assert_eq!(seg.backends().len(), 1);
        let slab = seg.backend(0).unwrap();
        assert_eq!(slab.info().open_mode, OpenMode::Create);
        assert_eq!(slab.info().capacity, 4096);
        let h = seg.header(0).unwrap();
        assert_eq!(h.magic, MAGIC);
        assert_eq!(h.version, VERSION);
        assert_eq!(h.header_len as usize, HEADER_LEN);
        assert_eq!(h.offset as usize, HEADER_LEN);
        assert_eq!(h.len as usize, 4096 - HEADER_LEN);
        assert_eq!(seg.arena(0).unwrap().len(), 4096 - HEADER_LEN);
    }

    #[test]
    fn add_second_slab_claims_same_talc() {
        let mut seg = Segment::new(Kind::Ram);
        seg.create(&Ram::build_url("a"), Some(4096)).unwrap();
        seg.create(&Ram::build_url("b"), Some(8192)).unwrap();
        assert_eq!(seg.backends().len(), 2);
        assert_eq!(seg.size_all(), (4096 - HEADER_LEN) + (8192 - HEADER_LEN));
        assert_eq!(seg.size_raw_all(), 4096 + 8192);
        assert_eq!(seg.header(0).unwrap().len as usize, 4096 - HEADER_LEN);
        assert_eq!(seg.header(1).unwrap().len as usize, 8192 - HEADER_LEN);
        assert_eq!(seg.arena(0).unwrap().len(), 4096 - HEADER_LEN);
        assert_eq!(seg.arena(1).unwrap().len(), 8192 - HEADER_LEN);
    }

    #[test]
    fn resize_grows_and_shrinks_buf() {
        let mut ram = Ram::create(&Ram::build_url("heap"), Some(4096)).unwrap();
        HasResize::resize(&mut ram, 8192).unwrap();
        assert_eq!(HasInfo::info(&ram).capacity, 8192);
        assert_eq!(HasMapping::bytes(&ram).len(), 8192);
        HasResize::resize(&mut ram, 2048).unwrap();
        assert_eq!(HasMapping::bytes(&ram).len(), 2048);
    }

    #[test]
    fn add_rejects_uninitialized_mapping() {
        let ram = Ram::create(&Ram::build_url("fresh"), Some(4096)).unwrap();
        let mut seg = Segment::new(Kind::Ram);
        assert!(matches!(
            seg.add(Box::new(ram)),
            Err(crate::pymergetic::cruspy::memory::segment::SegmentError::BadHeader)
        ));
    }

    #[test]
    fn rejects_bad_url() {
        let url = Url::builder().scheme("shm").host("x").build();
        assert!(matches!(
            Ram::create(&url, Some(1024)),
            Err(RamError::WrongScheme)
        ));
    }
}
