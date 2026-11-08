//! 验证提交侧栏的选择来源、校验状态、帮助内容与按钮样式。
//! 通过文件、区块和新旧行选择验证提交就绪状态的优先级。

use crate::app::components::system_settings_common::settings_select;
use crate::app::message::{GitMessage, ViewMessage, git, view};
use crate::app::state::ConventionalCommitType;
use crate::app::{App, Message};
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::widget::operation::{Operation, Outcome, focusable};
use iced::advanced::{Layout, clipboard, layout, widget::Tree};
use iced::futures::StreamExt;
use iced::keyboard::{self, Key, key::Named};
use iced::widget::{button, container, svg};
use iced::{
    Background, Color, Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse,
};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::DesktopLocale;

#[test]
fn task_719_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("left_panel_tests.rs"));
}

fn test_app() -> crate::app::App {
    #[cfg(not(target_arch = "wasm32"))]
    let _isolation = crate::app::config::isolated_settings();
    App::new().0
}

fn assert_panel_state(
    app: &crate::app::App,
    hint: &'static str,
    button: super::left_panel::CommitButtonState,
) {
    let state = super::left_panel::commit_panel_state(app);
    assert_eq!(state.hint, hint);
    assert_eq!(state.button, button);
}

#[test]
fn help_copy_lists_commit_format_and_common_types() {
    assert_eq!(super::left_panel::COMMIT_HELP_TITLE, "约定式提交");

    for expected in [
        "约定式提交格式",
        "<类型>[可选作用域]: <摘要>",
        "[可选详细描述]",
        "核心提交类型",
        "feat:",
        "fix:",
        "docs:",
        "style:",
        "refactor:",
        "perf:",
        "test:",
        "build:",
        "ci:",
        "chore:",
        "revert:",
        "扩展与特殊类型",
        "init:",
        "config:",
        "release:",
        "deploy:",
        "merge:",
        "wip:",
        "typo:",
        "locale:",
    ] {
        assert!(super::left_panel::COMMIT_HELP_TEXT.contains(expected), "{expected}");
    }
}

#[test]
fn selected_changes_detects_each_selection_source() {
    let initial_app = test_app();
    assert!(!super::left_panel::has_selected_changes(&initial_app));

    let mut files_app = test_app();
    files_app.staged_files_selected.push("src/lib.rs".to_string());
    assert!(super::left_panel::has_selected_changes(&files_app));

    let mut hunks_app = test_app();
    hunks_app.staged_hunks_selected.push(("src/lib.rs".to_string(), 1));
    assert!(super::left_panel::has_selected_changes(&hunks_app));

    let mut lines_app = test_app();
    lines_app.staged_lines_selected.push(("src/lib.rs".to_string(), 7));
    assert!(super::left_panel::has_selected_changes(&lines_app));

    let mut app = test_app();
    app.staged_old_lines_selected.push(("src/lib.rs".to_string(), 3));
    assert!(super::left_panel::has_selected_changes(&app));
}

#[test]
fn commit_panel_state_covers_priority_and_validation_paths() {
    let mut app = test_app();
    app.show_git_diff_summary = true;
    app.git_commit_in_progress = true;
    app.git_commit_message = "   ".to_string();
    assert_panel_state(&app, "正在提交已选变更", super::left_panel::CommitButtonState::InProgress);

    let mut no_selection_app = test_app();
    no_selection_app.git_commit_message = "cover left panel".to_string();
    no_selection_app.git_commit_type = Some(ConventionalCommitType::Feat);
    assert_panel_state(
        &no_selection_app,
        "请选择至少一项变更后再提交",
        super::left_panel::CommitButtonState::NoSelection,
    );

    let mut app = test_app();
    app.show_git_diff_summary = false;
    app.git_commit_message = "   ".to_string();
    app.staged_files_selected.push("src/lib.rs".to_string());
    assert_panel_state(
        &app,
        "填写摘要后即可提交",
        super::left_panel::CommitButtonState::MissingSummary,
    );

    let mut app = test_app();
    app.show_git_diff_summary = true;
    app.git_commit_type = None;
    app.git_commit_message = "cover left panel".to_string();
    app.staged_files_selected.push("src/lib.rs".to_string());
    assert_panel_state(
        &app,
        "选择提交类型后即可提交",
        super::left_panel::CommitButtonState::MissingType,
    );

    let mut app = test_app();
    app.show_git_diff_summary = false;
    app.git_commit_message = "cover left panel".to_string();
    app.staged_files_selected.push("src/lib.rs".to_string());
    assert_panel_state(
        &app,
        "已满足提交条件，可直接提交",
        super::left_panel::CommitButtonState::Ready,
    );
}

#[test]
fn commit_panel_state_is_ready_for_complete_conventional_commit() {
    let mut app = test_app();
    app.show_git_diff_summary = true;
    app.git_commit_type = Some(ConventionalCommitType::Fix);
    app.git_commit_message = "  cover complete commit form  ".to_string();
    app.staged_old_lines_selected.push(("src/lib.rs".to_string(), 3));

    assert_panel_state(
        &app,
        "已满足提交条件，可直接提交",
        super::left_panel::CommitButtonState::Ready,
    );
}

#[test]
fn help_icon_style_uses_muted_text_color_for_all_statuses() {
    let theme = Theme::Dark;
    let expected = Some(theme.palette().text.scale_alpha(0.78));

    assert_eq!(super::left_panel::help_icon_style(&theme, svg::Status::Idle).color, expected);
    assert_eq!(super::left_panel::help_icon_style(&theme, svg::Status::Hovered).color, expected);
}

#[test]
fn help_button_style_covers_idle_hovered_pressed_and_disabled_statuses() {
    let theme = Theme::Dark;
    let active = super::left_panel::help_button_style(&theme, button::Status::Active);
    let hovered = super::left_panel::help_button_style(&theme, button::Status::Hovered);
    let pressed = super::left_panel::help_button_style(&theme, button::Status::Pressed);
    let disabled = super::left_panel::help_button_style(&theme, button::Status::Disabled);

    assert!(active.background.is_none());
    assert!(disabled.background.is_none());
    assert_eq!(
        hovered.background,
        Some(Background::Color(theme.extended_palette().background.weak.color.scale_alpha(0.72)))
    );
    assert_eq!(
        pressed.background,
        Some(Background::Color(theme.extended_palette().background.weak.color.scale_alpha(0.72)))
    );
    assert_eq!(pressed.text_color, theme.palette().text);
    assert_eq!(pressed.border.color, Color::TRANSPARENT);
}

#[test]
fn view_builds_disabled_state_when_nothing_selected() {
    let mut app = test_app();
    app.git_commit_message = "add panel tests".to_string();
    app.git_commit_type = Some(ConventionalCommitType::Feat);

    let _element = super::left_panel::view(&app);
}

#[test]
fn view_builds_simple_mode_with_missing_summary() {
    let mut app = test_app();
    app.show_git_diff_summary = false;
    app.git_commit_message = "   ".to_string();
    app.staged_files_selected.push("src/lib.rs".to_string());

    let _element = super::left_panel::view(&app);
}

#[test]
fn view_builds_summary_mode_with_missing_summary() {
    let mut app = test_app();
    app.show_git_diff_summary = true;
    app.git_commit_type = None;
    app.git_commit_message = "   ".to_string();
    app.staged_files_selected.push("src/lib.rs".to_string());

    let _element = super::left_panel::view(&app);
}

#[test]
fn view_builds_summary_mode_with_missing_type() {
    let mut app = test_app();
    app.show_git_diff_summary = true;
    app.git_commit_type = None;
    app.git_commit_message = "cover left panel".to_string();
    app.staged_files_selected.push("src/lib.rs".to_string());

    let _element = super::left_panel::view(&app);
}

#[test]
fn view_builds_ready_state_from_hunk_selection() {
    let mut app = test_app();
    app.show_git_diff_summary = true;
    app.git_commit_type = Some(ConventionalCommitType::Fix);
    app.git_commit_scope = "git-panel".to_string();
    app.git_commit_message = "cover left panel".to_string();
    app.staged_hunks_selected.push(("src/lib.rs".to_string(), 1));

    let _element = super::left_panel::view(&app);
}

#[test]
fn view_builds_ready_state_from_new_line_selection() {
    let mut app = test_app();
    app.show_git_diff_summary = false;
    app.git_commit_message = "cover selected line".to_string();
    app.staged_lines_selected.push(("src/lib.rs".to_string(), 7));

    let _element = super::left_panel::view(&app);
}

#[test]
fn view_builds_ready_state_from_old_line_selection() {
    let mut app = test_app();
    app.show_git_diff_summary = false;
    app.git_commit_message = "cover selected old line".to_string();
    app.staged_old_lines_selected.push(("src/lib.rs".to_string(), 3));

    let _element = super::left_panel::view(&app);
}

#[test]
fn view_builds_in_progress_commit_state() {
    let mut app = test_app();
    app.show_git_diff_summary = true;
    app.git_commit_type = Some(ConventionalCommitType::Fix);
    app.git_commit_scope = "git-panel".to_string();
    app.git_commit_message = "cover left panel".to_string();
    app.staged_hunks_selected.push(("src/lib.rs".to_string(), 1));

    app.git_commit_in_progress = true;
    app.file_manager_refresh_frame = 2;
    let _element = super::left_panel::view(&app);
}

async fn form_renderer() -> Renderer {
    static FONTS: std::sync::Once = std::sync::Once::new();
    FONTS.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    });
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .expect("tiny-skia renderer")
}

fn key_event(key: Key, text: Option<&str>) -> Event {
    Event::Keyboard(keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: text.map(Into::into),
        repeat: false,
    })
}

struct FormScene<'a> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    layout: layout::Node,
    size: Size<u32>,
}

impl<'a> FormScene<'a> {
    fn new(mut view: Element<'a, Message>, renderer: &mut Renderer, width: u32) -> Self {
        let size = Size::new(width, 760);
        let bounds = Size::new(width as f32, size.height as f32);
        let mut tree = Tree::new(view.as_widget());
        let node = view.as_widget_mut().layout(
            &mut tree,
            renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        let ui = UserInterface::build(view, bounds, Cache::new(), renderer);
        Self { ui, layout: node, size }
    }

    fn bounds(&self, path: &[usize]) -> Rectangle {
        path.iter()
            .fold(Layout::new(&self.layout), |node, &index| {
                node.children().nth(index).unwrap_or_else(|| panic!("missing layout {path:?}"))
            })
            .bounds()
    }

    fn operate(&mut self, renderer: &Renderer, mut op: Box<dyn Operation>) {
        loop {
            self.ui.operate(renderer, op.as_mut());
            match op.finish() {
                Outcome::Chain(next) => op = next,
                _ => break,
            }
        }
    }

    fn send(
        &mut self,
        renderer: &mut Renderer,
        event: Event,
        cursor: mouse::Cursor,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        self.ui.update(&[event], cursor, renderer, &mut clipboard::Null, &mut messages);
        messages
    }

    fn key(&mut self, renderer: &mut Renderer, named: Named) -> Vec<Message> {
        self.send(renderer, key_event(Key::Named(named), None), mouse::Cursor::Unavailable)
    }

    fn click(&mut self, renderer: &mut Renderer, point: Point) -> Vec<Message> {
        let cursor = mouse::Cursor::Available(point);
        let mut messages = self.send(
            renderer,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            cursor,
        );
        messages.extend(self.send(
            renderer,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            cursor,
        ));
        messages
    }

    fn type_text(&mut self, renderer: &mut Renderer, value: &str) -> Vec<Message> {
        self.send(
            renderer,
            key_event(Key::Character(value.into()), Some(value)),
            mouse::Cursor::Unavailable,
        )
    }

    fn capture(&mut self, renderer: &mut Renderer, theme: &Theme) -> Vec<u8> {
        self.send(
            renderer,
            Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now())),
            mouse::Cursor::Unavailable,
        );
        self.ui.draw(
            renderer,
            theme,
            &Style { text_color: theme.palette().text },
            mouse::Cursor::Unavailable,
        );
        renderer.screenshot(self.size, 1.0, theme.palette().background)
    }
}

fn assert_type_selected(messages: &[Message], expected: ConventionalCommitType) {
    assert!(
        matches!(
            messages,
            [Message::Git(GitMessage::CommitTypeSelected(selected))] if *selected == expected
        ),
        "{messages:?}"
    );
}

fn pixel(pixels: &[u8], width: u32, x: f32, y: f32) -> &[u8] {
    let index = (y as usize * width as usize + x as usize) * 4;
    &pixels[index..index + 4]
}

fn mark_pixels(pixels: &[u8], width: u32, trigger: Rectangle) -> usize {
    let x = trigger.x + trigger.width - 25.0;
    let y = trigger.y + trigger.height + 30.0;
    let background = pixel(pixels, width, x - 10.0, y);
    (-8..8)
        .flat_map(|dy| (-8..8).map(move |dx| (dx, dy)))
        .filter(|&(dx, dy)| pixel(pixels, width, x + dx as f32, y + dy as f32) != background)
        .count()
}

#[tokio::test]
async fn git_commit_form_matches_general_language_select() {
    let mut renderer = form_renderer().await;
    let mut app = test_app();
    app.show_git_diff_summary = true;

    for theme in [Theme::Light, Theme::Dark] {
        let mut language = FormScene::new(
            container(settings_select(
                ["简体中文", "繁體中文", "English"],
                Some("简体中文"),
                |_| Message::Git(GitMessage::CommitHelpOpen),
            ))
            .padding([0, 16])
            .into(),
            &mut renderer,
            640,
        );
        language.operate(&renderer, Box::new(focusable::focus_next()));
        assert!(language.key(&mut renderer, Named::Enter).is_empty());
        let language_pixels = language.capture(&mut renderer, &theme);
        let reference = language.bounds(&[0]);
        let reference_marks = mark_pixels(&language_pixels, 640, reference);
        assert!(reference_marks > 10);

        for locale in [
            DesktopLocale::SimplifiedChinese,
            DesktopLocale::TraditionalChinese,
            DesktopLocale::English,
        ] {
            app.desktop_locale = locale;
            for width in [320, 640] {
                app.git_commit_type = None;
                let mut form = FormScene::new(super::view(&app), &mut renderer, width);
                let trigger = form.bounds(&[2, 0, 0, 1, 0]);
                let scope = form.bounds(&[2, 0, 0, 1, 1]);
                let summary = form.bounds(&[2, 0, 0, 2]);
                let description = form.bounds(&[2, 0, 0, 3, 1]);
                assert_eq!(trigger.height, reference.height);
                assert_eq!(trigger.height, scope.height);
                assert_eq!(trigger.height, summary.height);
                assert!(scope.width >= 120.0 && summary.width >= 280.0);
                assert_eq!(description.height, 88.0);

                // Render the localized placeholder before selecting a type.
                let placeholder = form.capture(&mut renderer, &theme);
                drop(form);

                app.git_commit_type = Some(ConventionalCommitType::Feat);
                let mut form = FormScene::new(super::view(&app), &mut renderer, width);
                assert_ne!(placeholder, form.capture(&mut renderer, &theme));
                form.operate(&renderer, Box::new(focusable::focus_next()));
                let closed = form.capture(&mut renderer, &theme);
                assert!(form.key(&mut renderer, Named::Enter).is_empty());
                let open = form.capture(&mut renderer, &theme);
                let menu_sample = |pixels: &[u8]| {
                    pixel(pixels, width, trigger.center_x(), trigger.y + trigger.height + 24.0)
                        .to_vec()
                };
                assert_ne!(
                    menu_sample(&closed),
                    menu_sample(&open),
                    "focused Enter should open the type menu"
                );
                assert_eq!(mark_pixels(&open, width, trigger), reference_marks);

                assert!(form.key(&mut renderer, Named::End).is_empty());
                let scrolled = form.capture(&mut renderer, &theme);
                assert_ne!(scrolled, open);
                let last_row = Point::new(trigger.center_x(), trigger.y + trigger.height + 302.0);
                assert_type_selected(
                    &form.click(&mut renderer, last_row),
                    ConventionalCommitType::Locale,
                );
            }
        }
    }
}

async fn tab(form: &mut FormScene<'_>, renderer: &mut Renderer, task: iced::Task<Message>) {
    let mut messages = Vec::new();
    let (_, status) = form.ui.update(
        &[key_event(Key::Named(Named::Tab), None)],
        mouse::Cursor::Unavailable,
        renderer,
        &mut clipboard::Null,
        &mut messages,
    );
    assert!(messages.is_empty());
    assert_eq!(status, [iced::event::Status::Ignored]);
    let mut actions = iced_runtime::task::into_stream(task).expect("Tab focus task");
    let Some(iced_runtime::Action::Widget(operation)) = actions.next().await else {
        panic!("expected a widget operation");
    };
    form.operate(renderer, operation);
}

fn apply_form_message(app: &mut App, messages: Vec<Message>) {
    let messages: [Message; 1] = messages.try_into().expect("one form message");
    let [Message::Git(message)] = messages else {
        panic!("expected a Git message");
    };
    assert!(matches!(
        message,
        GitMessage::CommitTypeSelected(_)
            | GitMessage::CommitScopeChanged(_)
            | GitMessage::CommitMessageChanged(_)
            | GitMessage::CommitDescriptionEditorAction(_)
    ));
    assert!(iced_runtime::task::into_stream(git::update(app, message)).is_none());
}

#[tokio::test]
async fn git_commit_form_preserves_messages_and_validation() {
    use super::left_panel::CommitButtonState;

    let mut renderer = form_renderer().await;
    let mut app = test_app();
    app.show_git_diff_summary = true;
    app.file_manager_show_changes = true;
    app.git_commit_type = None;
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.staged_files_selected.push("src/example.rs".into());
    assert_panel_state(&app, "填写摘要后即可提交", CommitButtonState::MissingSummary);

    let window = iced::window::Id::unique();
    let [type_focus, scope_focus, summary_focus, description_focus] = std::array::from_fn(|_| {
        view::update(&mut app, ViewMessage::TraverseFormFocus(window, false))
    });
    let mut form = FormScene::new(super::view(&app), &mut renderer, 320);
    tab(&mut form, &mut renderer, type_focus).await;
    let closed = form.capture(&mut renderer, &Theme::Dark);
    assert!(form.key(&mut renderer, Named::Enter).is_empty());
    assert_ne!(closed, form.capture(&mut renderer, &Theme::Dark));
    assert!(form.key(&mut renderer, Named::Escape).is_empty());
    assert_eq!(closed, form.capture(&mut renderer, &Theme::Dark));
    assert!(form.key(&mut renderer, Named::Enter).is_empty());
    assert!(form.key(&mut renderer, Named::ArrowDown).is_empty());
    let selected = form.key(&mut renderer, Named::Enter);
    assert_type_selected(&selected, ConventionalCommitType::Fix);

    tab(&mut form, &mut renderer, scope_focus).await;
    let scope = form.type_text(&mut renderer, "界");
    assert!(
        matches!(scope.as_slice(), [Message::Git(GitMessage::CommitScopeChanged(value))] if value == "界")
    );
    tab(&mut form, &mut renderer, summary_focus).await;
    let summary = form.type_text(&mut renderer, "修");
    assert!(
        matches!(summary.as_slice(), [Message::Git(GitMessage::CommitMessageChanged(value))] if value == "修")
    );
    tab(&mut form, &mut renderer, description_focus).await;
    let description = form.type_text(&mut renderer, "描");
    assert!(matches!(
        description.as_slice(),
        [Message::Git(GitMessage::CommitDescriptionEditorAction(_))]
    ));
    drop(form);

    apply_form_message(&mut app, summary);
    assert_panel_state(&app, "选择提交类型后即可提交", CommitButtonState::MissingType);
    for messages in [selected, scope, description] {
        apply_form_message(&mut app, messages);
    }
    assert_eq!(app.git_commit_scope, "界");
    assert_eq!(app.git_commit_message, "修");
    assert_eq!(app.git_commit_description, "描");
    assert_panel_state(&app, "已满足提交条件，可直接提交", CommitButtonState::Ready);

    app.show_git_diff_summary = false;
    app.git_commit_type = None;
    app.git_commit_message.clear();
    let mut simple = FormScene::new(super::left_panel::view(&app), &mut renderer, 320);
    simple.operate(&renderer, Box::new(focusable::focus_next()));
    let summary = simple.type_text(&mut renderer, "S");
    drop(simple);
    apply_form_message(&mut app, summary);
    assert_eq!(app.git_commit_message, "S");
    assert_panel_state(&app, "已满足提交条件，可直接提交", CommitButtonState::Ready);
}
