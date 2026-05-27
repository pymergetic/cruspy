//! Where a slab lives — [`Url`] is the key; `name` / `path` are per backend.
//!
use crate::utils::url::Url;

pub trait Address {
    /// Primary locator (registry [`Locator`](crate::registry::Locator) = this).
    fn url(&self) -> &Url;}
