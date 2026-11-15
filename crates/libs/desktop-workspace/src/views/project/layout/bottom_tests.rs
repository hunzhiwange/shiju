#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("bottom_tests"));
}

#[test]
fn terminal_content_height_subtracts_resize_handle() {
    assert_eq!(super::terminal_content_height(240.0, 8.0), 232.0);
}

#[test]
fn terminal_content_height_never_goes_negative() {
    assert_eq!(super::terminal_content_height(4.0, 8.0), 0.0);
    assert_eq!(super::terminal_content_height(-4.0, 8.0), 0.0);
    assert_eq!(super::terminal_content_height(10.0, -8.0), 10.0);
    assert_eq!(super::terminal_content_height(-10.0, -8.0), 0.0);
}

#[test]
fn terminal_content_height_zero_when_both_zero() {
    assert_eq!(super::terminal_content_height(0.0, 0.0), 0.0);
}

#[test]
fn terminal_content_height_zero_when_terminal_equals_handle() {
    assert_eq!(super::terminal_content_height(8.0, 8.0), 0.0);
}