
mod behavior_0 {
use crate::state::FindInFolderMatch;
#[test]
fn workspace_structs_keep_selected_git_and_find_state_2() {
    let find_match = FindInFolderMatch {
        path: "src/lib.rs".to_string(),
        line: 3,
        column: 5,
        preview: "hello world".to_string(),
        match_len: 5,
    };
    assert_eq!(find_match.path, "src/lib.rs");
    assert_eq!(find_match.line, 3);
    assert_eq!(find_match.column, 5);
    assert_eq!(find_match.preview, "hello world");
    assert_eq!(find_match.match_len, 5);
}
}
