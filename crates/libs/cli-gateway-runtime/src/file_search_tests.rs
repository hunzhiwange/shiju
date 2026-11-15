use serde_json::json;

use super::file_search::{map_file_search_response, safe_relative_path};

#[test]
fn file_search_response_maps_relative_paths_and_caps_results() {
    let paths = (0..25).map(|index| format!(r"src\file_{index}.rs")).collect::<Vec<_>>();
    let matches = map_file_search_response(json!(paths)).expect("valid file search response");

    assert_eq!(matches.len(), 20);
    assert_eq!(matches[0].id, "src/file_0.rs");
    assert_eq!(matches[19].path, "src/file_19.rs");
    assert!(matches.iter().all(|item| item.match_indices.is_empty()));
}

#[test]
fn file_search_response_rejects_paths_outside_the_workspace() {
    for path in ["../secret", "/etc/passwd", r"C:\secret.txt"] {
        assert!(safe_relative_path(path).is_err(), "unsafe path must be rejected: {path}");
    }
    assert!(map_file_search_response(json!({"path":"src/lib.rs"})).is_err());
}
