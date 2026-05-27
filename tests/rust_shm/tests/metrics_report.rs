use cruspy_rust_shm_demo::{
    mem::device::ram, Registry, NestedLayout, STORAGE_CAPACITY,
};

#[test]
fn usage_metrics_track_used_and_capacity() {
    let layout = NestedLayout::plan();
    let used = layout.used_len();

    let heap = ram::named()
        .url(ram::build_url("heap"))
        .capacity(STORAGE_CAPACITY)
        .create();
    let scratch = ram::named()
        .url(ram::build_url("scratch"))
        .capacity(1024)
        .create();

    let mut reg = Registry::new();
    let heap_reg = reg.register(heap).unwrap();
    let _scratch_reg = reg.register(scratch).unwrap();

    let empty = reg.usage(&heap_reg).unwrap();
    assert_eq!(empty.capacity, STORAGE_CAPACITY);
    assert_eq!(empty.used_len, 0);
    assert_eq!(empty.free_len(), STORAGE_CAPACITY);
    assert!((empty.utilization_pct() - 0.0).abs() < f64::EPSILON);

    layout.init_in(reg.write(&heap_reg).unwrap(), 1, 2, 3);
    reg.set_used_len(&heap_reg, used).unwrap();
    reg.set_used_len(&_scratch_reg, 256).unwrap();

    let u = reg.usage(&heap_reg).unwrap();
    assert_eq!(u.used_len, used);
    assert!(u.utilization() > 0.0 && u.utilization() < 0.1);

    let report = reg.usage_report().format(16);
    assert!(report.contains("ram://heap"));
    assert!(report.contains("ram://scratch"));
    assert!(report.contains("TOTAL (2 slabs)"));

    let totals = reg.totals();
    assert_eq!(totals.slab_count, 2);
    assert_eq!(totals.total_capacity, STORAGE_CAPACITY + 1024);
    assert_eq!(totals.total_used, used + 256);
}
