use super::*;
use pretty_assertions::assert_eq;
use ratatui::text::Line;
use std::path::PathBuf;

fn test_cwd() -> PathBuf {
    // 这些测试只需要稳定的绝对目录；使用临时目录可避免固化 Unix 或 Windows 根路径语义。
    // These tests only need a stable absolute cwd; using temp_dir() avoids baking Unix- or
    // Windows-specific root semantics into the fixtures.
    std::env::temp_dir()
}

#[test]
fn drain_n_clamps_to_available_lines() {
    let mut state = StreamState::new(/*width*/ None, &test_cwd());
    state.enqueue(vec![HyperlinkLine::new(Line::from("one"))]);

    let drained = state.drain_n(/*max_lines*/ 8);
    assert_eq!(drained, vec![HyperlinkLine::new(Line::from("one"))]);
    assert!(state.is_idle());
}
