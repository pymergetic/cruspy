//! Visual demo: register RAM + POSIX SHM + file, seed graph, migrate, print usage.
//!
//! Run from `packages/cruspy/tests/rust_shm`:
//!   cargo run --bin mem_report

use std::process;

use cruspy_rust_shm_demo::{
    mem::device::{file, ram, shm}, Registry, NestedLayout, STORAGE_CAPACITY,
};

fn main() {
    if let Err(e) = run() {
        eprintln!("mem_report: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let layout = NestedLayout::plan();
    let used = layout.used_len();

    let shm_name = format!("cruspy_report_{}", process::id());
    let path = std::env::temp_dir().join(format!("cruspy_report_{}.bin", process::id()));

    let mut reg = Registry::new();
    let heap = reg.register(
        ram::named()
            .url(ram::build_url("heap"))
            .capacity(STORAGE_CAPACITY)
            .create(),
    )?;
    let session = reg.register(
        shm::named()
            .url(shm::build_url(&shm_name))
            .capacity(STORAGE_CAPACITY)
            .create(),
    )?;
    let snapshot = reg.register(
        file::named()
            .url(file::build_url(&path))
            .capacity(STORAGE_CAPACITY)
            .create(),
    )?;

    println!("=== 1) empty slabs (used_len = 0) ===\n");
    print_report(&reg);

    layout.init_in(reg.write(&heap)?, 0xCAFE, 0xBEEF, 0xBABE);
    reg.set_used_len(&heap, used)?;

    println!("\n=== 2) after init_in on ram://heap ===\n");
    print_report(&reg);

    reg.migrate(&heap, &session)?;
    println!("\n=== 3) after migrate ram → shm ===\n");
    print_report(&reg);

    reg.migrate(&session, &snapshot)?;
    reg.flush(&snapshot)?;
    println!("\n=== 4) after migrate shm → file ===\n");
    print_report(&reg);

    println!("\nGraph at file URL root:");
    let node = layout.read_root(reg.segment(&snapshot)?);
    println!(
        "  Node.magic = {:#x}, Child.x = {:#x}, Deep.z = {:#x}",
        node.read().magic,
        node.child().read().x,
        node.child().deep().read().z,
    );

    reg.unlink_posix_shm(&session).ok();
    let _ = std::fs::remove_file(&path);
    Ok(())
}

fn print_report(reg: &Registry) {
    print!("{}", reg.usage_report());
}
