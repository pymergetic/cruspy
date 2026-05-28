use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeError {
    OutOfBounds,
    BadHeader,
    CapacityExceeded,
    InvalidUtf8,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::OutOfBounds => write!(f, "type region out of slab bounds"),
            TypeError::BadHeader => write!(f, "invalid type header"),
            TypeError::CapacityExceeded => write!(f, "value exceeds reserved capacity"),
            TypeError::InvalidUtf8 => write!(f, "stored bytes are not valid utf-8"),
        }
    }
}

impl std::error::Error for TypeError {}
