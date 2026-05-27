//! POSIX shared memory (`shm_open` + `mmap`).

use std::any::Any;
use std::ffi::CString;

use nix::fcntl::OFlag;
use nix::sys::mman::shm_unlink;
use nix::sys::stat::Mode;
use nix::unistd::ftruncate;

use crate::mem::device::info::{Info, InfoData, State};
use crate::mem::io::{OpenMode, Access, Address, Open, Read, Write};
use crate::mem::kind::Kind;
use crate::mem::mapped::MappedRegion;
use crate::utils::url::Url;

/// Build `shm://<host>`.
pub fn build_url(host: impl AsRef<str>) -> Url {
    Url::builder()
        .scheme("shm")
        .host(host.as_ref())
        .build()
}

/// Registration recipe with empty host (set via [`build_url`] + [`.url()`](InfoData::url)).
pub fn named() -> InfoData {
    InfoData::empty(build_url(""))
}

/// Opened POSIX SHM storage.
pub struct Storage {
    info: InfoData,
    cname: CString,
    map: MappedRegion,
}

unsafe impl Send for Storage {}
unsafe impl Sync for Storage {}

impl Open for Storage {
    type Error = nix::Error;

    fn open(mode: OpenMode, url: &Url, len: usize) -> Result<Self, Self::Error> {
        if mode == OpenMode::None {
            return Err(nix::Error::EINVAL);
        }
        let seg = url.host();
        if seg.is_empty() {
            return Err(nix::Error::EINVAL);
        }
        let cname = posix_name_cstr(seg)?;
        let fd = match mode {
            OpenMode::None => unreachable!(),
            OpenMode::Create => nix::sys::mman::shm_open(
                cname.as_c_str(),
                OFlag::O_CREAT | OFlag::O_EXCL | OFlag::O_RDWR,
                Mode::S_IRUSR | Mode::S_IWUSR,
            )?,
            OpenMode::Attach => {
                nix::sys::mman::shm_open(cname.as_c_str(), OFlag::O_RDWR, Mode::empty())?
            }
        };
        ftruncate(&fd, len as i64)?;
        let map = MappedRegion::map_shared(&fd, len)?;
        drop(fd);
        Ok(Self {
            info: InfoData {
                url: url.clone(),
                capacity: len,
                open_mode: mode,
                state: State::Open,
            },
            cname,
            map,
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
        shm_unlink(self.cname.as_c_str()).map_err(io_err)
    }
}

impl Read for Storage {
    fn kind(&self) -> Kind {
        Kind::PosixShm
    }

    fn base(&self) -> *mut u8 {
        self.map.ptr
    }

    fn len(&self) -> usize {
        self.info().capacity
    }
}

impl Write for Storage {}

fn posix_name_cstr(name: &str) -> Result<CString, nix::Error> {
    CString::new(format!("/{}", name)).map_err(|_| nix::Error::EINVAL)
}

fn io_err(e: nix::Error) -> std::io::Error {
    std::io::Error::from_raw_os_error(e as i32)
}

pub fn as_shm(storage: &dyn Write) -> Option<&Storage> {
    (storage as &dyn Any).downcast_ref::<Storage>()
}
