//! Manual smoke test for `memory` (ram backend + segment + talc).
//!
//! Run from `packages/cruspy`:
//!   cargo run --bin mem_demo

use pymergetic_cruspy::pymergetic::cruspy::io::HasAccess;
use pymergetic_cruspy::pymergetic::cruspy::memory::backend::ram::{self, Ram};
use pymergetic_cruspy::pymergetic::cruspy::memory::backend::Backend;
use pymergetic_cruspy::pymergetic::cruspy::memory::segment::{Segment, HEADER_LEN, MAGIC, VERSION};

fn main() {
    println!("cruspy mem_demo\n");
    demo_single_slab();
    demo_multi_slab();
    println!("\nok");
}

fn demo_single_slab() {
    println!("== single RAM slab ==");
    let url = ram::build_url("heap");
    let seg = Segment::<Ram>::create(&url, Some(4096)).expect("create segment");
    print_segment(&seg);
}

fn demo_multi_slab() {
    println!("\n== two RAM slabs, one segment / one talc ==");
    let mut seg = Segment::<Ram>::new();
    seg.add(Ram::create(&ram::build_url("a"), Some(4096)).expect("open a"))
        .expect("add a");
    seg.add(Ram::create(&ram::build_url("b"), Some(8192)).expect("open b"))
        .expect("add b");
    print_segment(&seg);
}

fn print_segment<B: Backend>(seg: &Segment<B>) {
    println!("  backends: {}", seg.backends().len());
    for i in 0..seg.backends().len() {
        let b = seg.backend(i).unwrap();
        let info = b.info();
        let h = seg.header(i).unwrap();
        let arena_len = seg.arena(i).map(<[u8]>::len).unwrap_or(0);
        println!(
            "  [{i}] url={} capacity={} mode={:?} state={:?}",
            info.url, info.capacity, info.open_mode, info.state
        );
        println!(
            "      header magic={:#x} version={} offset={} len={} (HEADER_LEN={HEADER_LEN})",
            h.magic, h.version, h.offset, h.len
        );
        println!(
            "      arena_len={arena_len} magic_ok={}",
            h.magic == MAGIC && h.version == VERSION
        );
    }
}
