//! Terminal-specific row caps for resize reflow.
//!
//! This preserves the copied conservative defaults without depending on Codex configuration or
//! terminal-detection crates. VibeWindow supplies an explicit override when it has one; `Auto`
//! uses stable environment probes only.

const FALLBACK_MAX_ROWS: usize = 10_000;
const VSCODE_MAX_ROWS: usize = 1_000;
const WINDOWS_TERMINAL_MAX_ROWS: usize = 9_001;
const WEZTERM_MAX_ROWS: usize = 3_500;
const ALACRITTY_MAX_ROWS: usize = 10_000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ResizeReflowCap {
    #[default]
    Auto,
    Disabled,
    Limit(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalKind {
    VsCode,
    WindowsTerminal,
    WezTerm,
    Alacritty,
    Other,
}

pub fn resize_reflow_max_rows(cap: ResizeReflowCap) -> Option<usize> {
    resize_reflow_max_rows_for(cap, detected_terminal_kind())
}

fn resize_reflow_max_rows_for(cap: ResizeReflowCap, terminal: TerminalKind) -> Option<usize> {
    match cap {
        ResizeReflowCap::Disabled => None,
        ResizeReflowCap::Limit(rows) => Some(rows),
        ResizeReflowCap::Auto => Some(match terminal {
            TerminalKind::VsCode => VSCODE_MAX_ROWS,
            TerminalKind::WindowsTerminal => WINDOWS_TERMINAL_MAX_ROWS,
            TerminalKind::WezTerm => WEZTERM_MAX_ROWS,
            TerminalKind::Alacritty => ALACRITTY_MAX_ROWS,
            TerminalKind::Other => FALLBACK_MAX_ROWS,
        }),
    }
}

fn detected_terminal_kind() -> TerminalKind {
    if std::env::var_os("VSCODE_INJECTION").is_some()
        || std::env::var_os("TERM_PROGRAM").as_deref() == Some("vscode".as_ref())
    {
        return TerminalKind::VsCode;
    }
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default().to_ascii_lowercase();
    let term = std::env::var("TERM").unwrap_or_default().to_ascii_lowercase();
    if std::env::var_os("WT_SESSION").is_some() {
        TerminalKind::WindowsTerminal
    } else if term_program.contains("wezterm") || term.contains("wezterm") {
        TerminalKind::WezTerm
    } else if term_program.contains("alacritty") || term.contains("alacritty") {
        TerminalKind::Alacritty
    } else {
        TerminalKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
    fn configured_resize_reflow_max_rows_overrides_auto_detection() {
        assert_eq!(resize_reflow_max_rows_for(ResizeReflowCap::Limit(42), TerminalKind::VsCode), Some(42));
    }

    #[test]
    fn disabled_resize_reflow_max_rows_keeps_all_rows() {
        assert_eq!(resize_reflow_max_rows_for(ResizeReflowCap::Disabled, TerminalKind::Other), None);
    }
}
