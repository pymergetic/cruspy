//! Runtime state of a backing in this process.

/// Whether the backing is open, closed, or not yet opened (recipe only).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Not opened yet (registration recipe only).
    Unopened,
    /// Currently open and usable.
    Open,
    /// Was opened, then closed in this process.
    Closed,
}

/// Types that expose runtime open/closed state.
pub trait HasState {
    fn state(&self) -> State;
    fn state_mut(&mut self) -> &mut State;

    fn is_unopened(&self) -> bool {
        self.state() == State::Unopened
    }

    fn is_open(&self) -> bool {
        self.state() == State::Open
    }

    fn is_closed(&self) -> bool {
        self.state() == State::Closed
    }
}
