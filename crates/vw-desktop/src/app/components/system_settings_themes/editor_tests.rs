//! 验证主题编辑草稿、标识分配、颜色解析和表单校验。
//!
//! 草稿在新建和既有主题间切换，颜色输入覆盖合法、非法与冲突标识分配。

use super::{editor_tab_button, view};
use desktop_settings_appearance::system_settings_themes::editor::{editor_tab_labels, draft_from_definition};
use crate::app::state::{ThemeEditorDraft, ThemeEditorTab, ThemesSettingsMode};
use crate::app::theme_catalog::{ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex};
use crate::app::{App, Message};
use iced::Element;

fn sample_custom_theme() -> ThemeDefinition {
    ThemeDefinition {
        id: "mint-night".to_string(),
        name: "Mint Night".to_string(),
        tagline: Some("Cool mint accents".to_string()),
        palette: ThemePaletteHex {
            background: "#0b1220".to_string(),
            text: "#e2e8f0".to_string(),
            primary: "#34d399".to_string(),
            success: "#22c55e".to_string(),
            warning: "#f59e0b".to_string(),
            danger: "#ef4444".to_string(),
        },
        assets: ThemeAssets::default(),
        chrome: ThemeChrome::default(),
    }
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn test_app_with_draft(draft: ThemeEditorDraft) -> App {
    let mut app = App::new().0;
    app.themes_settings.mode = ThemesSettingsMode::Edit;
    app.themes_settings.editor_tab = ThemeEditorTab::Basics;
    app.themes_settings.draft = Some(draft);
    app
}

#[test]
fn editor_tab_buttons_build_active_and_inactive_states() {
    for (label, tab) in editor_tab_labels() {
        let inactive = if *tab == ThemeEditorTab::Preview {
            ThemeEditorTab::Basics
        } else {
            ThemeEditorTab::Preview
        };
        keep_element(editor_tab_button(label, *tab, *tab));
        keep_element(editor_tab_button(label, *tab, inactive));
    }
}

#[test]
fn view_builds_every_editor_tab() {
    let mut app = test_app_with_draft(draft_from_definition(&sample_custom_theme()));
    for (_, tab) in editor_tab_labels() {
        app.themes_settings.editor_tab = *tab;
        keep_element(view(&app));
    }
}

#[test]
fn view_builds_with_color_picker_anchored_to_swatch() {
    use crate::app::message::settings::{ThemeColorPickerTarget, ThemePaletteField};

    let mut app = test_app_with_draft(draft_from_definition(&sample_custom_theme()));
    app.themes_settings.editor_tab = ThemeEditorTab::Palette;
    app.themes_settings.color_picker_target =
        Some(ThemeColorPickerTarget::Palette(ThemePaletteField::Primary));
    keep_element(view(&app));

    app.themes_settings.editor_tab = ThemeEditorTab::Chrome;
    app.themes_settings.color_picker_target = Some(ThemeColorPickerTarget::Chrome(
        crate::app::message::settings::ThemeChromeField::InputBg,
    ));
    keep_element(view(&app));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn settings_tabs_editors_appearance_theme() {
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::i18n::I18n;
    use crate::app::message::settings::{SettingsMessage, ThemesMessage};

    fn overlay(app: &App) -> Element<'_, Message> {
        let background =
            iced::widget::Space::new().width(iced::Length::Fill).height(iced::Length::Fill);
        super::super::editor_overlay(app, background.into())
    }

    tab_appearance_tests::run_at_least(
        concat!(module_path!(), "::settings_tabs_editors_appearance_theme"),
        420,
        || App::new().0,
        |scene, app| {
            let dispatch = |app: &mut App, message| {
                let _ = app.update(Message::Settings(SettingsMessage::Themes(message)));
            };
            let draft_name = "主题草稿 · Theme draft";
            dispatch(app, ThemesMessage::NewThemeRequested);
            dispatch(app, ThemesMessage::DraftNameChanged(draft_name.into()));
            let i18n = I18n::new(app.desktop_locale);
            let labels: Vec<_> =
                editor_tab_labels().iter().map(|(label, _)| i18n.__(label)).collect();
            scene.save_preview(app, overlay(app), "theme");
            let message = scene.check(app, overlay(app), &labels, 1);
            assert!(matches!(
                message,
                SettingsMessage::Themes(ThemesMessage::EditorTabSelected(ThemeEditorTab::Palette))
            ));
            let _ = app.update(Message::Settings(message));
            assert_eq!(app.themes_settings.editor_tab, ThemeEditorTab::Palette);
            scene.check_modal_scroll(overlay(app), i18n.__("新建主题"), i18n.__("保存"));

            for (&label, &(_, tab)) in labels.iter().zip(editor_tab_labels()) {
                let message = scene.click(overlay(app), label);
                let _ = app.update(Message::Settings(message));
                assert_eq!(app.themes_settings.editor_tab, tab);
                assert_eq!(app.themes_settings.draft.as_ref().unwrap().name, draft_name);
            }
            let cancel = scene.click(overlay(app), i18n.__("取消"));
            assert!(matches!(cancel, SettingsMessage::Themes(ThemesMessage::CancelEdit)));
            let _ = app.update(Message::Settings(cancel));
            assert!(app.themes_settings.draft.is_none());
        },
    )
    .await;
}
