#[test]
fn acp_mount_update_native_capability_is_enabled() {
    let meta = super::mounts::capabilities_meta(true, true);
    assert_eq!(meta[crate::session_mounts::SESSION_MOUNTS_META_KEY]["setMounts"], true);
}

#[test]
fn acp_mount_update_prompt_load_and_update_share_exclusive_reservation() {
    let mut state = super::state::ConnectionState::default();
    // Every operation holds the same reservation across its entire prepare/publish window.
    assert!(state.reserve_prompt("session"));
    assert!(!state.reserve_prompt("session"), "prompt must reject during mount preparation");
    assert!(!state.reserve_prompt("session"), "second update must reject immediately");
    assert!(state.reserve_prompt("other"), "other sessions remain independent");
    state.finish_prompt("session");
    assert!(state.reserve_prompt("session"));
}
