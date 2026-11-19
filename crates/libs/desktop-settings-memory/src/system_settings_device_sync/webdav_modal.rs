//! WebDAV settings keep credentials write-only and transfer choices explicit.
#[derive(Clone)]
pub enum Event {
    Enabled(bool),
    Auto(bool),
    BaseUrl(String),
    Username(String),
    Password(String),
    Root(String),
    Profile(String),
    Principal(String),
    ConfirmAuto(bool),
    ConfirmClear(bool),
    Close,
    Load,
    Test,
    Clear,
    Save,
    Selection(gateway_device_sync::DeviceSyncSelection),
    Preview(crate::system_settings_device_sync::preview::PreviewMessage),
    TaskPanel(super::task_panel::TaskPanelMessage),
}
fn message(event: Event) -> Event {
    event
}
pub struct ViewState<'a> {
    pub busy: bool,
    pub draft: &'a vw_api_device_sync::device_sync_config::UpdateWebDavSyncConfigRequest,
    pub principal: &'a str,
    pub secret_present: bool,
    pub auto_confirmation: bool,
    pub task_busy: bool,
    pub preview_busy: bool,
    pub clear_confirmation: bool,
}
impl ViewState<'_> {
    pub fn editable(&self) -> bool {
        !self.busy && !self.task_busy && !self.preview_busy
    }
}
use desktop_ui_controls::system_settings_common::*;
use iced::Element;
use iced::widget::{button, column, row, text};
use vw_api_device_sync::device_sync_config::SyncSecretUpdate;

fn field<'a>(
    label: &'static str,
    value: &'a str,
    event: fn(String) -> Event,
    enabled: bool,
    secret: bool,
) -> Element<'a, Event> {
    settings_responsive_row(label, "", move || {
        settings_text_input("", value).secure(secret).on_input_maybe(enabled.then_some(event))
    })
}

fn action(label: &'static str, event: Event, enabled: bool) -> Element<'static, Event> {
    button(text(label))
        .on_press_maybe(enabled.then(|| message(event)))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .into()
}
pub fn view<'a>(
    s: ViewState<'a>,
    selection: Element<'a, Event>,
    preview: Element<'a, Event>,
    task_panel: Element<'a, Event>,
    inline: bool,
) -> Element<'a, Event> {
    let editable = s.editable();
    let password = match &s.draft.secret {
        SyncSecretUpdate::Replace { value } => value.as_str(),
        _ => "",
    };
    let mut body = column![
        settings_responsive_row(
            "服务商",
            "Nextcloud 预设中的服务器和用户名需要替换。",
            move || {
                const NEXTCLOUD_URL: &str =
                    "https://cloud.example.com/remote.php/dav/files/USERNAME/";
                let preset =
                    if s.draft.base_url == NEXTCLOUD_URL { "Nextcloud" } else { "自定义" };
                settings_select(["自定义", "Nextcloud"], Some(preset), |choice| {
                    Event::BaseUrl(if choice == "Nextcloud" {
                        NEXTCLOUD_URL.into()
                    } else {
                        String::new()
                    })
                })
                .width(iced::Length::Fill)
                .disabled(!editable)
            }
        ),
        field("WebDAV 地址", &s.draft.base_url, Event::BaseUrl, editable, false),
        field("用户名", &s.draft.username, Event::Username, editable, false),
        field(
            if s.secret_present { "密码（已保存；留空保留）" } else { "密码" },
            password,
            Event::Password,
            editable,
            true
        ),
        field("远端根目录", &s.draft.remote_root, Event::Root, editable, false),
        field("配置档案", &s.draft.profile, Event::Profile, editable, false),
        field("同步用户标识", s.principal, Event::Principal, editable, false),
        settings_responsive_row("启用 WebDAV 同步", "", move || {
            settings_toggle(s.draft.enabled).on_toggle_maybe(editable.then_some(Event::Enabled))
        }),
        settings_responsive_row(
            "自动上传",
            "由网关在后台上传所选数据。",
            move || {
                settings_toggle(s.draft.auto_sync).on_toggle_maybe(editable.then_some(Event::Auto))
            }
        ),
    ]
    .spacing(12);
    if s.auto_confirmation {
        body = body.push(
            column![
                text("启用后网关将在后台自动上传当前选择，是否确认？"),
                row![
                    action("确认启用", Event::ConfirmAuto(true), editable),
                    action("取消", Event::ConfirmAuto(false), editable)
                ]
                .spacing(8)
            ]
            .spacing(8),
        );
    }
    if s.clear_confirmation {
        body = body.push(
            column![
                text("清除此设备连接与凭据？远端对象将保留。"),
                row![
                    action("确认清除", Event::ConfirmClear(true), editable),
                    action("保留", Event::ConfirmClear(false), editable)
                ]
                .spacing(8)
            ]
            .spacing(8),
        );
    }
    let mut actions = row![
        action("刷新", Event::Load, editable),
        action("测试连接", Event::Test, editable),
        action("清除", Event::Clear, editable),
        button(text(if s.busy { "处理中…" } else { "保存配置" }))
            .on_press_maybe(editable.then(|| message(Event::Save)))
            .padding([8, 14])
            .style(primary_action_btn_style)
    ]
    .spacing(8);
    if !inline {
        actions = actions.push(action("取消", Event::Close, !s.busy));
    }
    let actions = actions.wrap();

    if inline {
        return settings_group_card(
            body.push(actions)
                .push(settings_divider())
                .push(selection)
                .push(preview)
                .push(task_panel),
        )
        .padding([16, 18])
        .into();
    }
    let body = body.push(selection).push(preview).push(task_panel);
    let header = settings_modal_header(
        text("WebDAV 设备同步").size(20),
        settings_close_button(message(Event::Close)),
    );
    settings_modal_layout(header, body, Some(actions.into())).width(760).height(720).into()
}
