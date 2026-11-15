//! Explicit native acceptance contract; native execution uses the same module.
#[test]
fn archived_chats_native_acceptance_has_explicit_isolated_entry() {
    let manifest = include_str!("../../../Cargo.toml");
    assert!(
        manifest.contains("name = \"archived-chats-window\""),
        "native archive acceptance target missing"
    );
}

pub(super) use desktop_settings_sessions::acceptance::fixture::fixture;
