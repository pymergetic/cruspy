//! Four-character ASCII codes (FourCC) packed into a `u32` (big-endian / left-to-right order).
//!
//! Wire layouts store the value with [`to_le_bytes`](u32::to_le_bytes); the numeric
//! constant matches readable strings like `"CTLG"` → `0x4354_4C47`.

use std::fmt::{self, Display, Formatter};

/// Required length of a FourCC tag.
pub const LEN: usize = 4;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FourccError(pub &'static str);

impl Display for FourccError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "fourcc: {}", self.0)
    }
}

impl std::error::Error for FourccError {}

const fn pack_be(a: u8, b: u8, c: u8, d: u8) -> u32 {
    ((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | (d as u32)
}

/// Pack exactly four ASCII characters (`"CTLG"` → `0x4354_4C47`).
///
/// # Panics (const)
/// If `s` is not exactly four bytes long.
pub const fn fourcc(s: &str) -> u32 {
    let bytes = s.as_bytes();
    assert!(
        bytes.len() == LEN,
        "fourcc must be exactly 4 ASCII characters"
    );
    pack_be(bytes[0], bytes[1], bytes[2], bytes[3])
}

/// Alias for [`fourcc`] (explicit `u32` return type at call sites).
pub const fn to_u32(s: &str) -> u32 {
    fourcc(s)
}

/// Pack four bytes (const-friendly).
pub const fn from_bytes(bytes: [u8; LEN]) -> u32 {
    pack_be(bytes[0], bytes[1], bytes[2], bytes[3])
}

/// Unpack a FourCC to four bytes (ASCII order).
pub const fn from_u32(v: u32) -> [u8; LEN] {
    [(v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8]
}

/// Runtime parse with validation.
pub fn try_fourcc(s: &str) -> Result<u32, FourccError> {
    if s.len() != LEN {
        return Err(FourccError("expected exactly 4 characters"));
    }
    let b = s.as_bytes();
    Ok(pack_be(b[0], b[1], b[2], b[3]))
}

/// Decode a FourCC as an owned string (ASCII tags from [`fourcc`] always succeed).
pub fn to_string(v: u32) -> Result<String, FourccError> {
    let bytes = from_u32(v);
    String::from_utf8(bytes.to_vec()).map_err(|_| FourccError("invalid UTF-8 in tag bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_tags_match_prior_hex_constants() {
        assert_eq!(fourcc("CTLG"), 0x4354_4C47);
        assert_eq!(fourcc("CRUS"), 0x4352_5553);
        assert_eq!(fourcc("MTYP"), 0x4D54_5950);
        assert_eq!(fourcc("STRS"), 0x5354_5253);
    }

    #[test]
    fn roundtrip_bytes() {
        for s in ["CTLG", "CRUS", "MTYP", "STRS"] {
            let v = fourcc(s);
            assert_eq!(from_u32(v), s.as_bytes());
            assert_eq!(to_string(v).unwrap(), s);
        }
    }

    #[test]
    fn try_fourcc_validates_length() {
        assert!(try_fourcc("CTL").is_err());
        assert!(try_fourcc("CTLGX").is_err());
        assert_eq!(try_fourcc("CTLG").unwrap(), fourcc("CTLG"));
    }
}
