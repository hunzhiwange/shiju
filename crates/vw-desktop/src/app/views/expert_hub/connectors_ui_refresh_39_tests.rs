//! Render the expert hub forms and drive their existing messages.

use super::super::{install_guide, installed_skills, public_skill_detail, shiju_apps};
use super::*;
use crate::app::i18n::I18n;
use crate::app::message::{ShijuAppsMessage, SkillHubMessage, skill_hub::PublicSkillMessage};
use crate::app::shiju_apps::ShijuAppLaunchState;
use crate::app::skill_hub::SkillInstallScope;
use iced::advanced::widget::{Id, Operation, operation};
use iced::advanced::{clipboard, renderer::Headless};
use iced::keyboard::{self, Key, key::Named};
use iced::{Event, Font, mouse};
use iced::{Point, Rectangle, Renderer, Size, Theme, Vector};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};
use vw_gateway_client::{ShijuConnector, ShijuIamApiKeyDto};

type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_39_CHILD";
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

fn send(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point, events: &[Event]) -> Vec<Message> {
    let mut messages = Vec::new();
    ui.update(
        events,
        mouse::Cursor::Available(point),
        renderer,
        &mut clipboard::Null,
        &mut messages,
    );
    messages
}

fn click(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point) -> Vec<Message> {
    send(
        ui,
        renderer,
        point,
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
        Point::ORIGIN,
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
    point: Point,
) -> Vec<u8> {
    assert!(
        send(
            ui,
            renderer,
            point,
            &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now(),))]
        )
        .is_empty()
    );
    ui.draw(
        renderer,
        theme,
        &iced::advanced::renderer::Style { text_color: theme.palette().text },
        mouse::Cursor::Available(point),
    );
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]), "{name} drew no content");
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_39_SCREENSHOTS") {
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

#[derive(Default, Debug)]
struct Bounds {
    labels: Vec<(String, Rectangle)>,
    inputs: Vec<(String, Rectangle)>,
    scrolls: Vec<(Rectangle, Vector)>,
    selects: Vec<Rectangle>,
    menu: Option<Rectangle>,
}

impl Operation for Bounds {
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
        input: &mut dyn operation::TextInput,
    ) {
        self.inputs.push((input.text().into(), bounds));
    }

    fn focusable(&mut self, _: Option<&Id>, bounds: Rectangle, _: &mut dyn operation::Focusable) {
        self.selects.push(bounds);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        _: Rectangle,
        offset: Vector,
        _: &mut dyn operation::Scrollable,
    ) {
        if id == Some(&Id::from("settings-select-menu")) {
            self.menu = Some(bounds);
        } else {
            self.scrolls.push((bounds, offset));
        }
    }
}

fn bounds(ui: &mut Ui<'_>, renderer: &Renderer) -> Bounds {
    let mut result = Bounds::default();
    ui.operate(renderer, &mut result);
    result
}

fn position(controls: &[(String, Rectangle)], value: &str) -> Rectangle {
    controls
        .iter()
        .find(|(text, _)| text == value)
        .unwrap_or_else(|| panic!("missing {value:?} in {controls:?}"))
        .1
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_39_uninstall_keeps_confirmation_visible_while_paths_scroll() {
    if !isolated("ui_refresh_39_uninstall_keeps_confirmation_visible_while_paths_scroll") {
        return;
    }
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    let state = &mut app.expert_hub.install_guide;
    state.uninstall_confirmation_open = true;
    state.selected_targets = install_guide::install_target_presentations(state.install_scope)
        .into_iter()
        .map(|item| item.target)
        .collect();
    let i18n = I18n::new(app.desktop_locale);
    let mut renderer = renderer().await;
    let size = Size::new(420, 330);
    let mut ui = scene(
        install_guide::with_uninstall_confirmation(
            &app,
            Space::new().width(Length::Fill).height(Length::Fill).into(),
        ),
        &mut renderer,
        size,
    );
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "uninstall-420", Point::ORIGIN);
    let before = bounds(&mut ui, &renderer);
    let confirm = position(&before.labels, i18n.__("确认卸载"));
    let cancel = position(&before.labels, i18n.__("取消"));
    assert!(confirm.y + confirm.height < size.height as f32, "confirmation is clipped");
    assert_eq!(before.scrolls.len(), 1, "long path list needs its own viewport");
    let viewport = before.scrolls[0].0;
    assert!(confirm.y >= viewport.y + viewport.height);
    assert!(
        send(
            &mut ui,
            &mut renderer,
            viewport.center(),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -150.0 },
            })],
        )
        .is_empty()
    );
    let after = bounds(&mut ui, &renderer);
    assert_ne!(before.scrolls[0].1, after.scrolls[0].1);
    assert_eq!(confirm, position(&after.labels, i18n.__("确认卸载")));
    assert_eq!(cancel, position(&after.labels, i18n.__("取消")));
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::Escape), None).is_empty());
    assert!(matches!(
        click(&mut ui, &mut renderer, cancel.center()).as_slice(),
        [Message::SkillHub(SkillHubMessage::CancelShijuUninstall)]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, confirm.center()).as_slice(),
        [Message::SkillHub(SkillHubMessage::ConfirmShijuUninstall)]
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_39_layout_and_interaction() {
    if !isolated("ui_refresh_39_layout_and_interaction") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    let connector: ShijuConnector = serde_json::from_value(serde_json::json!({
        "platform_id": 17,
        "company_platform_id": 73,
        "platform_code": "sample",
        "platform_name": "文件處理與自動化工具",
        "authorization_available": false,
        "account_binding_variables": [
            {"name":"token", "description":"虛構的第三方授權密鑰", "type":"password", "required":true},
            {"name":"region", "description":"A long display name for the connector configuration variable", "type":"string"}
        ]
    })).unwrap();
    app.expert_hub.section = ExpertHubSection::Connectors;
    app.expert_hub.connectors.list = LoadState::Loaded(vec![connector.clone()]);
    let mut form = ConnectorForm::new(connector);
    form.account_id = "fixture-account".into();
    form.values.insert("token".into(), "sample-secret-A".into());

    let size = Size::new(640, 600);
    let i18n = I18n::new(app.desktop_locale);
    let mut page = scene(super::view(&app), &mut renderer, size);
    let edit = position(&bounds(&mut page, &renderer).labels, i18n.__("手动绑定"));
    assert!(matches!(
        click(&mut page, &mut renderer, edit.center()).as_slice(),
        [Message::SkillHub(SkillHubMessage::Connectors(ConnectorMessage::Edit(73)))]
    ));
    drop(page);
    app.expert_hub.connectors.form = Some(form);
    for (size, locale, theme, name) in [
        (Size::new(420, 480), DesktopLocale::English, Theme::Light, "connector-420-en"),
        (
            Size::new(960, 720),
            DesktopLocale::TraditionalChinese,
            Theme::Dark,
            "connector-960-zh-hant",
        ),
    ] {
        app.desktop_locale = locale;
        let i18n = I18n::new(locale);
        let mut ui = scene(with_binding_modal(&app, super::view(&app)), &mut renderer, size);
        let original = capture(&mut ui, &mut renderer, &theme, size, name, Point::ORIGIN);
        let geometry = bounds(&mut ui, &renderer);
        let account = position(&geometry.inputs, "fixture-account");
        let save = position(&geometry.labels, i18n.__("保存绑定"));
        let cancel = position(&geometry.labels, i18n.__("取消"));
        assert!(save.y >= geometry.scrolls[0].0.y + geometry.scrolls[0].0.height);
        assert!(save.x + save.width <= size.width as f32 - 12.0);
        assert!(click(&mut ui, &mut renderer, account.center()).is_empty());
        assert!(key(&mut ui, &mut renderer, Key::Named(Named::End), None).is_empty());
        assert!(matches!(
            key(&mut ui, &mut renderer, Key::Character("B".into()), Some("B")).as_slice(),
            [Message::SkillHub(SkillHubMessage::Connectors(ConnectorMessage::AccountId(value)))]
                if value == "fixture-accountB"
        ));
        assert!(key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).is_empty());
        assert!(key(&mut ui, &mut renderer, Key::Named(Named::Escape), None).is_empty());
        assert!(matches!(
            click(&mut ui, &mut renderer, cancel.center()).as_slice(),
            [Message::SkillHub(SkillHubMessage::Connectors(ConnectorMessage::Cancel))]
        ));
        assert!(matches!(
            click(&mut ui, &mut renderer, save.center()).as_slice(),
            [Message::SkillHub(SkillHubMessage::Connectors(ConnectorMessage::Save))]
        ));
        drop(ui);
        if size.width == 960 {
            let token = geometry.inputs[2].1;
            assert!(
                token.y + token.height < geometry.scrolls[0].0.y + geometry.scrolls[0].0.height
            );
            let form = app.expert_hub.connectors.form.as_mut().unwrap();
            form.values.insert("token".into(), "sample-secret-Z".into());
            let mut ui = scene(with_binding_modal(&app, super::view(&app)), &mut renderer, size);
            let masked =
                capture(&mut ui, &mut renderer, &theme, size, "connector-masked", Point::ORIGIN);
            assert!(original == masked, "equal-length secrets must render identically");
        }
    }

    app.expert_hub.connectors.begin(73, ConnectorWork::Saving);
    let i18n = I18n::new(app.desktop_locale);
    let size = Size::new(420, 480);
    let mut ui = scene(with_binding_modal(&app, super::view(&app)), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "connector-saving", Point::ORIGIN);
    let geometry = bounds(&mut ui, &renderer);
    for label in [i18n.__("保存中…"), i18n.__("取消")] {
        assert!(
            click(&mut ui, &mut renderer, position(&geometry.labels, label).center()).is_empty()
        );
    }
    let account = position(&geometry.inputs, "fixture-account");
    assert!(click(&mut ui, &mut renderer, account.center()).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Character("X".into()), Some("X")).is_empty());
}

fn api_key(api_key_id: u64, key_name: String, status: &str) -> ShijuIamApiKeyDto {
    ShijuIamApiKeyDto {
        api_key_id,
        key_name,
        api_key_mask: format!("sk-****{api_key_id:04}"),
        status: status.into(),
        expires_at: None,
        updated_at: None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_39_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_39_preserves_values_and_disabled_states") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    let size = Size::new(420, 510);
    let mut options = vec![
        api_key(77, "歷史密鑰".into(), "disabled"),
        api_key(29, "測試金鑰".repeat(20), "enabled"),
    ];
    options.extend((100..125).map(|id| api_key(id, format!("測試密鑰 {id}"), "enabled")));
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::SelectingApiKey {
        generation: 2,
        installation_id: "sample-install".into(),
        can_edit: true,
        api_keys: options,
        selected_api_key_id: Some(77),
        error: Some("虚构的绑定失败，可刷新重试。".into()),
    };
    let i18n = I18n::new(app.desktop_locale);
    let mut ui = scene(shiju_apps::view(&app), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Light, size, "api-failed-420", Point::ORIGIN);
    let geometry = bounds(&mut ui, &renderer);
    position(&geometry.labels, "虚构的绑定失败，可刷新重试。");
    position(&geometry.labels, "sk-****0077");
    let save = position(&geometry.labels, i18n.__("保存 API 密钥"));
    let refresh = position(&geometry.labels, i18n.__("刷新"));
    let scroll = geometry.scrolls[0].0;
    assert!(save.y > scroll.y + scroll.height && refresh.y > scroll.y + scroll.height);
    assert!(click(&mut ui, &mut renderer, save.center()).is_empty());
    assert!(matches!(
        click(&mut ui, &mut renderer, refresh.center()).as_slice(),
        [Message::ShijuApps(ShijuAppsMessage::RefreshApiKeys)]
    ));
    let select = geometry.selects[0];
    assert!(click(&mut ui, &mut renderer, select.center()).is_empty());
    let menu = bounds(&mut ui, &renderer).menu.unwrap();
    assert!(scroll.intersection(&menu).is_some_and(|intersection| intersection == menu));
    let mut hover_frames = Vec::new();
    for (name, point) in [
        ("api-menu-first", Point::new(menu.center().x, menu.y + 14.0)),
        ("api-menu-second", Point::new(menu.center().x, menu.y + 52.0)),
        ("api-menu-leave", Point::ORIGIN),
    ] {
        assert!(
            send(
                &mut ui,
                &mut renderer,
                point,
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })]
            )
            .is_empty()
        );
        hover_frames.push(capture(&mut ui, &mut renderer, &Theme::Light, size, name, point));
    }
    assert!(hover_frames[0] != hover_frames[1] && hover_frames[1] != hover_frames[2]);
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::Escape), None).is_empty());
    assert!(bounds(&mut ui, &renderer).menu.is_none());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::ArrowDown), None).is_empty());
    assert!(matches!(
        key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).as_slice(),
        [Message::ShijuApps(ShijuAppsMessage::SelectApiKey(29))]
    ));
    drop(ui);

    if let ShijuAppLaunchState::SelectingApiKey { selected_api_key_id, error, .. } =
        &mut app.expert_hub.shiju_apps.launch
    {
        *selected_api_key_id = Some(29);
        *error = None;
    }
    let mut ui = scene(shiju_apps::view(&app), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "api-selected-420", Point::ORIGIN);
    let geometry = bounds(&mut ui, &renderer);
    position(&geometry.labels, "sk-****0029");
    assert!(matches!(
        click(
            &mut ui,
            &mut renderer,
            position(&geometry.labels, i18n.__("保存 API 密钥")).center()
        )
        .as_slice(),
        [Message::ShijuApps(ShijuAppsMessage::SaveApiKeyBinding)]
    ));
    drop(ui);
    for (name, state) in [
        (
            "api-loading",
            ShijuAppLaunchState::LoadingApiKeys {
                generation: 2,
                installation_id: "sample-install".into(),
                can_edit: true,
            },
        ),
        (
            "api-empty",
            ShijuAppLaunchState::SelectingApiKey {
                generation: 2,
                installation_id: "sample-install".into(),
                can_edit: true,
                api_keys: Vec::new(),
                selected_api_key_id: None,
                error: None,
            },
        ),
    ] {
        app.expert_hub.shiju_apps.launch = state;
        let mut ui = scene(shiju_apps::view(&app), &mut renderer, size);
        capture(&mut ui, &mut renderer, &Theme::Dark, size, name, Point::ORIGIN);
        let geometry = bounds(&mut ui, &renderer);
        assert!(geometry.selects.is_empty());
        let save = position(&geometry.labels, i18n.__("保存 API 密钥"));
        assert!(click(&mut ui, &mut renderer, save.center()).is_empty());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_39_guide_and_skill_confirmations_keep_original_actions() {
    if !isolated("ui_refresh_39_guide_and_skill_confirmations_keep_original_actions") {
        return;
    }
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::TraditionalChinese;
    let i18n = I18n::new(app.desktop_locale);
    let mut renderer = renderer().await;
    let state = &mut app.expert_hub.install_guide;
    state.current_package = Some(crate::app::skill_hub::ShijuPackageId::DarwinArm64);
    state.is_installed = true;
    let install_label = install_guide::direct_install_button_label(i18n, state);
    for (width, theme) in [(640, Theme::Light), (960, Theme::Dark)] {
        let size = Size::new(width, 650);
        let mut ui = scene(install_guide::view(&app), &mut renderer, size);
        capture(&mut ui, &mut renderer, &theme, size, &format!("guide-{width}"), Point::ORIGIN);
        let before = bounds(&mut ui, &renderer);
        let viewport = before.scrolls[0].0;
        let install = position(&before.labels, &install_label);
        let uninstall = position(&before.labels, i18n.__("卸载"));
        assert!(install.y >= viewport.y + viewport.height);
        assert!(
            send(
                &mut ui,
                &mut renderer,
                viewport.center(),
                &[Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -420.0 },
                })]
            )
            .is_empty()
        );
        let after = bounds(&mut ui, &renderer);
        assert_ne!(before.scrolls[0].1, after.scrolls[0].1);
        assert_eq!(install, position(&after.labels, &install_label));
        assert!(matches!(
            click(&mut ui, &mut renderer, install.center()).as_slice(),
            [Message::SkillHub(SkillHubMessage::DirectInstallShiju)]
        ));
        assert!(matches!(
            click(&mut ui, &mut renderer, uninstall.center()).as_slice(),
            [Message::SkillHub(SkillHubMessage::UninstallShiju)]
        ));
    }

    let size = Size::new(420, 480);
    app.expert_hub.section = ExpertHubSection::Experts;
    let project = format!("/fixture/繁體中文/{}/repository", "project-path/".repeat(14));
    app.project_path = Some(project.clone());
    app.recent_projects.clear();
    app.expert_hub.public_market.open_suite_detail("fixture".into(), "sample".into());
    app.expert_hub.public_market.detail.as_mut().unwrap().install_confirmation_open = true;
    let mut ui = scene(public_skill_detail::view(&app), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "expert-install-unselected", Point::ORIGIN);
    let geometry = bounds(&mut ui, &renderer);
    let confirm = position(&geometry.labels, i18n.__("安装到项目"));
    assert!(click(&mut ui, &mut renderer, confirm.center()).is_empty());
    let select = geometry.selects[0];
    let label = position(&geometry.labels, i18n.__("目标项目"));
    assert!((select.center().y - label.center().y).abs() < 0.5);
    assert!(click(&mut ui, &mut renderer, select.center()).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::Home), None).is_empty());
    assert!(matches!(
        key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).as_slice(),
        [Message::SkillHub(SkillHubMessage::Public(PublicSkillMessage::SelectDetailInstallProject(path)))]
            if path == &project
    ));
    drop(ui);
    let detail = app.expert_hub.public_market.detail.as_mut().unwrap();
    assert_eq!(detail.install_scope, SkillInstallScope::Project);
    detail.selected_install_project_path = Some(project);
    let mut ui = scene(public_skill_detail::view(&app), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "expert-install-long-path", Point::ORIGIN);
    let geometry = bounds(&mut ui, &renderer);
    assert!(matches!(
        click(&mut ui, &mut renderer, position(&geometry.labels, i18n.__("取消")).center())
            .as_slice(),
        [Message::SkillHub(SkillHubMessage::Public(PublicSkillMessage::CancelExpertInstall))]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, position(&geometry.labels, i18n.__("安装到项目")).center())
            .as_slice(),
        [Message::SkillHub(SkillHubMessage::Public(PublicSkillMessage::InstallCurrent))]
    ));
    drop(ui);

    app.expert_hub.public_market.delete_confirmation_open = true;
    app.expert_hub.public_market.installed_selection.insert("sample".into());
    let mut ui = scene(
        installed_skills::with_delete_confirmation(
            &app,
            Space::new().width(Length::Fill).height(Length::Fill).into(),
        ),
        &mut renderer,
        size,
    );
    capture(&mut ui, &mut renderer, &Theme::Light, size, "batch-delete-420", Point::ORIGIN);
    let geometry = bounds(&mut ui, &renderer);
    for (label, confirm) in [("取消", false), ("确认卸载", true)] {
        let action = position(&geometry.labels, i18n.__(label));
        let messages = click(&mut ui, &mut renderer, action.center());
        assert!(match messages.as_slice() {
            [
                Message::SkillHub(SkillHubMessage::Public(PublicSkillMessage::ConfirmBatchDelete)),
            ] => confirm,
            [Message::SkillHub(SkillHubMessage::Public(PublicSkillMessage::CancelBatchDelete))] =>
                !confirm,
            _ => false,
        });
    }
}
