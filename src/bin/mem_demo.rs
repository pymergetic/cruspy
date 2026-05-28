//! Manual smoke test for `memory` (ram backend + segment + talc).
//!
//! Run from `packages/cruspy`:
//!   cargo run --bin mem_demo

use pymergetic_cruspy::pymergetic::cruspy::io::{Kind, OpenMode};
use pymergetic_cruspy::pymergetic::cruspy::memory::backend::ram::Ram;
use pymergetic_cruspy::pymergetic::cruspy::memory::defaults::MIN_SLAB_CAPACITY;
use pymergetic_cruspy::pymergetic::cruspy::memory::manager::{format_talc_counters, Locator, Manager};
use pymergetic_cruspy::pymergetic::cruspy::memory::segment::{
    Header, MetaTypeCatalog, ObjectCatalog, Segment, MAGIC, METATYPE_CATALOG_MAGIC,
    METATYPE_CATALOG_SELF_INDEX, METATYPE_CATALOG_VERSION, OBJECT_CATALOG_MAGIC, VERSION,
};
use pymergetic_cruspy::pymergetic::cruspy::memory::types::{
    FlexString, HasMetaType, MetaTypeHeader,
};
use pymergetic_cruspy::pymergetic::cruspy::utils::{fourcc, uuid::Uuid};

struct KnownType {
    name: &'static str,
    uuid: [u8; 16],
}

const KNOWN_TYPES: &[KnownType] = &[
    KnownType {
        name: MetaTypeCatalog::TYPE_NAME,
        uuid: MetaTypeCatalog::TYPE_UUID,
    },
    KnownType {
        name: FlexString::TYPE_NAME,
        uuid: FlexString::TYPE_UUID,
    },
];

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
    seg.create(&url, Some(MIN_SLAB_CAPACITY))
        .expect("create slab");
    print_segment(&seg);
    print_talc_usage("  ", &seg);
    println!("  size_all={} size_raw_all={}", seg.size_all(), seg.size_raw_all());
}

fn demo_multi_slab() {
    println!("\n== two RAM slabs, one segment / one talc ==");
    let mut seg = Segment::new(Kind::Ram);
    seg.create(&Ram::build_url("a"), Some(MIN_SLAB_CAPACITY))
        .expect("create a");
    seg.create(&Ram::build_url("b"), Some(MIN_SLAB_CAPACITY))
        .expect("create b");
    print_segment(&seg);
    print_talc_usage("  ", &seg);
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
        .create(&Ram::build_url("mgr-a"), Some(MIN_SLAB_CAPACITY))
        .expect("register mgr-a");
    let b = mgr
        .create(&Ram::build_url("mgr-b"), Some(MIN_SLAB_CAPACITY))
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
    println!("\n== manager workflow (seg0 manual names + seg1 base-0/-1 extensions) ==");
    let mut mgr = Manager::new();

    // 1) Create the default-default slab (ram://...default).
    let default_locator = Locator::default();
    let root = mgr
        .create(default_locator.as_url(), Some(MIN_SLAB_CAPACITY))
        .expect("create default slab");
    println!(
        "  1) default slab: locator={} id={} seg={} idx={}",
        root.locator, root.id.0, root.segment_id.0, root.slab_index
    );

    // 2) Add additional slabs in the same storage family.
    let user_idx = mgr
        .create(&Ram::build_url("users"), Some(MIN_SLAB_CAPACITY))
        .expect("create users slab");
    let cache_idx = mgr
        .create(&Ram::build_url("cache"), Some(MIN_SLAB_CAPACITY))
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

    // 6) Second segment: formal base locator + indexed heap extensions (-0, -1).
    let base: Locator = Ram::build_url("indexed-seg").into();
    let seg1_id = mgr
        .open_segment(base.clone(), Some(MIN_SLAB_CAPACITY))
        .expect("open indexed segment");
    mgr.add_extension(base.clone(), 0, Some(MIN_SLAB_CAPACITY))
        .expect("extension 0");
    mgr.add_extension(base.clone(), 1, Some(MIN_SLAB_CAPACITY))
        .expect("extension 1");

    {
        let seg1 = mgr.segment(seg1_id).expect("indexed segment");
        let h1 = seg1.header(0).expect("primary header");
        println!(
            "  6) second segment: base={} seg={} (manual seg0={})",
            base,
            seg1_id.0,
            root.segment_id.0
        );
        assert_ne!(root.segment_id, seg1_id);
        println!(
            "     locators: primary={} ext0={} ext1={}",
            base,
            base.extension(0),
            base.extension(1)
        );
        for i in 0..seg1.backends().len() {
            let url = seg1.backend(i).expect("backend").info().url.clone();
            let hdr = seg1.header(i).expect("header");
            println!(
                "     slab[{i}] url={} role={} ext_count={}",
                url,
                if hdr.is_primary() { "primary" } else { "heap_ext" },
                if i == 0 { h1.extension_count } else { 0 }
            );
        }
    }

    let flex_idx = mgr
        .segment_mut(seg1_id)
        .expect("indexed segment mut")
        .register_metatype_for::<FlexString>()
        .expect("register FlexString");
    println!("     register_metatype_for::<FlexString>() -> type_index={flex_idx}");

    let seg1 = mgr.segment(seg1_id).expect("indexed segment");
    let h1 = seg1.header(0).expect("primary header");
    let cat = seg1.metatype_catalog().expect("metatype catalog");
    print_metatype_catalog("     ", &cat, h1.metatype_catalog_len);
    print_registered_metatypes("     ", &cat);

    let obj = seg1.object_catalog().expect("object catalog");
    print_object_catalog("     ", &obj, h1.object_catalog_len);

    // 7) Per-segment talc (manager report sums both segments).
    println!("  7) talc per segment:");
    let segment_ids: Vec<_> = mgr.segment_ids().collect();
    for seg_id in &segment_ids {
        let seg = mgr.segment(*seg_id).expect("segment");
        println!("     segment {} ({} slabs):", seg_id.0, seg.backends().len());
        print_talc_usage("       ", seg);
    }
    let backend_count: usize = segment_ids
        .iter()
        .map(|id| mgr.segment(*id).expect("segment").backends().len())
        .sum();
    let report = mgr.usage_report();
    println!(
        "     manager totals: segments={} backends={} registered_locators={} talc_claimed={} talc_allocated={}",
        segment_ids.len(),
        backend_count,
        report.totals.slab_count,
        report.totals.talc.claimed_bytes,
        report.totals.talc.allocated_bytes
    );
    println!(
        "     (registered_locators counts create() entries only; open_segment slabs are not in the locator catalog)"
    );
}

fn print_talc_usage(prefix: &str, seg: &Segment) {
    println!("{}", format_talc_counters(prefix, &seg.talc().counters()));
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
        print_slab_header("      ", &h);
        if i == 0 && h.is_primary() && h.is_mounted() {
            if let Ok(cat) = seg.metatype_catalog() {
                print_metatype_catalog("      ", &cat, h.metatype_catalog_len);
                print_registered_metatypes("      ", &cat);
            }
        }
    }
}

fn print_object_catalog(prefix: &str, cat: &ObjectCatalog, slab_reserved_len: u32) {
    let magic_tag =
        fourcc::to_string(OBJECT_CATALOG_MAGIC).unwrap_or_else(|_| "COBJ?".to_string());
    println!("{prefix}object catalog wire ({magic_tag} v1):");
    println!(
        "{prefix}  object_count={} capacity={} slots_free={}",
        cat.object_count(),
        cat.capacity(),
        cat.slots_remaining()
    );
    println!(
        "{prefix}  used_wire={} bytes  allocated_wire={} bytes  slab.object_catalog_len={}",
        cat.used_len(),
        cat.allocated_len(),
        slab_reserved_len
    );
}

fn print_slab_header(prefix: &str, h: &Header) {
    let magic_tag = fourcc::to_string(h.magic).unwrap_or_else(|_| format!("{:#010x}", h.magic));
    println!(
        "{prefix}slab header: magic={magic_tag} ({:#010x}) version={} role={} idx={} offset={} arena_len={}",
        h.magic,
        h.version,
        if h.is_primary() { "primary" } else { "heap_ext" },
        h.slab_index,
        h.offset,
        h.len,
    );
    if h.is_primary() {
        println!(
            "{prefix}  metatype_catalog: offset={} reserved_len={}",
            h.metatype_catalog_offset, h.metatype_catalog_len
        );
        if h.object_catalog_len > 0 {
            println!(
                "{prefix}  object_catalog: offset={} reserved_len={}",
                h.object_catalog_offset, h.object_catalog_len
            );
        }
    }
}

fn print_metatype_catalog(prefix: &str, cat: &MetaTypeCatalog, slab_reserved_len: u32) {
    let magic_tag =
        fourcc::to_string(METATYPE_CATALOG_MAGIC).unwrap_or_else(|_| "CTLG?".to_string());
    println!("{prefix}metatype catalog wire ({magic_tag} v{METATYPE_CATALOG_VERSION}):");
    println!(
        "{prefix}  count={} capacity={} slots_free={}",
        cat.count(),
        cat.capacity(),
        cat.slots_remaining()
    );
    println!(
        "{prefix}  used_wire={} bytes  allocated_wire={} bytes  slab.metatype_catalog_len={}",
        cat.used_len(),
        cat.allocated_len(),
        slab_reserved_len
    );
}

fn print_registered_metatypes(prefix: &str, cat: &MetaTypeCatalog) {
    println!("{prefix}registered metatypes (resolve via HasMetaType UUID):");
    for (i, row) in cat.metatypes().iter().enumerate() {
        let name = resolve_type_name(row);
        let uuid = Uuid::from_bytes(row.type_uuid);
        let boot = if i == METATYPE_CATALOG_SELF_INDEX as usize {
            " [bootstrap]"
        } else {
            ""
        };
        println!(
            "{prefix}  [{i}] {name}{boot}",
        );
        println!(
            "{prefix}      uuid={uuid} schema_version={} flags={:#x}",
            row.type_schema_version, row.flags
        );
    }
}

fn resolve_type_name(row: &MetaTypeHeader) -> &'static str {
    KNOWN_TYPES
        .iter()
        .find(|k| k.uuid == row.type_uuid)
        .map(|k| k.name)
        .unwrap_or("<?>")
}
