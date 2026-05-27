use cruspy_rust_shm_demo::{
    assert_graph, mem::device::{file, ram, shm}, Loc, Registry, NestedLayout, Node,
    STORAGE_CAPACITY,
};

#[test]
fn registry_named_backends_and_migrate() {
    let layout = NestedLayout::plan();
    let used = layout.used_len();

    let shm_name = unique_name("reg_shm");
    let path = std::env::temp_dir().join(format!("cruspy_reg_{}.bin", std::process::id()));

    let mut reg = Registry::new();
    let heap = reg
        .register(
            ram::named()
                .url(ram::build_url("heap"))
                .capacity(STORAGE_CAPACITY)
                .create(),
        )
        .expect("ram");
    let session = reg
        .register(
            shm::named()
                .url(shm::build_url(&shm_name))
                .capacity(STORAGE_CAPACITY)
                .create(),
        )
        .expect("shm");
    let snapshot = reg
        .register(
            file::named()
                .url(file::build_url(&path))
                .capacity(STORAGE_CAPACITY)
                .create(),
        )
        .expect("file");

    layout.init_in(reg.write(&heap).expect("storage"), 9, 8, 7);
    reg.set_used_len(&heap, used).expect("used");
    assert_graph(layout.read_root(reg.segment(&heap).expect("seg")), 9, 8, 7);

    reg.migrate(&heap, &session).expect("ram→shm");
    assert_graph(
        layout.read_root(reg.segment(&session).expect("seg")),
        9,
        8,
        7,
    );

    reg.migrate(&session, &snapshot).expect("shm→file");
    reg.flush(&snapshot).expect("flush");
    assert_graph(
        layout.read_root(reg.segment(&snapshot).expect("seg")),
        9,
        8,
        7,
    );

    let loc = Loc {
        mem: session.id,
        off: layout.node,
    };
    let seg = reg.segment_at(loc).expect("resolve");
    assert_eq!(seg.at::<Node>(loc.off).read().magic, 9);

    reg.unlink_posix_shm(&session).ok();
    let _ = std::fs::remove_file(&path);
}

fn unique_name(tag: &str) -> String {
    format!(
        "cruspy_{tag}_{}_{}",
        std::process::id(),
        rand::random::<u32>()
    )
}
