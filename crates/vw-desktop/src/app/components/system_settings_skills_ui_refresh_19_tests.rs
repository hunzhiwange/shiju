#![cfg(not(target_arch = "wasm32"))]

use super::*;
use crate::app::state::{
    PluginCatalogItem, PluginInstallForm, PluginInstallPhase, SkillsCatalogItem, SkillsCatalogKind,
    SkillsDirectoryScope, SkillsSelectedDetail,
};
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::{Event, Font, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};
use vw_gateway_client::{
    DesktopPluginCapabilitiesDto, DesktopPluginDependencyDto, DesktopPluginPreviewDto,
};

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_19_CHILD";
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_19_SCREENSHOTS") {
        image::save_buffer(
            std::path::PathBuf::from(directory).join(format!("{name}.png")),
            &pixels,
            size.width,
            size.height,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
    pixels
}

fn snapshot(
    view: Element<'_, Message>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
) {
    let mut ui = scene(view, renderer, size);
    capture(&mut ui, renderer, theme, size, name, mouse::Cursor::Unavailable);
}

const PATH: &str = "/虚构專案/fixture-project";
const RECENT: &str = "/其他專案/second-project";
const URL: &str = "git+https://example.invalid/測試/plugin.git#v2.0.0";

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    let mut app = App::new().0;
    app.project_path = Some(PATH.into());
    app.recent_projects = vec![RECENT.into()];
    let s = &mut app.skills_settings;
    s.query = "Fixture".into();
    s.directory_scope = SkillsDirectoryScope::All;
    s.catalog = vec![SkillsCatalogItem {
        id: "fixture-skill".into(),
        title: "示例技能 Fixture Skill".into(),
        description: "虛構技能說明 / A sample description for the skills catalog.".into(),
        kind: SkillsCatalogKind::Recommended,
        resource_count: 3,
        installed: false,
        enabled: true,
        source: "bundled".into(),
        source_path: None,
    }];
    s.plugin_catalog = vec![PluginCatalogItem {
        id: "fixture-plugin".into(),
        display_name: "示例外掛 Fixture Plugin".into(),
        version: "1.0.0".into(),
        description: "A sample plugin with a long package source path.".into(),
        enabled: true,
        component_count: 4,
        source_path: Some(format!("{PATH}/{}", "子目錄/long-segment/".repeat(8))),
        dependency_checks: vec![dependency()],
        origin: Default::default(),
        hook_review: "pending".into(),
    }];
    app
}

fn dependency() -> DesktopPluginDependencyDto {
    DesktopPluginDependencyDto {
        component: "fixture".into(),
        dependency: "fixture-bin".into(),
        detail: "未找到測試程式 / Missing fixture dependency".into(),
        status: "missing_host".into(),
    }
}

fn preview() -> DesktopPluginPreviewDto {
    DesktopPluginPreviewDto {
        token: "fixture-token".into(),
        plugin_id: "fixture-plugin".into(),
        source: format!(
            "git+https://example.invalid/{}plugin.git#v2.0.0",
            "測試長路徑/segment/".repeat(12)
        ),
        old_version: Some("1.0.0".into()),
        version: Some("2.0.0".into()),
        capabilities: DesktopPluginCapabilitiesDto {
            skills: 2,
            mcp_servers: 1,
            hooks: 1,
            interface: false,
        },
        audit_status: "passed".into(),
        dependency_status: "unverified".into(),
        origin: Default::default(),
        dependency_checks: vec![dependency(); 8],
        previous_capabilities: None,
    }
}

fn skill_detail() -> SkillsSelectedDetail {
    SkillsSelectedDetail {
        id: "fixture-skill".into(),
        title: "示例技能 Fixture Skill".into(),
        description: "虛構技能說明 / A sample description for the skills catalog.".into(),
        kind: SkillsCatalogKind::Recommended,
        installed: false,
        enabled: true,
        source: "bundled".into(),
        source_path: Some(format!("{PATH}/{}", "測試路徑/segment/".repeat(7))),
        document_name: "SKILL.md".into(),
        document_content: "# 示例技能\n示例技能文檔 / Example skill content.\n".repeat(20),
        can_install: true,
        can_toggle: false,
        can_delete: false,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_19_renders_catalogs_and_modals() {
    if !isolated("ui_refresh_19_renders_catalogs_and_modals") {
        return;
    }
    let mut renderer = renderer().await;
    for (locale, width) in [
        (DesktopLocale::SimplifiedChinese, 640),
        (DesktopLocale::TraditionalChinese, 960),
        (DesktopLocale::English, 640),
    ] {
        for theme in [Theme::Light, Theme::Dark] {
            let mut app = fixture();
            app.desktop_locale = locale;
            app.recent_projects.push(format!("{PATH}/{}", "長目錄/long-segment/".repeat(20)));
            let size = Size::new(width, 820);
            let narrow = Size::new(420, 660);
            let stem = format!("{locale:?}-{theme:?}");
            snapshot(view(&app), &mut renderer, &theme, size, &format!("{stem}-skills"));
            snapshot(plugins::view(&app), &mut renderer, &theme, size, &format!("{stem}-plugins"));
            app.skills_settings.active_tab = SkillsSettingsTab::DiscoveryOrder;
            snapshot(view(&app), &mut renderer, &theme, size, &format!("{stem}-order"));
            app.skills_settings.active_tab = SkillsSettingsTab::SystemConfig;
            snapshot(view(&app), &mut renderer, &theme, size, &format!("{stem}-config"));
            app.skills_settings.active_tab = SkillsSettingsTab::Skills;
            app.skills_settings.selected_skill_id = Some("fixture-skill".into());
            app.skills_settings.selected_skill_detail = Some(skill_detail());
            snapshot(
                view_overlays(&app, view(&app)),
                &mut renderer,
                &theme,
                narrow,
                &format!("{stem}-detail"),
            );
            app.skills_settings.selected_skill_id = None;
            app.skills_settings.show_help_modal = true;
            snapshot(
                view_overlays(&app, view(&app)),
                &mut renderer,
                &theme,
                narrow,
                &format!("{stem}-help"),
            );
            let mut form = PluginInstallForm::new(7, None);
            form.source = URL.into();
            form.phase = PluginInstallPhase::Review(preview());
            app.skills_settings.plugin_install = Some(form);
            snapshot(
                with_plugin_install(&app, plugins::view(&app)),
                &mut renderer,
                &theme,
                narrow,
                &format!("{stem}-review"),
            );
            let form = app.skills_settings.plugin_install.as_mut().unwrap();
            form.phase = PluginInstallPhase::Failed("示例失敗 / Fixture audit failed".into());
            snapshot(
                with_plugin_install(&app, plugins::view(&app)),
                &mut renderer,
                &theme,
                narrow,
                &format!("{stem}-failure"),
            );
            app.skills_settings.catalog.clear();
            app.project_path = None;
            app.recent_projects.clear();
            app.skills_settings.plugin_catalog.clear();
            snapshot(view(&app), &mut renderer, &theme, size, &format!("{stem}-empty"));
            app.skills_settings.plugin_loading = true;
            snapshot(plugins::view(&app), &mut renderer, &theme, size, &format!("{stem}-loading"));
        }
    }
}

#[derive(Default)]
struct Geometry {
    labels: Vec<(String, iced::Rectangle)>,
    inputs: Vec<(String, iced::Rectangle)>,
    focusable: Vec<iced::Rectangle>,
    scroll_offset: iced::Vector,
}

impl iced::advanced::widget::Operation for Geometry {
    fn scrollable(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        _: iced::Rectangle,
        _: iced::Rectangle,
        translation: iced::Vector,
        _: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        self.scroll_offset = translation;
    }

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
        self.focusable.push(bounds);
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
            .unwrap_or_else(|| panic!("missing fixture input: {value}"))
            .1
    }
}

fn geometry(ui: &mut Ui<'_>, renderer: &Renderer) -> Geometry {
    let mut positions = Geometry::default();
    ui.operate(renderer, &mut positions);
    positions
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

fn edit(ui: &mut Ui<'_>, renderer: &mut Renderer, point: iced::Point, value: &str) -> Vec<Message> {
    assert!(click(ui, renderer, point).is_empty());
    send(
        ui,
        renderer,
        mouse::Cursor::Available(point),
        &[
            key(iced::keyboard::key::Named::End),
            Event::InputMethod(iced::advanced::input_method::Event::Commit(value.into())),
        ],
    )
}

fn scroll(ui: &mut Ui<'_>, renderer: &mut Renderer, point: iced::Point, delta: f32) {
    assert!(
        send(
            ui,
            renderer,
            mouse::Cursor::Available(point),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: delta },
            })],
        )
        .is_empty()
    );
}

fn press_label(ui: &mut Ui<'_>, renderer: &mut Renderer, value: &str) -> Vec<Message> {
    let point = geometry(ui, renderer).label(value).center();
    click(ui, renderer, point)
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_19_layout_and_interaction() {
    if !isolated("ui_refresh_19_layout_and_interaction") {
        return;
    }
    use iced::keyboard::key::Named;
    use message::SettingsMessage::*;
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.recent_projects.extend(
        (0..30).map(|index| format!("/示例專案/{index:02}/{}", "長目錄/long-segment/".repeat(12))),
    );
    let size = Size::new(640, 820);
    let mut ui = scene(view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let project_trigger = *positions
        .focusable
        .iter()
        .find(|bounds| (bounds.height - 38.0).abs() < 0.5)
        .expect("project selector trigger");
    assert!((project_trigger.center_y() - positions.label("技能作用域").center_y()).abs() < 0.5);
    assert!(matches!(
        edit(&mut ui, &mut renderer, positions.input("Fixture").center(), "示例").as_slice(),
        [Message::Settings(SkillsQueryChanged(query))] if query == "Fixture示例"
    ));
    assert!(matches!(
        press_label(&mut ui, &mut renderer, "刷新").as_slice(),
        [Message::Settings(SkillsRefresh)]
    ));
    assert!(matches!(
        press_label(&mut ui, &mut renderer, "全局目录").as_slice(),
        [Message::Settings(SkillsDirectoryScopeChanged(SkillsDirectoryScope::Global))]
    ));
    assert!(matches!(
        press_label(&mut ui, &mut renderer, "示例技能 Fixture Skill").as_slice(),
        [Message::Settings(SkillsDetailRequested(id))] if id == "fixture-skill"
    ));

    assert!(click(&mut ui, &mut renderer, project_trigger.center()).is_empty());
    let mut hovered = Vec::new();
    for (index, offset) in [25.0, 62.0].into_iter().enumerate() {
        let point = iced::Point::new(
            project_trigger.center_x(),
            project_trigger.y + project_trigger.height + offset,
        );
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
        hovered.push(capture(
            &mut ui,
            &mut renderer,
            &Theme::Light,
            size,
            &format!("project-hover-{index}"),
            cursor,
        ));
    }
    let outside = iced::Point::new(2.0, 2.0);
    let cursor = mouse::Cursor::Available(outside);
    assert!(
        send(
            &mut ui,
            &mut renderer,
            cursor,
            &[Event::Mouse(mouse::Event::CursorMoved { position: outside })]
        )
        .is_empty()
    );
    let left = capture(&mut ui, &mut renderer, &Theme::Light, size, "project-hover-left", cursor);
    assert_ne!(hovered[0], hovered[1]);
    assert!(hovered.iter().all(|pixels| *pixels != left));

    scroll(
        &mut ui,
        &mut renderer,
        iced::Point::new(
            project_trigger.center_x(),
            project_trigger.y + project_trigger.height + 25.0,
        ),
        -800.0,
    );
    assert!(
        send(
            &mut ui,
            &mut renderer,
            cursor,
            &[Event::Mouse(mouse::Event::CursorMoved { position: outside })]
        )
        .is_empty()
    );
    let scrolled =
        capture(&mut ui, &mut renderer, &Theme::Light, size, "project-menu-scrolled", cursor);
    assert_ne!(left, scrolled, "wheel should reveal later project paths");
    let keyboard = mouse::Cursor::Unavailable;
    assert!(send(&mut ui, &mut renderer, keyboard, &[key(Named::Escape)]).is_empty());
    assert!(send(&mut ui, &mut renderer, keyboard, &[key(Named::Enter)]).is_empty());
    assert!(send(&mut ui, &mut renderer, keyboard, &[key(Named::ArrowDown)]).is_empty());
    assert!(matches!(
        send(&mut ui, &mut renderer, keyboard, &[key(Named::Enter)]).as_slice(),
        [Message::Settings(SkillsProjectSelected(path))] if path == RECENT
    ));

    drop(ui);
    app.skills_settings.active_tab = SkillsSettingsTab::SystemConfig;
    let mut ui = scene(view(&app), &mut renderer, size);
    assert!(matches!(
        press_label(&mut ui, &mut renderer, "默认加载项目技能").as_slice(),
        [Message::Settings(SkillsProjectAutoLoadToggled(false))]
    ));
    assert!(matches!(
        press_label(&mut ui, &mut renderer, "使用 full（关闭时为 compact）").as_slice(),
        [Message::Settings(SkillsPromptInjectionModeChanged(SkillsPromptInjectionMode::Full))]
    ));
    let trigger = geometry(&mut ui, &renderer).focusable[0];
    assert!((trigger.height - 38.0).abs() < 0.5);
    assert!(click(&mut ui, &mut renderer, trigger.center()).is_empty());
    assert!(send(&mut ui, &mut renderer, keyboard, &[key(Named::ArrowDown)]).is_empty());
    assert!(matches!(
        send(&mut ui, &mut renderer, keyboard, &[key(Named::Enter)]).as_slice(),
        [Message::Settings(SkillsDirectoryProviderChanged(SkillsDirectoryProvider::Codex))]
    ));

    drop(ui);
    app.skills_settings.active_tab = SkillsSettingsTab::Skills;
    app.skills_settings.selected_skill_id = Some("fixture-skill".into());
    app.skills_settings.selected_skill_detail = Some(skill_detail());
    let narrow = Size::new(420, 660);
    let mut ui = scene(view_overlays(&app, view(&app)), &mut renderer, narrow);
    assert!(matches!(
        press_label(&mut ui, &mut renderer, "安装到项目").as_slice(),
        [Message::Settings(SkillsInstallBuiltInRequested(id))] if id == "fixture-skill"
    ));
    drop(ui);
    app.project_path = None;
    let mut ui = scene(view_overlays(&app, view(&app)), &mut renderer, narrow);
    assert!(press_label(&mut ui, &mut renderer, "安装到项目").is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_19_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_19_preserves_values_and_disabled_states") {
        return;
    }
    use iced::keyboard::key::Named;
    use message::SettingsMessage::*;
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    let size = Size::new(960, 1000);
    let mut ui = scene(plugins::view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    assert!(matches!(
        edit(&mut ui, &mut renderer, positions.input("Fixture").center(), "fixture-plugin").as_slice(),
        [Message::Settings(SkillsQueryChanged(value))] if value == "Fixturefixture-plugin"
    ));
    assert!(matches!(
        press_label(&mut ui, &mut renderer, "示例外掛 Fixture Plugin").as_slice(),
        [Message::Settings(PluginsSelected(id))] if id == "fixture-plugin"
    ));
    assert!(matches!(
        press_label(&mut ui, &mut renderer, "禁用").as_slice(),
        [Message::Settings(PluginsSetEnabledRequested { plugin_id, enabled: false })] if plugin_id == "fixture-plugin"
    ));
    assert!(matches!(
        press_label(&mut ui, &mut renderer, "删除").as_slice(),
        [Message::Settings(PluginsDeleteRequested(id))] if id == "fixture-plugin"
    ));
    drop(ui);

    app.skills_settings.plugin_loading = true;
    app.skills_settings.plugin_catalog[0].enabled = false;
    let mut ui = scene(plugins::view(&app), &mut renderer, size);
    assert!(press_label(&mut ui, &mut renderer, "刷新中").is_empty());
    assert!(press_label(&mut ui, &mut renderer, "安装插件").is_empty());
    assert!(press_label(&mut ui, &mut renderer, "启用").is_empty());
    assert!(press_label(&mut ui, &mut renderer, "删除").is_empty());
    drop(ui);

    app.skills_settings.plugin_loading = false;
    let mut ui = scene(plugins::view(&app), &mut renderer, size);
    assert!(matches!(
        press_label(&mut ui, &mut renderer, "启用").as_slice(),
        [Message::Settings(PluginsSetEnabledRequested { plugin_id, enabled: true })] if plugin_id == "fixture-plugin"
    ));
    drop(ui);

    let mut form = PluginInstallForm::new(7, None);
    form.source = URL.into();
    let narrow = Size::new(420, 660);
    app.skills_settings.plugin_loading = false;
    app.skills_settings.plugin_install = Some(form);
    let mut ui = scene(with_plugin_install(&app, plugins::view(&app)), &mut renderer, narrow);
    let source = geometry(&mut ui, &renderer).input(URL).center();
    assert!(matches!(
        edit(&mut ui, &mut renderer, source, "-beta").as_slice(),
        [Message::Settings(PluginsInstallSourceChanged(value))] if value == &format!("{URL}-beta")
    ));
    assert!(
        send(
            &mut ui,
            &mut renderer,
            mouse::Cursor::Available(source),
            &[key(Named::Enter), key(Named::Escape)]
        )
        .is_empty()
    );
    assert!(matches!(
        press_label(&mut ui, &mut renderer, "审计预览").as_slice(),
        [Message::Settings(PluginsInstallPrepare)]
    ));
    assert!(press_label(&mut ui, &mut renderer, "确认安装").is_empty());
    drop(ui);

    let form = app.skills_settings.plugin_install.as_mut().unwrap();
    form.update_id = Some("fixture-plugin".into());
    form.source = format!("{PATH}/fixture-plugin");
    form.phase = PluginInstallPhase::Review(preview());
    let mut ui = scene(with_plugin_install(&app, plugins::view(&app)), &mut renderer, narrow);
    let positions = geometry(&mut ui, &renderer);
    let title = positions.label("更新插件");
    let confirm = positions.label("确认更新");
    assert!(confirm.y + confirm.height < narrow.height as f32);
    assert!(positions.input(&format!("{PATH}/fixture-plugin")).width > 250.0);
    scroll(&mut ui, &mut renderer, iced::Point::new(210.0, 360.0), -800.0);
    let scrolled = geometry(&mut ui, &renderer);
    assert!(scrolled.scroll_offset.y > 0.0);
    assert_eq!(scrolled.label("更新插件"), title);
    assert_eq!(scrolled.label("确认更新"), confirm);
    assert!(matches!(
        press_label(&mut ui, &mut renderer, "确认更新").as_slice(),
        [Message::Settings(PluginsInstallConfirm)]
    ));
    assert!(matches!(
        press_label(&mut ui, &mut renderer, "取消").as_slice(),
        [Message::Settings(PluginsInstallClose)]
    ));
    drop(ui);

    let form = app.skills_settings.plugin_install.as_mut().unwrap();
    form.phase = PluginInstallPhase::Committing(preview());
    let mut ui = scene(with_plugin_install(&app, plugins::view(&app)), &mut renderer, narrow);
    let source = geometry(&mut ui, &renderer).input(&format!("{PATH}/fixture-plugin")).center();
    assert!(edit(&mut ui, &mut renderer, source, "-blocked").is_empty());
    for action in ["审计预览", "确认更新", "取消"] {
        assert!(press_label(&mut ui, &mut renderer, action).is_empty(), "{action}");
    }
}
