use super::stream_render::StreamRenderState;

#[test]
fn stream_render_appends_without_full_message_clone() {
    let mut state = StreamRenderState::default();
    let mut content = String::new();
    state.begin("session", 7, 2, &content);
    let delta = "界".repeat(512 * 1024 / "界".len());
    let chunk_chars = delta.chars().count().div_ceil(1_000);
    for chars in delta.chars().collect::<Vec<_>>().chunks(chunk_chars) {
        content.extend(chars);
        assert!(state.append("session", 7, 2, &content));
    }
    assert_eq!(content, delta);
    let work = state.work();
    assert_eq!(work.full_message_clones, 0);
    assert_eq!(work.full_content_hashes, 0);
    assert_eq!(work.full_content_parses, 0);
    assert_eq!(work.height_index_rebuilds, 0);
}

#[test]
fn stream_render_reparses_mutable_tail_only() {
    let mut state = StreamRenderState::default();
    let mut content = "stable paragraph\n\n".repeat(2_000);
    state.begin("session", 1, 0, &content);
    content.push_str("```rust\nfn main() {");
    assert!(state.append("session", 1, 0, &content));
    assert!(state.stable_prefix_bytes() > 0);
    assert!(state.mutable_tail().starts_with("```rust"));
    assert!(state.mutable_tail().len() <= 16 * 1024);
}

#[test]
fn stream_render_drains_before_completion() {
    let mut state = StreamRenderState::default();
    state.begin("session", 1, 0, "answer");
    assert!(state.complete("session", 1, "answer"));
    assert_eq!(state.work().canonical_parses, 1);
    assert!(state.mutable_tail().is_empty());
}

#[test]
fn stream_render_handles_markdown_think_tools_and_unicode() {
    let content =
        "完成段落。\n<think>推理\n```rust\nlet 名字 = 1;\n```\ntool read\n{\"status\":\"running\"}";
    let mut state = StreamRenderState::default();
    state.begin("session", 2, 1, content);
    assert!(state.mutable_tail().contains("<think>"));
    assert!(state.mutable_tail().contains("名字"));
}

#[test]
fn stream_render_rejects_stale_generation() {
    let mut state = StreamRenderState::default();
    state.begin("session", 3, 0, "current");
    assert!(!state.append("session", 2, 0, "stale"));
    assert_eq!(state.revision(), 0);
}
