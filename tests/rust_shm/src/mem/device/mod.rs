//! Memory devices — how bytes are obtained (heap, POSIX SHM, file mmap).
//!
//! Each submodule: **`Storage`** + [`InfoData`](info::InfoData) recipes via `named` / `build_url`;
//! [`Address`](crate::mem::io::Address) + [`Open`](crate::mem::io::Open) / [`Access`](crate::mem::io::Access).

pub mod file;
pub mod info;
pub mod ram;
pub mod register;
pub mod shm;

pub use info::{Info, InfoData};
