//! Memory layer: devices (ram / shm / file), shared mmap helper, IO traits.
//!
//! | Path | Meaning |
//! |------|--------|
//! | [`device::ram`] / [`device::shm`] / [`device::file`] | One device kind each (`Storage` + `InfoData`) |
//! | [`backing`] | [`Url`](crate::utils::url::Url) scheme → [`Kind`] |
//! | [`mapped`] | `mmap(MAP_SHARED)` helper for shm + file |
//! | [`kind`] | [`Kind`] label (ram / posix_shm / file) |
//! | [`io`] | [`Access`] → [`Read`] → [`Write`]; [`File`] for path |

pub mod backing;
pub mod device;
pub mod io;
pub mod kind;
pub mod mapped;

pub use kind::Kind;
pub use io::{segment, Access, Address, File, Open, OpenMode, Read, Write};
