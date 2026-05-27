//! Backing metadata ([`Info`]) and [`HasInfo`] for recipes and opened resources.

use super::access::OpenMode;
use super::state::{HasState, State};

pub use crate::pymergetic::cruspy::utils::url::Url;

/// Backing [`Url`] + capacity + open mode + runtime state.
#[derive(Clone, Debug)]
pub struct Info {
    pub url: Url,
    pub capacity: usize,
    /// How the backing should be / was opened (`create` vs `attach`).
    pub open_mode: OpenMode,
    /// Runtime state in this process (`open` vs `closed`).
    pub state: State,
}

impl Info {
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

    pub fn open_mode(mut self, mode: OpenMode) -> Self {
        self.open_mode = mode;
        self
    }
}

impl HasState for Info {
    fn state(&self) -> State {
        self.state
    }

    fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}

/// Types that carry [`Info`] (recipes and opened backends).
pub trait HasInfo: Sized {
    fn info(&self) -> &Info;
    fn info_mut(&mut self) -> &mut Info;

    fn url(mut self, url: Url) -> Self {
        self.info_mut().url = url;
        self
    }

    fn capacity(mut self, bytes: usize) -> Self {
        self.info_mut().capacity = bytes;
        self
    }

    fn open_mode(mut self, mode: OpenMode) -> Self {
        self.info_mut().open_mode = mode;
        self
    }

    fn state(mut self, state: State) -> Self {
        *HasState::state_mut(self.info_mut()) = state;
        self
    }
}
