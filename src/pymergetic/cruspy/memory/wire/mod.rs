//! On-segment wire conventions (distinct from logical type identity).
//!
//! ## Two different identification layers
//!
//! | Mechanism | Purpose | Example |
//! |-----------|---------|---------|
//! | **FourCC** ([`tags`]) | “What shape is this byte range?” — fast scan, no registry | `CRUS` slab, `CTLG` catalog, `STRS` string header |
//! | **UUID** ([`HasMetaType`]) | “What logical type is this?” — stable across tools and versions | `FlexString::TYPE_UUID` |
//!
//! FourCC answers structural questions at attach time. UUID answers semantic questions
//! when resolving catalog rows or application types. They do not replace each other.
//!
//! ## FourCC categories ([`tags`])
//!
//! - [`tags::slab`] — whole mapping prefix (one per backend file / ram region)
//! - [`tags::catalog`] — pinned index tables in the primary arena
//! - [`tags::record`] — small fixed headers on rows or heap objects (layout guards)

pub mod tags;

pub use tags::{catalog, record, slab};
