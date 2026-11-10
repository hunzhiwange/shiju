//! Exercise the permission and confirmation dialogs through Iced's headless layout, event and draw paths.
#![cfg(not(target_arch = "wasm32"))]

use super::confirm_dialog;
use crate::app::i18n::I18n;
use crate::app::message::{ChatMessage, ProjectMessage, SkillHubMessage};
use crate::app::skill_hub::SkillCacheSyncState;
use crate::app::state::ToastKind;
use crate::app::{App, Message};
use iced::advanced::{
    Layout, clipboard, layout,
    renderer::{Headless, Style},
    widget::{Id, Operation, Tree, operation},
};
use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_33_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let home = tempfile::tempdir().unwrap();
    let config = GatewayClientSystemSettingsConfig {
        host: "[".into(),
        servers: Vec::new(),
        ..Default::default()
    };
    std::fs::write(
        home.path().join("gateway-client-bootstrap.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("{}::{test}", module_path!().split_once("::").unwrap().1)])
        .env(CHILD, "1")
        .env("HOME", home.path())
        .env("VIBEWINDOW_CONFIG_DIR", home.path())
        .env("VIBEWINDOW_WORKSPACE", home.path())
        .current_dir(home.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed; 0 failed"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    false
}

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    App::new().0
}

async fn renderer() -> Renderer {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .unwrap()
}

fn scene<'a>(view: Element<'a, Message>, renderer: &mut Renderer, size: Size<u32>) -> Ui<'a> {
    UserInterface::build(
        view,
        Size::new(size.width as f32, size.height as f32),
        Cache::new(),
        renderer,
    )
}

fn send(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    cursor: mouse::Cursor,
    events: &[Event],
) -> Vec<Message> {
    let mut messages = Vec::new();
    ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
    messages
}

fn click(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point) -> Vec<Message> {
    send(
        ui,
        renderer,
        mouse::Cursor::Available(point),
        &[
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
    )
}

fn key(ui: &mut Ui<'_>, renderer: &mut Renderer, key: Key, text: Option<&str>) -> Vec<Message> {
    send(
        ui,
        renderer,
        mouse::Cursor::Unavailable,
        &[Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: text.map(Into::into),
            repeat: false,
        })],
    )
}

fn capture(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
    cursor: mouse::Cursor,
) -> Vec<u8> {
    assert!(
        send(
            ui,
            renderer,
            cursor,
            &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
        )
        .is_empty()
    );
    ui.draw(renderer, theme, &Style { text_color: theme.palette().text }, cursor);
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_33_SCREENSHOTS") {
        ::image::save_buffer(
            std::path::PathBuf::from(directory).join(format!("{name}.png")),
            &pixels,
            size.width,
            size.height,
            ::image::ColorType::Rgba8,
        )
        .unwrap();
    }
    pixels
}

#[derive(Default)]
struct Geometry {
    labels: Vec<(String, Rectangle)>,
    inputs: Vec<(String, Rectangle)>,
    scroll: Vec<(Rectangle, iced::Vector)>,
}

impl Operation for Geometry {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, value: &str) {
        self.labels.push((value.into(), bounds));
    }

    fn text_input(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        state: &mut dyn operation::TextInput,
    ) {
        self.inputs.push((state.text().into(), bounds));
    }

    fn scrollable(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        _: Rectangle,
        translation: iced::Vector,
        _: &mut dyn operation::Scrollable,
    ) {
        self.scroll.push((bounds, translation));
    }
}

fn geometry(ui: &mut Ui<'_>, renderer: &Renderer) -> Geometry {
    let mut geometry = Geometry::default();
    ui.operate(renderer, &mut geometry);
    geometry
}

fn label(geometry: &Geometry, value: &str) -> Rectangle {
    geometry
        .labels
        .iter()
        .find(|(label, _)| label == value)
        .unwrap_or_else(|| panic!("missing {value}: {:?}", geometry.labels))
        .1
}

fn find_layout<'a>(
    layout: Layout<'a>,
    matches: impl Fn(Rectangle) -> bool + Copy,
) -> Option<Layout<'a>> {
    if matches(layout.bounds()) {
        return Some(layout);
    }
    layout.children().find_map(|child| find_layout(child, matches))
}

fn assert_centered_controls(
    mut view: Element<'_, Message>,
    renderer: &Renderer,
    title: Rectangle,
    input: Rectangle,
    toast: Rectangle,
) -> Point {
    let mut tree = Tree::new(view.as_widget());
    let limits =
        layout::Limits::new(Size::ZERO, Size::new(NARROW.width as f32, NARROW.height as f32));
    let node = view.as_widget_mut().layout(&mut tree, renderer, &limits);
    let root = Layout::new(&node);
    let close = find_layout(root, |bounds| {
        bounds.width == 28.0
            && bounds.height == 28.0
            && (bounds.center_y() - title.center_y()).abs() < 0.5
            && bounds.x > title.x
    })
    .expect("close button aligned with the title");
    let icon = find_layout(close, |bounds| bounds.width == 12.0 && bounds.height == 12.0)
        .expect("close icon");
    assert!((close.bounds().center_y() - icon.bounds().center_y()).abs() < 0.5);
    let field = find_layout(root, |bounds| bounds == input).expect("custom input");
    let glyphs = field.children().next().expect("input text");
    assert!((input.center_y() - glyphs.bounds().center_y()).abs() < 0.5);
    let badge = find_layout(root, |bounds| {
        bounds.width == 34.0
            && bounds.height == 34.0
            && bounds.x < toast.x
            && bounds.y <= toast.center_y()
            && bounds.y + bounds.height >= toast.center_y()
    })
    .expect("toast badge");
    let icon = find_layout(badge, |bounds| bounds.width == 14.0 && bounds.height == 14.0)
        .expect("toast icon");
    assert!((badge.bounds().center_y() - icon.bounds().center_y()).abs() < 0.5);
    close.bounds().center()
}

fn permission(id: &str, title: &str) -> vw_gateway_client::PendingPermissionRequestDto {
    vw_gateway_client::PendingPermissionRequestDto {
        id: id.into(),
        session_id: "fixture-session".into(),
        permission: "shell".into(),
        patterns: vec!["/fixture/工作區/".to_owned() + &"example/".repeat(12)],
        metadata: serde_json::Map::from_iter([
            ("title".into(), serde_json::json!(title)),
            ("reason".into(), serde_json::json!("Allow this fixture command. ".repeat(22))),
            ("arguments".into(), serde_json::json!("echo 測試資料")),
        ]),
        always: vec!["echo *".into()],
        tool: None,
    }
}

const NARROW: Size<u32> = Size::new(420, 480);
const TITLE: &str = "权限审查 · 權限審查 · Permission review";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn toast_text_wraps_as_needed_and_leaves_room_for_retry() {
    if !isolated("toast_text_wraps_as_needed_and_leaves_room_for_retry") {
        return;
    }
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    let mut renderer = renderer().await;
    let _ = app.show_localized_toast(ToastKind::Success, "已删除", vec![]);
    let mut ui = scene(app.view(), &mut renderer, NARROW);
    let short = label(&geometry(&mut ui, &renderer), "操作已完成 · 已删除");
    assert!(short.height < 20.0, "{short:?}");
    for (theme, name) in [(Theme::Light, "light-short-toast"), (Theme::Dark, "dark-short-toast")] {
        capture(&mut ui, &mut renderer, &theme, NARROW, name, mouse::Cursor::Unavailable);
    }
    drop(ui);

    let path = "/fixture/".to_owned() + &"filename".repeat(16);
    let _ = app.show_toast(ToastKind::Error, path.clone());
    let mut ui = scene(app.view(), &mut renderer, NARROW);
    let long = label(&geometry(&mut ui, &renderer), &format!("操作失败 · {path}"));
    assert!(long.height > short.height && long.x + long.width <= NARROW.width as f32, "{long:?}");
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        NARROW,
        "dark-long-toast",
        mouse::Cursor::Unavailable,
    );
    drop(ui);

    app.expert_hub.cache_sync =
        SkillCacheSyncState::PendingRetry { skill_id: 7, refresh_detail: false };
    let _ = app.show_toast(ToastKind::Warning, format!("本地缓存待同步：{path}"));
    let mut ui = scene(app.view(), &mut renderer, NARROW);
    let retry = label(&geometry(&mut ui, &renderer), "重试");
    assert!(retry.width > 0.0 && retry.x + retry.width <= NARROW.width as f32, "{retry:?}");
    assert!(matches!(
        click(&mut ui, &mut renderer, retry.center()).as_slice(),
        [Message::SkillHub(SkillHubMessage::RetrySkillCacheSync)]
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_33_layout_and_interaction() {
    if !isolated("ui_refresh_33_layout_and_interaction") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    let current = permission("fixture-current", TITLE);
    let mut next = permission("fixture-next", "Next fixture");
    next.metadata.insert("arguments".into(), serde_json::json!("echo 下一個"));
    app.permission_modal_request = Some(current.clone());
    app.permission_modal_requests = vec![current, next];
    let _ = app.show_toast(ToastKind::Info, "Fixture notice");
    for (locale, language) in [
        (DesktopLocale::English, "en"),
        (DesktopLocale::SimplifiedChinese, "zh"),
        (DesktopLocale::TraditionalChinese, "zh-tw"),
    ] {
        app.desktop_locale = locale;
        let i18n = I18n::new(locale);
        let toast_label = format!("{} · Fixture notice", i18n.__("提示"));
        for (theme, mode) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
            for size in [NARROW, Size::new(640, 560), Size::new(960, 720)] {
                let mut ui = scene(app.view(), &mut renderer, size);
                let bounds = geometry(&mut ui, &renderer);
                capture(
                    &mut ui,
                    &mut renderer,
                    &theme,
                    size,
                    &format!("{language}-{mode}-permission-{}", size.width),
                    mouse::Cursor::Unavailable,
                );
                let approve = label(&bounds, i18n.__("仅此一次"));
                let reject = label(&bounds, i18n.__("拒绝"));
                for action in [approve, reject] {
                    assert!(action.x >= 0.0 && action.y >= 0.0);
                    assert!(action.x + action.width <= size.width as f32, "{action:?}");
                    assert!(action.y + action.height <= size.height as f32, "{action:?}");
                }
                let Some(&(viewport, initial)) = bounds.scroll.first() else {
                    panic!("permission details must scroll independently from actions");
                };
                assert!(label(&bounds, &toast_label).y > approve.y + approve.height);
                let (_, selected) = bounds
                    .labels
                    .iter()
                    .find(|(value, _)| value.ends_with(" · echo 測試資料"))
                    .expect("selected request");
                assert!(click(&mut ui, &mut renderer, selected.center()).is_empty());
                assert!(matches!(
                    click(&mut ui, &mut renderer, label(&bounds, i18n.__("下一项")).center()).as_slice(),
                    [Message::Chat(ChatMessage::PermissionSelectRequest(id))] if id == "fixture-next"
                ));
                for key_name in [Named::Enter, Named::Escape] {
                    assert!(key(&mut ui, &mut renderer, Key::Named(key_name), None).is_empty());
                }
                assert!(
                    send(
                        &mut ui,
                        &mut renderer,
                        mouse::Cursor::Available(viewport.center()),
                        &[Event::Mouse(mouse::Event::WheelScrolled {
                            delta: mouse::ScrollDelta::Lines { x: 0.0, y: -8.0 },
                        })],
                    )
                    .is_empty()
                );
                let scrolled = geometry(&mut ui, &renderer);
                assert!(
                    scrolled
                        .scroll
                        .iter()
                        .any(|(rect, offset)| *rect == viewport && offset.y > initial.y)
                );
                for label_text in [TITLE, i18n.__("仅此一次"), i18n.__("拒绝")] {
                    assert_eq!(label(&bounds, label_text), label(&scrolled, label_text));
                }
                if size == NARROW {
                    capture(
                        &mut ui,
                        &mut renderer,
                        &theme,
                        size,
                        &format!("{language}-{mode}-permission-scrolled"),
                        mouse::Cursor::Available(approve.center()),
                    );
                }
                assert!(matches!(
                    click(&mut ui, &mut renderer, label(&bounds, i18n.__("始终允许")).center())
                        .as_slice(),
                    [Message::Chat(ChatMessage::PermissionApproveAlways)]
                ));
                assert!(matches!(
                    click(&mut ui, &mut renderer, label(&bounds, i18n.__("全部始终允许")).center())
                        .as_slice(),
                    [Message::Chat(ChatMessage::PermissionApproveAllAlways)]
                ));
                assert!(matches!(
                    click(&mut ui, &mut renderer, approve.center()).as_slice(),
                    [Message::Chat(ChatMessage::PermissionApproveOnce)]
                ));
                assert!(matches!(
                    click(&mut ui, &mut renderer, reject.center()).as_slice(),
                    [Message::Chat(ChatMessage::PermissionReject)]
                ));
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_33_confirmation_preserves_messages() {
    if !isolated("ui_refresh_33_confirmation_preserves_messages") {
        return;
    }
    const PATH: &str = "/fixture/測試";
    let mut app = fixture();
    app.recent_projects = vec![PATH.into()];
    let mut renderer = renderer().await;
    for (theme, mode) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        app.recent_project_remove_confirm_path = Some(PATH.into());
        let dialog = confirm_dialog(
            "移除項目 · Remove fixture",
            "移除虚构记录。 Remove the fixture record. ".repeat(24),
            "Remove fixture",
            "Cancel",
            Message::Project(ProjectMessage::RecentRemoveConfirmed(PATH.into())),
            Message::Project(ProjectMessage::RecentRemoveCanceled),
        );
        let mut ui = scene(dialog, &mut renderer, NARROW);
        let bounds = geometry(&mut ui, &renderer);
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            NARROW,
            &format!("{mode}-confirm"),
            mouse::Cursor::Unavailable,
        );
        let confirm = label(&bounds, "Remove fixture");
        let cancel = label(&bounds, "Cancel");
        let (viewport, _) = bounds.scroll[0];
        assert!(
            send(
                &mut ui,
                &mut renderer,
                mouse::Cursor::Available(viewport.center()),
                &[Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Lines { x: 0.0, y: -10.0 },
                })],
            )
            .is_empty()
        );
        let after = geometry(&mut ui, &renderer);
        assert!(after.scroll[0].1.y > 0.0);
        assert_eq!(confirm, label(&after, "Remove fixture"));
        assert_eq!(cancel, label(&after, "Cancel"));
        for key_name in [Named::Enter, Named::Escape] {
            assert!(key(&mut ui, &mut renderer, Key::Named(key_name), None).is_empty());
        }
        assert!(matches!(
            click(&mut ui, &mut renderer, confirm.center()).as_slice(),
            [Message::Project(ProjectMessage::RecentRemoveConfirmed(path))] if path == PATH
        ));
        let messages = click(&mut ui, &mut renderer, cancel.center());
        assert!(matches!(
            messages.as_slice(),
            [Message::Project(ProjectMessage::RecentRemoveCanceled)]
        ));
        let _ = app.update(messages.into_iter().next().unwrap());
        assert!(app.recent_project_remove_confirm_path.is_none());
        assert_eq!(app.recent_projects, [PATH]);
    }
}

fn question_option(label: &str, preview: Option<&str>) -> vw_shared::question::OptionInfo {
    vw_shared::question::OptionInfo {
        label: label.into(),
        description: "虚构选项 · 虛構選項 · Fictional option".into(),
        preview: preview.map(str::to_owned),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_33_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_33_preserves_values_and_disabled_states") {
        return;
    }
    const SINGLE: &str = "单选 · 單選 · Single";
    const MULTI: &str = "多选 · 多選 · Multiple";
    const A: &str = "一 · One";
    const B: &str = "二 · Two";
    const C: &str = "三 · Three";
    const D: &str = "四 · Four";
    const DRAFT: &str = "草稿 · Draft";
    let mut app = fixture();
    let mut renderer = renderer().await;
    let request = vw_shared::question::Request {
        id: "fixture-question".into(),
        session_id: "fixture-session".into(),
        questions: vec![
            vw_shared::question::Info {
                header: SINGLE.into(),
                question: "Choose a fixture option.".into(),
                options: vec![
                    question_option(A, Some("示例预览 · Preview")),
                    question_option(B, None),
                ],
                multiple: Some(false),
                custom: Some(true),
            },
            vw_shared::question::Info {
                header: MULTI.into(),
                question: MULTI.into(),
                options: vec![question_option(C, None), question_option(D, None)],
                multiple: Some(true),
                custom: Some(false),
            },
        ],
        tool: None,
        mcp_form: None,
    };
    let _ = app.show_toast(ToastKind::Error, "Fixture error · 測試錯誤");
    for (locale, language) in [
        (DesktopLocale::English, "en"),
        (DesktopLocale::SimplifiedChinese, "zh"),
        (DesktopLocale::TraditionalChinese, "zh-tw"),
    ] {
        app.desktop_locale = locale;
        let i18n = I18n::new(locale);
        let toast_label = format!("{} · Fixture error · 測試錯誤", i18n.__("操作失败"));
        for (theme, mode) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
            app.question_modal_request = Some(request.clone());
            app.question_modal_request_id = Some(request.id.clone());
            app.question_modal_answers = vec![vec![A.into()], vec![C.into()]];
            app.question_modal_custom = vec![DRAFT.into(), String::new()];
            let mut ui = scene(app.view(), &mut renderer, NARROW);
            let bounds = geometry(&mut ui, &renderer);
            let submit = label(&bounds, i18n.__("确定"));
            let cancel = label(&bounds, i18n.__("取消"));
            assert!(label(&bounds, &toast_label).y > submit.y + submit.height);
            let option = label(&bounds, B).center();
            let neutral = capture(
                &mut ui,
                &mut renderer,
                &theme,
                NARROW,
                &format!("{language}-{mode}-question"),
                mouse::Cursor::Unavailable,
            );
            let hovered = capture(
                &mut ui,
                &mut renderer,
                &theme,
                NARROW,
                &format!("{language}-{mode}-question-hover"),
                mouse::Cursor::Available(option),
            );
            assert_ne!(neutral, hovered, "option hover must be visible");
            assert_eq!(
                neutral,
                capture(
                    &mut ui,
                    &mut renderer,
                    &theme,
                    NARROW,
                    &format!("{language}-{mode}-question-left"),
                    mouse::Cursor::Unavailable,
                ),
                "hover must clear after cursor leaves",
            );
            let [(_, input)] = bounds.inputs.as_slice() else {
                panic!("single-choice custom value must have one input");
            };
            assert_eq!(bounds.inputs[0].0, DRAFT);
            let close = assert_centered_controls(
                app.view(),
                &renderer,
                label(&bounds, SINGLE),
                *input,
                label(&bounds, &toast_label),
            );
            assert!(matches!(
                click(&mut ui, &mut renderer, close).as_slice(),
                [Message::Chat(ChatMessage::QuestionReject)]
            ));
            assert!(
                click(
                    &mut ui,
                    &mut renderer,
                    Point::new(input.x + input.width - 7.0, input.center_y())
                )
                .is_empty()
            );
            let changed = key(&mut ui, &mut renderer, Key::Character("x".into()), Some("x"));
            assert!(matches!(
                changed.as_slice(),
                [Message::Chat(ChatMessage::QuestionCustomChanged(0, value))] if value == &format!("{DRAFT}x")
            ));
            for name in [Named::Enter, Named::Escape] {
                assert!(
                    !key(&mut ui, &mut renderer, Key::Named(name), None).iter().any(
                        |message| matches!(message, Message::Chat(ChatMessage::QuestionSubmit))
                    )
                );
            }
            assert!(matches!(
                click(&mut ui, &mut renderer, submit.center()).as_slice(),
                [Message::Chat(ChatMessage::QuestionSubmit)]
            ));
            assert!(matches!(
                click(&mut ui, &mut renderer, cancel.center()).as_slice(),
                [Message::Chat(ChatMessage::QuestionReject)]
            ));
            let single = click(&mut ui, &mut renderer, option);
            assert!(matches!(
                single.as_slice(),
                [Message::Chat(ChatMessage::QuestionOptionToggled(0, value))] if value == B
            ));
            drop(ui);
            let _ = app.update(single.into_iter().next().unwrap());
            assert_eq!(app.question_modal_answers[0], [B]);

            let mut ui = scene(app.view(), &mut renderer, NARROW);
            let before = geometry(&mut ui, &renderer);
            let (viewport, _) = before.scroll[0];
            assert!(
                send(
                    &mut ui,
                    &mut renderer,
                    mouse::Cursor::Available(viewport.center()),
                    &[Event::Mouse(mouse::Event::WheelScrolled {
                        delta: mouse::ScrollDelta::Lines { x: 0.0, y: -20.0 },
                    })],
                )
                .is_empty()
            );
            let after = geometry(&mut ui, &renderer);
            assert_eq!(label(&before, SINGLE), label(&after, SINGLE));
            assert_eq!(label(&before, i18n.__("确定")), label(&after, i18n.__("确定")));
            let translation = after.scroll[0].1;
            assert!(translation.y > 0.0);
            let multi = click(&mut ui, &mut renderer, label(&after, D).center() - translation);
            assert!(matches!(
                multi.as_slice(),
                [Message::Chat(ChatMessage::QuestionOptionToggled(1, value))] if value == D
            ));
            drop(ui);
            let _ = app.update(multi.into_iter().next().unwrap());
            assert_eq!(app.question_modal_answers[1], [C, D]);
        }
    }

    let _ = app.show_toast(ToastKind::Error, "Fixture error · 測試錯誤. ".repeat(40));
    let mut ui = scene(app.view(), &mut renderer, NARROW);
    let i18n = I18n::new(app.desktop_locale);
    let submit = label(&geometry(&mut ui, &renderer), i18n.__("确定"));
    assert!(submit.y + submit.height < NARROW.height as f32);
    assert!(matches!(
        click(&mut ui, &mut renderer, submit.center()).as_slice(),
        [Message::Chat(ChatMessage::QuestionSubmit)]
    ));
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        NARROW,
        "dark-question-long-toast",
        mouse::Cursor::Unavailable,
    );
    drop(ui);
    app.active_toast = None;
    app.question_modal_request.as_mut().unwrap().questions.clear();
    let mut ui = scene(app.view(), &mut renderer, NARROW);
    let bounds = geometry(&mut ui, &renderer);
    assert!(bounds.inputs.is_empty());
    label(&bounds, I18n::new(app.desktop_locale).__("需要确认"));
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        NARROW,
        "dark-question-empty",
        mouse::Cursor::Unavailable,
    );
}
