//! URL-keyed registry of mem slabs — ids, locators, unified lookup.

use std::collections::HashMap;
use std::fmt;
use crate::layout::{export_snapshot, import_snapshot, Segment, Off, MigrateError};
use crate::utils::url::Url;
use crate::mem::io::{Access, Read, Write};
use crate::metrics::{RegistryTotals, Usage, UsageReport};
use crate::register::RegisterSpec;

/// Handle returned from [`Registry::register`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Registered {
    pub id: Id,
    pub locator: Locator,
}

/// Anything that references a registered locator.
pub trait LocatorRef {
    fn locator_key(&self) -> &str;
}

impl LocatorRef for str {
    fn locator_key(&self) -> &str {
        self
    }
}

impl LocatorRef for Url {
    fn locator_key(&self) -> &str {
        self.as_str()
    }
}

impl LocatorRef for Registered {
    fn locator_key(&self) -> &str {
        self.locator.as_str()
    }
}

impl<T: LocatorRef + ?Sized> LocatorRef for &T {
    fn locator_key(&self) -> &str {
        T::locator_key(self)
    }
}

/// Opaque id (like cruspy `DomainId::low`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Id(pub u64);

/// How this slab is reached outside the registry (generic [`Url`]).
pub type Locator = Url;

/// Address inside a **registered** slab (portable offset + which mem id).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Loc {
    pub mem: Id,
    pub off: Off,
}

#[derive(Debug)]
pub enum RegistryError {
    DuplicateLocator(String),
    UnknownLocator(String),
    UnknownId(Id),
    OpenModeUnset,
    Backend(nix::Error),
    Io(std::io::Error),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateLocator(l) => write!(f, "mem locator already registered: {l}"),
            Self::UnknownLocator(l) => write!(f, "unknown mem locator: {l}"),
            Self::UnknownId(id) => write!(f, "unknown mem id: {}", id.0),
            Self::OpenModeUnset => {
                write!(f, "open mode unset: call .create() or .attach() on the spec")
            }
            Self::Backend(e) => write!(f, "backend error: {e}"),
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for RegistryError {}

impl From<MigrateError> for RegistryError {
    fn from(e: MigrateError) -> Self {
        match e {
            MigrateError::CapacityTooSmall { need, have } => {
                RegistryError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("migrate: need {need} bytes, mapping has {have}"),
                ))
            }
        }
    }
}

struct Entry {
    locator: Locator,
    storage: Box<dyn Write>,
    /// High-water mark for snapshot migrate / future bump allocator.
    used_len: usize,
}

/// Central catalog of all mem slabs in this process.
#[derive(Default)]
pub struct Registry {
    next_id: u64,
    by_locator: HashMap<String, Id>,
    by_id: HashMap<Id, Entry>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    fn alloc_id(&mut self) -> Id {
        let id = Id(self.next_id);
        self.next_id += 1;
        id
    }

    fn insert(
        &mut self,
        locator: Locator,
        storage: Box<dyn Write>,
    ) -> Result<Id, RegistryError> {
        if self.by_locator.contains_key(locator.as_str()) {
            return Err(RegistryError::DuplicateLocator(locator.as_str().to_owned()));
        }
        let id = self.alloc_id();
        self.by_locator.insert(locator.as_str().to_owned(), id);
        self.by_id.insert(
            id,
            Entry {
                locator,
                storage,
                used_len: 0,
            },
        );
        Ok(id)
    }

    /// Register any device recipe ([`InfoData`](crate::mem::device::info::InfoData) + URL scheme).
    pub fn register<S: RegisterSpec>(&mut self, spec: S) -> Result<Registered, RegistryError> {
        let locator = spec.locator().clone();
        if self.by_locator.contains_key(locator.as_str()) {
            return Err(RegistryError::DuplicateLocator(locator.as_str().to_owned()));
        }
        let (storage, _opened_locator) = spec.open()?;
        let id = self.insert(locator.clone(), storage)?;
        Ok(Registered { id, locator })
    }

    pub fn id<S: LocatorRef + ?Sized>(&self, locator: &S) -> Result<Id, RegistryError> {
        self.by_locator
            .get(locator.locator_key())
            .copied()
            .ok_or_else(|| RegistryError::UnknownLocator(locator.locator_key().to_owned()))
    }

    pub fn locator(&self, id: Id) -> Result<&Locator, RegistryError> {
        self.by_id
            .get(&id)
            .map(|e| &e.locator)
            .ok_or(RegistryError::UnknownId(id))
    }

    pub fn entries(&self) -> impl Iterator<Item = (Id, &Locator)> {
        self.by_id
            .iter()
            .map(|(id, e)| (*id, &e.locator))
    }

    pub fn read<S: LocatorRef + ?Sized>(&self, locator: &S) -> Result<&dyn Read, RegistryError> {
        let id = self.id(locator)?;
        Ok(self.by_id.get(&id).expect("id map consistent").storage.as_ref())
    }

    pub fn write<S: LocatorRef + ?Sized>(
        &mut self,
        locator: &S,
    ) -> Result<&mut dyn Write, RegistryError> {
        let id = self.id(locator)?;
        Ok(self.by_id.get_mut(&id).expect("id map consistent").storage.as_mut())
    }

    pub fn access<S: LocatorRef + ?Sized>(&self, locator: &S) -> Result<&dyn Access, RegistryError> {
        let id = self.id(locator)?;
        Ok(self.by_id.get(&id).expect("id map consistent").storage.as_ref())
    }

    pub fn access_mut<S: LocatorRef + ?Sized>(
        &mut self,
        locator: &S,
    ) -> Result<&mut dyn Access, RegistryError> {
        let id = self.id(locator)?;
        Ok(self.by_id.get_mut(&id).expect("id map consistent").storage.as_mut())
    }

    pub fn storage_mut<S: LocatorRef + ?Sized>(
        &mut self,
        locator: &S,
    ) -> Result<&mut Box<dyn Write>, RegistryError> {
        let id = self.id(locator)?;
        Ok(&mut self.by_id.get_mut(&id).expect("id map consistent").storage)
    }

    pub fn segment<S: LocatorRef + ?Sized>(&self, locator: &S) -> Result<Segment<'_>, RegistryError> {
        Ok(crate::mem::io::segment(self.read(locator)?))
    }

    /// Record how many bytes are in use (for migrate / allocator high-water).
    pub fn set_used_len<S: LocatorRef + ?Sized>(
        &mut self,
        locator: &S,
        used_len: usize,
    ) -> Result<(), RegistryError> {
        let id = self.id(locator)?;
        self.by_id.get_mut(&id).expect("id map consistent").used_len = used_len;
        Ok(())
    }

    pub fn used_len<S: LocatorRef + ?Sized>(&self, locator: &S) -> Result<usize, RegistryError> {
        let id = self.id(locator)?;
        Ok(self.by_id.get(&id).expect("id map consistent").used_len)
    }

    pub fn flush<S: LocatorRef + ?Sized>(&mut self, locator: &S) -> Result<(), RegistryError> {
        self.write(locator)?.flush().map_err(RegistryError::Io)
    }

    /// CPU copy between two **registered** slabs by locator.
    ///
    /// Uses an owned [`Snapshot`] between export and import so we never hold two
    /// mapping borrows from the internal map at once.
    pub fn migrate<F: LocatorRef + ?Sized, T: LocatorRef + ?Sized>(
        &mut self,
        from: &F,
        to: &T,
    ) -> Result<(), RegistryError> {
        let snap = self.export(from)?;
        let used = snap.used_len;
        self.import(to, &snap)?;
        self.set_used_len(to, used)
    }

    pub fn export<S: LocatorRef + ?Sized>(&self, locator: &S) -> Result<crate::layout::Snapshot, RegistryError> {
        let used = self.used_len(locator)?;
        Ok(export_snapshot(self.read(locator)?, used))
    }

    pub fn import<S: LocatorRef + ?Sized>(
        &mut self,
        locator: &S,
        snap: &crate::layout::Snapshot,
    ) -> Result<(), RegistryError> {
        import_snapshot(self.write(locator)?, snap).map_err(RegistryError::from)
    }

    /// Backing file path (file storage only).
    pub fn file_path<S: LocatorRef + ?Sized>(&self, locator: &S) -> Result<std::path::PathBuf, RegistryError> {
        let id = self.id(locator)?;
        let entry = self.by_id.get(&id).ok_or(RegistryError::UnknownId(id))?;
        crate::mem::device::file::as_file(entry.storage.as_ref())
            .map(|f| crate::mem::io::File::path(f).to_path_buf())
            .ok_or_else(|| {
                RegistryError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "file_path: not a file backend",
                ))
            })
    }

    /// `Access::close` on a registered symbol (shm unlink, file flush, ram no-op).
    pub fn close<S: LocatorRef + ?Sized>(&mut self, locator: &S) -> Result<(), RegistryError> {
        self.access_mut(locator)?.close().map_err(RegistryError::Io)
    }

    /// Drop a POSIX SHM name from the namespace (SHM only; prefer [`Self::close`]).
    pub fn unlink_posix_shm<S: LocatorRef + ?Sized>(&mut self, locator: &S) -> Result<(), RegistryError> {
        let id = self.id(locator)?;
        let entry = self.by_id.get(&id).ok_or(RegistryError::UnknownId(id))?;
        if entry.storage.kind() != crate::mem::kind::Kind::PosixShm {
            return Err(RegistryError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "unlink_posix_shm: not a posix_shm backend",
            )));
        }
        self.close(locator)
    }

    /// Segment view for a [`Loc`] (use `.at(loc.off)` on the segment for typed access).
    pub fn segment_at(&self, loc: Loc) -> Result<Segment<'_>, RegistryError> {
        let entry = self
            .by_id
            .get(&loc.mem)
            .ok_or(RegistryError::UnknownId(loc.mem))?;
        Ok(crate::mem::io::segment(entry.storage.as_ref()))
    }

    /// Usage metrics for one registered slab.
    pub fn usage<S: LocatorRef + ?Sized>(&self, locator: &S) -> Result<Usage, RegistryError> {
        let id = self.id(locator)?;
        let entry = self.by_id.get(&id).ok_or(RegistryError::UnknownId(id))?;
        let read = entry.storage.as_ref();
        Ok(Usage {
            id,
            kind: read.kind(),
            locator: entry.locator.clone(),
            capacity: read.len(),
            used_len: entry.used_len,
        })
    }

    /// Usage metrics for every registered slab (stable id order).
    pub fn usages(&self) -> Vec<Usage> {
        let mut ids: Vec<_> = self.by_id.keys().copied().collect();
        ids.sort_by_key(|id| id.0);
        ids.into_iter()
            .filter_map(|id| {
                let entry = self.by_id.get(&id)?;
                let read = entry.storage.as_ref();
                Some(Usage {
                    id,
                    kind: read.kind(),
                    locator: entry.locator.clone(),
                    capacity: read.len(),
                    used_len: entry.used_len,
                })
            })
            .collect()
    }

    /// Rolled-up capacity / used bytes across all slabs.
    pub fn totals(&self) -> RegistryTotals {
        let slabs = self.usages();
        RegistryTotals {
            slab_count: slabs.len(),
            total_capacity: slabs.iter().map(|u| u.capacity).sum(),
            total_used: slabs.iter().map(|u| u.used_len).sum(),
        }
    }

    /// Printable usage report (see `src/bin/mem_report.rs`).
    pub fn usage_report(&self) -> UsageReport {
        UsageReport {
            slabs: self.usages(),
            totals: self.totals(),
        }
    }
}
