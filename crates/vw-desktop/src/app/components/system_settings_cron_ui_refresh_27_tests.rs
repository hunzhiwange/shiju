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
async fn ui_refresh_27_layout_and_interaction() {
    if !isolated("ui_refresh_27_layout_and_interaction") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    let cron = &mut app.cron_settings;
    cron.active_tab = CronSettingsTab::Jobs;
    cron.jobs_loading = false;
    let mut job = super::system_settings_cron_tests::job(JOB_ID);
    job.name = Some("定期巡檢 / Health check".into());
    job.project_path = Some(PROJECT.into());
    cron.jobs = vec![job];
    cron.expanded_job_ids = vec![JOB_ID.into()];
    cron.editing_job_id = Some(JOB_ID.into());
    cron.edit_draft.project_path = PROJECT.into();
    cron.edit_draft.schedule = "0 9 * * MON".into();
    cron.edit_draft.frequency = crate::app::state::CronFrequency::Weekly;
    cron.save_error = Some("範例錯誤 / Sample save failure".into());
    app.desktop_locale = DesktopLocale::TraditionalChinese;

    let size = Size::new(420, 560);
    let mut ui = scene(view_overlays(&app, frame(view(&app))), &mut renderer, size);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "edit-zh-tw-420",
        mouse::Cursor::Unavailable,
    );
    let tr = I18n::new(app.desktop_locale);
    let before = geometry(&mut ui, &renderer);
    let save = before.label(tr.__("保存"));
    let cancel = before.label(tr.__("取消"));
    assert!(
        save.y + save.height < size.height as f32,
        "edit actions must fit a 420px window: {save:?}"
    );
    let (scroll, translation) = before.scroll.expect("edit form body scrolls independently");
    let title = before.label(tr.__("编辑"));
    let weekday = before.focusables.iter().find(|bounds| bounds.width == 126.0).unwrap();
    let shift = (weekday.center_y() - scroll.center_y()).max(0.0);
    assert!(
        send(
            &mut ui,
            &mut renderer,
            at(scroll.center()),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -shift },
            })],
        )
        .is_empty()
    );
    let translated = weekday.center() - geometry(&mut ui, &renderer).scroll.unwrap().1;
    assert!(scroll.contains(translated));
    assert!(click(&mut ui, &mut renderer, translated).is_empty());

    let menu = geometry(&mut ui, &renderer).scroll.unwrap().0;
    assert_fits(menu, size);
    assert!(menu.height >= 76.0, "the first two options must be visible: {menu:?}");
    let mut hovers = Vec::new();
    for (name, y) in [("monday", 18.0), ("tuesday", 56.0)] {
        let point = Point::new(menu.center_x(), menu.y + y);
        assert!(
            send(
                &mut ui,
                &mut renderer,
                at(point),
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })],
            )
            .is_empty()
        );
        hovers.push(capture(
            &mut ui,
            &mut renderer,
            &Theme::Dark,
            size,
            &format!("edit-menu-{name}-zh-tw-420"),
            at(point),
        ));
    }
    assert_ne!(hovers[0], hovers[1], "hover must follow the cursor after rebuilding the overlay");
    assert!(
        send(
            &mut ui,
            &mut renderer,
            mouse::Cursor::Unavailable,
            &[Event::Mouse(mouse::Event::CursorLeft)],
        )
        .is_empty()
    );
    assert!(key_at(&mut ui, &mut renderer, Named::Escape).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::Enter).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::ArrowDown).is_empty());
    assert!(matches!(key_at(&mut ui, &mut renderer, Named::Enter).as_slice(),
        [Message::Settings(SettingsMessage::CronJobEditScheduleChanged(value))] if value == "0 9 * * TUE"));

    assert!(wheel(&mut ui, &mut renderer, scroll.center()).is_empty());
    let after = geometry(&mut ui, &renderer);
    assert_eq!(after.label(tr.__("保存")), save);
    assert_eq!(after.label(tr.__("取消")), cancel);
    assert_eq!(after.label(tr.__("编辑")), title);
    assert!(after.scroll.unwrap().1.y > translation.y);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "edit-scrolled-zh-tw-420",
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
    app.cron_settings.add_draft.project_path = PROJECT.into();
    app.cron_settings.add_draft.delivery_enabled = true;
    app.cron_settings.add_draft.delivery_channel = "telegram".into();
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
        let mut ui = scene(frame(view(&app)), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        assert!(positions.input(PROJECT).width > 0.0);
        assert_fits(positions.input("telegram"), size);
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            size,
            &format!("add-{tag}"),
            mouse::Cursor::Unavailable,
        );
        let target = positions.input("telegram");
        assert!(click(&mut ui, &mut renderer, target.center()).is_empty());
        assert!(key_at(&mut ui, &mut renderer, Named::End).is_empty());
        assert!(matches!(key_at(&mut ui, &mut renderer, Named::Backspace).as_slice(),
            [Message::Settings(SettingsMessage::CronAddDeliveryChannelChanged(value))] if value == "telegra"));
        let create =
            positions.labels.iter().rev().find(|(label, _)| label == tr.__("新增任务")).unwrap().1;
        assert_fits(create, size);
        assert!(matches!(
            click(&mut ui, &mut renderer, create.center()).as_slice(),
            [Message::Settings(SettingsMessage::CronAddSubmit)]
        ));
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
                let hint = positions.label(tr.__("可选的心跳结果投递通道。"));
                let field = positions.input("telegram");
                assert_fits(field, size);
                assert!((field.center_y() - (label.y + hint.y + hint.height) / 2.0).abs() < 0.5);
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
        assert_fits(bounds, size);
        let messages = click(&mut ui, &mut renderer, bounds.center());
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
                scene.check(app, view(app), &labels, 2),
                SettingsMessage::CronTabSelected(CronSettingsTab::Scheduler)
            ));
        },
    )
    .await;
}
