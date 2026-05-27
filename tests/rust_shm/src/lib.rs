//! Memory backends demo (RAM / POSIX SHM / file) + snapshot migration.
//!
//! Typed graph uses **relative offsets** (`Off`), not raw pointers.
//! Cross-slab moves are always a **CPU copy** (`export_snapshot` / `import_snapshot`).

mod layout;
pub mod mem;
mod metrics;
mod register;
mod registry;
mod schema;
pub mod utils;

pub use layout::{
    export_snapshot, import_snapshot, migrate, Segment, Off, Ref, MigrateError,
    Snapshot, SNAPSHOT_ABI,
};
pub use mem::device::{Info, InfoData};
pub use mem::io::{segment, Access, Address, File, Open, OpenMode, Read, Write};
pub use mem::kind::Kind;
pub use metrics::{RegistryTotals, Usage, UsageReport};
pub use register::RegisterSpec;
pub use registry::{
    Id, Loc, Locator, Registry, Registered, RegistryError, LocatorRef,
};
pub use schema::{assert_graph, Child, Deep, NestedLayout, Node};
pub use utils::url::{ParseError, Url};

pub const STORAGE_CAPACITY: usize = 4096;

// Back-compat aliases from the first demo iteration.
pub type ShmOff = Off;
pub const SHM_CAPACITY: usize = STORAGE_CAPACITY;
pub type ShmMap = mem::device::shm::Storage;
