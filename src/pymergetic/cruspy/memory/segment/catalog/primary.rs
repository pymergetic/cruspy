//! Primary-slab catalog blob addressing (shared by CTLG and COBJ).

use crate::pymergetic::cruspy::memory::segment::{
    arena_range, read_header, Header, Segment, SegmentError,
};

pub(crate) fn primary_header(segment: &Segment) -> Result<Header, SegmentError> {
    let backend = segment.backends().first().ok_or(SegmentError::BadIndex)?;
    let h = read_header(backend.bytes()).ok_or(SegmentError::BadHeader)?;
    if !h.is_primary() || !h.is_mounted() {
        return Err(SegmentError::NotMounted);
    }
    Ok(h)
}

pub(crate) fn blob_bounds(
    segment: &Segment,
    offset: u32,
    len: u32,
) -> Result<(usize, usize), SegmentError> {
    let backend = segment.backends().first().ok_or(SegmentError::BadIndex)?;
    let capacity = backend.info().capacity;
    let range = arena_range(backend.bytes(), capacity)?;
    let off = offset as usize;
    let blob_len = len as usize;
    if len == 0 || off + blob_len > range.len() {
        return Err(SegmentError::BadHeader);
    }
    Ok((range.start + off, blob_len))
}

pub(crate) fn blob<'a>(segment: &'a Segment, offset: u32, len: u32) -> Result<&'a [u8], SegmentError> {
    let (start, blob_len) = blob_bounds(segment, offset, len)?;
    Ok(&segment.backends()[0].bytes()[start..start + blob_len])
}

pub(crate) fn blob_mut<'a>(
    segment: &'a mut Segment,
    offset: u32,
    len: u32,
) -> Result<&'a mut [u8], SegmentError> {
    let (start, blob_len) = blob_bounds(segment, offset, len)?;
    Ok(&mut segment.backends_mut()[0].bytes_mut()[start..start + blob_len])
}
