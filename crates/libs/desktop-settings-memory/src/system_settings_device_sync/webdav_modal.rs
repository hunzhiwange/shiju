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
fn message(event: Event) -> Event { event }
pub struct ViewState<'a> {
    pub busy: bool,
    pub draft: &'a vw_api_device_sync::device_sync_config::UpdateWebDavSyncConfigRequest,
    pub principal: &'a str,
    pub secret_present: bool,
    pub auto_confirmation: bool,
    pub task_busy: bool,
    pub clear_confirmation: bool,
}
use desktop_ui_controls::system_settings_common::*;
use iced::Element;
use iced::widget::{button, checkbox, column, row, text};
use vw_api_device_sync::device_sync_config::SyncSecretUpdate;

fn field<'a>(
    label: &'static str,
    value: &'a str,
    event: fn(String) -> Event,
    enabled: bool,
    secret: bool,
) -> Element<'a, Event> {
    let mut input = settings_text_input(label, value).secure(secret);
    if enabled {
        input = input.on_input(move |v| message(event(v)));
    }
    column![text(label).size(13), input].spacing(5).into()
}
fn action(label: &'static str, event: Event, enabled: bool) -> Element<'static, Event> {
    button(text(label))
        .on_press_maybe(enabled.then(|| message(event)))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .into()
}
pub fn view<'a>(s: ViewState<'a>, selection: Element<'a, Event>, preview: Element<'a, Event>, task_panel: Element<'a, Event>) -> Element<'a, Event> {
    let editable = !s.busy && !s.task_busy;
    let password = match &s.draft.secret {
        SyncSecretUpdate::Replace { value } => value.as_str(),
        _ => "",
    };
    let mut enabled = checkbox(s.draft.enabled).label("启用此连接").style(settings_checkbox_style);
    let mut auto = checkbox(s.draft.auto_sync).label("自动上传").style(settings_checkbox_style);
    if editable {
        enabled = enabled.on_toggle(|v| message(Event::Enabled(v)));
        auto = auto.on_toggle(|v| message(Event::Auto(v)));
    }
    let mut body = column![
        text("连接仅作用于当前项目与主机；不会同步凭据或设备专属数据。"),
        text("服务预设"),
        row![
            action("自定义", Event::BaseUrl(String::new()), editable),
            action(
                "Nextcloud",
                Event::BaseUrl("https://cloud.example.com/remote.php/dav/files/USERNAME/".into()),
                editable
            )
        ]
        .spacing(8),
        text("示例地址须替换为自己的服务器与用户名。"),
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
        enabled,
        auto,
        selection,
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
    body = body
        .push(preview)
        .push(task_panel);
    let actions = row![
        action("取消", Event::Close, !s.busy),
        action("刷新", Event::Load, editable),
        action("测试连接", Event::Test, editable),
        action("清除", Event::Clear, editable),
        button(text(if s.busy { "处理中…" } else { "保存" }))
            .on_press_maybe(editable.then(|| message(Event::Save)))
            .padding([8, 14])
            .style(primary_action_btn_style)
    ]
    .spacing(8)
    .wrap();
    let header = settings_modal_header(
        text("WebDAV 设备同步").size(20),
        settings_close_button(message(Event::Close)),
    );
    let card = settings_modal_layout(header, body, Some(actions.into())).width(760).height(720);
    settings_modal_overlay(None, message(Event::Close), card)
}
