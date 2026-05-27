//! Manual smoke test for `memory` (ram backend + segment + talc).
//!
//! Run from `packages/cruspy`:
//!   cargo run --bin mem_demo

use pymergetic_cruspy::pymergetic::cruspy::io::HasSlab;
use pymergetic_cruspy::pymergetic::cruspy::memory::backend::ram::{self, Ram};
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
    let mut seg = Segment::<Ram>::new();
    seg.create(&url, Some(4096)).expect("create slab");
    print_segment(&seg);
    println!("  size_all={} size_raw_all={}", seg.size_all(), seg.size_raw_all());
}

fn demo_multi_slab() {
    println!("\n== two RAM slabs, one segment / one talc ==");
    let mut seg = Segment::<Ram>::new();
    seg.create(&ram::build_url("a"), Some(4096)).expect("create a");
    seg.create(&ram::build_url("b"), Some(8192)).expect("create b");
    print_segment(&seg);
    println!("  size_all={} size_raw_all={}", seg.size_all(), seg.size_raw_all());
}

fn print_segment<B: HasSlab>(seg: &Segment<B>) {
    println!("  backends: {}", seg.backends().len());
    for i in 0..seg.backends().len() {
        let b = seg.backend(i).unwrap();
        let info = b.info();
        let h = seg.header(i).unwrap();
        println!(
            "  [{i}] url={} capacity={} mode={:?} state={:?} size={} size_raw={}",
            info.url,
            info.capacity,
            info.open_mode,
            info.state,
            seg.size(i).unwrap_or(0),
            seg.size_raw(i).unwrap_or(0),
        );
        println!(
            "      header magic={:#x} version={} offset={} len={} (HEADER_LEN={HEADER_LEN})",
            h.magic, h.version, h.offset, h.len
        );
        println!(
            "      magic_ok={}",
            h.magic == MAGIC && h.version == VERSION
        );
    }
}
