use super::hermes_memory::*;
use vw_gateway_client::vw_api_types::hermes_memory::*;
fn response(content: &str) -> HermesMemoryResponse {
    HermesMemoryResponse { kind: HermesMemoryKind::Memory, content: content.into(), exists: true, path: Some("/executor/hermes/memories/MEMORY.md".into()) }
}
#[test]
fn hermes_drafts_preserve_inflight_edits() {
    let mut draft = Draft::default();
    draft.hydrate(response("旧内容"));
    draft.editor = iced::widget::text_editor::Content::with_text("保存快照");
    draft.saving = Some(draft.editor.text());
    draft.editor = iced::widget::text_editor::Content::with_text("保存期间的新编辑");
    draft.saved();
    assert_eq!(draft.saved.as_deref(), Some("保存快照"));
    assert_eq!(draft.editor.text(), "保存期间的新编辑");
    assert!(draft.dirty());
    draft.hydrate(response("重新读取"));
    assert_eq!(draft.editor.text(), "保存期间的新编辑");
}
#[test]
fn hermes_drafts_preserve_inflight_edits_before_hydration_and_across_tabs() {
    let mut state = State::default();
    state.drafts[0].edited = true;
    state.drafts[0].editor = iced::widget::text_editor::Content::with_text("提前编辑");
    state.drafts[0].hydrate(response("磁盘内容"));
    state.active = 1;
    state.drafts[1].hydrate(response("用户内容"));
    state.active = 0;
    assert_eq!(state.drafts[0].editor.text(), "提前编辑");
    assert_eq!(state.drafts[1].editor.text(), "用户内容");
    assert_eq!(state.drafts[0].path.as_deref(), Some("/executor/hermes/memories/MEMORY.md"));
    let generation = state.generation;
    state.invalidate();
    assert!(state.generation != generation);
    assert!(state.drafts.iter().all(|draft| draft.saved.is_none() && draft.saving.is_none()));
    assert!(!state.open);
}
