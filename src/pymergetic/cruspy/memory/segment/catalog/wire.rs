//! Generic pinned catalog blob: FourCC header + fixed-size rows.
//!
//! Every catalog chunk uses one wire header (`magic`, `version`, `count`, `capacity`,
//! `next_offset`, `next_len`). Tail chunks set `next_offset` / `next_len` to zero.

use std::marker::PhantomData;

use crate::pymergetic::cruspy::memory::types::TypeError;

/// Catalog wire header: `magic`, `version`, `count`, `capacity`, `next_offset`, `next_len`.
pub const CATALOG_HEADER_LEN: usize = 24;

/// Identifies a catalog blob on disk (magic FourCC + schema version).
pub trait CatalogKind {
    const MAGIC: u32;
    const VERSION: u32;
    const DEFAULT_CAPACITY: u32;
    type Row: CatalogRow;
}

/// Fixed-size row inside a [`Catalog`].
pub trait CatalogRow: Clone {
    fn row_len() -> usize;
    fn encode_into(&self, dst: &mut [u8]) -> Result<(), TypeError>;
    fn decode_from(src: &[u8]) -> Result<Self, TypeError>
    where
        Self: Sized;
    fn validate(&self) -> Result<(), TypeError> {
        let _ = self;
        Ok(())
    }
}

/// In-memory view of a reserved catalog table in talc.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalog<K: CatalogKind> {
    pub rows: Vec<K::Row>,
    pub capacity: u32,
    /// Arena-relative offset to the next catalog chunk (`0` when [`Self::next_len`] is `0`).
    pub next_offset: u32,
    /// Reserved byte length of the next catalog chunk (`0` = none).
    pub next_len: u32,
    _kind: PhantomData<fn() -> K>,
}

impl<K: CatalogKind> Catalog<K> {
    pub fn with_capacity(capacity: u32) -> Self {
        Self {
            rows: Vec::new(),
            capacity,
            next_offset: 0,
            next_len: 0,
            _kind: PhantomData,
        }
    }

    pub fn new(rows: Vec<K::Row>) -> Self {
        let capacity = rows.len().try_into().unwrap_or(u32::MAX);
        Self {
            rows,
            capacity,
            next_offset: 0,
            next_len: 0,
            _kind: PhantomData,
        }
    }

    pub fn count(&self) -> usize {
        self.rows.len()
    }

    pub fn slots_remaining(&self) -> usize {
        self.capacity as usize - self.rows.len()
    }

    pub fn has_next(&self) -> bool {
        self.next_len > 0
    }

    pub fn encoded_len_used(count: usize) -> usize {
        CATALOG_HEADER_LEN + count * K::Row::row_len()
    }

    pub fn encoded_len_reserved(capacity: usize) -> usize {
        CATALOG_HEADER_LEN + capacity * K::Row::row_len()
    }

    pub fn used_len(&self) -> usize {
        Self::encoded_len_used(self.rows.len())
    }

    pub fn allocated_len(&self) -> usize {
        Self::encoded_len_reserved(self.capacity as usize)
    }

    pub(crate) fn from_rows_capacity_next(
        rows: Vec<K::Row>,
        capacity: u32,
        next_offset: u32,
        next_len: u32,
    ) -> Self {
        Self {
            rows,
            capacity,
            next_offset,
            next_len,
            _kind: PhantomData,
        }
    }

    pub fn append(&mut self, row: K::Row) -> Result<u32, TypeError> {
        if self.rows.len() >= self.capacity as usize {
            return Err(TypeError::CapacityExceeded);
        }
        row.validate()?;
        let index = u32::try_from(self.rows.len()).map_err(|_| TypeError::CapacityExceeded)?;
        self.rows.push(row);
        Ok(index)
    }

    pub fn write_into(&self, dst: &mut [u8]) -> Result<(), TypeError> {
        if self.rows.len() > self.capacity as usize {
            return Err(TypeError::CapacityExceeded);
        }
        let need = self.allocated_len();
        if dst.len() < need {
            return Err(TypeError::OutOfBounds);
        }
        dst[0..4].copy_from_slice(&K::MAGIC.to_le_bytes());
        dst[4..8].copy_from_slice(&K::VERSION.to_le_bytes());
        dst[8..12].copy_from_slice(&(self.rows.len() as u32).to_le_bytes());
        dst[12..16].copy_from_slice(&self.capacity.to_le_bytes());
        dst[16..20].copy_from_slice(&self.next_offset.to_le_bytes());
        dst[20..24].copy_from_slice(&self.next_len.to_le_bytes());
        let row_len = K::Row::row_len();
        let mut off = CATALOG_HEADER_LEN;
        for row in &self.rows {
            row.encode_into(&mut dst[off..off + row_len])?;
            off += row_len;
        }
        Ok(())
    }

    pub fn read_from(src: &[u8]) -> Result<Self, TypeError> {
        if src.len() < CATALOG_HEADER_LEN {
            return Err(TypeError::OutOfBounds);
        }
        let magic = u32::from_le_bytes([src[0], src[1], src[2], src[3]]);
        let version = u32::from_le_bytes([src[4], src[5], src[6], src[7]]);
        if magic != K::MAGIC || version != K::VERSION {
            return Err(TypeError::BadHeader);
        }
        let count = u32::from_le_bytes([src[8], src[9], src[10], src[11]]) as usize;
        let capacity = u32::from_le_bytes([src[12], src[13], src[14], src[15]]);
        let next_offset = u32::from_le_bytes([src[16], src[17], src[18], src[19]]);
        let next_len = u32::from_le_bytes([src[20], src[21], src[22], src[23]]);
        let capacity_usize = capacity as usize;
        if count > capacity_usize {
            return Err(TypeError::BadHeader);
        }
        let need = CATALOG_HEADER_LEN + capacity_usize * K::Row::row_len();
        if src.len() < need {
            return Err(TypeError::OutOfBounds);
        }
        let row_len = K::Row::row_len();
        let mut rows = Vec::with_capacity(count);
        let mut off = CATALOG_HEADER_LEN;
        for _ in 0..count {
            rows.push(K::Row::decode_from(&src[off..off + row_len])?);
            off += row_len;
        }
        Ok(Self {
            rows,
            capacity,
            next_offset,
            next_len,
            _kind: PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pymergetic::cruspy::memory::segment::catalog::metatype::MetaTypeCatalogKind;
    use crate::pymergetic::cruspy::memory::types::{FlexString, MetaType};

    #[test]
    fn header_next_roundtrip() {
        let row = MetaType::from_type::<FlexString>().to_header();
        let mut cat = Catalog::<MetaTypeCatalogKind>::with_capacity(4);
        cat.append(row).unwrap();
        cat.next_offset = 8192;
        cat.next_len = 4096;
        let mut buf = vec![0u8; cat.allocated_len()];
        cat.write_into(&mut buf).unwrap();
        let decoded = Catalog::<MetaTypeCatalogKind>::read_from(&buf).unwrap();
        assert_eq!(decoded.next_offset, 8192);
        assert_eq!(decoded.next_len, 4096);
    }

    #[test]
    fn tail_has_zero_next() {
        let cat = Catalog::<MetaTypeCatalogKind>::with_capacity(4);
        let mut buf = vec![0u8; cat.allocated_len()];
        cat.write_into(&mut buf).unwrap();
        let decoded = Catalog::<MetaTypeCatalogKind>::read_from(&buf).unwrap();
        assert!(!decoded.has_next());
    }
}
