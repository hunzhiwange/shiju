use super::*;

#[test]
// 自动模式应为每个已知终端及未知终端返回各自的稳定默认值。
fn auto_resize_reflow_max_rows_uses_terminal_defaults() {
    let cases = [
        (TerminalKind::VsCode, VSCODE_MAX_ROWS),
        (TerminalKind::WindowsTerminal, WINDOWS_TERMINAL_MAX_ROWS),
        (TerminalKind::WezTerm, WEZTERM_MAX_ROWS),
        (TerminalKind::Alacritty, ALACRITTY_MAX_ROWS),
        (TerminalKind::Other, FALLBACK_MAX_ROWS),
    ];
    for (terminal, expected) in cases {
        assert_eq!(resize_reflow_max_rows_for(ResizeReflowCap::Auto, terminal), Some(expected));
    }
}

#[test]
// 显式行数优先于终端自动检测。
fn configured_resize_reflow_max_rows_overrides_auto_detection() {
    assert_eq!(
        resize_reflow_max_rows_for(ResizeReflowCap::Limit(42), TerminalKind::VsCode),
        Some(42)
    );
}

#[test]
// 禁用上限以 `None` 表示保留全部重排行。
fn disabled_resize_reflow_max_rows_keeps_all_rows() {
    assert_eq!(
        resize_reflow_max_rows_for(ResizeReflowCap::Disabled, TerminalKind::Other),
        None
    );
}
