//! Manual smoke test for `memory` (ram backend + segment + talc).
//!
//! Run from `packages/cruspy`:
//!   cargo run --bin mem_demo

use pymergetic_cruspy::pymergetic::cruspy::io::{Kind, OpenMode};
use pymergetic_cruspy::pymergetic::cruspy::memory::backend::ram::Ram;
use pymergetic_cruspy::pymergetic::cruspy::memory::manager::{Locator, Manager};
use pymergetic_cruspy::pymergetic::cruspy::memory::segment::{Segment, HEADER_LEN, MAGIC, VERSION};

fn main() {
    println!("cruspy mem_demo\n");
    demo_create_from_scheme();
    demo_create_from_url();
    demo_single_slab();
    demo_multi_slab();
    demo_manager_layers();
    demo_manager_full_workflow();
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

fn demo_manager_layers() {
    println!("\n== manager full-stack (locator -> catalog -> segment/slab) ==");
    let mut mgr = Manager::new();

    // Layer 1: locator defaults (external identity layer).
    let default_loc = Locator::default();
    println!("  locator defaults:");
    println!("    default={}", default_loc);
    println!("    shm={}", Locator::default_for_kind(Kind::Shm));
    println!("    file={}", Locator::default_for_kind(Kind::File));

    // Layer 2+3: register two slabs through manager into one RAM segment.
    let a = mgr
        .create(&Ram::build_url("mgr-a"), Some(4096))
        .expect("register mgr-a");
    let b = mgr
        .create(&Ram::build_url("mgr-b"), Some(8192))
        .expect("register mgr-b");
    println!(
        "  registered: a(id={}, seg={}) b(id={}, seg={}) same_segment={}",
        a.id.0,
        a.segment_id.0,
        b.id.0,
        b.segment_id.0,
        a.segment_id == b.segment_id
    );

    // Catalog resolution and measured usage metadata.
    let roundtrip_id = mgr.id(&a.locator).expect("resolve a by locator");
    let report = mgr.usage_report();
    println!(
        "  catalog: locator->id={} totals(slabs={}, raw={}, arena={}, header={}, arena%={:.1}, header%={:.1})",
        roundtrip_id.0,
        report.totals.slab_count,
        report.totals.total_raw_len,
        report.totals.total_arena_len,
        report.totals.total_header_len,
        report.totals.arena_pct_raw(),
        report.totals.header_pct_raw()
    );

    // Segment/slab inspection through manager.
    let seg = mgr.segment(a.segment_id).expect("segment");
    let idx = mgr.slab_index(a.id).expect("slab index");
    let hdr = seg.header(idx).expect("header");
    println!(
        "  segment/slab: backends={} idx(a)={} size={} header_ok={}",
        seg.backends().len(),
        idx,
        seg.size(idx).unwrap_or(0),
        hdr.magic == MAGIC && hdr.version == VERSION
    );

    // Close behavior: unregister catalog, mapping remains closed for talc safety.
    mgr.close(a.id).expect("close a");
    let seg = mgr.segment(a.segment_id).expect("segment post-close");
    let state = seg.backend(idx).expect("backend").info().state;
    println!(
        "  close(a): in_catalog={} state={:?} size_after_close={}",
        mgr.try_id(&a.locator).is_some(),
        state,
        seg.size(idx).unwrap_or(0)
    );
}

fn demo_manager_full_workflow() {
    println!("\n== manager workflow (default + additional slabs) ==");
    let mut mgr = Manager::new();

    // 1) Create the default-default slab (ram://...default).
    let default_locator = Locator::default();
    let root = mgr
        .create(default_locator.as_url(), Some(4096))
        .expect("create default slab");
    println!(
        "  1) default slab: locator={} id={} seg={} idx={}",
        root.locator, root.id.0, root.segment_id.0, root.slab_index
    );

    // 2) Add additional slabs in the same storage family.
    let user_idx = mgr
        .create(&Ram::build_url("users"), Some(8192))
        .expect("create users slab");
    let cache_idx = mgr
        .create(&Ram::build_url("cache"), Some(16384))
        .expect("create cache slab");
    println!(
        "  2) added slabs: users(id={}, seg={}) cache(id={}, seg={})",
        user_idx.id.0, user_idx.segment_id.0, cache_idx.id.0, cache_idx.segment_id.0
    );

    // 3) Show all entries and segment placement.
    println!("  3) entries:");
    for (id, locator) in mgr.entries() {
        let seg_id = mgr.segment_id_for(id).expect("segment id");
        let slab_idx = mgr.slab_index(id).expect("slab index");
        println!(
            "     - id={} locator={} seg={} slab={}",
            id.0, locator, seg_id.0, slab_idx
        );
    }

    // 4) Print measured totals (raw/header/arena + talc counters).
    let report = mgr.usage_report();
    println!(
        "  4) usage totals: slabs={} raw={} arena={} header={} (arena%={:.1} header%={:.1}) talc(claimed={}, avail={}, alloc={}, overhead={}, alloc%={:.1}, overhead%={:.1})",
        report.totals.slab_count,
        report.totals.total_raw_len,
        report.totals.total_arena_len,
        report.totals.total_header_len,
        report.totals.arena_pct_raw(),
        report.totals.header_pct_raw(),
        report.totals.talc.claimed_bytes,
        report.totals.talc.available_bytes,
        report.totals.talc.allocated_bytes,
        report.totals.talc.overhead_bytes(),
        report.totals.talc_allocated_pct_claimed(),
        report.totals.talc_overhead_pct_claimed()
    );
    println!("     details:\n{}", report);

    // 5) Close one slab and show safe logical teardown.
    mgr.close(user_idx.id).expect("close users");
    let user_exists = mgr.try_id(&user_idx.locator).is_some();
    let seg = mgr.segment(user_idx.segment_id).expect("segment");
    let state = seg.backend(user_idx.slab_index).expect("backend").info().state;
    println!(
        "  5) close users: in_catalog={} backend_state={:?} backends={}",
        user_exists,
        state,
        seg.backends().len()
    );
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
