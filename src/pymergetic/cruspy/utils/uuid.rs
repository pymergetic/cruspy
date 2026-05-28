//! RFC 4122 UUIDs — parse, format, and version-4 generation.
//!
//! Wire layout is 16 octets in canonical field order (same as hyphenated string groups).

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

const HEX: &[u8; 16] = b"0123456789abcdef";

/// 128-bit identifier (RFC 4122).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Uuid(pub [u8; 16]);

/// All-zero UUID.
pub const NIL: Uuid = Uuid([0; 16]);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError(pub &'static str);

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "invalid UUID: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

impl Uuid {
    pub const LEN: usize = 16;
    pub const STR_LEN: usize = 36;

    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 16] {
        self.0
    }

    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Parse a compile-time UUID literal (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`).
    pub const fn must_parse(s: &str) -> Self {
        match Self::try_parse(s) {
            Ok(u) => u,
            Err(_) => panic!("invalid UUID literal"),
        }
    }

    pub const fn try_parse(s: &str) -> Result<Self, ParseError> {
        let b = s.as_bytes();
        if b.len() != 36
            || b[8] != b'-'
            || b[13] != b'-'
            || b[18] != b'-'
            || b[23] != b'-'
        {
            return Err(ParseError("expected 36-char hyphenated form"));
        }
        let mut out = [0u8; 16];
        let mut i = 0;
        let mut j = 0;
        while i < 36 {
            if b[i] == b'-' {
                i += 1;
                continue;
            }
            let hi = match hex_nibble(b[i]) {
                Some(v) => v,
                None => return Err(ParseError("invalid hex digit")),
            };
            let lo = match hex_nibble(b[i + 1]) {
                Some(v) => v,
                None => return Err(ParseError("invalid hex digit")),
            };
            out[j] = (hi << 4) | lo;
            j += 1;
            i += 2;
        }
        Ok(Self(out))
    }

    /// Random RFC 4122 version-4 UUID.
    pub fn new_v4() -> Self {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).expect("OS random source unavailable");
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Self(bytes)
    }

    pub fn version(&self) -> Option<u8> {
        let v = (self.0[6] >> 4) & 0x0f;
        if v == 0 {
            None
        } else {
            Some(v)
        }
    }

    pub fn hyphenated(self) -> String {
        self.to_string()
    }

    pub fn write_hyphenated(self, dst: &mut [u8; Self::STR_LEN]) {
        write_group(&mut dst[0..8], &self.0[0..4]);
        dst[8] = b'-';
        write_group(&mut dst[9..13], &self.0[4..6]);
        dst[13] = b'-';
        write_group(&mut dst[14..18], &self.0[6..8]);
        dst[18] = b'-';
        write_group(&mut dst[19..23], &self.0[8..10]);
        dst[23] = b'-';
        write_group(&mut dst[24..36], &self.0[10..16]);
    }
}

impl Display for Uuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut buf = [0u8; Uuid::STR_LEN];
        self.write_hyphenated(&mut buf);
        // SAFETY: ASCII hex and hyphens only.
        f.write_str(unsafe { std::str::from_utf8_unchecked(&buf) })
    }
}

impl FromStr for Uuid {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_parse(s)
    }
}

impl From<[u8; 16]> for Uuid {
    fn from(bytes: [u8; 16]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<Uuid> for [u8; 16] {
    fn from(value: Uuid) -> Self {
        value.bytes()
    }
}

const fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Compile-time UUID literal, e.g. `uuid!("8f3c2154-9a4e-074b-2cb1-6d18440a7f92")`.
#[macro_export]
macro_rules! uuid {
    ($lit:literal) => {
        $crate::pymergetic::cruspy::utils::uuid::Uuid::must_parse($lit)
    };
}

fn write_group(dst: &mut [u8], src: &[u8]) {
    let mut i = 0;
    for &b in src {
        dst[i] = HEX[(b >> 4) as usize];
        dst[i + 1] = HEX[(b & 0x0f) as usize];
        i += 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIT: &str = "8f3c2154-9a4e-074b-2cb1-6d18440a7f92";

    #[test]
    fn parse_and_display_roundtrip() {
        let u = Uuid::try_parse(LIT).unwrap();
        assert_eq!(u.to_string(), LIT);
        assert_eq!(format!("{u}"), LIT);
    }

    #[test]
    fn const_parse_matches_runtime() {
        const U: Uuid = Uuid::must_parse(LIT);
        assert_eq!(U, Uuid::try_parse(LIT).unwrap());
    }

    #[test]
    fn new_v4_sets_version_and_variant() {
        let u = Uuid::new_v4();
        assert_eq!(u.version(), Some(4));
        assert_eq!(u.0[8] & 0xc0, 0x80);
        assert_ne!(u, NIL);
        assert_eq!(u.to_string().len(), 36);
    }

    #[test]
    fn rejects_bad_shapes() {
        assert!(Uuid::try_parse("not-a-uuid").is_err());
        assert!(Uuid::try_parse("00000000-0000-0000-0000-00000000000").is_err());
    }
}
