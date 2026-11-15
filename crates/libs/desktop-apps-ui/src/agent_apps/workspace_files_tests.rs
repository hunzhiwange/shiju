use super::*;

#[test]
fn workspace_editor_ignores_stale_reads_after_switch() {
    let mut state = State { open: true, ..State::default() };
    let first = state.select(WorkspaceFilename::Agents);
    assert!(!state.loaded);
    let second = state.select(WorkspaceFilename::Soul);
    assert!(state.accept_read(second, WorkspaceFileContent(Some("中文草稿".into()))));
    assert!(!state.accept_read(first, WorkspaceFileContent(Some("旧内容".into()))));
    assert_eq!(state.editor.text(), "中文草稿");
}

#[test]
fn workspace_editor_ignores_stale_reads_after_close_and_reopen() {
    let mut state = State { open: true, ..State::default() };
    let old_generation = state.generation;
    let request = state.select(WorkspaceFilename::Agents);
    state.invalidate();
    assert!(!state.accept_read(request, WorkspaceFileContent(Some("旧内容".into()))));
    assert_ne!(state.generation, old_generation);
    assert!(!state.loaded);
}

#[test]
fn workspace_editor_missing_file_opens_empty_draft() {
    let mut state = State { open: true, ..State::default() };
    let request = state.select(WorkspaceFilename::Boot);
    assert!(state.accept_read(request, WorkspaceFileContent(None)));
    assert!(state.loaded);
    assert_eq!(state.editor.text(), "");
    assert_eq!(FILES.len(), 9);
}
