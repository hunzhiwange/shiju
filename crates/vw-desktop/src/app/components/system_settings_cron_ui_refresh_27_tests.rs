//! Real Iced layout, rendering and event regression for scheduled settings.
#![cfg(not(target_arch = "wasm32"))]

use super::super::{
    system_settings_heartbeat as heartbeat, system_settings_scheduler as scheduler,
};
use super::*;
use crate::app::i18n::I18n;
use crate::app::message::SettingsMessage;
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::keyboard::key::Named;
use iced::{Event, Font, Point, Rectangle, Renderer, Size, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_27_CHILD";
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_27_SCREENSHOTS") {
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
    scroll: Option<(iced::Rectangle, iced::Vector)>,
    scrolls: Vec<iced::Rectangle>,
    containers: Vec<iced::Rectangle>,
}

impl iced::advanced::widget::Operation for Geometry {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        children(self);
    }

    fn container(&mut self, _: Option<&iced::advanced::widget::Id>, bounds: Rectangle) {
        self.containers.push(bounds);
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
        self.scrolls.push(bounds);
        self.scroll = Some((bounds, translation));
    }
}

impl Geometry {
    fn label(&self, value: &str) -> iced::Rectangle {
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

fn key_at(ui: &mut Ui<'_>, renderer: &mut Renderer, name: Named) -> Vec<Message> {
    send(ui, renderer, mouse::Cursor::Unavailable, &[key(name)])
}

fn assert_fits(bounds: Rectangle, size: Size<u32>) {
    assert!(bounds.width > 0.0 && bounds.height > 0.0, "{bounds:?}");
    assert!(bounds.x >= 0.0 && bounds.y >= 0.0, "{bounds:?}");
    assert!(bounds.x + bounds.width <= size.width as f32 + 0.1, "{bounds:?}");
    assert!(bounds.y + bounds.height <= size.height as f32 + 0.1, "{bounds:?}");
}

fn frame(content: Element<'_, Message>) -> Element<'_, Message> {
    container(content).width(Length::Fill).height(Length::Fill).into()
}

fn wheel(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point) -> Vec<Message> {
    send(
        ui,
        renderer,
        at(point),
        &[Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: -8.0 },
        })],
    )
}

const PROJECT: &str = "/fixture/包含空格的项目/abcdefghijklmnopqrstuvwxyz/abcdefghijklmnopqrstuvwxyz/abcdefghijklmnopqrstuvwxyz";
const JOB_ID: &str = "fixture-job";

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    App::new().0
}

#[tokio::test(flavor = "multi_thread")]
async fn cron_project_selector_clears_selected_path() {
    if !isolated("cron_project_selector_clears_selected_path") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    let size = Size::new(420, 260);
    let mut ui = scene(
        project_field(&app, PROJECT, |path| {
            Message::Settings(SettingsMessage::CronJobEditProjectPathChanged(path))
        }),
        &mut renderer,
        size,
    );
    let trigger = geometry(&mut ui, &renderer).focusables[0];
    assert!(click(&mut ui, &mut renderer, trigger.center()).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::Home).is_empty());
    for theme in [Theme::Light, Theme::Dark] {
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            size,
            "cron-project-menu",
            mouse::Cursor::Unavailable,
        );
    }
    assert!(matches!(
        key_at(&mut ui, &mut renderer, Named::Enter).as_slice(),
        [Message::Settings(SettingsMessage::CronJobEditProjectPathChanged(path))] if path.is_empty()
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_27_layout_and_interaction() {
    if !isolated("ui_refresh_27_layout_and_interaction") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = gap_10_fixture(true);
    app.desktop_locale = DesktopLocale::TraditionalChinese;
    app.cron_settings.edit_draft.project_path = PROJECT.into();
    app.cron_settings.edit_draft.preview = Some(Ok(vw_gateway_client::CronPreviewResponse {
        normalized: Default::default(),
        timezone: "UTC".into(),
        candidates: vec!["2099-01-01T00:00:00Z".into()],
    }));
    let tr = I18n::new(app.desktop_locale);
    let size = Size::new(420, 560);
    let mut ui = scene(view_overlays(&app, frame(view(&app))), &mut renderer, size);
    let before = geometry(&mut ui, &renderer);
    let save = before.label(tr.__("保存"));
    let cancel = before.label(tr.__("取消"));
    assert_fits(save, size);
    assert_fits(cancel, size);
    let title = before.label(tr.__("编辑定时任务"));
    let scroll = before.scroll.unwrap().0;
    assert!(wheel(&mut ui, &mut renderer, scroll.center()).is_empty());
    let after = geometry(&mut ui, &renderer);
    assert_eq!(after.label(tr.__("保存")), save);
    assert_eq!(after.label(tr.__("编辑定时任务")), title);
    assert!(after.scroll.unwrap().1.y > before.scroll.unwrap().1.y);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "edit-fixed-actions",
        mouse::Cursor::Unavailable,
    );
    assert!(matches!(
        click(&mut ui, &mut renderer, cancel.center()).as_slice(),
        [Message::Settings(SettingsMessage::CronJobEditCanceled)]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, save.center()).as_slice(),
        [Message::Settings(SettingsMessage::CronJobEditSave)]
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_27_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_27_preserves_values_and_disabled_states") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.cron_settings.active_tab = CronSettingsTab::Add;
    app.cron_settings.show_add_modal = true;
    app.cron_settings.add_draft.project_path = PROJECT.into();
    app.cron_settings.add_draft.job_type = CronAddJobType::Shell;
    app.cron_settings.add_draft.delivery_enabled = true;
    app.cron_settings.add_draft.delivery_channel = "telegram".into();
    app.cron_settings.add_draft.delivery_to = "fixture-recipient".into();
    app.heartbeat_settings.enabled = false;
    app.heartbeat_settings.target_input = "telegram".into();
    app.scheduler_settings.enabled = false;
    for (locale, width, theme, tag) in [
        (DesktopLocale::English, 640, Theme::Light, "en-640-light"),
        (DesktopLocale::SimplifiedChinese, 960, Theme::Dark, "zh-cn-960-dark"),
        (DesktopLocale::TraditionalChinese, 640, Theme::Dark, "zh-tw-640-dark"),
    ] {
        app.desktop_locale = locale;
        let tr = I18n::new(locale);
        let size = Size::new(width, 2100);
        let mut ui = scene(frame(add_tab(&app)), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        let project = positions.input(PROJECT);
        let target = positions.input("fixture-recipient");
        assert!(project.width > 0.0);
        assert!(target.width > 0.0);
        assert!((project.x - positions.label(tr.__("项目路径")).x).abs() < 1.0);
        assert!((target.x - positions.label("投递目标").x).abs() < 1.0);
        assert!((project.x - target.x).abs() < 1.0);
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            size,
            &format!("add-{tag}"),
            mouse::Cursor::Unavailable,
        );
        assert!(click(&mut ui, &mut renderer, target.center()).is_empty());
        assert!(key_at(&mut ui, &mut renderer, Named::End).is_empty());
        assert!(matches!(key_at(&mut ui, &mut renderer, Named::Backspace).as_slice(),
            [Message::Settings(SettingsMessage::CronAddDeliveryToChanged(value))] if value == "fixture-recipien"));
        for (view, key, name) in [
            (heartbeat::view(&app), "开启周期心跳", "heartbeat"),
            (scheduler::embedded_view(&app), "开启内置调度器", "scheduler"),
        ] {
            let size = Size::new(width, 660);
            let mut ui = scene(frame(view), &mut renderer, size);
            let positions = geometry(&mut ui, &renderer);
            let bounds = positions.label(tr.__(key));
            if name == "heartbeat" {
                let label = positions.label(tr.__("目标通道"));
                let field = positions.input("telegram");
                assert_fits(field, size);
                assert!((field.center_y() - label.center_y()).abs() < 0.5);
            }
            assert_fits(bounds, size);
            let messages = click(&mut ui, &mut renderer, bounds.center());
            assert!(matches!(
                (name, messages.as_slice()),
                ("heartbeat", [Message::Settings(SettingsMessage::HeartbeatEnabledToggled(true))])
                    | (
                        "scheduler",
                        [Message::Settings(SettingsMessage::SchedulerEnabledToggled(true))]
                    )
            ));
            capture(
                &mut ui,
                &mut renderer,
                &theme,
                size,
                &format!("{name}-{tag}"),
                mouse::Cursor::Unavailable,
            );
        }
    }

    // A missing provider catalog leaves an empty picker inert.
    let size = Size::new(420, 120);
    let mut ui = scene(
        frame(pick_field(Vec::new(), None, |value| {
            Message::Settings(SettingsMessage::CronAddModelChanged(value))
        })),
        &mut renderer,
        size,
    );
    assert!(geometry(&mut ui, &renderer).focusables.is_empty());
    assert!(click(&mut ui, &mut renderer, Point::new(150.0, 19.0)).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::Enter).is_empty());

    app.desktop_locale = DesktopLocale::English;
    app.cron_settings.active_tab = CronSettingsTab::Config;
    app.cron_settings.enabled = false;
    app.cron_settings.max_run_history = 500;
    app.heartbeat_settings.show_help_modal = true;
    app.scheduler_settings.show_help_modal = true;
    let tr = I18n::new(app.desktop_locale);
    let size = Size::new(640, 640);
    let mut ui = scene(frame(view(&app)), &mut renderer, size);
    let toggle = geometry(&mut ui, &renderer).label(tr.__("开启 Cron 子系统"));
    assert!(matches!(
        click(&mut ui, &mut renderer, toggle.center()).as_slice(),
        [Message::Settings(SettingsMessage::CronEnabledToggled(true))]
    ));
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "cron-config-en-640",
        mouse::Cursor::Unavailable,
    );
    drop(ui);
    let mut ui = scene(frame(scheduler::embedded_view(&app)), &mut renderer, Size::new(640, 1080));
    assert!(geometry(&mut ui, &renderer).label(tr.__("Scheduler 配置帮助")).width > 0.0);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        Size::new(640, 1080),
        "scheduler-help-en-640",
        mouse::Cursor::Unavailable,
    );
    let size = Size::new(420, 500);
    let mut ui =
        scene(heartbeat::view_overlays(&app, frame(heartbeat::view(&app))), &mut renderer, size);
    assert_fits(geometry(&mut ui, &renderer).label(tr.__("Heartbeat 配置帮助")), size);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "heartbeat-help-en-420",
        mouse::Cursor::Unavailable,
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_27_history_scroll_and_job_actions() {
    if !isolated("ui_refresh_27_history_scroll_and_job_actions") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    let tr = I18n::new(app.desktop_locale);
    app.cron_settings.active_tab = CronSettingsTab::Jobs;
    app.cron_settings.jobs_loading = true;
    let size = Size::new(640, 700);
    let mut ui = scene(frame(view(&app)), &mut renderer, size);
    assert_fits(geometry(&mut ui, &renderer).label(tr.__("正在加载定时任务...")), size);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "jobs-loading-en-640",
        mouse::Cursor::Unavailable,
    );
    drop(ui);
    app.cron_settings.jobs_loading = false;
    let mut ui = scene(frame(view(&app)), &mut renderer, size);
    assert_fits(geometry(&mut ui, &renderer).label(tr.__("当前没有定时任务。")), size);
    drop(ui);

    let mut job = super::system_settings_cron_tests::job(JOB_ID);
    job.project_path = Some(PROJECT.into());
    job.enabled = false;
    app.cron_settings.jobs.push(job);
    app.cron_settings.expanded_job_ids.push(JOB_ID.into());
    let mut ui = scene(frame(view(&app)), &mut renderer, size);
    let buttons = geometry(&mut ui, &renderer);
    for (label, expected) in
        [("编辑", "edit"), ("历史", "history"), ("启用", "enable"), ("删除", "delete")]
    {
        let bounds = buttons.labels.iter().rev().find(|(text, _)| text == tr.__(label)).unwrap().1;
        let point = reveal_form_point(&mut ui, &mut renderer, bounds.center());
        let messages = click(&mut ui, &mut renderer, point);
        assert!(matches!((expected, messages.as_slice()),
            ("edit", [Message::Settings(SettingsMessage::CronJobEditStarted(id))])
                | ("history", [Message::Settings(SettingsMessage::CronJobRunsOpen(id))])
                | ("enable", [Message::Settings(SettingsMessage::CronJobEnabledChanged(id, true))])
                | ("delete", [Message::Settings(SettingsMessage::CronJobDelete(id))]) if id == JOB_ID));
    }
    capture(&mut ui, &mut renderer, &Theme::Light, size, "jobs-en-640", mouse::Cursor::Unavailable);
    drop(ui);

    app.cron_settings.runs_modal_job_id = Some(JOB_ID.into());
    let size = Size::new(420, 500);
    for (loading, error, label, name) in [
        (true, None, "正在加载历史记录...", "loading"),
        (false, Some("Fixture failure"), "Fixture failure", "error"),
        (false, None, "暂无执行历史", "empty"),
    ] {
        app.cron_settings.runs_modal_loading = loading;
        app.cron_settings.runs_modal_error = error.map(str::to_owned);
        let mut ui = scene(view_overlays(&app, frame(view(&app))), &mut renderer, size);
        assert_fits(geometry(&mut ui, &renderer).label(tr.__(label)), size);
        capture(
            &mut ui,
            &mut renderer,
            &Theme::Light,
            size,
            &format!("history-{name}-en-420"),
            mouse::Cursor::Unavailable,
        );
    }
    let runs = (0..40)
        .map(|id| {
            let mut run = super::system_settings_cron_tests::run("success");
            run.id = id;
            run.job_id = JOB_ID.into();
            run.output = Some(format!("Fixture output {id} / 巡檢完成"));
            run
        })
        .collect();
    drop(crate::app::message::settings::update(
        &mut app,
        SettingsMessage::CronJobRunsLoaded(JOB_ID.into(), Ok(runs)),
    ));

    let mut ui = scene(view_overlays(&app, frame(view(&app))), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let header = positions.label(tr.__("执行历史"));
    let editor = positions.focusables[0];
    assert_fits(editor, size);
    let before = capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "history-en-420",
        mouse::Cursor::Unavailable,
    );
    let messages = wheel(&mut ui, &mut renderer, editor.center());
    let [
        Message::Settings(SettingsMessage::CronJobRunsEditorAction(
            action @ text_editor::Action::Scroll { .. },
        )),
    ] = messages.as_slice()
    else {
        panic!("history wheel must reach the read-only editor");
    };
    let cache = ui.into_cache();
    drop(crate::app::message::settings::update(
        &mut app,
        SettingsMessage::CronJobRunsEditorAction(action.clone()),
    ));
    let mut ui = UserInterface::build(
        view_overlays(&app, frame(view(&app))),
        Size::new(size.width as f32, size.height as f32),
        cache,
        &mut renderer,
    );
    assert_eq!(geometry(&mut ui, &renderer).label(tr.__("执行历史")), header);
    let after = capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "history-scrolled-en-420",
        mouse::Cursor::Unavailable,
    );
    assert_ne!(before, after);
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_core_appearance_cron() {
    use super::super::system_settings_common::tab_appearance_tests;

    tab_appearance_tests::run(
        concat!(module_path!(), "::settings_tabs_core_appearance_cron"),
        fixture,
        |scene, app| {
            app.cron_settings.active_tab = CronSettingsTab::Jobs;
            let i18n = I18n::new(app.desktop_locale);
            let labels: Vec<_> = CRON_TABS.iter().map(|tab| i18n.__(tab.label)).collect();
            assert!(matches!(
                scene.check(app, view(app), &labels, 3),
                SettingsMessage::CronTabSelected(CronSettingsTab::Scheduler)
            ));
        },
    )
    .await;
}

fn assert_schedule_help(
    app: &App,
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    size: Size<u32>,
    entry: &str,
    keys: (&str, &str),
) {
    let tr = I18n::new(app.desktop_locale);
    let (title, explanation) = (tr.__(keys.0), tr.__(keys.1));
    let positions = geometry(ui, renderer);
    assert!(
        positions.labels.iter().all(|(label, _)| label != explanation),
        "{entry}: {title} still shows its static explanation in the initial layout"
    );
    let bounds = positions.labels.iter().rfind(|(label, _)| label == title).unwrap().1;
    let icon = Point::new(bounds.x + bounds.width + 15.0, bounds.center_y());
    let icon = reveal_form_point(ui, renderer, icon);
    let prefix = format!("rollout-15-{entry}-{:?}-{title}", app.desktop_locale);
    let theme = &app.app_theme;
    let away = mouse::Cursor::Unavailable;
    let idle = capture(ui, renderer, theme, size, &format!("{prefix}-idle"), away);
    assert!(
        send(ui, renderer, at(icon), &[Event::Mouse(mouse::Event::CursorMoved { position: icon })])
            .is_empty()
    );
    let hover = capture(ui, renderer, theme, size, &format!("{prefix}-hover"), at(icon));
    assert!(idle != hover, "{entry}: {title} hover preview must render");
    assert!(click(ui, renderer, icon).is_empty());
    let open = capture(ui, renderer, theme, size, &format!("{prefix}-open"), away);
    assert!(idle != open, "{entry}: {title} pinned help must render");

    if entry == "heartbeat" && keys.0 == "间隔分钟" && app.desktop_locale == DesktopLocale::English
    {
        assert_interval_help_actions(
            ui,
            renderer,
            Point::new(icon.x + 29.0, icon.y + 5.0),
            size,
            explanation,
        );
    }

    assert!(key_at(ui, renderer, Named::Escape).is_empty(), "{entry}: Escape closed the parent");
    let closed = capture(ui, renderer, theme, size, &format!("{prefix}-closed"), away);
    assert!(idle == closed, "{entry}: {title} Escape must restore the initial frame");
}

fn assert_interval_help_actions(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    body: Point,
    size: Size<u32>,
    explanation: &str,
) {
    let search = format!("https://www.google.com/search?q={}", urlencoding::encode(explanation));
    for index in [0, 3] {
        let messages = help_menu_action(ui, renderer, body, size, index);
        assert!(
            match (index, messages.as_slice()) {
                (0, [Message::CopyCode(value)]) => value == explanation,
                (3, [Message::View(crate::app::message::ViewMessage::OpenUrlExternal(url))]) => {
                    url == &search
                }
                _ => false,
            },
            "unexpected contextual help action: {messages:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_15_renders_contextual_help() {
    if !isolated("help_rollout_15_renders_contextual_help") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    let error = "fixture invalid Cron expression";
    let expression = "fixture invalid schedule";
    let mut job = super::system_settings_cron_tests::job(JOB_ID);
    job.enabled = false;
    let next_run = format_cron_datetime(&job.next_run);
    let current_expression = job.expression.clone();
    app.cron_settings.active_tab = CronSettingsTab::Jobs;
    app.cron_settings.jobs_loading = false;
    app.cron_settings.jobs = vec![job];
    app.cron_settings.expanded_job_ids = vec![JOB_ID.into()];
    app.cron_settings.save_error = Some(error.into());
    app.cron_settings.edit_draft.schedule = expression.into();
    app.cron_settings.edit_draft.frequency = crate::app::state::CronFrequency::Advanced;

    for (locale, theme) in [
        (DesktopLocale::SimplifiedChinese, Theme::Light),
        (DesktopLocale::English, Theme::Dark),
        (DesktopLocale::TraditionalChinese, Theme::Dark),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let tr = I18n::new(locale);
        let size = Size::new(480, 680);
        app.cron_settings.editing_job_id = None;
        let mut ui = scene(frame(view(&app)), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        for label in [tr.__("下次运行"), tr.__("禁用"), &next_run, &current_expression, error]
        {
            assert_fits(positions.label(label), size);
        }
        for help in [
            ("定时任务配置", "配置 Cron 子系统的开关与历史保留策略。"),
            ("当前任务", "默认展示已配置的定时任务，并支持单项或批量管理。"),
        ] {
            assert_schedule_help(&app, &mut ui, &mut renderer, size, "cron-jobs", help);
        }
        drop(ui);

        app.cron_settings.editing_job_id = Some(JOB_ID.into());
        let mut ui = scene(view_overlays(&app, frame(view(&app))), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        assert_fits(positions.label(error), size);
        assert!(positions.input(expression).width > 0.0);
        positions.label("5 字段：分 时 日 月 星期；6 字段在最前增加秒。使用当前时区。");
        assert_schedule_help(
            &app,
            &mut ui,
            &mut renderer,
            size,
            "cron-edit",
            ("任务类型", JOB_TYPE_HELP),
        );
        for (label, expected) in [
            ("保存", SettingsMessage::CronJobEditSave),
            ("取消", SettingsMessage::CronJobEditCanceled),
        ] {
            let bounds = geometry(&mut ui, &renderer).label(tr.__(label));
            let actions = click(&mut ui, &mut renderer, bounds.center());
            if label == "保存" {
                assert!(actions.is_empty());
            } else {
                assert!(
                    matches!(actions.as_slice(), [Message::Settings(action)] if std::mem::discriminant(action) == std::mem::discriminant(&expected))
                );
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_15_heartbeat_preserves_status_and_actions() {
    use crate::app::components::system_settings_agents;
    use crate::app::state::AgentConfigTab;

    if !isolated("help_rollout_15_heartbeat_preserves_status_and_actions") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    let error = "fixture heartbeat validation failed";
    app.agents_settings.active_config_tab = AgentConfigTab::Heartbeat;
    app.heartbeat_settings.enabled = false;
    app.heartbeat_settings.interval_minutes = 37;
    app.heartbeat_settings.save_error = Some(error.into());
    app.heartbeat_settings.to_input = "fixture-recipient".into();
    for (locale, theme) in [
        (DesktopLocale::SimplifiedChinese, Theme::Light),
        (DesktopLocale::English, Theme::Dark),
        (DesktopLocale::TraditionalChinese, Theme::Dark),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let tr = I18n::new(locale);
        let size = Size::new(480, 860);
        let mut ui = scene(frame(system_settings_agents::view(&app)), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        let interval = crate::__!(tr, "%u 分钟", app.heartbeat_settings.interval_minutes).unwrap();
        assert_fits(positions.label(&interval), size);
        assert_fits(positions.label(error), size);
        assert_fits(positions.input("fixture-recipient"), size);
        assert_schedule_help(
            &app,
            &mut ui,
            &mut renderer,
            size,
            "heartbeat",
            ("间隔分钟", "周期性触发心跳的时间间隔。"),
        );
        assert!(matches!(
            click(&mut ui, &mut renderer, positions.label(tr.__("开启周期心跳")).center())
                .as_slice(),
            [Message::Settings(SettingsMessage::HeartbeatEnabledToggled(true))]
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_15_cron_add_config_and_scheduler_render_help() {
    if !isolated("help_rollout_15_cron_add_config_and_scheduler_render_help") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.cron_settings.add_draft.schedule_kind = CronAddScheduleKind::Every;
    app.cron_settings.add_draft.schedule_draft = Some(crate::app::state::CronScheduleDraft {
        rule: crate::app::state::CronScheduleRule::Every {
            amount: 71,
            unit: crate::app::state::IntervalUnit::Seconds,
        },
        ..Default::default()
    });
    app.scheduler_settings.enabled = false;
    app.scheduler_settings.max_tasks = 63;
    for (locale, theme) in [
        (DesktopLocale::SimplifiedChinese, Theme::Light),
        (DesktopLocale::English, Theme::Dark),
        (DesktopLocale::TraditionalChinese, Theme::Dark),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let size = Size::new(480, 940);
        let tr = I18n::new(locale);
        for (tab, entry, help) in [
            (CronSettingsTab::Add, "cron-add", ("任务类型", JOB_TYPE_HELP)),
            (
                CronSettingsTab::Config,
                "cron-config",
                ("基础行为", "控制 Cron 子系统是否运行与历史留存数量。"),
            ),
            (CronSettingsTab::Scheduler, "scheduler", ("最大任务", "单次轮询最多处理的任务数量。")),
        ] {
            app.cron_settings.active_tab = tab;
            let content = if tab == CronSettingsTab::Add { add_tab(&app) } else { view(&app) };
            let mut ui = scene(frame(content), &mut renderer, size);
            let positions = geometry(&mut ui, &renderer);
            if tab == CronSettingsTab::Add {
                assert_fits(positions.input("71"), size);
            }
            if tab == CronSettingsTab::Scheduler {
                assert_fits(positions.label("63"), size);
            }
            assert_schedule_help(&app, &mut ui, &mut renderer, size, entry, help);
            if tab == CronSettingsTab::Scheduler {
                assert!(matches!(
                    click(
                        &mut ui,
                        &mut renderer,
                        positions.label(tr.__("开启内置调度器")).center()
                    )
                    .as_slice(),
                    [Message::Settings(SettingsMessage::SchedulerEnabledToggled(true))]
                ));
            }
        }
    }
}

const JOB_TYPE_HELP: &str = "Shell 执行命令，Agent 执行提示词任务。";
const FALLBACK_HELP: &str = "主模型失败后依次尝试，使用逗号或换行分隔。";
const WORKDAY_EXAMPLE: &str = "示例：0 9 * * MON-FRI 表示工作日 UTC 09:00 执行。";
const CRON_FORMAT_HELP: &str =
    "按 UTC 执行。5 字段依次为分、时、日、月、星期；6 字段在最前面增加秒。";

fn reveal_form_point(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point) -> Point {
    let Some((viewport, mut translation)) = geometry(ui, renderer).scroll else {
        return point;
    };
    if point.y < viewport.y {
        return point;
    }
    if !viewport.contains(point - translation) {
        let offset = point.y - translation.y - viewport.center_y();
        assert!(
            send(
                ui,
                renderer,
                at(viewport.center()),
                &[Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -offset },
                })],
            )
            .is_empty()
        );
        translation = geometry(ui, renderer).scroll.unwrap().1;
    }
    let visible = point - translation;
    assert!(viewport.contains(visible), "{point:?} is clipped at {visible:?}");
    visible
}

fn help_menu_action(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    body: Point,
    size: Size<u32>,
    index: usize,
) -> Vec<Message> {
    assert!(
        send(
            ui,
            renderer,
            at(body),
            &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))],
        )
        .is_empty()
    );
    let line_height = iced::advanced::text::LineHeight::default().to_absolute(12.0.into());
    let item_height = f32::from(line_height) + 10.0;
    // The context menu flips above the pointer when it reaches the bottom of the window.
    let menu_height = 6.0 + 5.0 * item_height;
    let top = if body.y + menu_height > size.height as f32 {
        (body.y - menu_height).max(0.0)
    } else {
        body.y
    };
    let left = body.x.min(size.width as f32 - 140.0);
    let item = Point::new(left + 24.0, top + 3.0 + item_height * (index as f32 + 0.5));
    click(ui, renderer, item)
}

fn gap_10_fixture(editing: bool) -> App {
    let mut app = fixture();
    app.cron_settings.jobs_loading = false;
    app.cron_settings.save_error = Some("Fixture schedule error".into());
    if editing {
        app.cron_settings.active_tab = CronSettingsTab::Jobs;
        app.cron_settings.jobs = vec![super::system_settings_cron_tests::job(JOB_ID)];
        app.cron_settings.editing_job_id = Some(JOB_ID.into());
    } else {
        app.cron_settings.active_tab = CronSettingsTab::Add;
        app.cron_settings.show_add_modal = true;
    }
    app
}

fn gap_10_view(app: &App) -> Element<'_, Message> {
    view_overlays(app, frame(view(app)))
}

fn gap_10_draft(app: &mut App, editing: bool) -> &mut crate::app::state::CronJobDraft {
    if editing { &mut app.cron_settings.edit_draft } else { &mut app.cron_settings.add_draft }
}

fn assert_body_hides(ui: &mut Ui<'_>, renderer: &Renderer, explanation: &str) {
    assert!(
        geometry(ui, renderer).labels.iter().all(|(label, _)| label != explanation),
        "static explanation is still shown in the form: {explanation}"
    );
}

fn assert_gap_10_help(
    app: &App,
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    size: Size<u32>,
    keys: (&str, &str),
) {
    let tr = I18n::new(app.desktop_locale);
    let (title, explanation) = (tr.__(keys.0), tr.__(keys.1));
    assert_body_hides(ui, renderer, explanation);
    let bounds = geometry(ui, renderer).labels.iter().rfind(|(label, _)| label == title).unwrap().1;
    let icon = reveal_form_point(
        ui,
        renderer,
        Point::new(bounds.x + bounds.width + 15.0, bounds.center_y()),
    );
    let tag = format!("gap-10-{:?}-{}-{title}", app.desktop_locale, size.width);
    let idle = capture(
        ui,
        renderer,
        &app.app_theme,
        size,
        &format!("{tag}-idle"),
        mouse::Cursor::Unavailable,
    );
    assert!(
        send(ui, renderer, at(icon), &[Event::Mouse(mouse::Event::CursorMoved { position: icon })])
            .is_empty()
    );
    let hover = capture(ui, renderer, &app.app_theme, size, &format!("{tag}-hover"), at(icon));
    assert!(idle != hover, "{title}: hover did not render");
    assert!(click(ui, renderer, icon).is_empty());
    let pinned = capture(
        ui,
        renderer,
        &app.app_theme,
        size,
        &format!("{tag}-pinned"),
        mouse::Cursor::Unavailable,
    );
    assert!(idle != pinned, "{title}: the help card did not stay open");
    // The card opens on the side with enough room for the 300px panel.
    let side = if icon.x < size.width as f32 / 2.0 { 29.0 } else { -29.0 };
    let body = Point::new(icon.x + side, icon.y + 5.0);
    let expected = if keys.1 == WORKDAY_EXAMPLE {
        format!("{}\n{}", tr.__(CRON_FORMAT_HELP), explanation)
    } else {
        explanation.to_owned()
    };
    let messages = help_menu_action(ui, renderer, body, size, 0);
    assert!(
        matches!(messages.as_slice(), [Message::CopyCode(value)] if value == &expected),
        "{title}: the card did not expose its localized explanation: {messages:?}"
    );
    assert!(key_at(ui, renderer, Named::Escape).is_empty(), "{title}: Escape closed the parent");
    let closed = capture(
        ui,
        renderer,
        &app.app_theme,
        size,
        &format!("{tag}-closed"),
        mouse::Cursor::Unavailable,
    );
    assert!(idle == closed, "{title}: Escape did not dismiss the help card");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_10_add_shell_hides_task_type_explanation() {
    if !isolated("settings_help_gap_10_add_shell_hides_task_type_explanation") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = gap_10_fixture(false);
    app.desktop_locale = DesktopLocale::TraditionalChinese;
    app.app_theme = Theme::Dark;
    app.cron_settings.add_draft.job_type = CronAddJobType::Shell;
    app.cron_settings.add_draft.project_path = PROJECT.into();
    let tr = I18n::new(app.desktop_locale);
    let size = Size::new(420, 740);
    let mut ui = scene(gap_10_view(&app), &mut renderer, size);
    for help in [
        ("任务类型", JOB_TYPE_HELP),
        ("任务名称", "可选，便于在列表中识别。"),
        ("项目路径", "可手工输入项目目录；留空使用默认工作区。"),
        ("执行命令", "按安全策略执行的 Shell 命令。"),
        ("执行后删除", "适合指定时间的一次性任务。"),
        ("结果投递", "将任务执行结果投递到配置好的通道。"),
    ] {
        assert_gap_10_help(&app, &mut ui, &mut renderer, size, help);
    }
    let positions = geometry(&mut ui, &renderer);
    for label in [
        "Fixture schedule error",
        tr.__("选择任务执行所在项目，Shell 和 Agent 都会切到该目录。"),
        tr.__("运行成功后删除任务"),
    ] {
        assert!(positions.label(label).width > 0.0);
    }

    let project = positions.input(PROJECT);
    let project_picker = positions
        .focusables
        .iter()
        .rev()
        .find(|bounds| bounds.x == project.x && bounds.y < project.y)
        .unwrap();
    let trigger = reveal_form_point(&mut ui, &mut renderer, project_picker.center());
    assert!(click(&mut ui, &mut renderer, trigger).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::Home).is_empty());
    assert!(matches!(
        key_at(&mut ui, &mut renderer, Named::Enter).as_slice(),
        [Message::Settings(SettingsMessage::CronAddProjectPathChanged(value))] if value.is_empty()
    ));

    let point = reveal_form_point(&mut ui, &mut renderer, project.center());
    assert!(click(&mut ui, &mut renderer, point).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::End).is_empty());
    assert!(matches!(
        key_at(&mut ui, &mut renderer, Named::Backspace).as_slice(),
        [Message::Settings(SettingsMessage::CronAddProjectPathChanged(value))]
            if value == &PROJECT[..PROJECT.len() - 1]
    ));
    let submit = positions.label(tr.__("保存"));
    assert_fits(submit, size);
    let submit = submit.center();
    assert!(click(&mut ui, &mut renderer, submit).is_empty(), "未预览的新增草稿不能保存");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_10_edit_agent_hides_fallback_explanation() {
    if !isolated("settings_help_gap_10_edit_agent_hides_fallback_explanation") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = gap_10_fixture(true);
    app.desktop_locale = DesktopLocale::English;
    app.app_theme = Theme::Light;
    app.cron_settings.edit_draft.job_type = CronAddJobType::Agent;
    let tr = I18n::new(app.desktop_locale);
    let size = Size::new(640, 700);
    let mut ui = scene(gap_10_view(&app), &mut renderer, size);
    for help in [
        ("回退模型", FALLBACK_HELP),
        ("智能体", "默认使用 Main，套用代理配置中的 provider、模型和温度。"),
        ("Agent 提示词", "创建 Agent 类型定时任务。"),
        ("模型提供商", "与智能体配置中的模型选择一致。"),
        ("模型下拉", "从当前提供商的模型列表选择，会写入 provider/model。"),
        ("模型手工输入", "可选，留空使用代理或全局默认模型。"),
        ("唤醒", "保留唤醒标记，供桌面侧识别需要关注的任务。"),
    ] {
        assert_gap_10_help(&app, &mut ui, &mut renderer, size, help);
    }
    let positions = geometry(&mut ui, &renderer);
    for label in [
        "Fixture schedule error",
        tr.__("获取完全访问权限"),
        tr.__("开启后触发时投递到项目任务池，由任务看板接管执行。"),
    ] {
        assert!(positions.label(label).width > 0.0);
    }
    assert!(positions.labels.iter().all(|(label, _)| label != tr.__("投递目标")));
    let toggle =
        reveal_form_point(&mut ui, &mut renderer, positions.label(tr.__("开启结果投递")).center());
    assert!(matches!(
        click(&mut ui, &mut renderer, toggle).as_slice(),
        [Message::Settings(SettingsMessage::CronJobEditDeliveryEnabledToggled(true))]
    ));
    drop(ui);

    let draft = &mut app.cron_settings.edit_draft;
    draft.delivery_enabled = true;
    draft.delivery_channel = "fixture-channel".into();
    draft.delivery_to = "fixture-recipient".into();
    let mut ui = scene(gap_10_view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    assert!(
        positions
            .label("填写接收用户 ID / 会话 ID；不会自动选择最近联系人。旧通道沿用原目标格式。")
            .width
            > 0.0
    );
    assert!(positions.input("fixture-recipient").width > 0.0);
    let toggle = reveal_form_point(
        &mut ui,
        &mut renderer,
        positions.label(tr.__("投递失败不影响任务状态")).center(),
    );
    assert!(matches!(
        click(&mut ui, &mut renderer, toggle).as_slice(),
        [Message::Settings(SettingsMessage::CronJobEditDeliveryBestEffortToggled(false))]
    ));
    for (key, action) in
        [("保存", SettingsMessage::CronJobEditSave), ("取消", SettingsMessage::CronJobEditCanceled)]
    {
        let bounds = geometry(&mut ui, &renderer).label(tr.__(key));
        assert_fits(bounds, size);
        let messages = click(&mut ui, &mut renderer, bounds.center());
        if key == "保存" {
            assert!(messages.is_empty());
        } else {
            assert!(
                matches!(messages.as_slice(), [Message::Settings(actual)] if std::mem::discriminant(actual) == std::mem::discriminant(&action))
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_10_advanced_hides_workday_example() {
    if !isolated("settings_help_gap_10_advanced_hides_workday_example") {
        return;
    }
    let mut renderer = renderer().await;
    let size = Size::new(420, 740);
    for editing in [false, true] {
        let mut app = gap_10_fixture(editing);
        let draft = gap_10_draft(&mut app, editing);
        draft.schedule_draft = Some(crate::app::state::CronScheduleDraft {
            rule: crate::app::state::CronScheduleRule::Advanced("0 9 * * MON-FRI".into()),
            ..Default::default()
        });
        let mut ui = scene(gap_10_view(&app), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        assert!(
            positions.label("5 字段：分 时 日 月 星期；6 字段在最前增加秒。使用当前时区。").width
                > 0.0
        );
        let point =
            reveal_form_point(&mut ui, &mut renderer, positions.input("0 9 * * MON-FRI").center());
        assert!(click(&mut ui, &mut renderer, point).is_empty());
        assert!(key_at(&mut ui, &mut renderer, Named::End).is_empty());
        assert!(matches!(key_at(&mut ui, &mut renderer, Named::Backspace).as_slice(),
            [Message::Settings(SettingsMessage::CronScheduleDraftChanged { editing: actual, draft })]
            if *actual == editing && matches!(&draft.rule, crate::app::state::CronScheduleRule::Advanced(value) if value == "0 9 * * MON-FR")));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn cron_agent_and_model_choices_render_below_project() {
    if !isolated("cron_agent_and_model_choices_render_below_project") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.cron_settings.active_tab = CronSettingsTab::Add;
    app.cron_settings.add_draft.project_path = PROJECT.into();
    app.acp_agents = vec!["codex".into(), "claude".into()];
    assert_eq!(app.cron_settings.add_draft.job_type, CronAddJobType::Agent);
    let size = Size::new(960, 2800);
    for theme in [Theme::Light, Theme::Dark] {
        app.app_theme = theme.clone();
        let mut ui = scene(frame(add_tab(&app)), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        assert!(!positions.labels.iter().any(|(label, _)| label == "ACP 智能体"));
        let mut previous = positions.label("项目路径");
        for label in ["智能体", "模型提供商", "模型下拉", "模型手工输入", "Agent 提示词"]
        {
            let bounds = positions.label(label);
            assert_fits(bounds, size);
            assert!(bounds.y > previous.y, "{label} must follow the project execution choices");
            previous = bounds;
        }
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            size,
            "cron-agent-choices",
            mouse::Cursor::Unavailable,
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn cron_edit_form_omits_separate_acp_selector() {
    if !isolated("cron_edit_form_omits_separate_acp_selector") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.acp_agents = vec!["codex".into(), "claude".into()];
    app.cron_settings.edit_draft.job_type = CronAddJobType::Agent;
    let size = Size::new(960, 2800);
    for theme in [Theme::Light, Theme::Dark] {
        let mut ui = scene(frame(edit_form(&app)), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        assert!(!positions.labels.iter().any(|(label, _)| label == "ACP 智能体"));
        assert_fits(positions.label("智能体"), size);
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            size,
            "cron-edit-agent-choices",
            mouse::Cursor::Unavailable,
        );
    }
}

#[path = "system_settings_cron/list_tests.rs"]
mod list_tests;

#[tokio::test(flavor = "multi_thread")]
async fn cron_modal_renders_add_and_edit() {
    if !isolated("cron_modal_renders_add_and_edit") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    let size = Size::new(900, 700);
    for editing in [false, true] {
        app.cron_settings.show_add_modal = !editing;
        app.cron_settings.editing_job_id = editing.then(|| "job".into());
        for theme in [Theme::Light, Theme::Dark] {
            let mut ui = scene(view_overlays(&app, frame(view(&app))), &mut renderer, size);
            let positions = geometry(&mut ui, &renderer);
            assert_fits(
                positions.label(if editing { "编辑定时任务" } else { "新建定时任务" }),
                size,
            );
            capture(&mut ui, &mut renderer, &theme, size, "cron-modal", mouse::Cursor::Unavailable);
            let before = geometry(&mut ui, &renderer);
            let title =
                before.label(if editing { "编辑定时任务" } else { "新建定时任务" });
            let scroll = before.scroll.unwrap().0;
            assert!(wheel(&mut ui, &mut renderer, scroll.center()).is_empty());
            let after = geometry(&mut ui, &renderer);
            assert_eq!(after.label(if editing { "编辑定时任务" } else { "新建定时任务" }), title);
            assert!(after.scroll.unwrap().1.y > before.scroll.unwrap().1.y);
            let messages = send(
                &mut ui,
                &mut renderer,
                mouse::Cursor::Unavailable,
                &[key(iced::keyboard::key::Named::Escape)],
            );
            assert!(messages.iter().any(|m| matches!(
                m,
                Message::Settings(message::SettingsMessage::CronJobEditCanceled)
            )));
        }
    }
}
