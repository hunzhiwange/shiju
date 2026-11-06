// 验证终端外观配置更新及稳定持久化键选择。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("config_tests"));
}

fn terminal() -> crate::app::TerminalState {
    crate::app::TerminalState::default()
}

#[test]
fn shell_selection_updates_state_and_returns_stable_config_key() {
    for (shell, expected_key) in
        [(crate::app::Shell::Bash, "bash"), (crate::app::Shell::Zsh, "zsh")]
    {
        let mut terminal = terminal();

        let update = super::apply_update(
            &mut terminal,
            &iced::Theme::Dark,
            super::TerminalMessage::ShellSelected(shell),
        );

        assert_eq!(terminal.shell, shell);
        assert!(matches!(
            update,
            Some(super::TerminalConfigUpdate::Shell(key)) if key == expected_key
        ));
    }
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn system_theme_selection_updates_state_and_returns_config_key() {
    let mut terminal = terminal();

    let update = super::apply_update(
        &mut terminal,
        &iced::Theme::Dark,
        super::TerminalMessage::ThemeSelected(crate::app::TerminalTheme::System),
    );

    assert_eq!(terminal.theme, crate::app::TerminalTheme::System);
    assert!(matches!(update, Some(super::TerminalConfigUpdate::Theme("system"))));
}

#[test]
fn font_selection_preserves_exact_family_for_state_and_persistence() {
    let mut terminal = terminal();

    let update = super::apply_update(
        &mut terminal,
        &iced::Theme::Dark,
        super::TerminalMessage::FontSelected("  Fira Code  ".to_string()),
    );

    assert_eq!(terminal.font_family, "  Fira Code  ");
    assert!(matches!(
        update,
        Some(super::TerminalConfigUpdate::FontFamily(family)) if family == "  Fira Code  "
    ));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn font_size_selection_preserves_boundary_values() {
    for size in [0.0, 18.5, f32::INFINITY] {
        let mut terminal = terminal();

        let update = super::apply_update(
            &mut terminal,
            &iced::Theme::Dark,
            super::TerminalMessage::FontSizeChanged(size),
        );

        assert_eq!(terminal.font_size, size);
        assert!(matches!(
            update,
            Some(super::TerminalConfigUpdate::FontSize(value)) if value == size
        ));
    }
}

#[test]
fn unrelated_message_leaves_terminal_unchanged_and_skips_persistence() {
    let mut terminal = terminal();
    let original_shell = terminal.shell;
    let original_theme = terminal.theme;
    let original_family = terminal.font_family.clone();
    let original_size = terminal.font_size;

    let update =
        super::apply_update(&mut terminal, &iced::Theme::Dark, super::TerminalMessage::DragStarted);

    assert!(update.is_none());
    assert_eq!(terminal.shell, original_shell);
    assert_eq!(terminal.theme, original_theme);
    assert_eq!(terminal.font_family, original_family);
    assert_eq!(terminal.font_size, original_size);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn update_with_unrelated_message_leaves_app_terminal_unchanged() {
    let (mut app, _task) = crate::app::App::new();
    let original_shell = app.terminal.shell;
    let original_theme = app.terminal.theme;
    let original_family = app.terminal.font_family.clone();
    let original_size = app.terminal.font_size;

    let _task = super::update(&mut app, super::TerminalMessage::DragStarted);

    assert_eq!(app.terminal.shell, original_shell);
    assert_eq!(app.terminal.theme, original_theme);
    assert_eq!(app.terminal.font_family, original_family);
    assert_eq!(app.terminal.font_size, original_size);
}

#[test]
fn consecutive_settings_merge_latest_values_and_ignore_stale_timers() {
    let mut persistence = super::ConfigPersistence::default();
    let stale = persistence.enqueue(super::TerminalConfigUpdate::FontSize(12.0));
    persistence.enqueue(super::TerminalConfigUpdate::Shell("zsh"));
    let latest = persistence.enqueue(super::TerminalConfigUpdate::FontSize(17.0));

    assert!(persistence.begin(stale).is_none());
    assert_eq!(
        persistence.begin(latest),
        Some(serde_json::json!({"terminal_font_size": 17.0, "terminal_shell": "zsh"}))
    );
    assert!(persistence.begin(latest).is_none());
}

#[test]
fn edits_during_save_wait_and_preserve_latest_value() {
    let mut persistence = super::ConfigPersistence::default();
    let first = persistence.enqueue(super::TerminalConfigUpdate::FontSize(12.0));
    assert!(persistence.begin(first).is_some());
    let next = persistence.enqueue(super::TerminalConfigUpdate::FontSize(14.0));
    assert!(persistence.begin(next).is_none());
    let latest = persistence.enqueue(super::TerminalConfigUpdate::FontSize(18.0));
    persistence.saving = false;
    assert_eq!(persistence.begin(latest), Some(serde_json::json!({"terminal_font_size": 18.0})));
    persistence.saving = false;
    assert!(persistence.begin(latest).is_none());
}

#[test]
fn font_size_drag_updates_display_without_resizing_or_saving_until_release() {
    let mut app = crate::app::App::new().0;
    app.terminal.font_size = 13.0;
    for index in 0..1_000 {
        let size = 10.0 + (index % 9) as f32;
        let _ = super::update(&mut app, super::TerminalMessage::FontSizePreview(size));
        assert_eq!(app.terminal.font_size, 13.0);
        assert_eq!(app.terminal_config_persistence.displayed_font_size(13.0), size);
        assert!(app.terminal_config_persistence.pending.is_empty());
    }
    let _ = super::update(&mut app, super::TerminalMessage::FontSizePreview(18.0));
    let _ = super::update(&mut app, super::TerminalMessage::FontSizePreviewCommit);
    assert_eq!(app.terminal.font_size, 18.0);
    assert_eq!(app.terminal_config_persistence.pending["terminal_font_size"], 18.0);
    let saved_generation = app.terminal_config_persistence.generation;
    let _ = super::update(&mut app, super::TerminalMessage::FontSizePreviewCommit);
    assert_eq!(app.terminal_config_persistence.generation, saved_generation);
}

#[test]
fn font_size_idle_commit_ignores_old_timers_and_supports_non_drag_input() {
    let mut app = crate::app::App::new().0;
    app.terminal.font_size = 13.0;
    let _ = super::update(&mut app, super::TerminalMessage::FontSizePreview(14.0));
    let old_generation = app.terminal_config_persistence.font_size_generation;
    let _ = super::update(&mut app, super::TerminalMessage::FontSizePreview(16.0));
    let latest = app.terminal_config_persistence.font_size_generation;
    let _ =
        super::update(&mut app, super::TerminalMessage::FontSizePreviewCommitDue(old_generation));
    assert_eq!(app.terminal.font_size, 13.0);
    assert!(app.terminal_config_persistence.pending.is_empty());
    let _ = super::update(&mut app, super::TerminalMessage::FontSizePreviewCommitDue(latest));
    assert_eq!(app.terminal.font_size, 16.0);
    assert_eq!(app.terminal_config_persistence.pending["terminal_font_size"], 16.0);
    let saved_generation = app.terminal_config_persistence.generation;
    let _ = super::update(&mut app, super::TerminalMessage::FontSizePreviewCommitDue(latest));
    assert_eq!(app.terminal_config_persistence.generation, saved_generation);
}
