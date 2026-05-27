//! URL interpretation helpers shared across backends.

use crate::mem::kind::Kind;
use crate::utils::url::Url;

pub fn kind(url: &Url) -> Option<Kind> {
    match url.scheme() {
        "ram" => Some(Kind::Ram),
        "shm" => Some(Kind::PosixShm),
        "file" => Some(Kind::File),
        _ => None,
    }
}
