//! Shared [`InfoData`] and [`Info`] for registration recipes and opened slabs.
use crate::mem::io::OpenMode;
use crate::utils::url::Url;

/// Runtime state of the backing in this process.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Not opened yet (registration recipe only).
    Unopened,
    /// Currently open and usable.
    Open,
    /// Was opened, then closed in this process.
    Closed,
}

/// Backing [`Url`] + capacity + open mode + runtime state.
#[derive(Clone, Debug)]
pub struct InfoData {
    pub url: Url,
    pub capacity: usize,
    /// How the backing should be / was opened (`create` vs `attach`).
    pub open_mode: OpenMode,
    /// Runtime state in this process (`open` vs `closed`).
    pub state: State,
}

impl InfoData {
    pub fn empty(url: Url) -> Self {
        Self {
            url,
            capacity: 0,
            open_mode: OpenMode::None,
            state: State::Unopened,
        }
    }

    pub fn url(mut self, url: Url) -> Self {
        self.url = url;
        self
    }

    pub fn capacity(mut self, bytes: usize) -> Self {
        self.capacity = bytes;
        self
    }

    pub fn create(mut self) -> Self {
        self.open_mode = OpenMode::Create;
        self
    }

    pub fn attach(mut self) -> Self {
        self.open_mode = OpenMode::Attach;
        self
    }
}

/// [`InfoData`] access + builders (recipes and opened [`Storage`](crate::mem::device::ram::Storage)).
pub trait Info: Sized {
    fn info(&self) -> &InfoData;
    fn info_mut(&mut self) -> &mut InfoData;

    fn url(mut self, url: Url) -> Self {
        self.info_mut().url = url;
        self
    }

    fn capacity(mut self, bytes: usize) -> Self {
        self.info_mut().capacity = bytes;
        self
    }

    fn create(mut self) -> Self {
        self.info_mut().open_mode = OpenMode::Create;
        self
    }

    fn attach(mut self) -> Self {
        self.info_mut().open_mode = OpenMode::Attach;
        self
    }
}

impl Info for InfoData {
    fn info(&self) -> &InfoData {
        self
    }

    fn info_mut(&mut self) -> &mut InfoData {
        self
    }
}
