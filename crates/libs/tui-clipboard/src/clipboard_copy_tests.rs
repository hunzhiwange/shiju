use pretty_assertions::assert_eq;
use std::cell::Cell;

use super::CopyEnvironment;
use super::OSC52_MAX_RAW_BYTES;
use super::copy_to_clipboard_with;
use super::osc52_sequence;
use super::tmux_clipboard_copy_ready;
use super::write_osc52_to_writer;

fn remote_environment() -> CopyEnvironment {
    CopyEnvironment { ssh_session: true, wsl_session: true, tmux_session: false }
}

fn remote_tmux_environment() -> CopyEnvironment {
    CopyEnvironment { tmux_session: true, ..remote_environment() }
}

fn local_environment() -> CopyEnvironment {
    CopyEnvironment { ssh_session: false, wsl_session: false, tmux_session: false }
}

fn local_wsl_environment() -> CopyEnvironment {
    CopyEnvironment { wsl_session: true, ..local_environment() }
}

fn local_tmux_environment() -> CopyEnvironment {
    CopyEnvironment { tmux_session: true, ..local_environment() }
}

#[test]
fn osc52_encoding_roundtrips() {
    use base64::Engine;
    let text = "# Hello\n\n```rust\nfn main() {}\n```\n";
    let sequence = osc52_sequence(text, /*tmux*/ false).expect("OSC 52 sequence");
    let encoded = sequence.trim_start_matches("\u{1b}]52;c;").trim_end_matches('\u{7}');
    let decoded = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap();
    assert_eq!(decoded, text.as_bytes());
}

#[test]
fn osc52_rejects_payload_larger_than_limit() {
    let text = "x".repeat(OSC52_MAX_RAW_BYTES + 1);
    assert_eq!(
        osc52_sequence(&text, /*tmux*/ false),
        Err(format!(
            "OSC 52 payload too large ({} bytes; max {OSC52_MAX_RAW_BYTES})",
            OSC52_MAX_RAW_BYTES + 1
        ))
    );
}

#[test]
fn osc52_wraps_tmux_passthrough() {
    assert_eq!(
        osc52_sequence("hello", /*tmux*/ true),
        Ok("\u{1b}Ptmux;\u{1b}\u{1b}]52;c;aGVsbG8=\u{7}\u{1b}\\".to_string())
    );
}

#[test]
fn write_osc52_to_writer_emits_sequence_verbatim() {
    let sequence = "\u{1b}]52;c;aGVsbG8=\u{7}";
    let mut output = Vec::new();
    assert_eq!(write_osc52_to_writer(&mut output, sequence), Ok(()));
    assert_eq!(output, sequence.as_bytes());
}

#[test]
fn ssh_uses_osc52_and_skips_native_on_success() {
    let tmux_calls = Cell::new(0_u8);
    let osc_calls = Cell::new(0_u8);
    let native_calls = Cell::new(0_u8);
    let wsl_calls = Cell::new(0_u8);
    let result = copy_to_clipboard_with(
        "hello",
        remote_environment(),
        |_| {
            tmux_calls.set(tmux_calls.get() + 1);
            Ok(())
        },
        |_| {
            osc_calls.set(osc_calls.get() + 1);
            Ok(())
        },
        |_| {
            native_calls.set(native_calls.get() + 1);
            Ok(None)
        },
        |_| {
            wsl_calls.set(wsl_calls.get() + 1);
            Ok(())
        },
    );

    assert!(matches!(result, Ok(None)));
    assert_eq!(tmux_calls.get(), 0);
    assert_eq!(osc_calls.get(), 1);
    assert_eq!(native_calls.get(), 0);
    assert_eq!(wsl_calls.get(), 0);
}

#[test]
fn ssh_returns_osc52_error_and_skips_native() {
    let tmux_calls = Cell::new(0_u8);
    let osc_calls = Cell::new(0_u8);
    let native_calls = Cell::new(0_u8);
    let wsl_calls = Cell::new(0_u8);
    let result = copy_to_clipboard_with(
        "hello",
        remote_environment(),
        |_| {
            tmux_calls.set(tmux_calls.get() + 1);
            Ok(())
        },
        |_| {
            osc_calls.set(osc_calls.get() + 1);
            Err("blocked".into())
        },
        |_| {
            native_calls.set(native_calls.get() + 1);
            Ok(None)
        },
        |_| {
            wsl_calls.set(wsl_calls.get() + 1);
            Ok(())
        },
    );

    let Err(error) = result else {
        panic!("expected OSC 52 error");
    };
    assert_eq!(error, "OSC 52 clipboard copy failed over SSH: blocked");
    assert_eq!(tmux_calls.get(), 0);
    assert_eq!(osc_calls.get(), 1);
    assert_eq!(native_calls.get(), 0);
    assert_eq!(wsl_calls.get(), 0);
}

#[test]
fn ssh_inside_tmux_prefers_tmux_clipboard() {
    let tmux_calls = Cell::new(0_u8);
    let osc_calls = Cell::new(0_u8);
    let native_calls = Cell::new(0_u8);
    let wsl_calls = Cell::new(0_u8);
    let result = copy_to_clipboard_with(
        "hello",
        remote_tmux_environment(),
        |_| {
            tmux_calls.set(tmux_calls.get() + 1);
            Ok(())
        },
        |_| {
            osc_calls.set(osc_calls.get() + 1);
            Ok(())
        },
        |_| {
            native_calls.set(native_calls.get() + 1);
            Ok(None)
        },
        |_| {
            wsl_calls.set(wsl_calls.get() + 1);
            Ok(())
        },
    );

    assert!(matches!(result, Ok(None)));
    assert_eq!(tmux_calls.get(), 1);
    assert_eq!(osc_calls.get(), 0);
    assert_eq!(native_calls.get(), 0);
    assert_eq!(wsl_calls.get(), 0);
}

#[test]
fn ssh_inside_tmux_falls_back_to_osc52_when_tmux_copy_fails() {
    let tmux_calls = Cell::new(0_u8);
    let osc_calls = Cell::new(0_u8);
    let native_calls = Cell::new(0_u8);
    let wsl_calls = Cell::new(0_u8);
    let result = copy_to_clipboard_with(
        "hello",
        remote_tmux_environment(),
        |_| {
            tmux_calls.set(tmux_calls.get() + 1);
            Err("tmux unavailable".into())
        },
        |_| {
            osc_calls.set(osc_calls.get() + 1);
            Ok(())
        },
        |_| {
            native_calls.set(native_calls.get() + 1);
            Ok(None)
        },
        |_| {
            wsl_calls.set(wsl_calls.get() + 1);
            Ok(())
        },
    );

    assert!(matches!(result, Ok(None)));
    assert_eq!(tmux_calls.get(), 1);
    assert_eq!(osc_calls.get(), 1);
    assert_eq!(native_calls.get(), 0);
    assert_eq!(wsl_calls.get(), 0);
}

#[test]
fn ssh_inside_tmux_reports_tmux_and_osc52_errors_when_both_fail() {
    let result = copy_to_clipboard_with(
        "hello",
        remote_tmux_environment(),
        |_| Err("tmux unavailable".into()),
        |_| Err("osc blocked".into()),
        |_| Ok(None),
        |_| Ok(()),
    );

    let Err(error) = result else {
        panic!("expected tmux and OSC 52 errors");
    };
    assert_eq!(
        error,
        "terminal clipboard copy failed over SSH: tmux clipboard: tmux unavailable; OSC 52 fallback: osc blocked"
    );
}

#[test]
fn tmux_clipboard_copy_ready_accepts_forwarding_configuration() {
    let result = tmux_clipboard_copy_ready(
        || Ok("external\n".to_string()),
        || Ok("193: Ms: (string) \\033]52;%p1%s;%p2%s\\a\n".to_string()),
    );

    assert_eq!(result, Ok(()));
}

#[test]
fn tmux_clipboard_copy_ready_rejects_disabled_forwarding() {
    let result = tmux_clipboard_copy_ready(
        || Ok("off\n".to_string()),
        || panic!("tmux info should not be queried when forwarding is disabled"),
    );

    assert_eq!(result, Err("tmux clipboard forwarding is disabled".to_string()));
}

#[test]
fn tmux_clipboard_copy_ready_rejects_missing_ms_capability() {
    let result = tmux_clipboard_copy_ready(
        || Ok("external\n".to_string()),
        || Ok("193: Ms: [missing]\n".to_string()),
    );

    assert_eq!(
        result,
        Err("tmux clipboard forwarding is unavailable: missing Ms capability".to_string())
    );
}

#[test]
fn local_uses_native_clipboard_first() {
    let osc_calls = Cell::new(0_u8);
    let native_calls = Cell::new(0_u8);
    let wsl_calls = Cell::new(0_u8);
    let result = copy_to_clipboard_with(
        "hello",
        local_wsl_environment(),
        |_| Ok(()),
        |_| {
            osc_calls.set(osc_calls.get() + 1);
            Ok(())
        },
        |_| {
            native_calls.set(native_calls.get() + 1);
            Ok(Some(super::ClipboardLease::test()))
        },
        |_| {
            wsl_calls.set(wsl_calls.get() + 1);
            Ok(())
        },
    );

    assert!(matches!(result, Ok(Some(_))));
    assert_eq!(osc_calls.get(), 0);
    assert_eq!(native_calls.get(), 1);
    assert_eq!(wsl_calls.get(), 0);
}

#[test]
fn local_non_wsl_falls_back_to_osc52_when_native_fails() {
    let osc_calls = Cell::new(0_u8);
    let native_calls = Cell::new(0_u8);
    let wsl_calls = Cell::new(0_u8);
    let result = copy_to_clipboard_with(
        "hello",
        local_environment(),
        |_| Ok(()),
        |_| {
            osc_calls.set(osc_calls.get() + 1);
            Ok(())
        },
        |_| {
            native_calls.set(native_calls.get() + 1);
            Err("native unavailable".into())
        },
        |_| {
            wsl_calls.set(wsl_calls.get() + 1);
            Ok(())
        },
    );

    assert!(matches!(result, Ok(None)));
    assert_eq!(osc_calls.get(), 1);
    assert_eq!(native_calls.get(), 1);
    assert_eq!(wsl_calls.get(), 0);
}

#[test]
fn local_tmux_fallback_prefers_tmux_when_native_fails() {
    let tmux_calls = Cell::new(0_u8);
    let osc_calls = Cell::new(0_u8);
    let native_calls = Cell::new(0_u8);
    let wsl_calls = Cell::new(0_u8);
    let result = copy_to_clipboard_with(
        "hello",
        local_tmux_environment(),
        |_| {
            tmux_calls.set(tmux_calls.get() + 1);
            Ok(())
        },
        |_| {
            osc_calls.set(osc_calls.get() + 1);
            Ok(())
        },
        |_| {
            native_calls.set(native_calls.get() + 1);
            Err("native unavailable".into())
        },
        |_| {
            wsl_calls.set(wsl_calls.get() + 1);
            Ok(())
        },
    );

    assert!(matches!(result, Ok(None)));
    assert_eq!(tmux_calls.get(), 1);
    assert_eq!(osc_calls.get(), 0);
    assert_eq!(native_calls.get(), 1);
    assert_eq!(wsl_calls.get(), 0);
}

#[test]
fn local_wsl_native_failure_uses_powershell_and_skips_osc52_on_success() {
    let osc_calls = Cell::new(0_u8);
    let native_calls = Cell::new(0_u8);
    let wsl_calls = Cell::new(0_u8);
    let result = copy_to_clipboard_with(
        "hello",
        local_wsl_environment(),
        |_| Ok(()),
        |_| {
            osc_calls.set(osc_calls.get() + 1);
            Ok(())
        },
        |_| {
            native_calls.set(native_calls.get() + 1);
            Err("native unavailable".into())
        },
        |_| {
            wsl_calls.set(wsl_calls.get() + 1);
            Ok(())
        },
    );

    assert!(matches!(result, Ok(None)));
    assert_eq!(osc_calls.get(), 0);
    assert_eq!(native_calls.get(), 1);
    assert_eq!(wsl_calls.get(), 1);
}

#[test]
fn local_wsl_falls_back_to_osc52_when_native_and_powershell_fail() {
    let osc_calls = Cell::new(0_u8);
    let native_calls = Cell::new(0_u8);
    let wsl_calls = Cell::new(0_u8);
    let result = copy_to_clipboard_with(
        "hello",
        local_wsl_environment(),
        |_| Ok(()),
        |_| {
            osc_calls.set(osc_calls.get() + 1);
            Ok(())
        },
        |_| {
            native_calls.set(native_calls.get() + 1);
            Err("native unavailable".into())
        },
        |_| {
            wsl_calls.set(wsl_calls.get() + 1);
            Err("powershell unavailable".into())
        },
    );

    assert!(matches!(result, Ok(None)));
    assert_eq!(osc_calls.get(), 1);
    assert_eq!(native_calls.get(), 1);
    assert_eq!(wsl_calls.get(), 1);
}

#[test]
fn local_reports_both_errors_when_native_and_osc52_fail() {
    let osc_calls = Cell::new(0_u8);
    let native_calls = Cell::new(0_u8);
    let wsl_calls = Cell::new(0_u8);
    let result = copy_to_clipboard_with(
        "hello",
        local_environment(),
        |_| Ok(()),
        |_| {
            osc_calls.set(osc_calls.get() + 1);
            Err("osc blocked".into())
        },
        |_| {
            native_calls.set(native_calls.get() + 1);
            Err("native unavailable".into())
        },
        |_| {
            wsl_calls.set(wsl_calls.get() + 1);
            Ok(())
        },
    );

    let Err(error) = result else {
        panic!("expected native and OSC 52 errors");
    };
    assert_eq!(error, "native clipboard: native unavailable; OSC 52 fallback: osc blocked");
    assert_eq!(osc_calls.get(), 1);
    assert_eq!(native_calls.get(), 1);
    assert_eq!(wsl_calls.get(), 0);
}

#[test]
fn local_wsl_reports_native_powershell_and_osc52_errors_when_all_fail() {
    let osc_calls = Cell::new(0_u8);
    let native_calls = Cell::new(0_u8);
    let wsl_calls = Cell::new(0_u8);
    let result = copy_to_clipboard_with(
        "hello",
        local_wsl_environment(),
        |_| Ok(()),
        |_| {
            osc_calls.set(osc_calls.get() + 1);
            Err("osc blocked".into())
        },
        |_| {
            native_calls.set(native_calls.get() + 1);
            Err("native unavailable".into())
        },
        |_| {
            wsl_calls.set(wsl_calls.get() + 1);
            Err("powershell unavailable".into())
        },
    );

    let Err(error) = result else {
        panic!("expected native, WSL, and OSC 52 errors");
    };
    assert_eq!(
        error,
        "native clipboard: native unavailable; WSL fallback: powershell unavailable; OSC 52 fallback: osc blocked"
    );
    assert_eq!(osc_calls.get(), 1);
    assert_eq!(native_calls.get(), 1);
    assert_eq!(wsl_calls.get(), 1);
}
