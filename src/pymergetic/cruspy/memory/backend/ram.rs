//! Process-local byte slab (`Vec<u8>`).

use std::fmt;

use super::{HasAccess, HasInfo, HasMapping, Info, OpenMode, State};
use crate::pymergetic::cruspy::utils::url::Url;

/// Build `ram://<host>`.
pub fn build_url(host: impl AsRef<str>) -> Url {
    Url::builder()
        .scheme("ram")
        .host(host.as_ref())
        .build()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RamError {
    WrongScheme,
    HostRequired,
    ModeRequired,
    CapacityRequired,
}

impl fmt::Display for RamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RamError::WrongScheme => write!(f, "url scheme must be ram"),
            RamError::HostRequired => write!(f, "ram url requires a host"),
            RamError::ModeRequired => write!(f, "open mode must be create or attach"),
            RamError::CapacityRequired => write!(f, "capacity required and must be > 0"),
        }
    }
}

impl std::error::Error for RamError {}

pub struct Ram {
    info: Info,
    buf: Vec<u8>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pymergetic::cruspy::memory::segment::{Segment, HEADER_LEN, MAGIC, VERSION};

    #[test]
    fn open_create_and_segment_layout() {
        let url = build_url("heap");
        let seg = Segment::<Ram>::create(&url, Some(4096)).unwrap();
        assert_eq!(seg.backends().len(), 1);
        let ram = seg.backend(0).unwrap();
        assert_eq!(ram.info().open_mode, OpenMode::Create);
        assert_eq!(ram.info().capacity, 4096);
        let h = seg.header(0).unwrap();
        assert_eq!(h.magic, MAGIC);
        assert_eq!(h.version, VERSION);
        assert_eq!(h.offset as usize, HEADER_LEN);
        assert_eq!(h.len as usize, 4096 - HEADER_LEN);
        assert_eq!(seg.arena(0).unwrap().len(), 4096 - HEADER_LEN);
    }

    #[test]
    fn add_second_slab_claims_same_talc() {
        let mut seg = Segment::<Ram>::new();
        seg.add(Ram::create(&build_url("a"), Some(4096)).unwrap())
            .unwrap();
        seg.add(Ram::create(&build_url("b"), Some(8192)).unwrap())
            .unwrap();
        assert_eq!(seg.backends().len(), 2);
        assert_eq!(seg.header(0).unwrap().len as usize, 4096 - HEADER_LEN);
        assert_eq!(seg.header(1).unwrap().len as usize, 8192 - HEADER_LEN);
        assert_eq!(seg.arena(0).unwrap().len(), 4096 - HEADER_LEN);
        assert_eq!(seg.arena(1).unwrap().len(), 8192 - HEADER_LEN);
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
