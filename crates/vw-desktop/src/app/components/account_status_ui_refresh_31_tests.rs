//! Exercise the account forms through the headless Iced runtime.
#![cfg(not(target_arch = "wasm32"))]

use super::{App, Element, Message, menu, view};
use crate::app::account::{
    AccountIdentity, AccountOrigin, AccountPasswordDraft, AccountProfile, AccountProfileDraft,
    AccountRepository, ManualSkeyInput, SecretSkey, VerifiedAccount,
};
use crate::app::account::{AccountPasswordField, LoginChallenge};
use crate::app::i18n::I18n;
use crate::app::message::{AccountMessage, SkillHubMessage};
use crate::app::state::{AccountOperation, AccountPasswordEditorState, AccountProfileEditorState};
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::keyboard::key::Named;
use iced::{Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_31_CHILD";
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_31_SCREENSHOTS") {
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

fn key_at(ui: &mut Ui<'_>, renderer: &mut Renderer, name: Named) -> Vec<Message> {
    send(ui, renderer, mouse::Cursor::Unavailable, &[key(name)])
}

fn assert_fits(bounds: Rectangle, size: Size<u32>) {
    assert!(bounds.width > 0.0 && bounds.height > 0.0, "{bounds:?}");
    assert!(bounds.x >= 0.0 && bounds.y >= 0.0, "{bounds:?}");
    assert!(bounds.x + bounds.width <= size.width as f32 + 0.1, "{bounds:?}");
    assert!(bounds.y + bounds.height <= size.height as f32 + 0.1, "{bounds:?}");
}

const SKEY: &str = "fixture-0000";
const NAME: &str = "示例使用者 Fixture";
const USERNAME: &str = "fixture-user";
const COMPANY: &str = "虚构公司・示例企業・Example company with a long name";
const CURRENT: &str = "MMMM1111";
const NEW: &str = "NewFixture123";
const ERROR: &str = "Fixture unavailable / 測試連線失敗";

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    App::new().0
}

fn repository() -> AccountRepository {
    let mut repository = AccountRepository::default();
    let active = repository.upsert_verified(
        VerifiedAccount::from_parts_with_profile(
            AccountIdentity::from_profile(COMPANY, "enabled").unwrap(),
            AccountProfile::new("enabled", NAME, USERNAME, "enabled", vec![], vec![]).unwrap(),
            SecretSkey::new("active-fixture").unwrap(),
        ),
        AccountOrigin::BrowserLogin,
    );
    for index in 0..8 {
        repository.upsert_verified(
            VerifiedAccount::from_parts(
                AccountIdentity::from_profile(&format!("Fixture {index}"), "enabled").unwrap(),
                SecretSkey::new(format!("fixture-{index}")).unwrap(),
            ),
            AccountOrigin::ManualSkey,
        );
    }
    repository.activate(&active).unwrap();
    repository
}

fn profile_draft() -> AccountProfileDraft {
    AccountProfileDraft::new(NAME, USERNAME).unwrap()
}

fn password_draft() -> AccountPasswordDraft {
    AccountPasswordDraft::new(CURRENT, NEW, NEW)
}

fn click_label(ui: &mut Ui<'_>, renderer: &mut Renderer, label: &str) -> Vec<Message> {
    let center = geometry(ui, renderer).label(label).center();
    click(ui, renderer, center)
}

fn redraw(ui: &mut Ui<'_>, renderer: &mut Renderer, theme: &Theme, size: Size<u32>, name: &str) {
    capture(ui, renderer, theme, size, name, mouse::Cursor::Unavailable);
}

fn login_view(app: &App) -> Element<'_, Message> {
    iced::widget::container(view(app)).align_bottom(iced::Fill).align_right(iced::Fill).into()
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_31_layout_and_interaction() {
    if !isolated("ui_refresh_31_layout_and_interaction") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    for (locale, width, height, theme, language) in [
        (DesktopLocale::SimplifiedChinese, 1280, 700, Theme::Light, "zh-cn"),
        (DesktopLocale::TraditionalChinese, 640, 600, Theme::Dark, "zh-tw"),
        (DesktopLocale::English, 420, 450, Theme::Dark, "en"),
    ] {
        let size = Size::new(width, height);
        app.desktop_locale = locale;
        app.account.menu_open = true;
        app.account.replace_repository(AccountRepository::default());
        app.account.manual_input = ManualSkeyInput::new(SKEY);
        let mut ui = scene(login_view(&app), &mut renderer, size);
        redraw(&mut ui, &mut renderer, &theme, size, &format!("login-{language}"));
        let positions = geometry(&mut ui, &renderer);
        let input = positions.input(&masked(SKEY));
        assert_fits(input, size);
        assert_centered(input, positions.label(I18n::new(locale).__("确认")));
        assert_fits(positions.label(I18n::new(locale).__("暂无本地账户")), size);
        drop(ui);

        // A previously open menu must remain behind the modal and outside its focus order.
        app.account.replace_repository(repository());
        app.account.details_open = true;
        for (name, profile, password) in [
            ("details", AccountProfileEditorState::Closed, AccountPasswordEditorState::Closed),
            (
                "profile",
                AccountProfileEditorState::Editing(profile_draft()),
                AccountPasswordEditorState::Closed,
            ),
            (
                "password",
                AccountProfileEditorState::Closed,
                AccountPasswordEditorState::Failed {
                    draft: password_draft(),
                    message: ERROR.into(),
                },
            ),
        ] {
            app.account.profile_editor = profile;
            app.account.password_editor = password;
            let mut ui = scene(app.view(), &mut renderer, size);
            redraw(&mut ui, &mut renderer, &theme, size, &format!("{name}-{language}"));
            let positions = geometry(&mut ui, &renderer);
            assert!(!positions.inputs.iter().any(|(value, _)| *value == masked(SKEY)));
            let i18n = I18n::new(locale);
            let (title, action) = match name {
                "details" => ("账户信息", "更新资料"),
                "profile" => ("编辑我的资料", "保存资料"),
                _ => ("修改我的密码", "修改密码"),
            };
            let header = positions.label(i18n.__(title));
            let footer = positions.label(i18n.__(action));
            assert_fits(header, size);
            assert_fits(footer, size);
            let viewport = positions.scroll[0].0;
            assert!((viewport.center_x() - width as f32 / 2.0).abs() < 0.5, "{name}: {viewport:?}");
            assert!(header.y + header.height <= viewport.y);
            assert!(viewport.y + viewport.height <= footer.y);
            if name == "details" && width == 420 {
                scroll(&mut ui, &mut renderer, viewport.center(), -16.0);
                let after = geometry(&mut ui, &renderer);
                assert!(after.scroll[0].1.y > 0.0);
                assert_eq!(after.label(i18n.__(title)), header);
                assert_eq!(after.label(i18n.__(action)), footer);
                assert!(matches!(
                    click_label(&mut ui, &mut renderer, i18n.__("更新资料")).as_slice(),
                    [Message::Account(AccountMessage::OpenProfileEditor)]
                ));
                assert!(matches!(
                    click_label(&mut ui, &mut renderer, i18n.__("修改密码")).as_slice(),
                    [Message::Account(AccountMessage::OpenPasswordEditor)]
                ));
                redraw(&mut ui, &mut renderer, &theme, size, "details-scrolled-en");
            }
            if name == "profile" {
                let field = positions.input(NAME);
                assert_centered(field, positions.label(i18n.__("显示名称")));
                assert!(field.x + field.width <= viewport.x + viewport.width - 7.0);
            }
            if name == "password" {
                assert_centered(
                    positions.input(&masked(CURRENT)),
                    positions.label(i18n.__("当前密码")),
                );
                assert_eq!(
                    positions.inputs.iter().map(|(value, _)| value.clone()).collect::<Vec<_>>(),
                    [masked(CURRENT), masked(NEW), masked(NEW)],
                );
                assert_fits(positions.label(ERROR), size);
                if width == 420 {
                    scroll(&mut ui, &mut renderer, viewport.center(), -16.0);
                    let after = geometry(&mut ui, &renderer);
                    assert!(after.scroll[0].1.y > 0.0);
                    assert_eq!(after.label(i18n.__(title)), header);
                    assert_eq!(after.label(i18n.__(action)), footer);
                    redraw(&mut ui, &mut renderer, &theme, size, "password-scrolled-en");
                }
            }
        }
    }

    let mut legacy = repository();
    legacy.activate(app.account.accounts()[1].id()).unwrap();
    app.account.replace_repository(legacy);
    app.account.password_editor = AccountPasswordEditorState::Closed;
    let size = Size::new(420, 450);
    let mut ui = scene(app.view(), &mut renderer, size);
    let i18n = I18n::new(app.desktop_locale);
    let positions = geometry(&mut ui, &renderer);
    assert_fits(positions.label(i18n.__("旧账户尚未获取个人资料，请重新登录刷新。")), size);
    assert!(!positions.labels.iter().any(|(label, _)| label == i18n.__("更新资料")));
    redraw(&mut ui, &mut renderer, &Theme::Dark, size, "details-legacy-en");
}

fn masked(value: &str) -> String {
    "•".repeat(value.chars().count())
}

fn assert_centered(left: Rectangle, right: Rectangle) {
    assert!((left.center_y() - right.center_y()).abs() < 0.5, "{left:?} vs {right:?}");
}

fn backspace_after_end(ui: &mut Ui<'_>, renderer: &mut Renderer, input: Rectangle) -> Vec<Message> {
    assert!(click(ui, renderer, input.center()).is_empty());
    assert!(key_at(ui, renderer, Named::End).is_empty());
    key_at(ui, renderer, Named::Backspace)
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_31_login_menu_preserves_account_actions() {
    if !isolated("ui_refresh_31_login_menu_preserves_account_actions") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    let size = Size::new(420, 450);
    let trigger = Point::new(390.0, 438.0);
    app.desktop_locale = DesktopLocale::English;
    let i18n = I18n::new(app.desktop_locale);
    let mut ui = scene(login_view(&app), &mut renderer, size);
    assert!(matches!(
        click(&mut ui, &mut renderer, trigger).as_slice(),
        [Message::Account(AccountMessage::ToggleMenu)]
    ));
    drop(ui);

    app.account.menu_open = true;
    app.account.manual_input = ManualSkeyInput::new(SKEY);
    app.account.replace_repository(repository());
    let other_id = app.account.accounts()[1].id();
    let mut ui = scene(login_view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let input = positions.input(&masked(SKEY));
    assert!(matches!(
        backspace_after_end(&mut ui, &mut renderer, input).as_slice(),
        [Message::Account(AccountMessage::ManualInputChanged(value))]
            if value.expose() == &SKEY[..SKEY.len() - 1]
    ));
    assert!(matches!(
        key_at(&mut ui, &mut renderer, Named::Enter).as_slice(),
        [Message::Account(AccountMessage::SubmitManual)]
    ));
    assert!(key_at(&mut ui, &mut renderer, Named::Escape).is_empty());
    assert!(matches!(
        click_label(&mut ui, &mut renderer, i18n.__("确认")).as_slice(),
        [Message::Account(AccountMessage::SubmitManual)]
    ));
    assert!(matches!(
        click_label(&mut ui, &mut renderer, "Fixture 0").as_slice(),
        [Message::Account(AccountMessage::SelectSaved(id))] if id == other_id
    ));
    let delete =
        positions.labels.iter().filter(|(label, _)| label == i18n.__("删除")).nth(1).unwrap().1;
    assert!(matches!(
        click(&mut ui, &mut renderer, delete.center()).as_slice(),
        [Message::Account(AccountMessage::DeleteSaved(id))] if id == other_id
    ));
    for (label, expected) in [
        ("切换账号登录", AccountMessage::StartBrowserLogin),
        ("账户信息", AccountMessage::OpenDetails),
        ("退出登录", AccountMessage::SignOut),
    ] {
        let messages = click_label(&mut ui, &mut renderer, i18n.__(label));
        assert!(matches!(messages.as_slice(), [Message::Account(actual)]
            if std::mem::discriminant(actual) == std::mem::discriminant(&expected)));
    }
    assert!(matches!(
        click_label(&mut ui, &mut renderer, i18n.__("安装识矩技能")).as_slice(),
        [Message::SkillHub(SkillHubMessage::OpenInstallGuide)]
    ));

    let mut hovered = Vec::new();
    for (label, name) in [("Fixture 0", "saved-hover-first"), ("Fixture 1", "saved-hover-second")] {
        let point = positions.label(label).center();
        assert!(
            send(
                &mut ui,
                &mut renderer,
                at(point),
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })]
            )
            .is_empty()
        );
        hovered.push(capture(&mut ui, &mut renderer, &Theme::Dark, size, name, at(point)));
    }
    assert_ne!(hovered[0], hovered[1]);
    assert!(
        send(
            &mut ui,
            &mut renderer,
            mouse::Cursor::Unavailable,
            &[Event::Mouse(mouse::Event::CursorLeft)]
        )
        .is_empty()
    );
    let viewport = positions.scroll[0].0;
    assert_fits(viewport, size);
    scroll(&mut ui, &mut renderer, viewport.center(), -20.0);
    assert!(geometry(&mut ui, &renderer).scroll[0].1.y > 0.0);
    assert_eq!(geometry(&mut ui, &renderer).inputs[0].1, input);
    redraw(&mut ui, &mut renderer, &Theme::Dark, size, "saved-scrolled-en");
    assert!(matches!(
        click(&mut ui, &mut renderer, trigger).as_slice(),
        [Message::Account(AccountMessage::CloseMenu)]
    ));
    drop(ui);

    app.account.manual_input = ManualSkeyInput::new(" ");
    let mut ui = scene(login_view(&app), &mut renderer, size);
    assert!(click_label(&mut ui, &mut renderer, i18n.__("确认")).is_empty());
    let empty = geometry(&mut ui, &renderer).input(&masked(" "));
    assert!(click(&mut ui, &mut renderer, empty.center()).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::Enter).is_empty());
    drop(ui);

    let challenge: vw_gateway_client::ShijuLoginChallengeDto =
        serde_json::from_value(serde_json::json!({
            "state": "fixture",
            "authorization_url": "https://auth.shiju.cloud/fixture",
            "poll_interval_seconds": 1,
        }))
        .unwrap();
    app.account.operation = AccountOperation::WaitingAuthorization {
        challenge: LoginChallenge::try_from(challenge).unwrap(),
        started_at: std::time::Instant::now(),
    };
    let mut ui = scene(login_view(&app), &mut renderer, size);
    assert!(click_label(&mut ui, &mut renderer, "Fixture 0").is_empty());
    assert!(click_label(&mut ui, &mut renderer, i18n.__("删除")).is_empty());
    assert!(matches!(
        click_label(&mut ui, &mut renderer, i18n.__("取消等待")).as_slice(),
        [Message::Account(AccountMessage::CancelBrowserLogin)]
    ));
    redraw(&mut ui, &mut renderer, &Theme::Dark, size, "login-waiting");
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_31_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_31_preserves_values_and_disabled_states") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    let size = Size::new(420, 450);
    let i18n = I18n::new(app.desktop_locale);
    app.account.manual_input = ManualSkeyInput::new(SKEY);
    app.account.operation = AccountOperation::Validating;
    let mut ui = scene(menu(&app), &mut renderer, size);
    assert!(click_label(&mut ui, &mut renderer, i18n.__("确认")).is_empty());
    assert!(click_label(&mut ui, &mut renderer, i18n.__("登录")).is_empty());
    let center = geometry(&mut ui, &renderer).input(&masked(SKEY)).center();
    assert!(click(&mut ui, &mut renderer, center).is_empty());
    assert!(
        key_at(&mut ui, &mut renderer, Named::Enter).is_empty(),
        "busy manual login emitted SubmitManual on Enter"
    );
    redraw(&mut ui, &mut renderer, &Theme::Light, size, "login-validating");
    drop(ui);

    use crate::app::message::account::update;
    app.account.replace_repository(repository());
    app.account.details_open = true;
    app.account.profile_editor = AccountProfileEditorState::Loading;
    let mut ui = scene(app.view(), &mut renderer, size);
    assert!(click_label(&mut ui, &mut renderer, i18n.__("← 返回我的")).is_empty());
    redraw(&mut ui, &mut renderer, &Theme::Light, size, "profile-loading");
    drop(ui);

    app.account.profile_editor = AccountProfileEditorState::Failed(ERROR.into());
    let mut ui = scene(app.view(), &mut renderer, size);
    assert!(matches!(
        click_label(&mut ui, &mut renderer, i18n.__("重新获取")).as_slice(),
        [Message::Account(AccountMessage::OpenProfileEditor)]
    ));
    redraw(&mut ui, &mut renderer, &Theme::Light, size, "profile-load-failed");
    drop(ui);

    let mut invalid = profile_draft();
    invalid.set_name(" ".into());
    for (state, action, screenshot) in [
        (AccountProfileEditorState::Editing(invalid), "保存资料", "profile-invalid"),
        (AccountProfileEditorState::Saving(profile_draft()), "保存中…", "profile-saving"),
    ] {
        app.account.profile_editor = state;
        let mut ui = scene(app.view(), &mut renderer, size);
        assert!(click_label(&mut ui, &mut renderer, i18n.__(action)).is_empty());
        redraw(&mut ui, &mut renderer, &Theme::Light, size, screenshot);
    }
    app.account.profile_editor = AccountProfileEditorState::Editing(profile_draft());
    let mut ui = scene(app.view(), &mut renderer, size);
    let username = geometry(&mut ui, &renderer).input(USERNAME);
    assert!(matches!(
        backspace_after_end(&mut ui, &mut renderer, username).as_slice(),
        [Message::Account(AccountMessage::ProfileUsernameChanged(value))]
            if value == USERNAME.strip_suffix('r').unwrap()
    ));
    assert!(matches!(
        click_label(&mut ui, &mut renderer, i18n.__("保存资料")).as_slice(),
        [Message::Account(AccountMessage::SubmitProfileEditor)]
    ));
    let back = click_label(&mut ui, &mut renderer, i18n.__("← 返回我的"));
    assert!(matches!(back.as_slice(), [Message::Account(AccountMessage::CloseProfileEditor)]));
    drop(ui);
    let _ = update(&mut app, AccountMessage::CloseProfileEditor);
    assert!(matches!(app.account.profile_editor, AccountProfileEditorState::Closed));
    assert_eq!(app.account.active_account().unwrap().profile().unwrap().account_name(), USERNAME);

    app.account.password_editor = AccountPasswordEditorState::Saving(password_draft());
    let mut ui = scene(app.view(), &mut renderer, size);
    let current = geometry(&mut ui, &renderer).input(&masked(CURRENT));
    assert!(backspace_after_end(&mut ui, &mut renderer, current).is_empty());
    assert!(click_label(&mut ui, &mut renderer, i18n.__("修改中…")).is_empty());
    assert!(click_label(&mut ui, &mut renderer, i18n.__("← 返回我的")).is_empty());
    redraw(&mut ui, &mut renderer, &Theme::Dark, size, "password-saving");
    drop(ui);

    app.account.password_editor =
        AccountPasswordEditorState::Failed { draft: password_draft(), message: ERROR.into() };
    let mut ui = scene(app.view(), &mut renderer, size);
    assert!(matches!(
        click_label(&mut ui, &mut renderer, i18n.__("关闭")).as_slice(),
        [Message::Account(AccountMessage::DismissPasswordError)]
    ));
    drop(ui);
    let _ = update(&mut app, AccountMessage::DismissPasswordError);
    assert_eq!(app.account.password_editor.draft(), Some(&password_draft()));
    let mut ui = scene(app.view(), &mut renderer, size);
    let current = geometry(&mut ui, &renderer).input(&masked(CURRENT));
    let mut messages = backspace_after_end(&mut ui, &mut renderer, current);
    let [
        Message::Account(AccountMessage::PasswordFieldChanged {
            field: AccountPasswordField::Current,
            ..
        }),
    ] = messages.as_slice()
    else {
        panic!("editing the current password should emit one field change");
    };
    let Message::Account(change) = messages.remove(0) else { unreachable!() };
    assert!(matches!(
        click_label(&mut ui, &mut renderer, i18n.__("修改密码")).as_slice(),
        [Message::Account(AccountMessage::SubmitPasswordEditor)]
    ));
    assert!(matches!(
        click_label(&mut ui, &mut renderer, i18n.__("← 返回我的")).as_slice(),
        [Message::Account(AccountMessage::ClosePasswordEditor)]
    ));
    drop(ui);
    let _ = update(&mut app, change);
    assert_eq!(app.account.password_editor.draft().unwrap().current_password(), "MMMM111");
    let _ = update(&mut app, AccountMessage::ClosePasswordEditor);
    let _ = update(&mut app, AccountMessage::OpenPasswordEditor);
    assert_eq!(app.account.password_editor.draft(), Some(&AccountPasswordDraft::default()));

    let mut ui = scene(app.view(), &mut renderer, size);
    let messages = click_label(&mut ui, &mut renderer, i18n.__("修改密码"));
    assert!(matches!(
        messages.as_slice(),
        [Message::Account(AccountMessage::SubmitPasswordEditor)]
    ));
    assert_eq!(geometry(&mut ui, &renderer).inputs[0].0, i18n.__("请输入当前密码"));
    drop(ui);
    let _ = update(&mut app, AccountMessage::SubmitPasswordEditor);
    assert!(app.active_toast.is_some());
    assert!(!app.account.password_editor.is_busy());
    let mut ui = scene(app.view(), &mut renderer, size);
    redraw(&mut ui, &mut renderer, &Theme::Light, size, "password-invalid");
}

#[tokio::test(flavor = "multi_thread")]
async fn monthly_quota_renders_in_status_bar_and_refreshes() {
    if !isolated("monthly_quota_renders_in_status_bar_and_refreshes") {
        return;
    }
    use crate::app::state::MonthlyQuotaState;

    let mut app = fixture();
    app.account.replace_repository(repository());
    app.account.monthly_quota = MonthlyQuotaState::Ready(vw_gateway_client::ShijuMonthlyQuotaDto {
        limit: 100.0,
        used: 99.9941,
        remaining: Some(0.0059),
        reset_at: 1796054400,
    });
    let size = Size::new(800, 600);
    let mut renderer = renderer().await;
    for (locale, theme) in
        [(DesktopLocale::SimplifiedChinese, Theme::Dark), (DesktopLocale::English, Theme::Light)]
    {
        app.desktop_locale = locale;
        app.account.monthly_quota_open = false;
        let i18n = I18n::new(locale);
        let trigger = iced::widget::container(super::monthly_quota(&app).unwrap())
            .align_bottom(iced::Fill)
            .align_right(iced::Fill);
        let mut ui = scene(trigger.into(), &mut renderer, size);
        assert!(matches!(
            click(&mut ui, &mut renderer, Point::new(770.0, 587.0)).as_slice(),
            [Message::Account(AccountMessage::ToggleMonthlyQuota)]
        ));
        drop(ui);
        app.account.monthly_quota_open = true;
        let mut ui = scene(app.view(), &mut renderer, size);
        redraw(&mut ui, &mut renderer, &theme, size, &format!("monthly-quota-{locale:?}"));
        let positions = geometry(&mut ui, &renderer);
        for label in ["¥99.99", "¥100.00", "12/01", i18n.__("已用金额包含在途预占。")]
        {
            assert_fits(positions.label(label), size);
        }
        assert!(matches!(
            click_label(&mut ui, &mut renderer, i18n.__("刷新")).as_slice(),
            [Message::Account(AccountMessage::RefreshMonthlyQuota)]
        ));
    }
}
