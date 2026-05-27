//! Manual smoke test for `memory` (ram backend + segment + talc).
//!
//! Run from `packages/cruspy`:
//!   cargo run --bin mem_demo

use pymergetic_cruspy::pymergetic::cruspy::io::{Kind, OpenMode};
use pymergetic_cruspy::pymergetic::cruspy::memory::backend::ram::Ram;
use pymergetic_cruspy::pymergetic::cruspy::memory::segment::{Segment, HEADER_LEN, MAGIC, VERSION};

fn main() {
    println!("cruspy mem_demo\n");
    demo_create_from_scheme();
    demo_create_from_url();
    demo_single_slab();
    demo_multi_slab();
    println!("\nok");
}

fn demo_create_from_scheme() {
    println!("== Kind::create_from_scheme ==");
    let slab = Kind::create_from_scheme("ram").expect("ram scheme");
    println!("  kind={:?} state={:?}", slab.kind(), slab.info().state);
}

fn demo_create_from_url() {
    println!("\n== Kind::create_from_url + open ==");
    let url = Ram::build_url("factory");
    let mut slab = Kind::create_from_url(&url).expect("url scheme");
    slab.open(&url, OpenMode::Create, Some(4096))
        .expect("open slab");
    println!(
        "  kind={:?} url={} capacity={}",
        slab.kind(),
        slab.info().url,
        slab.info().capacity
    );
}

fn demo_single_slab() {
    println!("== single RAM slab ==");
    let url = Ram::build_url("heap");
    let mut seg = Segment::new(Kind::Ram);
    seg.create(&url, Some(4096)).expect("create slab");
    print_segment(&seg);
    println!("  size_all={} size_raw_all={}", seg.size_all(), seg.size_raw_all());
}

fn demo_multi_slab() {
    println!("\n== two RAM slabs, one segment / one talc ==");
    let mut seg = Segment::new(Kind::Ram);
    seg.create(&Ram::build_url("a"), Some(4096)).expect("create a");
    seg.create(&Ram::build_url("b"), Some(8192)).expect("create b");
    print_segment(&seg);
    println!("  size_all={} size_raw_all={}", seg.size_all(), seg.size_raw_all());
}

fn print_segment(seg: &Segment) {
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
            "      header magic={:#x} version={} header_len={} offset={} len={} (HEADER_LEN={HEADER_LEN})",
            h.magic, h.version, h.header_len, h.offset, h.len
        );
        println!(
            "      magic_ok={}",
            h.magic == MAGIC && h.version == VERSION
        );
    }
}
