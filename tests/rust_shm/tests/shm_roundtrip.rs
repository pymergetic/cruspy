use cruspy_rust_shm_demo::{
    assert_graph, mem::device::{file, ram, shm}, mem::io::{Access, Open, OpenMode},
    migrate, segment, Read, Write, NestedLayout, STORAGE_CAPACITY,
};

#[test]
fn posix_shm_attach_sees_same_bytes() {
    let name = unique_name("shm");
    let layout = NestedLayout::plan();
    let url = shm::build_url(&name);

    let mut creator = shm::Storage::open(OpenMode::Create, &url, STORAGE_CAPACITY).expect("create");
    layout.init_in(&mut creator, 0xCAFEBABE, 0x1234ABCD, 0xDEADBEEF);

    let reader = shm::Storage::open(OpenMode::Attach, &url, STORAGE_CAPACITY).expect("attach");
    assert_graph(
        layout.read_root(segment(&reader)),
        0xCAFEBABE,
        0x1234ABCD,
        0xDEADBEEF,
    );

    creator.close().ok();
}

#[test]
fn migrate_ram_to_shm_to_file() {
    let layout = NestedLayout::plan();
    let used = layout.used_len();

    let mut ram =
        ram::Storage::open(OpenMode::Create, &ram::build_url("ram"), STORAGE_CAPACITY).unwrap();
    layout.init_in(&mut ram, 1, 2, 3);

    let shm_name = unique_name("mig_shm");
    let mut shm =
        shm::Storage::open(OpenMode::Create, &shm::build_url(&shm_name), STORAGE_CAPACITY)
            .expect("shm create");
    migrate(&ram as &dyn Read, &mut shm, used).expect("ram→shm");
    assert_graph(layout.read_root(segment(&shm)), 1, 2, 3);

    let path = std::env::temp_dir().join(format!("cruspy_mig_{}.bin", std::process::id()));
    let file_url = file::build_url(&path);
    let mut file =
        file::Storage::open(OpenMode::Create, &file_url, STORAGE_CAPACITY).expect("file create");
    migrate(&shm, &mut file, used).expect("shm→file");
    file.flush().expect("flush");
    assert_graph(layout.read_root(segment(&file)), 1, 2, 3);

    let mut ram2 =
        ram::Storage::open(OpenMode::Create, &ram::build_url("ram2"), STORAGE_CAPACITY).unwrap();
    migrate(&file, &mut ram2, used).expect("file→ram");
    assert_graph(layout.read_root(segment(&ram2)), 1, 2, 3);

    shm.close().ok();
    let _ = std::fs::remove_file(&path);
}

fn unique_name(tag: &str) -> String {
    format!(
        "cruspy_{tag}_{}_{}",
        std::process::id(),
        rand::random::<u32>()
    )
}
