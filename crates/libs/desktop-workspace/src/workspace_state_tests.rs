use super::*;
#[test]
fn workspace_structs_keep_selected_git_state() {
 let selected = GitDiffSelectedLine { file: "src/lib.rs".into(), line: 12, is_old: false, text: "let value = 1;".into() };
 assert_eq!(selected.file, "src/lib.rs");
 assert_eq!(selected.line, 12);
 assert!(!selected.is_old);
 assert_eq!(selected.text, "let value = 1;");
 let menu = GitDiffFileMenuState { file: "src/lib.rs".into() };
 assert_eq!(menu.file, "src/lib.rs");
}
