#[test]
fn cc_route_033_pending_and_invalid_utf8_are_rejected() {
    let mut lines = super::sse_framing::Lines::default();
    assert!(lines.push(b"1234", 4).unwrap().is_empty());
    assert!(lines.push(b"5", 4).is_err());
    let mut lines = super::sse_framing::Lines::default();
    assert!(lines.push(b"data: \xff\n", 100).is_err());
    let mut lines = super::sse_framing::Lines::default();
    assert!(lines.push(b"data: a\rb\n", 100).is_err());
}
