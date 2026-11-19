//! S3 connection editor; secrets remain write-only and failed saves retain the draft.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum S3Preset {
    #[default]
    Aws,
    Minio,
    R2,
    Oss,
    Custom,
}
impl std::fmt::Display for S3Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Aws => "AWS",
            Self::Minio => "MinIO",
            Self::R2 => "R2",
            Self::Oss => "OSS",
            Self::Custom => "Custom",
        })
    }
}
#[derive(Clone)]
pub enum Event {
    Preset(S3Preset),
    ClearSecret,
    Region(String),
    Bucket(String),
    Endpoint(String),
    AccessKey(String),
    Secret(String),
    Root(String),
    Profile(String),
    Principal(String),
    Enabled(bool),
    AutoSync(bool),
    ConfirmAutoSync(bool),
    Save,
    Test,
    Load,
    Close,
    Selection(gateway_device_sync::DeviceSyncSelection),
    Preview(crate::system_settings_device_sync::preview::PreviewMessage),
    TaskPanel(super::task_panel::TaskPanelMessage),
}
fn message(event: Event) -> Event {
    event
}
pub struct ViewState<'a> {
    pub busy: bool,
    pub draft: &'a vw_api_device_sync::device_sync_config::UpdateS3SyncConfigRequest,
    pub principal: &'a str,
    pub secret_present: bool,
    pub auto_confirmation: bool,
    pub task_busy: bool,
    pub preview_busy: bool,
    pub preset: S3Preset,
}
impl ViewState<'_> {
    pub fn editable(&self) -> bool {
        !self.busy && !self.task_busy && !self.preview_busy
    }
}
use desktop_ui_controls::system_settings_common::*;
use iced::widget::{button, column, row, text};
use iced::{Element, Length};
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

pub fn view<'a>(
    state: ViewState<'a>,
    selection: Element<'a, Event>,
    preview: Element<'a, Event>,
    task_panel: Element<'a, Event>,
    inline: bool,
) -> Element<'a, Event> {
    let draft = state.draft;
    let enabled = state.editable();
    let secret = match &draft.secret {
        SyncSecretUpdate::Replace { value } => value.as_str(),
        _ => "",
    };
    let preset = state.preset;
    let preset_selector = settings_responsive_row("服务商", "", move || {
        settings_select(
            [S3Preset::Aws, S3Preset::Minio, S3Preset::R2, S3Preset::Oss, S3Preset::Custom],
            Some(preset),
            Event::Preset,
        )
        .width(Length::Fill)
        .disabled(!enabled)
    });
    let mut body = column![
        preset_selector,
        field("区域（Region）", &draft.region, Event::Region, enabled, false),
        field("存储桶（Bucket）", &draft.bucket, Event::Bucket, enabled, false),
        field(
            "Endpoint（AWS 可留空）",
            draft.endpoint.as_deref().unwrap_or(""),
            Event::Endpoint,
            enabled,
            false
        ),
        field("Access Key ID", &draft.access_key_id, Event::AccessKey, enabled, false),
        field("Secret Access Key", secret, Event::Secret, enabled, true),
        settings_responsive_row(
            "密钥管理",
            if state.secret_present {
                "留空保留已保存的密钥。"
            } else {
                "尚未保存密钥。"
            },
            move || {
                button("清除密钥")
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((enabled && state.secret_present).then_some(Event::ClearSecret))
            }
        ),
        field("远端根目录", &draft.remote_root, Event::Root, enabled, false),
        field("配置档案", &draft.profile, Event::Profile, enabled, false),
        field("同步用户标识", state.principal, Event::Principal, enabled, false),
        settings_responsive_row("启用 S3 同步", "", move || {
            settings_toggle(draft.enabled).on_toggle_maybe(enabled.then_some(Event::Enabled))
        }),
        settings_responsive_row(
            "自动上传",
            "由网关在后台上传所选数据。",
            move || {
                settings_toggle(draft.auto_sync).on_toggle_maybe(enabled.then_some(Event::AutoSync))
            }
        ),
    ]
    .spacing(12)
    .width(Length::Fill);
    if state.auto_confirmation {
        body = body.push(
            column![
                text("自动上传会由网关在后台运行。确认启用？"),
                row![
                    button("确认启用")
                        .padding([8, 12])
                        .style(primary_action_btn_style)
                        .on_press(message(Event::ConfirmAutoSync(true))),
                    button("取消")
                        .padding([8, 12])
                        .style(rounded_action_btn_style)
                        .on_press(message(Event::ConfirmAutoSync(false)))
                ]
                .spacing(8)
            ]
            .spacing(8),
        );
    }
    let mut save = button(if state.busy { "处理中…" } else { "保存配置" })
        .padding([9, 16])
        .style(primary_action_btn_style);
    let mut test = button("测试连接").padding([9, 16]).style(rounded_action_btn_style);
    let mut refresh = button("刷新").padding([9, 16]).style(rounded_action_btn_style);
    let mut cancel = button("取消").padding([9, 16]).style(rounded_action_btn_style);
    if enabled {
        save = save.on_press(message(Event::Save));
        test = test.on_press(message(Event::Test));
        refresh = refresh.on_press(message(Event::Load));
        cancel = cancel.on_press(message(Event::Close));
    }
    let mut actions = row![test, save, refresh].spacing(8);
    if !inline {
        actions = actions.push(cancel);
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
        text("S3 设备同步").size(22),
        settings_close_button(message(Event::Close)),
    );
    settings_modal_layout(header, body, Some(actions.into()))
        .width(Length::Fixed(760.0))
        .height(Length::Fill)
        .max_height(780.0)
        .into()
}
