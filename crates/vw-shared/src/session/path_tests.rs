#[test]
fn safe_scope_is_used_as_path_segment() {
    assert_eq!(super::session_scope_key("project_1.alpha"), "project_1.alpha");
}

#[test]
fn unsafe_scope_is_hashed_to_stable_segment() {
    let first = super::session_scope_key("../project");
    let second = super::session_scope_key("../project");

    assert_eq!(first, second);
    assert_eq!(first.len(), 16);
    assert!(first.bytes().all(|b| b.is_ascii_hexdigit()));
}

#[test]
fn empty_scope_is_hashed_instead_of_becoming_empty_segment() {
    let key = super::session_scope_key("");

    assert_eq!(key.len(), 16);
    assert!(key.bytes().all(|b| b.is_ascii_hexdigit()));
}

#[test]
fn session_artifact_paths_include_scope_and_snapshot_kind() {
    let data_dir = std::path::Path::new("/tmp/vw-data");

    let scoped_dir = super::sessions_dir_for_scope(data_dir, Some("repo")).unwrap();
    let snapshot =
        super::session_step_snapshot_file_path(data_dir, "abc", 3, "finish", Some("repo")).unwrap();
    let raw = super::session_step_llm_raw_file_path(data_dir, "abc", 3, Some("repo")).unwrap();

    assert_eq!(scoped_dir, data_dir.join("storage/session/scoped/repo"));
    assert_eq!(snapshot, scoped_dir.join("snapshots/session-abc-step-3-finish.json"));
    assert_eq!(raw, scoped_dir.join("snapshots/session-abc-step-3-llm_raw.json"));
}

#[test]
fn unscoped_artifact_paths_use_shared_session_directory() {
    let data_dir = std::path::Path::new("/tmp/vw-data");
    let session_dir = data_dir.join("storage/session");

    assert_eq!(super::sessions_dir_for_scope(data_dir, None).unwrap(), session_dir);
    assert_eq!(
        super::session_step_snapshot_file_path(data_dir, "abc", 0, "start", None).unwrap(),
        session_dir.join("snapshots/session-abc-step-0-start.json")
    );
    assert_eq!(
        super::session_step_llm_raw_file_path(data_dir, "abc", 0, None).unwrap(),
        session_dir.join("snapshots/session-abc-step-0-llm_raw.json")
    );
}

#[cfg(target_arch = "wasm32")]
#[test]
fn wasm_session_path_remains_scoped_json() {
    let data_dir = std::path::Path::new("/tmp/vw-data");

    assert_eq!(
        super::session_file_path(data_dir, "abc", Some("repo")).unwrap(),
        data_dir.join("storage/session/scoped/repo/session-abc.json")
    );
}
