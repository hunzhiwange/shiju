use super::super::{defaults, identity, mcp, profile_modal, skills, tools};
use crate::app::i18n::settings_extensions::text as tr;
use crate::app::message::settings::{AgentsMessage as A, SettingsMessage as S};
use crate::app::state::{
    AGENT_DETAIL_BASIC_TAB, AGENT_DETAIL_IDENTITY_TAB, AGENT_DETAIL_MCP_TAB,
    AGENT_DETAIL_SKILLS_TAB, AGENT_DETAIL_TOOLS_TAB, AGENT_TOOLS_BUILTIN_TAB,
    DelegateAgentSettingsEntry, ModelSummary, ProviderModelsSummary, ProviderSummary,
    SkillsCatalogItem, SkillsCatalogKind, WorkspaceIdentityFileState,
};
use crate::app::{App, Message};
use iced::advanced::widget::{
    Id, Operation, operation::Focusable, operation::Scrollable, operation::TextInput,
};
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::{button, container, text_editor};
use iced::{Element, Event, Font, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::agent::{AgentDefault, AgentModelReference, DelegateCapabilityPolicy};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};
use vw_gateway_client::vw_api_types::mcp::McpInspectResponse;

const AGENT: &str = "fixture-agent";
const MODEL: &str = "fixture/model-with-provider-specific-id";
const PROVIDER: &str = "fixture-provider";

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_16_CHILD";
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

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    let mut app = App::new().0;
    let mut draft = DelegateAgentSettingsEntry::from_config(AGENT, None);
    draft.label = "示例智慧體 / Example agent".into();
    draft.api_key_input = "fixture-secret".into();
    draft.system_prompt_editor =
        text_editor::Content::with_text("請先閱讀系統提示。\nTest prompt.");
    draft.defaults.permission = AgentDefault::Fixed(serde_json::json!({"*": "deny"}));
    draft.skill_policy = DelegateCapabilityPolicy::Strict;
    draft.allowed_skills = vec!["skill-one".into(), "skill-two".into()];
    let settings = &mut app.agents_settings;
    settings.config_ready = true;
    settings.selected_agent = AGENT.into();
    settings.profile_modal = Some(draft);
    settings.available_tools = ["screenshot", "http_request", "bash"].map(String::from).to_vec();
    settings.active_tools_list_tab = "optional".into();
    settings.providers = vec![ProviderSummary {
        id: PROVIDER.into(),
        name: "Fixture provider".into(),
        source_label: crate::app::i18n::settings_models::messages::Feedback::owned("配置", vec![]),
        connected: true,
    }];
    settings.provider_models = vec![ProviderModelsSummary {
        id: PROVIDER.into(),
        name: "Fixture provider".into(),
        models: (0..20)
            .map(|index| ModelSummary {
                id: if index == 0 { MODEL.into() } else { format!("model-{index:02}") },
                name: format!("Model {index}"),
                enabled: true,
                toolcall: true,
                attachment: false,
                context_limit: 4096,
                detail: serde_json::Value::Null,
            })
            .collect(),
    }];
    settings.skill_catalog = [
        ("skill-one", "第一項示例技能"),
        ("skill-two", "第二項示例技能"),
        ("skill-denied", "受限示例技能"),
    ]
    .map(|(id, title)| SkillsCatalogItem {
        id: id.into(),
        title: title.into(),
        description: "示例技能 / Example skill".into(),
        kind: SkillsCatalogKind::System,
        resource_count: 0,
        installed: true,
        enabled: true,
        source: "global".into(),
        source_path: None,
    })
    .to_vec();
    settings.skill_directory = Some(format!("/示例/{}/项目", "long-test-directory".repeat(8)));
    settings.workspace_identity_root_path = settings.skill_directory.clone();
    settings.workspace_identity_files = vec![WorkspaceIdentityFileState {
        file_name: "AGENTS.md".into(),
        label: "智能体 AGENTS".into(),
        editor: text_editor::Content::with_text("示例身份檔案\nExample identity file"),
        size_bytes: Some(64),
        modified_at_ms: None,
    }];
    app.mcp_settings.inspect_loading = false;
    app.mcp_settings.inspect_error = None;
    app.mcp_settings.inspect = Some(serde_json::from_value::<McpInspectResponse>(serde_json::json!({
        "servers": [
            {"name":"服务-one","transport":"stdio","enabled":true,"required":false,"connection_status":"connected"},
            {"name":"服务-two","transport":"stdio","enabled":true,"required":false,"connection_status":"connected"},
            {"name":"服务-disabled","transport":"stdio","enabled":false,"required":false,"connection_status":"disabled"}
        ]
    })).unwrap());
    app
}

#[derive(Default)]
struct Geometry {
    labels: Vec<(String, Rectangle)>,
    focusable: Vec<Rectangle>,
    inputs: Vec<(String, Rectangle)>,
    scrolls: Vec<(Rectangle, Rectangle, Vector)>,
}

impl Operation for Geometry {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, text: &str) {
        self.labels.push((text.into(), bounds));
    }

    fn focusable(&mut self, _: Option<&Id>, bounds: Rectangle, _: &mut dyn Focusable) {
        self.focusable.push(bounds);
    }

    fn text_input(&mut self, _: Option<&Id>, bounds: Rectangle, state: &mut dyn TextInput) {
        self.inputs.push((state.text().into(), bounds));
    }

    fn scrollable(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        content: Rectangle,
        translation: Vector,
        _: &mut dyn Scrollable,
    ) {
        self.scrolls.push((bounds, content, translation));
    }
}

impl Geometry {
    fn label(&self, label: &str) -> Rectangle {
        self.labels
            .iter()
            .find(|(text, _)| text == label)
            .unwrap_or_else(|| panic!("missing label {label:?} in {:?}", self.labels))
            .1
    }
}

struct Scene<'a> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    size: Size<u32>,
}

impl<'a> Scene<'a> {
    fn new(view: Element<'a, Message>, renderer: &mut Renderer, width: u32, height: u32) -> Self {
        Self {
            ui: UserInterface::build(
                view,
                Size::new(width as f32, height as f32),
                Cache::new(),
                renderer,
            ),
            size: Size::new(width, height),
        }
    }

    fn geometry(&mut self, renderer: &Renderer) -> Geometry {
        let mut geometry = Geometry::default();
        self.ui.operate(renderer, &mut geometry);
        geometry
    }

    fn events(
        &mut self,
        renderer: &mut Renderer,
        events: &[Event],
        cursor: mouse::Cursor,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        self.ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
        messages.retain(|message| !matches!(message, Message::None));
        messages
    }

    fn click(&mut self, renderer: &mut Renderer, point: Point) -> Vec<Message> {
        self.events(
            renderer,
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            mouse::Cursor::Available(point),
        )
    }

    fn click_label(&mut self, renderer: &mut Renderer, label: &str) -> Vec<Message> {
        let point = self.geometry(renderer).label(label).center();
        self.click(renderer, point)
    }

    fn key(&mut self, renderer: &mut Renderer, key: Named) -> Vec<Message> {
        let key = Key::Named(key);
        self.events(
            renderer,
            &[Event::Keyboard(keyboard::Event::KeyPressed {
                modified_key: key.clone(),
                key,
                physical_key: keyboard::key::Physical::Unidentified(
                    keyboard::key::NativeCode::Unidentified,
                ),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::empty(),
                text: None,
                repeat: false,
            })],
            mouse::Cursor::Unavailable,
        )
    }

    fn screenshot(
        &mut self,
        renderer: &mut Renderer,
        theme: &Theme,
        cursor: mouse::Cursor,
        name: &str,
    ) -> Vec<u8> {
        assert!(
            self.events(
                renderer,
                &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
                cursor
            )
            .is_empty()
        );
        // UserInterface reacquires the overlay before drawing, just as the runtime does.
        self.ui.draw(renderer, theme, &Style { text_color: theme.palette().text }, cursor);
        let pixels = renderer.screenshot(self.size, 1.0, theme.palette().background);
        if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_16_SCREENSHOTS") {
            let path = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&path).unwrap();
            image::save_buffer(
                path.join(format!("{name}.png")),
                &pixels,
                self.size.width,
                self.size.height,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
        pixels
    }
}

fn profile(app: &App) -> Element<'_, Message> {
    profile_modal::overlay(
        app,
        container(
            button("underlying action").on_press(Message::Settings(S::Agents(A::ProfileSave))),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into(),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_16_layout_and_interaction() {
    if !isolated("ui_refresh_16_layout_and_interaction") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    let tabs = [
        (AGENT_DETAIL_BASIC_TAB, "基础参数"),
        (AGENT_DETAIL_IDENTITY_TAB, "身份"),
        (AGENT_DETAIL_TOOLS_TAB, "工具"),
        (AGENT_DETAIL_SKILLS_TAB, "技能"),
        (AGENT_DETAIL_MCP_TAB, "MCP"),
    ];
    app.agents_settings.active_prompt_tab = "AGENTS.md".into();
    for (theme, theme_name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        for (locale, locale_name) in [
            (DesktopLocale::SimplifiedChinese, "zh-CN"),
            (DesktopLocale::TraditionalChinese, "zh-TW"),
            (DesktopLocale::English, "en"),
        ] {
            app.desktop_locale = locale;
            for (tab, label) in tabs {
                app.agents_settings.active_detail_tab = tab.into();
                let mut scene = Scene::new(profile(&app), &mut renderer, 420, 620);
                let geometry = scene.geometry(&renderer);
                for label in ["智能体配置", "保存配置", "取消", "删除智能体"] {
                    let bounds = geometry.label(tr(locale, label));
                    assert!(
                        Rectangle::new(Point::ORIGIN, Size::new(420.0, 620.0))
                            .contains(bounds.position())
                    );
                    assert!(bounds.x + bounds.width <= 420.0 && bounds.y + bounds.height <= 620.0);
                }
                assert!(matches!(
                    scene.click_label(&mut renderer, tr(locale, label)).as_slice(),
                    [Message::Settings(S::Agents(A::DetailTabSelected(selected)))] if selected == tab
                ));
                let footer = geometry.label(tr(locale, "保存配置"));
                scene.screenshot(
                    &mut renderer,
                    &theme,
                    mouse::Cursor::Unavailable,
                    &format!("modal-{tab}-420-{theme_name}-{locale_name}"),
                );
                let (scroll, content, _) = geometry.scrolls[0];
                if content.height > scroll.height {
                    let cursor =
                        mouse::Cursor::Available(Point::new(scroll.x + 2.0, scroll.center().y));
                    assert!(
                        scene
                            .events(
                                &mut renderer,
                                &[Event::Mouse(mouse::Event::WheelScrolled {
                                    delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -110.0 },
                                })],
                                cursor
                            )
                            .is_empty()
                    );
                    let after = scene.geometry(&renderer);
                    assert!(after.scrolls[0].2.y > 0.0);
                    assert_eq!(after.label(tr(locale, "保存配置")), footer);
                    if locale == DesktopLocale::SimplifiedChinese {
                        scene.screenshot(
                            &mut renderer,
                            &theme,
                            mouse::Cursor::Unavailable,
                            &format!("scrolled-{tab}-420-{theme_name}"),
                        );
                    }
                }
                assert!(matches!(
                    scene.click(&mut renderer, footer.center()).as_slice(),
                    [Message::Settings(S::Agents(A::ProfileSave))]
                ));
                assert!(matches!(
                    scene.click_label(&mut renderer, tr(locale, "取消")).as_slice(),
                    [Message::Settings(S::Agents(A::ProfileCancel))]
                ));
                assert!(scene.click(&mut renderer, Point::new(5.0, 5.0)).is_empty());
            }
        }
        for width in [640, 960] {
            let draft = app.agents_settings.profile_modal.as_ref().unwrap();
            let views = [
                (AGENT_DETAIL_BASIC_TAB, super::view(&app, draft, vec![])),
                (AGENT_DETAIL_IDENTITY_TAB, identity::view(&app, draft)),
                (AGENT_DETAIL_TOOLS_TAB, tools::view(&app, draft)),
                (AGENT_DETAIL_SKILLS_TAB, skills::view(&app, draft)),
                (AGENT_DETAIL_MCP_TAB, mcp::view(&app, draft)),
            ];
            for (tab, view) in views {
                let mut scene = Scene::new(view, &mut renderer, width, 1850);
                let geometry = scene.geometry(&renderer);
                assert!(
                    geometry
                        .labels
                        .iter()
                        .all(|(_, b)| b.x >= 0.0 && b.x + b.width <= width as f32)
                );
                scene.screenshot(
                    &mut renderer,
                    &theme,
                    mouse::Cursor::Unavailable,
                    &format!("content-{tab}-{width}-{theme_name}"),
                );
            }
        }
    }
    app.agents_settings.profile_saving = true;
    let mut scene = Scene::new(profile(&app), &mut renderer, 420, 620);
    for label in ["保存中…", "取消", "删除智能体"] {
        assert!(scene.click_label(&mut renderer, tr(app.desktop_locale, label)).is_empty());
    }
    scene.screenshot(&mut renderer, &Theme::Dark, mouse::Cursor::Unavailable, "modal-saving");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_16_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_16_preserves_values_and_disabled_states") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    {
        let draft = app.agents_settings.profile_modal.as_ref().unwrap();
        let mut scene = Scene::new(defaults::view(&app, draft), &mut renderer, 420, 550);
        assert!(scene.click_label(&mut renderer, "强制使用所选模型").is_empty());
        assert!(matches!(scene.click_label(&mut renderer, "完全访问权限").as_slice(),
            [Message::Settings(S::Agents(A::ProfileDefaultsChanged(value)))]
            if value.full_access && value.permission == draft.defaults.permission));
        let trigger = scene.geometry(&renderer).focusable[0];
        assert_eq!(trigger.height, 38.0);
        assert!(scene.click(&mut renderer, trigger.center()).is_empty());
        let (menu, content, _) = scene.geometry(&renderer).scrolls[0];
        assert!(content.height > menu.height && menu.y + menu.height <= 550.0);
        let first = Point::new(menu.x + 12.0, menu.y + 18.0);
        let second = Point::new(first.x, first.y + 38.0);
        assert!(
            scene
                .events(
                    &mut renderer,
                    &[Event::Mouse(mouse::Event::CursorLeft)],
                    mouse::Cursor::Unavailable,
                )
                .is_empty()
        );
        let outside = scene.screenshot(
            &mut renderer,
            &Theme::Light,
            mouse::Cursor::Unavailable,
            "model-menu",
        );
        for (point, name) in [(first, "model-hover-inherit"), (second, "model-hover-fixed")] {
            let cursor = mouse::Cursor::Available(point);
            assert!(
                scene
                    .events(
                        &mut renderer,
                        &[Event::Mouse(mouse::Event::CursorMoved { position: point })],
                        cursor
                    )
                    .is_empty()
            );
            let hover = scene.screenshot(&mut renderer, &Theme::Light, cursor, name);
            assert!(hover != outside);
        }
        assert!(
            scene
                .events(
                    &mut renderer,
                    &[Event::Mouse(mouse::Event::CursorLeft)],
                    mouse::Cursor::Unavailable
                )
                .is_empty()
        );
        assert!(
            scene.screenshot(
                &mut renderer,
                &Theme::Light,
                mouse::Cursor::Unavailable,
                "model-hover-cleared"
            ) == outside
        );
        assert!(scene.key(&mut renderer, Named::Escape).is_empty());
        assert!(scene.geometry(&renderer).scrolls.is_empty());
        assert!(scene.key(&mut renderer, Named::Enter).is_empty()); // focus survives Esc
        assert!(scene.key(&mut renderer, Named::ArrowDown).is_empty());
        assert!(matches!(scene.key(&mut renderer, Named::Enter).as_slice(),
            [Message::Settings(S::Agents(A::ProfileModelChanged(AgentDefault::Fixed(model))))]
            if model == &AgentModelReference { provider: PROVIDER.into(), model: MODEL.into() }));
    }
    {
        let draft = app.agents_settings.profile_modal.as_ref().unwrap();
        let mut scene = Scene::new(mcp::view(&app, draft), &mut renderer, 420, 550);
        assert!(scene.click_label(&mut renderer, "服务-one（已启用）").is_empty());
        let mode = scene.geometry(&renderer).focusable[0];
        assert!(scene.click(&mut renderer, mode.center()).is_empty());
        assert!(scene.key(&mut renderer, Named::End).is_empty());
        assert!(matches!(scene.key(&mut renderer, Named::Enter).as_slice(),
            [Message::Settings(S::Agents(A::ProfileDefaultsChanged(value)))]
            if value.mcp == AgentDefault::Fixed(vec![])));
    }
    {
        let draft = app.agents_settings.profile_modal.as_mut().unwrap();
        draft.defaults.mcp = AgentDefault::Fixed(vec!["服务-one".into()]);
        draft.defaults.skills = AgentDefault::Fixed(vec!["skill-one".into()]);
        let draft = app.agents_settings.profile_modal.as_ref().unwrap();
        let mut scene = Scene::new(mcp::view(&app, draft), &mut renderer, 420, 550);
        assert!(scene.click_label(&mut renderer, "服务-disabled（不可用）").is_empty());
        assert!(matches!(scene.click_label(&mut renderer, "服务-two（已启用）").as_slice(),
            [Message::Settings(S::Agents(A::ProfileDefaultsChanged(value)))]
            if value.mcp == AgentDefault::Fixed(vec!["服务-one".into(), "服务-two".into()])));
        let mut scene = Scene::new(skills::view(&app, draft), &mut renderer, 420, 1000);
        assert!(scene.click_label(&mut renderer, "skill-denied").is_empty());
        assert!(matches!(scene.click_label(&mut renderer, "skill-two").as_slice(),
            [Message::Settings(S::Agents(A::AllowedSkillToggled(id, skill, true)))]
            if id == AGENT && skill == "skill-two"));
        assert!(matches!(scene.click_label(&mut renderer, "skill-one").as_slice(),
            [Message::Settings(S::Agents(A::AllowedSkillToggled(id, skill, false)))]
            if id == AGENT && skill == "skill-one"));
        let directory = scene.geometry(&renderer).focusable[1];
        assert!(scene.click(&mut renderer, directory.center()).is_empty());
        assert!(scene.key(&mut renderer, Named::Home).is_empty());
        assert!(matches!(
            scene.key(&mut renderer, Named::Enter).as_slice(),
            [Message::Settings(S::Agents(A::SkillDirectorySelected(None)))]
        ));
        assert!(scene.click(&mut renderer, directory.center()).is_empty());
        assert!(scene.key(&mut renderer, Named::End).is_empty());
        assert!(matches!(
            scene.key(&mut renderer, Named::Enter).as_slice(),
            [Message::Settings(S::Agents(A::SkillDirectorySelected(Some(path))))]
            if Some(path) == app.agents_settings.skill_directory.as_ref()
        ));
    }
    {
        let draft = app.agents_settings.profile_modal.as_ref().unwrap();
        let mut scene = Scene::new(tools::view(&app, draft), &mut renderer, 640, 900);
        for tool in ["screenshot", "http_request"] {
            let allowed = draft.allowed_tools.iter().any(|id| id == tool);
            assert!(matches!(scene.click_label(&mut renderer, tool).as_slice(),
                [Message::Settings(S::Agents(A::AllowedToolToggled(id, value, next)))]
                if id == AGENT && value == tool && *next != allowed));
        }
    }
    {
        let draft = app.agents_settings.profile_modal.as_ref().unwrap();
        let mut scene = Scene::new(super::view(&app, draft, vec![]), &mut renderer, 640, 1850);
        let geometry = scene.geometry(&renderer);
        let label = geometry.label("API 密钥");
        let (masked, api_key) = geometry
            .inputs
            .iter()
            .find(|(_, input)| (label.center().y - input.center().y).abs() < 0.5)
            .expect("API key input aligned with its label");
        assert_ne!(masked, &draft.api_key_input);
        assert_eq!(masked.chars().count(), draft.api_key_input.len());
        let right_edge = api_key.x + api_key.width;
        for label in [
            "启用智能体模式",
            "启用上下文压缩",
            "启用并行工具执行",
            "启用该智能体",
            "允许作为子智能体",
        ] {
            let control = geometry.label(label);
            assert!((control.x + control.width - right_edge).abs() < 0.5, "{label}: {control:?}");
        }
        assert!(matches!(
            scene.click_label(&mut renderer, "启用智能体模式").as_slice(),
            [Message::Settings(S::Agents(A::AgenticToggled(id, next)))]
                if id == AGENT && *next != draft.agentic
        ));
        assert!(matches!(
            scene.click_label(&mut renderer, "启用上下文压缩").as_slice(),
            [Message::Settings(S::Agents(A::CompactContextToggled(id, next)))]
                if id == AGENT && *next != draft.compact_context
        ));
        assert!(scene.click(&mut renderer, api_key.center()).is_empty());
        assert!(scene.key(&mut renderer, Named::End).is_empty());
        assert!(matches!(scene.events(&mut renderer, &[Event::InputMethod(
            iced::advanced::input_method::Event::Commit("追加".into())
        )], mouse::Cursor::Available(api_key.center())).as_slice(),
            [Message::Settings(S::Agents(A::ApiKeyChanged(id, value)))]
            if id == AGENT && value == "fixture-secret追加"
        ));
    }
    app.agents_settings.active_prompt_tab = "AGENTS.md".into();
    {
        let draft = app.agents_settings.profile_modal.as_ref().unwrap();
        let mut scene = Scene::new(identity::view(&app, draft), &mut renderer, 420, 760);
        let original = draft.enabled_identity_files.iter().any(|file| file == "AGENTS.md");
        assert!(matches!(scene.click_label(&mut renderer, "启用后加载到提示词").as_slice(),
            [Message::Settings(S::Agents(A::WorkspaceIdentityEnabledToggled {
                agent_key, file_name, enabled
            }))] if agent_key == AGENT && file_name == "AGENTS.md" && *enabled != original
        ));
    }
    app.agents_settings.active_tools_list_tab = AGENT_TOOLS_BUILTIN_TAB.into();
    {
        let draft = app.agents_settings.profile_modal.as_ref().unwrap();
        let mut scene = Scene::new(tools::view(&app, draft), &mut renderer, 640, 550);
        assert!(scene.click_label(&mut renderer, "bash").is_empty());
    }
    app.agents_settings.active_detail_tab = AGENT_DETAIL_MCP_TAB.into();
    app.mcp_settings.inspect_loading = true;
    let mut scene = Scene::new(profile(&app), &mut renderer, 420, 620);
    scene.geometry(&renderer).label("正在加载服务目录；保留已有选择。");
    scene.screenshot(&mut renderer, &Theme::Dark, mouse::Cursor::Unavailable, "mcp-loading");
    drop(scene);
    app.mcp_settings.inspect_loading = false;
    app.mcp_settings.inspect_error = Some("fixture failure".into());
    let mut scene = Scene::new(profile(&app), &mut renderer, 420, 620);
    scene.geometry(&renderer).label("服务目录加载失败：fixture failure。已有选择保持不变。");
    scene.screenshot(&mut renderer, &Theme::Light, mouse::Cursor::Unavailable, "mcp-failure");
    drop(scene);
    app.agents_settings.active_detail_tab = AGENT_DETAIL_SKILLS_TAB.into();
    app.agents_settings.skill_catalog.clear();
    app.agents_settings.skill_catalog_loading = false;
    let draft = app.agents_settings.profile_modal.as_ref().unwrap();
    let mut scene = Scene::new(skills::view(&app, draft), &mut renderer, 420, 800);
    scene.geometry(&renderer).label("当前筛选范围暂无可用技能");
    assert!(matches!(
        scene.click_label(&mut renderer, "刷新技能").as_slice(),
        [Message::Settings(S::Agents(A::SkillCatalogRefresh))]
    ));
    scene.screenshot(&mut renderer, &Theme::Dark, mouse::Cursor::Unavailable, "skills-empty");
}

fn assert_contextual_help(
    scene: &mut Scene<'_>,
    renderer: &mut Renderer,
    theme: &Theme,
    locale: DesktopLocale,
    title: &'static str,
    description: &'static str,
) -> Rectangle {
    let geometry = scene.geometry(renderer);
    let label = geometry
        .labels
        .iter()
        .rev()
        .find(|(value, _)| value == tr(locale, title))
        .unwrap_or_else(|| panic!("missing help label {title}"))
        .1;
    let frame = |scene: &mut Scene<'_>, renderer: &mut Renderer, suffix: &str| {
        scene.screenshot(
            renderer,
            theme,
            mouse::Cursor::Unavailable,
            &format!("help-08-{locale:?}-{theme:?}-{}-{title}-{suffix}", scene.size.height),
        )
    };
    let idle = frame(scene, renderer, "idle");
    assert!(
        !geometry.labels.iter().any(|(text, _)| text == tr(locale, description)),
        "{title}: explanation is still visible while idle"
    );
    let icon = Rectangle::new(
        Point::new(label.x + label.width + 4.0, label.center().y - 11.0),
        Size::new(22.0, 22.0),
    );
    let viewport =
        Rectangle::with_size(Size::new(scene.size.width as f32, scene.size.height as f32));
    assert!(viewport.contains(icon.center()), "{title}: help icon is outside the viewport");
    assert!(scene.click(renderer, icon.center()).is_empty(), "{title}");
    assert!(idle != frame(scene, renderer, "open"), "{title}: click must pin a help card");
    icon
}

fn assert_help_dismissed(scene: &mut Scene<'_>, renderer: &mut Renderer, theme: &Theme) {
    let open =
        scene.screenshot(renderer, theme, mouse::Cursor::Unavailable, "help-08-before-escape");
    assert!(scene.key(renderer, Named::Escape).is_empty());
    let closed =
        scene.screenshot(renderer, theme, mouse::Cursor::Unavailable, "help-08-after-escape");
    assert_ne!(open, closed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_08_renders_contextual_help() {
    if !isolated("help_rollout_08_renders_contextual_help") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    let overview = "全局管理内建与自定义智能体的模型、身份文件、工具、技能和运行参数。";
    for (locale, theme, width) in [
        (DesktopLocale::English, Theme::Light, 420),
        (DesktopLocale::SimplifiedChinese, Theme::Dark, 640),
        (DesktopLocale::TraditionalChinese, Theme::Light, 520),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        let draft = app.agents_settings.profile_modal.take();
        let mut page = Scene::new(super::super::view(&app), &mut renderer, width, 840);
        assert_contextual_help(&mut page, &mut renderer, &theme, locale, "智能体配置", overview);
        assert_help_dismissed(&mut page, &mut renderer, &theme);

        drop(page);
        app.agents_settings.profile_modal = draft;
        let mut modal = Scene::new(profile(&app), &mut renderer, width, 820);
        modal
            .geometry(&renderer)
            .label(tr(locale, "身份文件正文即时保存；其余配置点击保存后生效。"));
        assert_contextual_help(&mut modal, &mut renderer, &theme, locale, "智能体配置", overview);
        assert_help_dismissed(&mut modal, &mut renderer, &theme);
        assert!(matches!(
            modal.click_label(&mut renderer, tr(locale, "保存配置")).as_slice(),
            [Message::Settings(S::Agents(A::ProfileSave))]
        ));
        assert!(matches!(
            modal.click_label(&mut renderer, tr(locale, "取消")).as_slice(),
            [Message::Settings(S::Agents(A::ProfileCancel))]
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_08_basic_help_uses_static_text() {
    if !isolated("help_rollout_08_basic_help_uses_static_text") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    for (locale, theme, width) in [
        (DesktopLocale::English, Theme::Light, 420),
        (DesktopLocale::SimplifiedChinese, Theme::Dark, 640),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        let draft = app.agents_settings.profile_modal.as_ref().unwrap();
        let mut scene = Scene::new(super::view(&app, draft, vec![]), &mut renderer, width, 2200);
        for label in [
            "首次保存成功后，运行引擎及 ACP backend 不可修改。",
            "完全访问权限",
            "权限始终受安全上限约束。",
            "关闭后不参与注册。",
            "范围 0.0–2.0",
        ] {
            scene.geometry(&renderer).label(tr(locale, label));
        }
        let description = "可选覆盖该代理的提供商凭证。";
        let icon = assert_contextual_help(
            &mut scene,
            &mut renderer,
            &theme,
            locale,
            "API 密钥",
            description,
        );
        let card_text = Point::new(icon.x + icon.width + 22.0, icon.y + 16.0);
        let description = tr(locale, description);
        let url = format!("https://www.google.com/search?q={}", urlencoding::encode(description));
        let item_height =
            f32::from(iced::advanced::text::LineHeight::default().to_absolute(12.0.into())) + 10.0;
        for index in [0.0, 3.0] {
            assert!(
                scene
                    .events(
                        &mut renderer,
                        &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))],
                        mouse::Cursor::Available(card_text),
                    )
                    .is_empty()
            );
            let item =
                Point::new(card_text.x + 24.0, card_text.y + 3.0 + item_height * (index + 0.5));
            let messages = scene.click(&mut renderer, item);
            assert!(
                match messages.as_slice() {
                    [Message::CopyCode(text)] if index == 0.0 => text == description,
                    [Message::View(crate::app::message::ViewMessage::OpenUrlExternal(actual))]
                        if index == 3.0 =>
                        actual == &url,
                    _ => false,
                },
                "{messages:?}"
            );
        }
        assert_help_dismissed(&mut scene, &mut renderer, &theme);
        for (title, description) in [
            ("可用性", "控制该智能体是否作为可用 Agent 暴露。"),
            ("子智能体数量", "该 Agent 每轮最多同时启动的子智能体数量。"),
            ("最大深度", "委托嵌套深度上限，防止无限递归。"),
            ("上下文压缩", "适合小模型，压缩该智能体的上下文体积。"),
            ("历史消息上限", "该智能体保留给模型的会话历史消息上限。"),
            ("最大迭代次数", "智能体模式下最多运行多少轮工具循环。"),
        ] {
            assert_contextual_help(&mut scene, &mut renderer, &theme, locale, title, description);
            assert_help_dismissed(&mut scene, &mut renderer, &theme);
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_08_identity_and_tools_keep_controls_visible() {
    if !isolated("help_rollout_08_identity_and_tools_keep_controls_visible") {
        return;
    }
    let mut app = fixture();
    app.agents_settings.active_detail_tab = AGENT_DETAIL_IDENTITY_TAB.into();
    app.agents_settings.available_tools.push("git_operations".into());
    let mut renderer = renderer().await;
    for (locale, theme, width) in [
        (DesktopLocale::English, Theme::Dark, 420),
        (DesktopLocale::SimplifiedChinese, Theme::Light, 640),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        let draft = app.agents_settings.profile_modal.as_ref().unwrap();
        let mut identity = Scene::new(profile(&app), &mut renderer, width, 840);
        for (title, description) in [
            ("身份与提示词", "管理当前 Agent 的系统提示词，以及独立工作区身份文件。"),
            ("系统提示词", "多行系统提示，将作为该 Agent 的专用角色提示。"),
        ] {
            assert_contextual_help(
                &mut identity,
                &mut renderer,
                &theme,
                locale,
                title,
                description,
            );
            assert_help_dismissed(&mut identity, &mut renderer, &theme);
        }
        let mut tools = Scene::new(tools::view(&app, draft), &mut renderer, width, 1000);
        tools.geometry(&renderer).label(tr(locale, "未选择的工具不可调用。"));
        assert_contextual_help(
            &mut tools,
            &mut renderer,
            &theme,
            locale,
            "快捷预选",
            "按当前可用工具自动套用常见权限组合。",
        );
        assert_help_dismissed(&mut tools, &mut renderer, &theme);
        for (title, description) in
            [("执行", "运行命令、补丁和版本控制操作。"), ("联网", "浏览页面、联网请求与外部检索。")]
        {
            assert_contextual_help(&mut tools, &mut renderer, &theme, locale, title, description);
            assert_help_dismissed(&mut tools, &mut renderer, &theme);
        }
        assert!(matches!(
            tools.click_label(&mut renderer, tr(locale, "全选")).as_slice(),
            [Message::Settings(S::Agents(A::AllowedToolsSelectAll(key)))] if key == AGENT
        ));
    }
}

fn scroll_to(scene: &mut Scene<'_>, renderer: &mut Renderer, bounds: Rectangle) -> Rectangle {
    let (viewport, _, offset) = scene.geometry(renderer).scrolls[0];
    let y = bounds.center().y - offset.y;
    let target = viewport.y + 90.0;
    assert!(
        scene
            .events(
                renderer,
                &[Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels { x: 0.0, y: target - y },
                })],
                mouse::Cursor::Available(viewport.center()),
            )
            .is_empty()
    );
    let (_, _, offset) = scene.geometry(renderer).scrolls[0];
    let bounds = bounds - offset;
    assert!(viewport.contains(bounds.center()), "{bounds:?} outside {viewport:?}");
    bounds
}

fn assert_modal_help(
    scene: &mut Scene<'_>,
    renderer: &mut Renderer,
    theme: &Theme,
    locale: DesktopLocale,
    title: &'static str,
    description: &'static str,
) {
    let geometry = scene.geometry(renderer);
    let description = tr(locale, description);
    assert!(!geometry.labels.iter().any(|(value, _)| value == description), "{title}");
    let label = scroll_to(scene, renderer, geometry.label(tr(locale, title)));
    let icon = Point::new(label.x + label.width + 15.0, label.center().y);
    let cursor = mouse::Cursor::Available(icon);
    let away = mouse::Cursor::Unavailable;
    let idle = scene.screenshot(renderer, theme, away, "agent-help-idle");
    assert!(
        scene
            .events(renderer, &[Event::Mouse(mouse::Event::CursorMoved { position: icon })], cursor)
            .is_empty()
    );
    assert!(idle != scene.screenshot(renderer, theme, cursor, "agent-help-hover"), "{title}");
    assert!(scene.click(renderer, icon).is_empty());
    assert!(scene.events(renderer, &[Event::Mouse(mouse::Event::CursorLeft)], away).is_empty());
    let pinned = scene.screenshot(renderer, theme, away, "agent-help-pinned");
    assert!(idle != pinned, "{title}");

    let left = icon.x - 15.0;
    let right = icon.x + 15.0;
    let right_space = scene.size.width as f32 - right;
    let width = 300.0_f32.min(left.max(right_space));
    let x = if right_space >= width { right } else { left - width };
    let card = Point::new(x + 22.0, icon.y + 5.0);
    assert!(
        scene
            .events(
                renderer,
                &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))],
                mouse::Cursor::Available(card),
            )
            .is_empty()
    );
    let menu = scene.screenshot(renderer, theme, away, "agent-help-menu");
    assert!(pinned != menu, "{title}");
    assert!(scene.key(renderer, Named::Escape).is_empty());
    assert!(
        pinned == scene.screenshot(renderer, theme, away, "agent-help-menu-dismissed"),
        "{title}"
    );
    assert!(
        scene
            .events(
                renderer,
                &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))],
                mouse::Cursor::Available(card),
            )
            .is_empty()
    );
    let item_height =
        f32::from(iced::advanced::text::LineHeight::default().to_absolute(12.0.into())) + 10.0;
    let menu_height = 5.0 * item_height + 6.0;
    let menu_y = if card.y + menu_height <= scene.size.height as f32 {
        card.y
    } else {
        (card.y - menu_height).max(0.0)
    };
    let menu_x = card.x.min(scene.size.width as f32 - 140.0);
    let messages =
        scene.click(renderer, Point::new(menu_x + 25.0, menu_y + 3.0 + item_height / 2.0));
    assert!(
        matches!(messages.as_slice(), [Message::CopyCode(value)] if value == description),
        "{locale:?} {title}: missing localized help copy, got {messages:?}"
    );
    assert!(scene.key(renderer, Named::Escape).is_empty());
    assert!(idle == scene.screenshot(renderer, theme, away, "agent-help-dismissed"), "{title}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_05_native_modal_moves_explanations_into_help() {
    if !isolated("settings_help_gap_05_native_modal_moves_explanations_into_help") {
        return;
    }
    let mut app = fixture();
    app.agents_settings.active_detail_tab = AGENT_DETAIL_BASIC_TAB.into();
    let mut renderer = renderer().await;
    for (locale, theme, width) in [
        (DesktopLocale::English, Theme::Light, 420),
        (DesktopLocale::TraditionalChinese, Theme::Dark, 420),
        (DesktopLocale::SimplifiedChinese, Theme::Light, 960),
        (DesktopLocale::English, Theme::Dark, 960),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        let mut scene = Scene::new(profile(&app), &mut renderer, width, 820);
        let geometry = scene.geometry(&renderer);
        for description in [
            "控制生成发散度，范围 0.0 - 2.0。",
            "该智能体是否允许同一轮并行执行多个工具。",
            "启用后允许该代理进入多轮工具调用循环。",
            "控制该 Agent 是否可被平台委托；原生引擎还可配置发起委托的并发上限。",
            "固定空列表表示不选择任何条目。权限始终受安全上限约束。",
        ] {
            assert!(
                !geometry.labels.iter().any(|(value, _)| value == tr(locale, description)),
                "{description}: explanation is still visible while idle"
            );
        }
        for (title, description) in [
            ("默认配置", "固定空列表表示不选择任何条目。"),
            ("启用", "关闭后保留配置。"),
            ("子智能体委托", "控制该 Agent 是否可被平台委托；原生引擎还可配置发起委托的并发上限。"),
            ("作为子智能体", "只有开启后，该 Agent 才会出现在 AgentTool 的可选列表中。"),
            ("温度", "控制生成发散度。"),
            ("智能体模式", "启用后允许该代理进入多轮工具调用循环。"),
            ("并行工具", "该智能体是否允许同一轮并行执行多个工具。"),
        ] {
            assert_modal_help(&mut scene, &mut renderer, &theme, locale, title, description);
        }
        for text in [
            "首次保存成功后，运行引擎及 ACP backend 不可修改。",
            "身份文件正文即时保存；其余配置点击保存后生效。",
            "关闭后不参与注册。",
            "关闭后不可被委托。",
            "权限始终受安全上限约束。",
            "范围 0.0–2.0",
        ] {
            geometry.label(tr(locale, text));
        }

        let value = scroll_to(&mut scene, &mut renderer, geometry.label("0.70"));
        let slider = Point::new(value.x + 3.0, value.y - 16.0);
        assert!(matches!(scene.click(&mut renderer, slider).as_slice(),
            [Message::Settings(S::Agents(A::TemperatureChanged(id, value)))]
            if id == AGENT && (0.0..0.7).contains(value)));

        let delegation =
            scroll_to(&mut scene, &mut renderer, geometry.label(tr(locale, "允许作为子智能体")));
        let draft = app.agents_settings.profile_modal.as_ref().unwrap();
        assert!(matches!(scene.click(&mut renderer, delegation.center()).as_slice(),
            [Message::Settings(S::Agents(A::AllowAsSubagentToggled(id, enabled)))]
            if id == AGENT && *enabled != draft.allow_as_subagent));
        assert!(matches!(
            scene.click_label(&mut renderer, tr(locale, "保存配置")).as_slice(),
            [Message::Settings(S::Agents(A::ProfileSave))]
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_05_acp_model_capability_controls_choices() {
    use vw_config_types::agent::AgentRuntime;
    use vw_gateway_client::vw_api_types::acp_model_source::{
        AcpModelSource, AcpModelSourceCapability,
    };

    if !isolated("settings_help_gap_05_acp_model_capability_controls_choices") {
        return;
    }
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.app_theme = Theme::Dark;
    app.agents_settings.active_detail_tab = AGENT_DETAIL_BASIC_TAB.into();
    let mut draft = DelegateAgentSettingsEntry::from_config("acp-fixture", None);
    draft.runtime = AgentRuntime::Acp { backend: "fixture".into() };
    draft.defaults.model =
        AgentDefault::Fixed(AgentModelReference { provider: PROVIDER.into(), model: MODEL.into() });
    app.agents_settings.profile_modal = Some(draft);
    let mut renderer = renderer().await;
    for (supported, width) in [(false, 420), (true, 960)] {
        if supported {
            let session = app.get_session_runtime_mut("fixture-session");
            session.acp_agent = Some("fixture".into());
            session.acp_options.model_source_capabilities =
                vec![AcpModelSourceCapability::Supported { source: AcpModelSource::SessionModel }];
        }
        let mut scene = Scene::new(profile(&app), &mut renderer, width, 820);
        let geometry = scene.geometry(&renderer);
        for absent in [
            "首次保存成功后，运行引擎及 ACP backend 不可修改。",
            "工具",
            "温度",
            "并行工具",
            "智能体模式",
            "子智能体数量",
            "最大深度",
            "会话模型来源下，显式选择优先于智能体默认；继承会话模型在无选择时使用环境默认。CLI 本地配置是独立来源，由会话单独选择。仅已声明会话模型能力的引擎可固定模型。",
        ] {
            assert!(!geometry.labels.iter().any(|(value, _)| value == absent), "{absent}");
        }
        geometry.label("固定运行引擎：fixture");
        geometry.label("允许作为子智能体");
        for hint in [
            "仅已声明会话模型能力的引擎可固定模型。",
            "模型不可用，或当前引擎尚未提供此模型来源能力，请重新选择或继承。",
        ] {
            assert_eq!(
                geometry.labels.iter().any(|(value, _)| value == hint),
                !supported,
                "{hint}"
            );
        }
        assert_modal_help(
            &mut scene,
            &mut renderer,
            &app.app_theme,
            app.desktop_locale,
            "模型",
            "会话模型来源下，显式选择优先于智能体默认；继承会话模型在无选择时使用环境默认。CLI 本地配置是独立来源，由会话单独选择。",
        );
        let force = geometry.label("强制使用所选模型");
        let picker = *geometry
            .focusable
            .iter()
            .rfind(|bounds| bounds.y < force.y && force.y - (bounds.y + bounds.height) <= 10.0)
            .expect("model picker above the force model switch");
        let picker = scroll_to(&mut scene, &mut renderer, picker);
        assert!(scene.click(&mut renderer, picker.center()).is_empty());
        assert!(scene.key(&mut renderer, Named::End).is_empty());
        let messages = scene.key(&mut renderer, Named::Enter);
        if supported {
            assert!(matches!(messages.as_slice(),
                [Message::Settings(S::Agents(A::ProfileModelChanged(AgentDefault::Fixed(model))))]
                if model.provider == PROVIDER && model.model == "model-19"));
        } else {
            assert!(matches!(
                messages.as_slice(),
                [Message::Settings(S::Agents(A::ProfileModelChanged(AgentDefault::Inherit)))]
            ));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_05_fixed_role_and_tool_controls_remain_visible() {
    if !isolated("settings_help_gap_05_fixed_role_and_tool_controls_remain_visible") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    app.agents_settings.active_detail_tab = AGENT_DETAIL_BASIC_TAB.into();
    app.agents_settings.profile_modal = Some(DelegateAgentSettingsEntry::from_config("main", None));
    {
        let mut scene = Scene::new(profile(&app), &mut renderer, 420, 820);
        let geometry = scene.geometry(&renderer);
        for label in [
            "固定运行引擎：native",
            "角色边界",
            "委托资格、并发上限和递归深度由内置角色目录固定，不能在此放宽。",
        ] {
            geometry.label(label);
        }
        for label in
            ["删除智能体", "允许作为子智能体", "首次保存成功后，运行引擎及 ACP backend 不可修改。"]
        {
            assert!(!geometry.labels.iter().any(|(value, _)| value == label), "{label}");
        }
        scene.screenshot(
            &mut renderer,
            &Theme::Light,
            mouse::Cursor::Unavailable,
            "agent-fixed-role",
        );
    }

    let mut app = fixture();
    app.agents_settings.active_detail_tab = AGENT_DETAIL_TOOLS_TAB.into();
    let mut scene = Scene::new(profile(&app), &mut renderer, 420, 820);
    let geometry = scene.geometry(&renderer);
    geometry.label("未选择的工具不可调用。");
    assert_modal_help(
        &mut scene,
        &mut renderer,
        &app.app_theme,
        app.desktop_locale,
        "注册工具",
        "选择在对话开始时注册给模型的工具。",
    );
    let toggle = scroll_to(&mut scene, &mut renderer, geometry.label("screenshot"));
    assert!(matches!(scene.click(&mut renderer, toggle.center()).as_slice(),
        [Message::Settings(S::Agents(A::AllowedToolToggled(id, tool, true)))]
        if id == AGENT && tool == "screenshot"));
}
