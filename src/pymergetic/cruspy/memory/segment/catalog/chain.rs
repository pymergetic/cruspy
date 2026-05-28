//! Linked catalog chains: load, flatten, append with automatic chunk growth.

use crate::pymergetic::cruspy::memory::segment::{Segment, SegmentError};
use crate::pymergetic::cruspy::memory::types::TypeError;

use super::pin::PinnedCatalog;
use super::primary::{blob, blob_mut};
use super::wire::{Catalog, CatalogKind, CatalogRow};

/// One pinned catalog chunk in the primary arena.
#[derive(Clone)]
pub(crate) struct CatalogChunk<K: CatalogKind> {
    pub offset: u32,
    pub len: u32,
    pub catalog: Catalog<K>,
}

pub(crate) fn load_chain<K: CatalogKind>(
    segment: &Segment,
    head_offset: u32,
    head_len: u32,
) -> Result<Vec<CatalogChunk<K>>, SegmentError>
where
    K::Row: CatalogRow,
{
    let mut chunks = Vec::new();
    let mut offset = head_offset;
    let mut len = head_len;
    loop {
        if len == 0 {
            break;
        }
        let bytes = blob(segment, offset, len)?;
        let catalog = Catalog::<K>::read_from(bytes).map_err(super::map_type_err)?;
        let next_offset = catalog.next_offset;
        let next_len = catalog.next_len;
        chunks.push(CatalogChunk {
            offset,
            len,
            catalog,
        });
        if next_len == 0 {
            break;
        }
        offset = next_offset;
        len = next_len;
    }
    Ok(chunks)
}

pub(crate) fn flatten_chain<K: CatalogKind>(chunks: &[CatalogChunk<K>]) -> Catalog<K>
where
    K::Row: CatalogRow,
{
    let total_capacity: u32 = chunks.iter().map(|c| c.catalog.capacity).sum();
    let mut rows = Vec::new();
    for chunk in chunks {
        rows.extend_from_slice(&chunk.catalog.rows);
    }
    Catalog::from_rows_capacity_next(rows, total_capacity, 0, 0)
}

fn write_chunk<K: CatalogKind>(
    segment: &mut Segment,
    chunk: &CatalogChunk<K>,
) -> Result<(), SegmentError>
where
    K::Row: CatalogRow,
{
    let bytes = blob_mut(segment, chunk.offset, chunk.len)?;
    chunk.catalog.write_into(bytes).map_err(super::map_type_err)
}

fn grow_chain<K, C>(
    segment: &mut Segment,
    chunks: &mut Vec<CatalogChunk<K>>,
    build_extension: impl FnOnce(u32) -> Result<C, SegmentError>,
) -> Result<(), SegmentError>
where
    K: CatalogKind,
    K::Row: CatalogRow,
    C: PinnedCatalog,
{
    let cap = chunks
        .last()
        .ok_or(SegmentError::BadHeader)?
        .catalog
        .capacity;
    let empty = build_extension(cap)?;
    let (next_offset, next_len) = segment.pin_catalog_on_primary(&empty)?;
    let mut tail = chunks.pop().ok_or(SegmentError::BadHeader)?;
    tail.catalog.next_offset = next_offset;
    tail.catalog.next_len = next_len;
    write_chunk(segment, &tail)?;
    chunks.push(tail);
    chunks.push(CatalogChunk {
        offset: next_offset,
        len: next_len,
        catalog: Catalog::<K>::read_from(blob(segment, next_offset, next_len)?)
            .map_err(super::map_type_err)?,
    });
    Ok(())
}

/// Append a row to a catalog chain; allocates and links a new chunk when the tail is full.
pub(crate) fn append_to_chain<K, C>(
    segment: &mut Segment,
    head_offset: u32,
    head_len: u32,
    row: K::Row,
    build_extension: impl Fn(u32) -> Result<C, SegmentError>,
) -> Result<u32, SegmentError>
where
    K: CatalogKind,
    K::Row: CatalogRow,
    C: PinnedCatalog,
{
    let mut chunks = load_chain::<K>(segment, head_offset, head_len)?;
    loop {
        let tail_idx = chunks.len() - 1;
        let global_base = chunks[..tail_idx].iter().map(|c| c.catalog.count()).sum::<usize>();
        match chunks[tail_idx].catalog.append(row.clone()) {
            Ok(local) => {
                write_chunk(segment, &chunks[tail_idx])?;
                return u32::try_from(global_base + local as usize)
                    .map_err(|_| SegmentError::CapacityRequired);
            }
            Err(TypeError::CapacityExceeded) => {
                grow_chain(segment, &mut chunks, |cap| build_extension(cap))?;
            }
            Err(e) => return Err(super::map_type_err(e)),
        }
    }
}

pub(crate) fn index_in_chain<K: CatalogKind>(
    segment: &Segment,
    head_offset: u32,
    head_len: u32,
    mut matches: impl FnMut(&K::Row) -> bool,
) -> Result<Option<u32>, SegmentError>
where
    K::Row: CatalogRow,
{
    let chunks = load_chain::<K>(segment, head_offset, head_len)?;
    let mut base = 0u32;
    for chunk in &chunks {
        for (i, row) in chunk.catalog.rows.iter().enumerate() {
            if matches(row) {
                return Ok(Some(base + i as u32));
            }
        }
        base += chunk.catalog.count() as u32;
    }
    Ok(None)
}

pub(crate) fn get_in_chain<K: CatalogKind>(
    segment: &Segment,
    head_offset: u32,
    head_len: u32,
    type_index: u32,
) -> Result<Option<K::Row>, SegmentError>
where
    K::Row: CatalogRow + Copy,
{
    let chunks = load_chain::<K>(segment, head_offset, head_len)?;
    let mut remaining = type_index;
    for chunk in &chunks {
        let count = chunk.catalog.count() as u32;
        if remaining < count {
            return Ok(chunk.catalog.rows.get(remaining as usize).copied());
        }
        remaining -= count;
    }
    Ok(None)
}

pub(crate) fn redistribute_chain<K: CatalogKind>(
    segment: &mut Segment,
    chunks: &[CatalogChunk<K>],
    merged: &Catalog<K>,
) -> Result<(), SegmentError>
where
    K::Row: CatalogRow,
{
    if chunks.is_empty() {
        return Err(SegmentError::BadHeader);
    }
    let mut row_off = 0usize;
    for (i, chunk) in chunks.iter().enumerate() {
        let cap = chunk.catalog.capacity as usize;
        let end = (row_off + cap).min(merged.rows.len());
        let (next_offset, next_len) = if i + 1 < chunks.len() {
            (chunks[i + 1].offset, chunks[i + 1].len)
        } else {
            (0, 0)
        };
        let cat = Catalog::<K>::from_rows_capacity_next(
            merged.rows[row_off..end].to_vec(),
            chunk.catalog.capacity,
            next_offset,
            next_len,
        );
        row_off = end;
        write_chunk(
            segment,
            &CatalogChunk {
                offset: chunk.offset,
                len: chunk.len,
                catalog: cat,
            },
        )?;
    }
    if row_off != merged.rows.len() {
        return Err(SegmentError::CapacityRequired);
    }
    Ok(())
}

pub(crate) fn chain_chunk_count<K: CatalogKind>(
    segment: &Segment,
    head_offset: u32,
    head_len: u32,
) -> Result<usize, SegmentError>
where
    K::Row: CatalogRow,
{
    Ok(load_chain::<K>(segment, head_offset, head_len)?.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pymergetic::cruspy::memory::segment::catalog::metatype::MetaTypeCatalogKind;
    use crate::pymergetic::cruspy::memory::types::{FlexString, MetaType};

    #[test]
    fn flatten_chain_preserves_order() {
        let a = MetaType::from_type::<FlexString>().to_header();
        let b = MetaType::from_type::<crate::pymergetic::cruspy::memory::segment::catalog::MetaTypeCatalog>().to_header();
        let chunks = vec![
            CatalogChunk {
                offset: 0,
                len: 128,
                catalog: {
                    let mut c = Catalog::<MetaTypeCatalogKind>::with_capacity(2);
                    c.append(a).unwrap();
                    c
                },
            },
            CatalogChunk {
                offset: 256,
                len: 128,
                catalog: {
                    let mut c = Catalog::<MetaTypeCatalogKind>::with_capacity(2);
                    c.append(b).unwrap();
                    c
                },
            },
        ];
        let flat = flatten_chain(&chunks);
        assert_eq!(flat.count(), 2);
        assert_eq!(flat.rows[0], a);
        assert_eq!(flat.rows[1], b);
    }
}
