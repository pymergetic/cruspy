//! File-backed storage (`open` + shared `mmap`).

use std::any::Any;
use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};

use nix::unistd::ftruncate;

use crate::mem::device::info::{Info, InfoData, State};
use crate::mem::io::{OpenMode, Access, Address, File as FileIo, Open, Read, Write};
use crate::mem::kind::Kind;
use crate::mem::mapped::MappedRegion;
use crate::utils::url::Url;

/// Build `file://<path>`.
pub fn build_url(path: impl AsRef<Path>) -> Url {
    let path = path.as_ref();
    let path_str = path.to_string_lossy();
    let path_str = if path_str.starts_with('/') {
        path_str.into_owned()
    } else {
        format!("/{path_str}")
    };
    Url::builder().scheme("file").path(path_str).build()
}

/// Registration recipe with empty path (set via [`build_url`] + [`.url()`](InfoData::url)).
pub fn named() -> InfoData {
    InfoData::empty(build_url(PathBuf::new()))
}

/// Opened file storage.
pub struct Storage {
    info: InfoData,
    file: std::fs::File,
    map: MappedRegion,
}

impl Open for Storage {
    type Error = io::Error;

    fn open(mode: OpenMode, url: &Url, len: usize) -> Result<Self, Self::Error> {
        if mode == OpenMode::None {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "open mode unset",
            ));
        }
        if !url.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "expected file:// URL",
            ));
        }
        let path = url.file_path().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "file URL missing path")
        })?;
        let file = match mode {
            OpenMode::None => unreachable!(),
            OpenMode::Create => OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)?,
            OpenMode::Attach => OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)?,
        };
        ftruncate(&file, len as i64).map_err(|e| io::Error::from_raw_os_error(e as i32))?;
        let map = MappedRegion::map_shared(&file, len)
            .map_err(|e| io::Error::from_raw_os_error(e as i32))?;
        Ok(Self {
            info: InfoData {
                url: url.clone(),
                capacity: len,
                open_mode: mode,
                state: State::Open,
            },
            file,
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

    fn close(&mut self) -> io::Result<()> {
        self.info_mut().state = State::Closed;
        self.flush()
    }
}

impl Read for Storage {
    fn kind(&self) -> Kind {
        Kind::File
    }

    fn base(&self) -> *mut u8 {
        self.map.ptr
    }

    fn len(&self) -> usize {
        self.info().capacity
    }
}

impl Write for Storage {
    fn flush(&self) -> io::Result<()> {
        self.file.sync_all()
    }
}

impl FileIo for Storage {
    fn path(&self) -> &Path {
        self.info()
            .url
            .file_path()
            .expect("file storage url has path")
    }
}

pub fn as_file(storage: &dyn Write) -> Option<&dyn FileIo> {
    (storage as &dyn Any)
        .downcast_ref::<Storage>()
        .map(|s| s as &dyn FileIo)
}
