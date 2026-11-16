#![cfg(not(target_arch = "wasm32"))]
use super::{list_editor, view};
use crate::autonomy_message::Message;
use crate::autonomy_test_support::*;
use config_ui::DesktopLocale;
use desktop_i18n::I18n;
use desktop_settings_security::app_init::state::{
    AutonomyListKind, AutonomySettingsState, AutonomySettingsTab,
};
use desktop_ui_controls::system_settings_common::settings_field_label;
use iced::{Event, Size, Theme, mouse};

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_22_tool_menu_hover_scroll_and_add_keep_original_ids() {
    use iced::{Point, keyboard::key::Named};

    if !isolated(module_path!(), "ui_refresh_22_tool_menu_hover_scroll_and_add_keep_original_ids") {
        return;
    }
    let mut renderer = renderer().await;
    let tools: Vec<String> = (0..28).map(|index| format!("fixture_tool_{index:02}")).collect();
    let locale = DesktopLocale::SimplifiedChinese;
    let kind = AutonomyListKind::AutoApprove;
    let size = Size::new(640, 460);
    let saved = ["fixture_tool_00".to_owned(), "fixture_tool_03".to_owned()];
    let editor = |draft| {
        list_editor(
            locale,
            kind,
            settings_field_label("自动审批").into(),
            Some("这些工具无需额外审批即可执行。"),
            &saved,
            draft,
            None,
            &tools,
        )
    };
    let mut ui = scene(editor("fixture_tool_01"), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let trigger = positions.focusable[0];
    let add = positions.label("添加");
    assert!((trigger.center_y() - add.center_y()).abs() < 0.5);
    assert!(click(&mut ui, &mut renderer, trigger.center()).is_empty());

    let mut pixels = Vec::new();
    for (index, point) in [
        Point::new(trigger.center_x(), trigger.y + trigger.height + 30.0),
        Point::new(trigger.center_x(), trigger.y + trigger.height + 68.0),
        Point::new(2.0, 2.0),
    ]
    .into_iter()
    .enumerate()
    {
        let cursor = mouse::Cursor::Available(point);
        assert!(
            send(
                &mut ui,
                &mut renderer,
                cursor,
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })]
            )
            .is_empty()
        );
        pixels.push(capture(
            &mut ui,
            &mut renderer,
            &Theme::Dark,
            size,
            &format!("tool-menu-hover-{index}"),
            cursor,
        ));
    }
    assert_ne!(pixels[0], pixels[1]);
    assert_ne!(pixels[1], pixels[2]);
    let cursor =
        mouse::Cursor::Available(Point::new(trigger.center_x(), trigger.y + trigger.height + 68.0));
    assert!(
        send(
            &mut ui,
            &mut renderer,
            cursor,
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -192.0 },
            })]
        )
        .is_empty()
    );
    let scrolled =
        capture(&mut ui, &mut renderer, &Theme::Dark, size, "tool-menu-scrolled", cursor);
    assert_ne!(scrolled, pixels[1]);
    // Outside press dismisses the overlay and must not activate Add underneath it.
    assert!(click(&mut ui, &mut renderer, add.center()).is_empty());
    assert!(click(&mut ui, &mut renderer, trigger.center()).is_empty());
    let cursor = mouse::Cursor::Unavailable;
    assert!(send(&mut ui, &mut renderer, cursor, &[key(Named::End)]).is_empty());
    assert!(matches!(
        send(&mut ui, &mut renderer, cursor, &[key(Named::Enter)]).as_slice(),
        [Message::AutonomyListDraftChanged(AutonomyListKind::AutoApprove, id)]
            if id == "fixture_tool_27"
    ));
    drop(ui);

    let mut ui = scene(editor("fixture_tool_27"), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    for id in &saved {
        positions.label(id);
    }
    assert!(matches!(
        click(&mut ui, &mut renderer, positions.label("添加").center()).as_slice(),
        [Message::AutonomyListAdd(AutonomyListKind::AutoApprove)]
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_11_list_editor_explanations() {
    if !isolated(module_path!(), "settings_help_gap_11_list_editor_explanations") {
        return;
    }
    let mut state = AutonomySettingsState::default();
    state.allowed_commands = vec!["fixture-tool --inspect".into()];
    state.forbidden_paths =
        vec![
            "/workspace/虚构/測試/long-fixture-directory/another-long-folder/deny-only-this-root"
                .into();
            12
        ];
    let tools: Vec<String> = (0..28).map(|index| format!("fixture_tool_{index:02}")).collect();
    state.active_tab = AutonomySettingsTab::ToolsAndPaths;
    let kind = AutonomyListKind::AllowedCommands;
    state.list_editor.set_draft(kind, "fixture-command".into());
    state.list_editor.request_remove(kind, 0);
    let mut renderer = renderer().await;
    for (locale, theme) in HELP_LOCALE_THEMES {
        let app = inputs(&state, &tools, locale, &theme);
        let t = |key| I18n::new(locale).__(key);
        for width in [420, 760] {
            let size = Size::new(width, 1300);
            let mut ui = scene(view(&app), &mut renderer, size);
            for (title, old_description, help) in [
                (
                    "允许命令",
                    "Shell 白名单，每次添加一个命令。",
                    "Shell 白名单，每次添加一个命令。",
                ),
                (
                    "环境变量透传",
                    "允许透传到 shell 的环境变量名，每次添加一个。",
                    "允许透传到 shell 的环境变量名，每次添加一个。",
                ),
                (
                    "允许根目录",
                    "额外允许访问的目录，每次添加一个。",
                    "额外允许访问的目录，每次添加一个。",
                ),
                (
                    "自动审批",
                    "从现有工具中选择无需额外审批即可执行的工具。",
                    "在下拉列表中选择工具，再点击添加。",
                ),
                (
                    "始终询问",
                    "从现有工具中选择始终要求确认的工具。",
                    "在下拉列表中选择工具，再点击添加。",
                ),
            ] {
                assert_help_card(
                    &app,
                    &mut ui,
                    &mut renderer,
                    size,
                    t(title),
                    t(old_description),
                    t(help),
                );
            }
            let positions = geometry(&mut ui, &renderer);
            for hint in [
                "确认删除该列表项？",
                "限制代理访问这些路径，即使在工作区外也保持拒绝。",
                "这些工具无需额外审批即可执行。",
                "这些工具始终要求确认。",
            ] {
                positions.label(t(hint));
            }
            assert!(matches!(
                click(&mut ui, &mut renderer, positions.label(t("添加")).center()).as_slice(),
                [Message::AutonomyListAdd(AutonomyListKind::AllowedCommands)]
            ));
            assert!(matches!(
                click(&mut ui, &mut renderer, positions.label(t("确认删除")).center()).as_slice(),
                [Message::AutonomyListRemoveConfirmed(AutonomyListKind::AllowedCommands, 0)]
            ));
        }
    }
}
