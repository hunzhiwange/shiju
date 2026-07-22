#[test]
fn startup_tests_module_is_wired() {
    assert!(module_path!().ends_with("startup_tests"));
}

#[test]
fn build_startup_task_constructs_native_bootstrap_task() {
    let (mut app, _task) = crate::app::App::new();

    let task = super::build_startup_task(&mut app);

    let _ = std::hint::black_box(task);
}
