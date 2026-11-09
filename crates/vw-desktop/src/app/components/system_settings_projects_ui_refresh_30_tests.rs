//! Exercise project settings through the headless Iced runtime.
#![cfg(not(target_arch = "wasm32"))]

use super::{App, Element, Message, view};
use crate::app::i18n::I18n;
use crate::app::message::{ProjectMessage, SettingsMessage};
use crate::app::state::ProjectEditTab;
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::keyboard::key::Named;
use iced::{Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_30_CHILD";
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

type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

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
    messages.retain(|message| !matches!(message, Message::None));
    messages
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_30_SCREENSHOTS") {
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

fn key(name: iced::keyboard::key::Named) -> Event {
    use iced::keyboard::{self, Key};
    let key = Key::Named(name);
    Event::Keyboard(keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    })
}

#[derive(Default)]
struct Geometry {
    labels: Vec<(String, iced::Rectangle)>,
    inputs: Vec<(String, iced::Rectangle)>,
    focusables: Vec<iced::Rectangle>,
    scroll: Vec<(iced::Rectangle, iced::Vector)>,
}

impl iced::advanced::widget::Operation for Geometry {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        children(self);
    }

    fn text(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        value: &str,
    ) {
        self.labels.push((value.into(), bounds));
    }

    fn text_input(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        state: &mut dyn iced::advanced::widget::operation::TextInput,
    ) {
        self.inputs.push((state.text().into(), bounds));
    }
    fn focusable(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        _: &mut dyn iced::advanced::widget::operation::Focusable,
    ) {
        self.focusables.push(bounds);
    }

    fn scrollable(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        _: iced::Rectangle,
        translation: iced::Vector,
        _: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        self.scroll.push((bounds, translation));
    }
}

impl Geometry {
    fn label(&self, value: &str) -> Rectangle {
        self.labels
            .iter()
            .find(|(label, _)| label == value)
            .unwrap_or_else(|| panic!("missing label {value}: {:?}", self.labels))
            .1
    }

    fn input(&self, value: &str) -> iced::Rectangle {
        self.inputs
            .iter()
            .find(|(input, _)| input == value)
            .unwrap_or_else(|| panic!("missing fixture input {value}"))
            .1
    }
}

fn geometry(ui: &mut Ui<'_>, renderer: &Renderer) -> Geometry {
    let mut geometry = Geometry::default();
    ui.operate(renderer, &mut geometry);
    geometry
}

fn click(ui: &mut Ui<'_>, renderer: &mut Renderer, point: iced::Point) -> Vec<Message> {
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

fn at(point: Point) -> mouse::Cursor {
    mouse::Cursor::Available(point)
}

fn scroll(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point, lines: f32) {
    assert!(
        send(
            ui,
            renderer,
            at(point),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Lines { x: 0.0, y: lines },
            })],
        )
        .is_empty()
    );
}

fn assert_visible_in_project_body(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    viewport: Rectangle,
    field: Rectangle,
) {
    let bottom = viewport.y + viewport.height;
    let hidden_height = field.y + field.height - bottom;
    if hidden_height > 0.0 {
        let event = Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -(hidden_height + 1.0) },
        });
        assert!(send(ui, renderer, at(viewport.center()), &[event]).is_empty());
    }
    let offset = geometry(ui, renderer).scroll[0].1.y;
    assert!(field.y - offset >= viewport.y);
    assert!(field.y + field.height - offset <= bottom);
}

fn key_at(ui: &mut Ui<'_>, renderer: &mut Renderer, name: Named) -> Vec<Message> {
    send(ui, renderer, mouse::Cursor::Unavailable, &[key(name)])
}

fn assert_fits(bounds: Rectangle, size: Size<u32>) {
    assert!(bounds.width > 0.0 && bounds.height > 0.0, "{bounds:?}");
    assert!(bounds.x >= 0.0 && bounds.y >= 0.0, "{bounds:?}");
    assert!(bounds.x + bounds.width <= size.width as f32 + 0.1, "{bounds:?}");
    assert!(bounds.y + bounds.height <= size.height as f32 + 0.1, "{bounds:?}");
}

const PATH: &str = "/fixture/很长的專案目錄/abcdefghijklmnopqrstuvwxyz/abcdefghijklmnopqrstuvwxyz/abcdefghijklmnopqrstuvwxyz";
const NAME: &str = "项目 Fixture 專案";

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    let mut app = App::new().0;
    app.project_path = None;
    app.project_edit_path = None;
    app.recent_projects = vec![PATH.into()];
    app.recent_projects_edits = vec![NAME.into()];
    app
}

const ICON: &str = "圖";
const COLOR: &str = "#8b5cf6";
const SCRIPT: &str = "printf '專案' > start-command-ran\nprintf '第二行'";

fn set_project_draft(app: &mut App) {
    app.project_edit_path = Some(PATH.into());
    app.project_edit_name = NAME.into();
    app.project_edit_icon = ICON.into();
    app.project_edit_icon_color = COLOR.into();
    app.project_edit_start_script = SCRIPT.into();
    app.project_edit_start_script_editor = iced::widget::text_editor::Content::with_text(SCRIPT);
    app.project_edit_worktree_enabled = true;
    app.project_edit_task_board_settings.max_concurrent = 4;
    app.project_edit_max_concurrent_input = "4".into();
    app.project_edit_session_refresh_interval_seconds_input = "123".into();
    app.project_edit_task_board_refresh_interval_seconds_input = "321".into();
    app.project_edit_task_board_scheduler_tick_interval_seconds_input = "27".into();
}

fn redraw_project(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
) {
    capture(ui, renderer, theme, size, name, mouse::Cursor::Unavailable);
}

fn assert_centered(left: Rectangle, right: Rectangle) {
    assert!((left.center_y() - right.center_y()).abs() < 0.5, "{left:?} vs {right:?}");
}

fn label_click(ui: &mut Ui<'_>, renderer: &mut Renderer, label: &str) -> Vec<Message> {
    let center = geometry(ui, renderer).label(label).center();
    click(ui, renderer, center)
}

fn assert_tab_labels_fit(positions: &Geometry, i18n: I18n, size: Size<u32>) {
    for label in ["编辑项目", "基础信息", "启动与工作区", "刷新策略", "调度与保护", "保存", "取消"]
    {
        assert_fits(positions.label(i18n.__(label)), size);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_30_layout_and_interaction() {
    if !isolated("ui_refresh_30_layout_and_interaction") {
        return;
    }
    let mut app = fixture();
    set_project_draft(&mut app);
    let mut renderer = renderer().await;
    for (locale, width, height, theme, language) in [
        (DesktopLocale::SimplifiedChinese, 960, 720, Theme::Light, "zh-cn"),
        (DesktopLocale::TraditionalChinese, 640, 640, Theme::Dark, "zh-tw"),
        (DesktopLocale::English, 420, 450, Theme::Dark, "en"),
    ] {
        app.desktop_locale = locale;
        let size = Size::new(width, height);
        let i18n = I18n::new(locale);
        for (tab, name, next_label, next) in [
            (ProjectEditTab::General, "general", "启动与工作区", ProjectEditTab::Launch),
            (ProjectEditTab::Launch, "launch", "刷新策略", ProjectEditTab::Refresh),
            (ProjectEditTab::Refresh, "refresh", "调度与保护", ProjectEditTab::Scheduling),
            (ProjectEditTab::Scheduling, "scheduling", "基础信息", ProjectEditTab::General),
        ] {
            app.project_edit_tab = tab;
            let mut ui = scene(app.view(), &mut renderer, size);
            let positions = geometry(&mut ui, &renderer);
            assert_tab_labels_fit(&positions, i18n, size);
            let save = positions.label(i18n.__("保存"));
            let cancel = positions.label(i18n.__("取消"));
            assert_centered(cancel, save);
            let viewport = positions.scroll[0].0;
            assert!(viewport.y > positions.label(i18n.__("基础信息")).center_y());
            assert!(viewport.y + viewport.height < save.y);
            let path_label = i18n.__("项目地址");
            let path_description = i18n.__("项目根目录的路径，仅供查看。");
            for value in [path_label, path_description, PATH] {
                let count = positions.labels.iter().filter(|(label, _)| label == value).count();
                assert_eq!(count, usize::from(tab == ProjectEditTab::General), "{tab:?}: {value}");
            }
            if tab == ProjectEditTab::General {
                let path = positions.label(PATH);
                assert!(path.x >= viewport.x + 7.0);
                assert!(path.x + path.width <= viewport.x + viewport.width - 7.0);
                let name = positions.input(NAME);
                let icon = positions.input(ICON);
                let name_label = positions.label(i18n.__("项目名称"));
                let icon_label = positions.label(i18n.__("图标文本或图片路径"));
                let address_label = positions.label(path_label);
                let description = positions.label(path_description);
                assert!(address_label.y < description.y);
                if width == 960 {
                    assert!(name_label.x + name_label.width < name.x);
                    assert!(icon_label.x + icon_label.width < icon.x);
                    assert!(description.x + description.width < path.x);
                    assert_eq!(name.x, icon.x);
                    assert_eq!(name.x, path.x);
                    assert_centered(name_label, name);
                    assert_centered(icon_label, icon);
                    let color_label = positions.label(i18n.__("图标颜色"));
                    assert!(
                        color_label.x + color_label.width < positions.label(i18n.__("预设颜色")).x
                    );
                } else {
                    assert!(name_label.y < name.y);
                    assert!(icon_label.y < icon.y);
                    assert!(description.y < path.y);
                }
                assert_visible_in_project_body(&mut ui, &mut renderer, viewport, name);
            }
            redraw_project(
                &mut ui,
                &mut renderer,
                &theme,
                size,
                &format!("{name}-{language}-{width}"),
            );

            assert!(matches!(
                label_click(&mut ui, &mut renderer, i18n.__(next_label)).as_slice(),
                [Message::Project(ProjectMessage::ProjectEditTabSelected(tab))] if *tab == next
            ));
            if tab == ProjectEditTab::Refresh {
                let label = positions.label(i18n.__("会话刷新(秒)"));
                let value = positions.input("123");
                assert_centered(label, value);
            }
            if tab == ProjectEditTab::Scheduling {
                assert_centered(positions.focusables[0], positions.label(i18n.__("管理工作流")));
                let header = positions.label(i18n.__("编辑项目"));
                let select = positions.focusables[0];
                assert_visible_in_project_body(&mut ui, &mut renderer, viewport, select);
                scroll(&mut ui, &mut renderer, viewport.center(), -40.0);
                let after = geometry(&mut ui, &renderer);
                let translation = after.scroll[0].1.y;
                assert!(translation > 0.0);
                // Iced rounds the scroll offset to whole pixels.
                let last_field = positions.label(i18n.__("合并锁超时(秒)"));
                assert!(
                    last_field.y + last_field.height - translation
                        <= viewport.y + viewport.height + 1.0
                );
                assert_eq!(after.label(i18n.__("编辑项目")), header);
                assert_eq!(after.label(i18n.__("保存")), save);
                assert_eq!(after.label(i18n.__("取消")), cancel);
                redraw_project(
                    &mut ui,
                    &mut renderer,
                    &theme,
                    size,
                    &format!("scrolled-{language}-{width}"),
                );
            }
            assert!(matches!(
                click(&mut ui, &mut renderer, cancel.center()).as_slice(),
                [Message::Project(ProjectMessage::ProjectEditCanceled)]
            ));
        }
    }

    app.project_edit_tab = ProjectEditTab::Scheduling;
    let size = Size::new(420, 450);
    let i18n = I18n::new(app.desktop_locale);
    let mut registry = crate::app::task::workflow::TaskWorkflowRegistry::new();
    use crate::app::views::workflow_manager::{self, WorkflowManagerMessage};
    let _ = workflow_manager::update(
        &mut app.workflow_manager,
        &mut registry,
        WorkflowManagerMessage::RefreshLifecycles,
    );
    let mut ui = scene(app.view(), &mut renderer, size);
    assert_fits(geometry(&mut ui, &renderer).label(i18n.__("生命周期列表正在从网关加载。")), size);
    redraw_project(&mut ui, &mut renderer, &Theme::Dark, size, "scheduling-loading");
    drop(ui);

    let _ = workflow_manager::update(
        &mut app.workflow_manager,
        &mut registry,
        WorkflowManagerMessage::LifecycleWorkflowsLoaded {
            result: Err("fixture unavailable / 測試連線失敗".into()),
            notify_error: false,
        },
    );
    let expected = i18n
        .format("生命周期列表加载失败：%s", &["fixture unavailable / 測試連線失敗".into()])
        .unwrap();
    let mut ui = scene(app.view(), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    assert_fits(positions.label(&expected), size);
    redraw_project(&mut ui, &mut renderer, &Theme::Dark, size, "scheduling-error");

    let select = positions.focusables[0];
    assert!(click(&mut ui, &mut renderer, select.center()).is_empty());
    let menu = geometry(&mut ui, &renderer).scroll.last().unwrap().0;
    assert_ne!(menu, positions.scroll[0].0);
    assert_fits(menu, size);
    let workflows = crate::app::task::all_task_workflows();
    assert!(workflows.len() >= 2);
    // A shorter menu may show only the upper part of the second option.
    let second_offset = 54.0_f32.min(menu.height - 1.0);
    assert!(second_offset > 38.0, "the second option should be partially visible: {menu:?}");
    let mut previous = None;
    for (offset, name) in [(18.0, "workflow-hover-first"), (second_offset, "workflow-hover-second")]
    {
        let point = Point::new(menu.center_x(), menu.y + offset);
        assert!(point.y < menu.y + menu.height);
        assert!(
            send(
                &mut ui,
                &mut renderer,
                at(point),
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })],
            )
            .is_empty()
        );
        let screenshot = capture(&mut ui, &mut renderer, &Theme::Dark, size, name, at(point));
        if let Some(previous) = previous {
            assert_ne!(screenshot, previous);
        }
        previous = Some(screenshot);
    }
    assert!(
        send(
            &mut ui,
            &mut renderer,
            mouse::Cursor::Unavailable,
            &[Event::Mouse(mouse::Event::CursorLeft)]
        )
        .is_empty()
    );
    scroll(&mut ui, &mut renderer, menu.center(), -8.0);
    let scrolled = geometry(&mut ui, &renderer);
    assert_eq!(scrolled.scroll[0].1.y, 0.0);
    assert!(scrolled.scroll.last().unwrap().1.y > 0.0);
    redraw_project(&mut ui, &mut renderer, &Theme::Dark, size, "workflow-scrolled");
    redraw_project(&mut ui, &mut renderer, &Theme::Dark, size, "workflow-no-hover");
    assert!(key_at(&mut ui, &mut renderer, Named::Escape).is_empty());
    assert_eq!(geometry(&mut ui, &renderer).scroll.len(), 1);
    assert!(key_at(&mut ui, &mut renderer, Named::Enter).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::ArrowDown).is_empty());
    assert!(matches!(
        key_at(&mut ui, &mut renderer, Named::Enter).as_slice(),
        [Message::Project(ProjectMessage::ProjectEditWorkflowIdChanged(id))] if id == &workflows[1].id
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, Point::new(4.0, 4.0)).as_slice(),
        [Message::Project(ProjectMessage::ProjectEditCanceled)]
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_30_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_30_preserves_values_and_disabled_states") {
        return;
    }
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::TraditionalChinese;
    let i18n = I18n::new(app.desktop_locale);
    let size = Size::new(640, 630);
    let mut renderer = renderer().await;
    app.project_path = Some(PATH.into());
    let narrow_size = Size::new(420, 630);
    let mut narrow = scene(view(&app), &mut renderer, narrow_size);
    let toggle = geometry(&mut narrow, &renderer).label(i18n.__("启用"));
    assert_fits(toggle, narrow_size);
    assert!(toggle.height < 22.0);
    assert!(matches!(
        click(&mut narrow, &mut renderer, toggle.center()).as_slice(),
        [Message::Settings(SettingsMessage::ProjectEnableWorktreeToggled(path, true))] if path == PATH
    ));
    redraw_project(&mut narrow, &mut renderer, &Theme::Light, narrow_size, "projects-420");
    drop(narrow);
    app.project_path = None;
    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let disabled = positions.label(i18n.__("请先打开项目"));
    assert!(click(&mut ui, &mut renderer, disabled.center()).is_empty());
    assert_fits(positions.input(i18n.__("搜索项目")), size);
    let open = positions.label(NAME).center();
    assert!(matches!(
        click(&mut ui, &mut renderer, open).as_slice(),
        [Message::Project(ProjectMessage::OpenRecentPressed(path))] if path == PATH
    ));
    assert_fits(positions.label(PATH), size);
    redraw_project(&mut ui, &mut renderer, &Theme::Dark, size, "projects-640");
    let actions = Point::new(size.width as f32 - 21.0, open.y + 7.0);
    assert!(matches!(
        click(&mut ui, &mut renderer, actions).as_slice(),
        [Message::Project(ProjectMessage::ProjectToolsMenuToggled(path))] if path == PATH
    ));
    drop(ui);

    app.recent_projects.insert(0, "/fixture/another-project".into());
    app.recent_projects_edits.insert(0, "Another project".into());
    app.recent_project_query = "  fIXTURE 專案  ".into();
    app.project_tools_menu_path = Some(PATH.into());
    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    assert!(!positions.labels.iter().any(|(label, _)| label == "Another project"));
    redraw_project(&mut ui, &mut renderer, &Theme::Light, size, "projects-menu-640");
    assert!(matches!(
        label_click(&mut ui, &mut renderer, i18n.__("编辑项目")).as_slice(),
        [Message::Project(ProjectMessage::ProjectEditOpened(path))] if path == PATH
    ));
    assert!(matches!(
        label_click(&mut ui, &mut renderer, i18n.__("删除")).as_slice(),
        [Message::Settings(SettingsMessage::RecentProjectDeleteRequested(1))]
    ));
    drop(ui);
    app.project_tools_menu_path = None;
    app.recent_project_query.clear();
    app.recent_projects.remove(0);
    app.recent_projects_edits.remove(0);

    app.recent_project_delete_confirm_idx = Some(0);
    let mut ui = scene(view(&app), &mut renderer, size);
    redraw_project(&mut ui, &mut renderer, &Theme::Light, size, "projects-delete-640");
    assert!(matches!(
        label_click(&mut ui, &mut renderer, i18n.__("取消")).as_slice(),
        [Message::Settings(SettingsMessage::RecentProjectDeleteCanceled)]
    ));
    assert!(matches!(
        label_click(&mut ui, &mut renderer, i18n.__("确认删除")).as_slice(),
        [Message::Settings(SettingsMessage::RecentProjectDeleteConfirmed(0))]
    ));
    drop(ui);

    app.recent_project_remove_confirm_path = Some(PATH.into());
    let mut ui = scene(app.view(), &mut renderer, Size::new(420, 450));
    redraw_project(&mut ui, &mut renderer, &Theme::Dark, Size::new(420, 450), "recent-remove-420");
    assert!(matches!(
        label_click(&mut ui, &mut renderer, i18n.__("取消")).as_slice(),
        [Message::Project(ProjectMessage::RecentRemoveCanceled)]
    ));
    assert!(matches!(
        label_click(&mut ui, &mut renderer, i18n.__("确认移除")).as_slice(),
        [Message::Project(ProjectMessage::RecentRemoveConfirmed(path))] if path == PATH
    ));
    drop(ui);
    app.recent_project_remove_confirm_path = None;

    set_project_draft(&mut app);
    let mut ui = scene(app.view(), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    assert_fits(positions.input(NAME), size);
    assert_fits(positions.input(ICON), size);
    assert!(matches!(
        label_click(&mut ui, &mut renderer, i18n.__("选择颜色")).as_slice(),
        [Message::Project(ProjectMessage::ProjectEditIconColorPickerToggled)]
    ));
    drop(ui);
    app.project_edit_icon_color_picker_open = true;
    let mut ui = scene(app.view(), &mut renderer, Size::new(420, 450));
    redraw_project(&mut ui, &mut renderer, &Theme::Dark, Size::new(420, 450), "color-picker-420");
    assert!(matches!(
        label_click(&mut ui, &mut renderer, i18n.__("确定")).as_slice(),
        [Message::Project(ProjectMessage::ProjectEditIconColorPickerClosed)]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, Point::new(4.0, 4.0)).as_slice(),
        [Message::Project(ProjectMessage::ProjectEditIconColorPickerClosed)]
    ));
    drop(ui);
    app.project_edit_icon_color_picker_open = false;
    app.project_edit_tab = ProjectEditTab::Launch;
    let mut ui = scene(app.view(), &mut renderer, size);
    let editor = geometry(&mut ui, &renderer).focusables[0];
    assert_fits(editor, size);
    assert!(matches!(
        click(&mut ui, &mut renderer, editor.center()).as_slice(),
        [Message::Project(ProjectMessage::ProjectEditStartScriptEditorAction(_))]
    ));
    assert!(matches!(
        key_at(&mut ui, &mut renderer, Named::Enter).as_slice(),
        [Message::Project(ProjectMessage::ProjectEditStartScriptEditorAction(
            iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Enter)
        ))]
    ));
    assert!(key_at(&mut ui, &mut renderer, Named::Escape).is_empty());
    let messages = label_click(&mut ui, &mut renderer, i18n.__("取消"));
    assert!(matches!(messages.as_slice(), [Message::Project(ProjectMessage::ProjectEditCanceled)]));
    drop(ui);
    let update = crate::app::message::project::update;
    let [Message::Project(cancel)] = messages.as_slice() else { unreachable!() };
    let dirs = crate::app::project_dirs().unwrap();
    let disk = dirs.data_local_dir().join("recent_projects_meta.json");
    let before = std::fs::read(&disk).ok();
    assert!(update(&mut app, cancel.clone()).units() == 0);
    assert!(app.project_edit_path.is_none());
    assert_eq!(std::fs::read(&disk).ok(), before);
    assert!(!std::path::Path::new("start-command-ran").exists());

    set_project_draft(&mut app);
    app.project_edit_tab = ProjectEditTab::Scheduling;
    let mut ui = scene(app.view(), &mut renderer, size);
    let input = geometry(&mut ui, &renderer).input("27");
    assert!(click(&mut ui, &mut renderer, input.center()).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::End).is_empty());
    assert!(matches!(
        key_at(&mut ui, &mut renderer, Named::Backspace).as_slice(),
        [Message::Project(ProjectMessage::ProjectEditTaskBoardSchedulerTickIntervalSecondsInputChanged(value))]
            if value == "2"
    ));
    let messages = label_click(&mut ui, &mut renderer, i18n.__("保存"));
    assert!(matches!(messages.as_slice(), [Message::Project(ProjectMessage::ProjectEditSaved)]));
    drop(ui);
    let [Message::Project(save)] = messages.as_slice() else { unreachable!() };
    drop(update(&mut app, save.clone()));
    for _ in 0..100 {
        if disk.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let saved = crate::app::projects::load_recent_projects_meta();
    let project = saved.iter().find(|meta| meta.path == PATH).unwrap();
    assert_eq!(project.name, NAME);
    assert_eq!(project.icon.as_deref(), Some(ICON));
    assert_eq!(project.icon_color.as_deref(), Some(COLOR));
    assert_eq!(project.worktree_start_command.as_deref(), Some(SCRIPT));
    assert_eq!(project.session_refresh_interval_seconds, 123);
    let schedule = project.task_board_settings.as_ref().unwrap();
    assert_eq!(schedule.refresh_interval_seconds, 321);
    assert_eq!(schedule.scheduler_tick_interval_seconds, 27);
    assert_eq!(schedule.max_concurrent, 4);
    drop(update(&mut app, ProjectMessage::ProjectEditOpened(PATH.into())));
    assert_eq!(app.project_edit_path.as_deref(), Some(PATH));
    assert_eq!(app.project_edit_name, NAME);
    assert_eq!(app.project_edit_icon, ICON);
    assert_eq!(app.project_edit_icon_color, COLOR);
    assert_eq!(app.project_edit_start_script, SCRIPT);
    assert_eq!(app.project_edit_max_concurrent_input, "4");
    assert!(!std::path::Path::new("start-command-ran").exists());

    app.recent_projects.clear();
    app.recent_projects_edits.clear();
    let mut ui = scene(view(&app), &mut renderer, size);
    assert_fits(geometry(&mut ui, &renderer).label(i18n.__("暂无最近项目")), size);
    redraw_project(&mut ui, &mut renderer, &Theme::Light, size, "projects-empty-640");
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_peripheral_appearance_project() {
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::impls::with_project_edit;

    fn overlay(app: &App) -> Element<'_, Message> {
        let background =
            iced::widget::Space::new().width(iced::Length::Fill).height(iced::Length::Fill);
        with_project_edit(app, background.into())
    }

    tab_appearance_tests::run_at_least(
        concat!(module_path!(), "::settings_tabs_peripheral_appearance_project"),
        420,
        fixture,
        |scene, app| {
            set_project_draft(app);
            app.project_edit_tab = ProjectEditTab::General;
            let i18n = I18n::new(app.desktop_locale);
            let tabs =
                ["基础信息", "启动与工作区", "刷新策略", "调度与保护"].map(|label| i18n.__(label));
            scene.save_preview(app, overlay(app), "project-edit");
            let message = scene.check_message(app, overlay(app), &tabs, 3, 14);
            assert!(matches!(
                message,
                Message::Project(ProjectMessage::ProjectEditTabSelected(
                    ProjectEditTab::Scheduling
                ))
            ));
            let _ = app.update(message);
            assert_eq!(app.project_edit_tab, ProjectEditTab::Scheduling);
            assert_eq!(app.project_edit_start_script, SCRIPT);
            scene.check_modal_scroll(overlay(app), i18n.__("编辑项目"), i18n.__("保存"));
        },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_21_project_settings_render_contextual_help() {
    use crate::app::components::system_settings_proxy::system_settings_proxy_tests::rollout::Scene;

    if !isolated("help_rollout_21_project_settings_render_contextual_help") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    let path = "/fixture/项目";
    app.recent_projects = vec![path.into()];
    for (locale, theme, has_project) in [
        (DesktopLocale::SimplifiedChinese, Theme::Light, true),
        (DesktopLocale::English, Theme::Dark, false),
        (DesktopLocale::TraditionalChinese, Theme::Dark, true),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme;
        app.project_path = has_project.then(|| path.into());
        let i18n = I18n::new(locale);
        let mut settings = Scene::new(&app, &mut renderer, "project-settings", view(&app));
        settings.label(path);
        let description = i18n.__("新建会话时可选工作区。");
        let icon = settings.help(i18n.__("启用 Worktree"), description);
        settings.actions(icon, description);
        let label = i18n.__(if has_project { "启用" } else { "请先打开项目" });
        let control = settings.label(label).center();
        let messages = settings.click(control);
        if has_project {
            assert!(matches!(
                messages.as_slice(),
                [Message::Settings(SettingsMessage::ProjectEnableWorktreeToggled(value, true))]
                    if value == path
            ));
        } else {
            assert!(messages.is_empty());
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_21_renders_contextual_help() {
    if !isolated("help_rollout_21_renders_contextual_help") {
        return;
    }
    let mut app = fixture();
    set_project_draft(&mut app);
    let mut renderer = renderer().await;
    for (locale, size, theme) in [
        (DesktopLocale::SimplifiedChinese, Size::new(960, 720), Theme::Light),
        (DesktopLocale::English, Size::new(420, 450), Theme::Dark),
        (DesktopLocale::TraditionalChinese, Size::new(420, 480), Theme::Dark),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        let i18n = I18n::new(locale);
        for (tab, title, description) in [
            (ProjectEditTab::General, "项目展示", "配置项目名称、图标来源与图标主题色。"),
            (
                ProjectEditTab::Launch,
                "启动与工作区",
                "设置 worktree 启用策略，以及创建工作区后的启动命令。",
            ),
            (ProjectEditTab::Refresh, "刷新策略", "统一项目会话与任务看板刷新策略。"),
            (
                ProjectEditTab::Scheduling,
                "调度与保护",
                "限制并发、设置重试与超时，并控制 worktree 回收与合并锁。",
            ),
        ] {
            app.project_edit_tab = tab;
            let mut ui = scene(app.view(), &mut renderer, size);
            let positions = geometry(&mut ui, &renderer);
            let viewport = positions.scroll[0].0;
            let mut targets = vec![(title, description)];
            if tab == ProjectEditTab::General {
                targets.push(("图标文本或图片路径", "支持 file:/// 或本地绝对路径。"));
            } else if tab == ProjectEditTab::Launch {
                targets.push(("工作区启动脚本", "在创建工作区后执行，可填写多行命令。"));
            }

            for (index, (title, description)) in targets.into_iter().enumerate() {
                let title = i18n.__(title);
                let description = i18n.__(description);
                assert!(
                    positions.labels.iter().all(|(label, _)| label != description),
                    "{tab:?}: static help is still inline: {description}"
                );
                let label = positions
                    .labels
                    .iter()
                    .find(|(label, bounds)| label == title && viewport.contains(bounds.center()))
                    .unwrap_or_else(|| panic!("missing body label {title}"))
                    .1;
                let icon = Point::new(label.x + label.width + 15.0, label.center_y());
                assert!(viewport.contains(icon), "{tab:?}: help icon is outside the viewport");
                let name = format!("project-help-{locale:?}-{tab:?}-{index}");
                let frame = |ui: &mut Ui<'_>, renderer: &mut Renderer, phase, cursor| {
                    capture(ui, renderer, &theme, size, &format!("{name}-{phase}"), cursor)
                };
                let idle = frame(&mut ui, &mut renderer, "idle", mouse::Cursor::Unavailable);
                assert!(
                    send(
                        &mut ui,
                        &mut renderer,
                        at(icon),
                        &[Event::Mouse(mouse::Event::CursorMoved { position: icon })],
                    )
                    .is_empty()
                );
                let hover = frame(&mut ui, &mut renderer, "hover", at(icon));
                assert!(hover != idle, "{tab:?}: hover must open help for {title}");
                assert!(click(&mut ui, &mut renderer, icon).is_empty());
                let pinned = frame(&mut ui, &mut renderer, "pinned", mouse::Cursor::Unavailable);
                assert!(pinned != idle, "{tab:?}: help must stay open after clicking {title}");
                assert!(key_at(&mut ui, &mut renderer, Named::Escape).is_empty());
                let closed = frame(&mut ui, &mut renderer, "closed", mouse::Cursor::Unavailable);
                assert!(closed == idle, "{tab:?}: Escape must close only help");
            }

            if tab == ProjectEditTab::General {
                positions.input(NAME);
                let color = positions.label(i18n.__("选择颜色"));
                assert_visible_in_project_body(&mut ui, &mut renderer, viewport, color);
                let offset = geometry(&mut ui, &renderer).scroll[0].1.y.round();
                assert!(matches!(
                    click(&mut ui, &mut renderer, color.center() - iced::Vector::new(0.0, offset))
                        .as_slice(),
                    [Message::Project(ProjectMessage::ProjectEditIconColorPickerToggled)]
                ));
            } else if tab == ProjectEditTab::Refresh {
                positions.input("123");
                positions.input("321");
            } else if tab == ProjectEditTab::Scheduling {
                positions.input("27");
                positions.label(i18n.__("管理工作流"));
            }
            assert!(matches!(
                label_click(&mut ui, &mut renderer, i18n.__("保存")).as_slice(),
                [Message::Project(ProjectMessage::ProjectEditSaved)]
            ));
            assert!(matches!(
                label_click(&mut ui, &mut renderer, i18n.__("取消")).as_slice(),
                [Message::Project(ProjectMessage::ProjectEditCanceled)]
            ));
        }
    }
}
