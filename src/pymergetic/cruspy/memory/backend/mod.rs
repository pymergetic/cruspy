//! Memory backends (`ram` / `shm` / `file`).
//!
//! | [`Kind`](crate::pymergetic::cruspy::io::Kind) | Type | [`HasKind::KIND`](crate::pymergetic::cruspy::io::HasKind::KIND) |
//! |---|---|---|
//! | `Ram` | [`Ram`] | `Kind::Ram` |
//! | `Shm` | [`Shm`] | `Kind::Shm` |
//! | `File` | [`File`] | `Kind::File` |
//!
//! Construct: [`Kind::create`](factory::Kind::create), [`Kind::create_from_scheme`](factory::Kind::create_from_scheme),
//! [`Kind::create_from_url`](factory::Kind::create_from_url) → [`HasSlab`](crate::pymergetic::cruspy::io::HasSlab).
//! Each backend: [`HasKind`] + [`HasAccess`](crate::pymergetic::cruspy::io::HasAccess) stack → [`HasSlab`] via blanket impl.

mod factory;
pub mod file;
pub mod ram;
pub mod shm;

pub use factory::UnknownScheme;
pub use file::File;
pub use ram::Ram;
pub use shm::Shm;
