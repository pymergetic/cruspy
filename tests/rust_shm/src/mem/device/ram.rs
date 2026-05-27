//! In-process heap storage (`Vec<u8>`).

use crate::mem::device::info::{Info, InfoData, State};
use crate::mem::io::{OpenMode, Access, Address, Open, Read, Write};
use crate::mem::kind::Kind;
use crate::utils::url::Url;

/// Build `ram://<host>`.
pub fn build_url(host: impl AsRef<str>) -> Url {
    Url::builder()
        .scheme("ram")
        .host(host.as_ref())
        .build()
}

/// Registration recipe with empty host (set via [`build_url`] + [`.url()`](InfoData::url)).
pub fn named() -> InfoData {
    InfoData::empty(build_url(""))
}

/// Opened RAM storage.
pub struct Storage {
    info: InfoData,
    #[allow(dead_code)] // owns allocation; `base` points into it
    buf: Vec<u8>,
    base: *mut u8,
}

impl Open for Storage {
    type Error = std::convert::Infallible;

    fn open(mode: OpenMode, url: &Url, len: usize) -> Result<Self, Self::Error> {
        assert!(mode != OpenMode::None, "ram open mode must be create or attach");
        if url.host().is_empty() {
            panic!("ram URL needs host");
        }
        let mut buf = vec![0u8; len];
        let base = buf.as_mut_ptr();
        Ok(Self {
            info: InfoData {
                url: url.clone(),
                capacity: len,
                open_mode: mode,
                state: State::Open,
            },
            buf,
            base,
        })
    }
}

impl Info for Storage {
    fn info(&self) -> &InfoData {
        &self.info
    }

    fn info_mut(&mut self) -> &mut InfoData {
        &mut self.info
    }
}

impl Address for Storage {
    fn url(&self) -> &Url {
        &self.info().url
    }
}

impl Access for Storage {
    fn open_mode(&self) -> OpenMode {
        self.info().open_mode
    }

    fn close(&mut self) -> std::io::Result<()> {
        self.info_mut().state = State::Closed;
        Ok(())
    }
}

impl Read for Storage {
    fn kind(&self) -> Kind {
        Kind::Ram
    }

    fn base(&self) -> *mut u8 {
        self.base
    }

    fn len(&self) -> usize {
        self.info().capacity
    }
}

impl Write for Storage {}
