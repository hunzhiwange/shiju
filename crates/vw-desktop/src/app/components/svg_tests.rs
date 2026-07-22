#[test]
fn svg_test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("svg_tests"));
}
