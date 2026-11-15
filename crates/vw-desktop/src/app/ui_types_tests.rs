#[test]
fn global_messages_exclude_removed_design_domain() {
    assert!(!include_str!("message/mod.rs").contains(concat!("Design", "Message")));
}
