//! Categorized FourCC tags for cruspy wire layouts.
//!
//! Use [`crate::pymergetic::cruspy::utils::fourcc`] only here (or in tests); domain code
//! should import from `slab`, `catalog`, or `record` so tag role stays obvious.

use crate::pymergetic::cruspy::utils::fourcc as pack;

/// Slab mapping envelope: 512-byte [`Header`](crate::pymergetic::cruspy::memory::segment::Header) on every backend.
pub mod slab {
    use super::pack;
    pub const CRUS: u32 = pack::fourcc("CRUS");
}

/// Pinned catalog blobs (shared catalog header + fixed rows).
pub mod catalog {
    use super::pack;
    pub const CTLG: u32 = pack::fourcc("CTLG");
    pub const COBJ: u32 = pack::fourcc("COBJ");
}

/// Fixed record headers inside catalogs or talc heap allocations.
///
/// These are **layout guards**, not type identity — see `type_uuid` on catalog rows and
/// [`HasMetaType::TYPE_UUID`](crate::pymergetic::cruspy::memory::types::HasMetaType::TYPE_UUID).
pub mod record {
    use super::pack;
    pub const MTYP: u32 = pack::fourcc("MTYP");
    pub const STRS: u32 = pack::fourcc("STRS");
    pub const OBJH: u32 = pack::fourcc("OBJH");
}

#[cfg(test)]
mod tests {
    use super::{catalog, record, slab};

    #[test]
    fn tags_match_legacy_hex() {
        assert_eq!(slab::CRUS, 0x4352_5553);
        assert_eq!(catalog::CTLG, 0x4354_4C47);
        assert_eq!(catalog::COBJ, 0x434F_424A);
        assert_eq!(record::MTYP, 0x4D54_5950);
        assert_eq!(record::STRS, 0x5354_5253);
        assert_eq!(record::OBJH, 0x4F42_4A48);
    }
}
