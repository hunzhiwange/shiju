//! 终端状态测试，覆盖平台 Shell、UTF-8 安全截断、标签管理和尺寸恢复。
//! 尾部保留按字节限制时不能留下残缺字符，未知终端操作应保持无副作用。

#[test]
fn shell_display_and_all_are_stable() {
    let shells = super::Shell::all();
    assert_eq!(shells, [super::Shell::Bash, super::Shell::Zsh]);
    assert_eq!(shells[0].to_string(), "bash");
    assert_eq!(shells[1].to_string(), "zsh");
    assert_eq!(super::TerminalTheme::System.to_string(), "跟随系统");
}

#[test]
fn truncate_string_preserves_utf8_boundary() {
    assert_eq!(super::truncate_string_to_limit("abc", 4), "abc");
    assert_eq!(super::truncate_string_to_limit("a你好b", 5), "好b");
    assert_eq!(super::truncate_string_to_limit("🙂abc", 4), "abc");
    assert_eq!(super::truncate_string_to_limit("abcdef", 0), "");
}

#[test]
fn truncate_string_drops_a_partial_multibyte_prefix_for_tiny_limits() {
    assert_eq!(super::truncate_string_to_limit("界", 1), "");
    assert_eq!(super::truncate_string_to_limit("a界", 2), "");
    assert_eq!(super::truncate_string_to_limit("a界", 3), "界");
}

#[test]
fn truncate_terminal_content_keeps_last_four_thousand_bytes() {
    let mut content = iced::widget::text_editor::Content::with_text(&format!(
        "{}{}",
        "x".repeat(10),
        "y".repeat(4_000)
    ));

    super::truncate_terminal_content(&mut content);

    let text = content.text();
    assert_eq!(text.len(), 4_000);
    assert!(text.chars().all(|ch| ch == 'y'));
}

#[test]
fn truncate_terminal_content_leaves_content_at_or_below_limit_unchanged() {
    for original in ["short terminal output".to_string(), "x".repeat(4_000)] {
        let mut content = iced::widget::text_editor::Content::with_text(&original);

        super::truncate_terminal_content(&mut content);

        assert_eq!(content.text(), original);
    }
}

#[test]
fn truncate_terminal_content_drops_partial_leading_utf8_character() {
    let mut content =
        iced::widget::text_editor::Content::with_text(&format!("{}{}", "🙂", "x".repeat(3_999)));

    super::truncate_terminal_content(&mut content);

    assert_eq!(content.text(), "x".repeat(3_999));
}

#[test]
fn default_terminal_state_uses_expected_settings() {
    let state = super::TerminalState::default();

    assert!(!state.is_visible);
    assert!(state.tabs.is_empty());
    assert_eq!(state.active_id, None);
    assert_eq!(state.next_id, 1);
    assert_eq!(state.height, 200.0);
    assert_eq!(state.shell, super::Shell::Bash);
    assert_eq!(state.theme, super::TerminalTheme::System);
    assert_eq!(state.font_family, "JetBrains Mono");
    assert_eq!(state.font_size, 13.0);
}

#[test]
fn blank_with_settings_preserves_user_preferences() {
    let state = super::TerminalState::blank_with_settings(
        true,
        super::Shell::Zsh,
        super::TerminalTheme::System,
        "Fira Code".into(),
        16.0,
        240.0,
    );

    assert!(state.is_visible);
    assert_eq!(state.shell, super::Shell::Zsh);
    assert_eq!(state.font_family, "Fira Code");
    assert_eq!(state.font_size, 16.0);
    assert_eq!(state.height, 240.0);
    assert!(state.tabs.is_empty());
    assert_eq!(state.active_id, None);
    assert_eq!(state.next_id, 1);
    assert!(!state.is_dragging);
    assert_eq!(state.drag_anchor_y, None);
    assert_eq!(state.drag_start_height, 0.0);
    assert_eq!(state.tab_context_menu_id, None);
    assert_eq!(state.tab_context_menu_pos, None);
}

#[test]
fn selecting_unknown_terminal_still_opens_panel_and_restores_min_height() {
    let mut state = super::TerminalState::default();
    state.height = 120.0;

    state.select_terminal(99);

    assert_eq!(state.active_id, Some(99));
    assert!(state.is_visible);
    assert_eq!(state.height, 200.0);
}

#[test]
fn selecting_terminal_preserves_height_at_minimum_boundary() {
    let mut state = super::TerminalState::default();
    state.height = 160.0;

    state.select_terminal(7);

    assert_eq!(state.active_id, Some(7));
    assert!(state.is_visible);
    assert_eq!(state.height, 160.0);
}

#[test]
fn empty_terminal_state_accepts_display_setting_updates() {
    let mut state = super::TerminalState::default();

    state.set_font_family("Fira Code".to_string());
    state.set_font_size(18.0);
    state.set_theme(super::TerminalTheme::System);

    assert_eq!(state.font_family, "Fira Code");
    assert_eq!(state.font_size, 18.0);
    assert_eq!(state.theme, super::TerminalTheme::System);
    assert!(state.tabs.is_empty());
}

#[test]
fn close_and_rename_unknown_terminal_are_noops_for_empty_state() {
    let mut state = super::TerminalState::default();

    state.close_terminal(1);
    state.start_rename(1);
    state.update_rename(1, "renamed".into());
    state.save_rename(1);
    state.cancel_rename(1);

    assert!(state.tabs.is_empty());
    assert_eq!(state.active_id, None);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn color_channels_are_clamped_before_hex_conversion() {
    assert_eq!(super::color_channel_to_u8(-1.0), 0);
    assert_eq!(super::color_channel_to_u8(2.0), 255);
    assert_eq!(super::to_hex(iced::Color::from_rgb(1.0, 0.5, 0.0)), "#ff8000");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn color_channel_conversion_rounds_midpoints_and_ignores_alpha() {
    assert_eq!(super::color_channel_to_u8(0.0), 0);
    assert_eq!(super::color_channel_to_u8(0.5), 128);
    assert_eq!(super::color_channel_to_u8(1.0), 255);
    assert_eq!(super::to_hex(iced::Color::from_rgba(0.25, 0.75, 1.0, 0.0)), "#40bfff");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn palette_uses_extended_theme_colors_for_each_terminal_role() {
    let theme = iced::Theme::Dark;
    let extended = theme.extended_palette();

    let palette = super::build_palette(&theme);

    assert_eq!(palette.background, super::to_hex(extended.background.base.color));
    assert_eq!(palette.foreground, super::to_hex(extended.background.base.text));
    assert_eq!(palette.black, super::to_hex(extended.background.weak.color));
    assert_eq!(palette.white, super::to_hex(extended.background.strong.color));
    assert_eq!(
        palette.bright_black,
        super::to_hex(extended.background.weak.color.scale_alpha(0.85))
    );
    assert_eq!(
        palette.bright_white,
        super::to_hex(extended.background.strong.color.scale_alpha(0.85))
    );
    assert_eq!(palette.red, super::to_hex(extended.danger.base.color));
    assert_eq!(palette.bright_red, super::to_hex(extended.danger.strong.color));
    assert_eq!(palette.green, super::to_hex(extended.success.base.color));
    assert_eq!(palette.bright_green, super::to_hex(extended.success.strong.color));
    assert_eq!(palette.yellow, super::to_hex(extended.warning.base.color));
    assert_eq!(palette.bright_yellow, super::to_hex(extended.warning.strong.color));
    assert_eq!(palette.blue, super::to_hex(extended.primary.base.color));
    assert_eq!(palette.bright_blue, super::to_hex(extended.primary.strong.color));
    assert_eq!(palette.magenta, super::to_hex(extended.secondary.base.color));
    assert_eq!(palette.bright_magenta, super::to_hex(extended.secondary.strong.color));
    assert_eq!(palette.cyan, super::to_hex(extended.secondary.base.color));
    assert_eq!(palette.bright_cyan, super::to_hex(extended.secondary.strong.color));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn font_settings_scale_is_clamped_for_tiny_sizes() {
    let tiny = super::build_font_settings("JetBrains Mono", 1.0);
    let normal = super::build_font_settings("JetBrains Mono", 13.0);

    assert_eq!(tiny.size, 13.0);
    assert_eq!(tiny.scale_factor, 0.5);
    assert!(normal.scale_factor > tiny.scale_factor);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn font_settings_default_blank_family_and_trim_custom_family() {
    let blank = super::build_font_settings("   ", 13.0);
    let custom = super::build_font_settings("  Fira Code  ", 26.0);

    assert_eq!(blank.font_type, iced::Font::with_name("JetBrains Mono"));
    assert_eq!(blank.scale_factor, 1.2);
    assert_eq!(custom.font_type, iced::Font::with_name("Fira Code"));
    assert_eq!(custom.scale_factor, 2.4);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn enter_bindings_cover_enter_newline_and_carriage_return() {
    let bindings = super::enter_sends_cr_bindings();

    assert_eq!(bindings.len(), 6);
    assert!(
        bindings
            .iter()
            .all(|(_, action)| matches!(action, iced_term::bindings::BindingAction::Char('\n')))
    );

    use iced::keyboard::{Modifiers, key::Named};
    use iced_term::bindings::InputKind;

    let targets_and_modifiers: Vec<_> =
        bindings.into_iter().map(|(binding, _)| (binding.target, binding.modifiers)).collect();
    assert_eq!(
        targets_and_modifiers,
        vec![
            (InputKind::KeyCode(Named::Enter), Modifiers::empty()),
            (InputKind::KeyCode(Named::Enter), Modifiers::SHIFT),
            (InputKind::Char("\n".to_string()), Modifiers::empty()),
            (InputKind::Char("\n".to_string()), Modifiers::SHIFT),
            (InputKind::Char("\r".to_string()), Modifiers::empty()),
            (InputKind::Char("\r".to_string()), Modifiers::SHIFT),
        ]
    );
}

#[test]
fn new_hidden_terminal_state_does_not_spawn_pty_tabs() {
    let state = super::TerminalState::new(
        false,
        super::Shell::Bash,
        super::TerminalTheme::System,
        "JetBrains Mono".to_string(),
        13.0,
        None,
    );

    assert!(!state.is_visible);
    assert!(state.tabs.is_empty());
    assert_eq!(state.active_id, None);
    assert_eq!(state.next_id, 1);
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
#[test]
fn ensure_default_terminal_creates_first_tab_once() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut state = super::TerminalState::new(
        false,
        super::Shell::Bash,
        super::TerminalTheme::System,
        "JetBrains Mono".to_string(),
        13.0,
        None,
    );

    assert!(state.ensure_default_terminal(Some(tmp.path().to_path_buf())));
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.active_id, Some(1));
    assert!(!state.ensure_default_terminal(Some(tmp.path().to_path_buf())));
    assert_eq!(state.tabs.len(), 1);
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
#[test]
fn new_visible_terminal_state_lazily_spawns_default_tab() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let state = super::TerminalState::new(
        true,
        super::Shell::Bash,
        super::TerminalTheme::System,
        "JetBrains Mono".to_string(),
        13.0,
        Some(tmp.path().to_path_buf()),
    );

    assert!(state.is_visible);
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.active_id, Some(1));
    assert_eq!(state.tabs[0].title, "终端 1");
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
#[test]
fn terminal_state_with_backend_manages_tabs_renames_and_display_updates() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut state = super::TerminalState::new(
        true,
        super::Shell::Bash,
        super::TerminalTheme::System,
        "JetBrains Mono".to_string(),
        13.0,
        Some(tmp.path().to_path_buf()),
    );

    assert!(state.is_visible);
    assert_eq!(state.active_id, Some(1));
    assert_eq!(state.next_id, 2);
    assert_eq!(state.height, 200.0);
    assert_eq!(state.tabs.first().map(|tab| tab.title.as_str()), Some("终端 1"));

    state.start_rename(1);
    assert_eq!(state.tabs[0].edit_title.as_deref(), Some("终端 1"));
    state.update_rename(1, "   ".to_string());
    state.save_rename(1);
    assert_eq!(state.tabs[0].title, "终端 1");

    state.start_rename(1);
    state.update_rename(1, " Dev Shell ".to_string());
    state.save_rename(1);
    assert_eq!(state.tabs[0].title, "Dev Shell");

    state.start_rename(1);
    state.cancel_rename(1);
    assert_eq!(state.tabs[0].edit_title, None);

    assert!(state.add_terminal(Some(tmp.path().to_path_buf())));
    assert_eq!(state.active_id, Some(2));
    assert_eq!(state.tabs.len(), 2);

    state.select_terminal(1);
    assert_eq!(state.active_id, Some(1));
    state.set_font_family("Fira Code".to_string());
    state.set_font_size(18.0);
    state.set_theme(super::TerminalTheme::System);
    state.apply_app_theme(&iced::Theme::Dark);
    state.apply_palette(super::build_palette(&iced::Theme::Light));
    assert_eq!(state.font_family, "Fira Code");
    assert_eq!(state.font_size, 18.0);

    state.close_terminal(1);
    assert_eq!(state.active_id, Some(2));
    state.close_terminal(2);
    assert_eq!(state.active_id, None);
    assert!(state.tabs.is_empty());
    assert!(state.has_retired());
    state.reap_retired();
    assert!(!state.has_retired());
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
#[test]
fn terminal_input_stays_responsive_when_backend_event_queue_is_full() {
    use std::sync::mpsc;
    use std::time::Duration;

    let settings = iced_term::settings::Settings {
        backend: iced_term::settings::BackendSettings {
            program: "/bin/sh".to_string(),
            args: vec![
                "-c".to_string(),
                concat!(
                    "i=0; ",
                    "while [ \"$i\" -lt 256 ]; do ",
                    "printf '\\033]0;%s\\007' \"$i\"; ",
                    "i=$((i + 1)); ",
                    "done; ",
                    "sleep 5"
                )
                .to_string(),
            ],
            ..Default::default()
        },
        ..Default::default()
    };
    let terminal = iced_term::Terminal::new(9_999, settings).expect("terminal");

    std::thread::sleep(Duration::from_millis(500));

    let (finished_tx, finished_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut terminal = terminal;
        let _ = terminal.handle(iced_term::Command::ProxyToBackend(
            iced_term::BackendCommand::Write(b"x".to_vec()),
        ));
        let _ = finished_tx.send(());
    });

    assert!(
        finished_rx.recv_timeout(Duration::from_secs(1)).is_ok(),
        "terminal input blocked after the backend event queue filled"
    );
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
#[test]
fn command_and_init_terminals_append_tabs_with_requested_titles() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut state = super::TerminalState::blank_with_settings(
        true,
        super::Shell::Bash,
        super::TerminalTheme::System,
        "JetBrains Mono".to_string(),
        13.0,
        200.0,
    );

    let command_id = state
        .add_terminal_with_command(
            "Command".to_string(),
            Some(tmp.path().to_path_buf()),
            "printf command".to_string(),
        )
        .expect("command terminal");
    let bash_id = state
        .add_bash_terminal_with_init(
            "Init".to_string(),
            Some(tmp.path().to_path_buf()),
            "printf init".to_string(),
        )
        .expect("bash init terminal");
    let tool_id = state
        .add_tool_terminal_with_init(
            "Tool".to_string(),
            Some(tmp.path().to_path_buf()),
            "printf tool".to_string(),
        )
        .expect("tool init terminal");

    assert_eq!(state.tabs.iter().map(|tab| tab.title.as_str()).collect::<Vec<_>>(), {
        vec!["Command", "Init", "Tool"]
    });
    assert_eq!(state.active_id, Some(tool_id));
    assert_eq!([command_id, bash_id, tool_id], [1, 2, 3]);
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
#[test]
fn spawn_pty_terminal_collects_shell_output_for_selected_cwd() {
    use std::io::Write;
    use std::time::{Duration, Instant};

    let tmp = tempfile::tempdir().expect("tempdir");
    let runtime =
        super::spawn_pty_terminal(Some(tmp.path().to_string_lossy().as_ref()), super::Shell::Bash)
            .expect("pty terminal");

    {
        let mut writer = runtime.writer.lock().expect("writer lock");
        writer.write_all(b"pwd\nexit\n").expect("write command");
        writer.flush().expect("flush command");
    }

    let _ = runtime.child.lock().expect("child lock").wait();
    let started = Instant::now();
    let output = loop {
        let bytes = runtime.output.lock().expect("output lock").iter().copied().collect::<Vec<_>>();
        let text = String::from_utf8_lossy(&bytes).to_string();
        if text.contains(tmp.path().to_string_lossy().as_ref())
            || started.elapsed() > Duration::from_secs(2)
        {
            break text;
        }
        std::thread::sleep(Duration::from_millis(20));
    };

    assert!(output.contains(tmp.path().to_string_lossy().as_ref()), "{output}");
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
#[test]
fn close_terminal_defers_drop_until_reap_to_avoid_iced_term_channel_panic() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut state = super::TerminalState::new(
        true,
        super::Shell::Bash,
        super::TerminalTheme::System,
        "JetBrains Mono".to_string(),
        13.0,
        Some(tmp.path().to_path_buf()),
    );
    assert!(state.add_terminal(Some(tmp.path().to_path_buf())));
    assert_eq!(state.tabs.len(), 2);

    // Keep a live subscription stream (same as App::subscription) while closing.
    let subscription = state.tabs[0].term.subscription();
    let closed_id = state.tabs[0].id;
    assert!(state.close_terminal(closed_id));
    assert!(state.has_retired());
    assert!(state.tabs.iter().all(|tab| tab.id != closed_id));

    // Dropping the subscription future first mirrors iced cancelling recipes on
    // rebuild; only then is it safe to drop the retired Terminal.
    drop(subscription);
    state.reap_retired();
    assert!(!state.has_retired());
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
#[test]
fn absorb_replaced_defers_live_tabs_from_previous_terminal_state() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let old = super::TerminalState::new(
        true,
        super::Shell::Bash,
        super::TerminalTheme::System,
        "JetBrains Mono".to_string(),
        13.0,
        Some(tmp.path().to_path_buf()),
    );
    assert!(!old.tabs.is_empty());

    let mut next = super::TerminalState::blank_with_settings(
        false,
        super::Shell::Bash,
        super::TerminalTheme::System,
        "JetBrains Mono".to_string(),
        13.0,
        200.0,
    );
    next.absorb_replaced(old);
    assert!(next.tabs.is_empty());
    assert!(next.has_retired());
    next.reap_retired();
    assert!(!next.has_retired());
}
