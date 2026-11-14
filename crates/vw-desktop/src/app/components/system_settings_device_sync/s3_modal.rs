//! S3 connection editor; secrets remain write-only and failed saves retain the draft.
use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_close_button, settings_field_label, settings_modal_header, settings_modal_layout,
    settings_modal_overlay, settings_text_input,
};
use crate::app::message::settings::device_sync_s3::{Event, S3Preset, message};
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, row, text};
use iced::{Element, Length};
use vw_gateway_client::vw_api_types::device_sync_config::SyncSecretUpdate;

fn field<'a>(
    label: &'static str,
    value: &str,
    event: fn(String) -> Event,
    enabled: bool,
    secret: bool,
) -> Element<'a, Message> {
    let mut input = settings_text_input("", value).secure(secret);
    if enabled {
        input = input.on_input(move |v| message(event(v)));
    }
    column![settings_field_label(label), input].spacing(6).into()
}

fn toggle<'a>(
    label: &'static str,
    checked: bool,
    event: fn(bool) -> Event,
    enabled: bool,
) -> Element<'a, Message> {
    let mut control = checkbox(checked).label(label).style(settings_checkbox_style);
    if enabled {
        control = control.on_toggle(move |v| message(event(v)));
    }
    control.into()
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.apps_catalog.device_sync_s3;
    let draft = &state.draft;
    let enabled = !state.busy && !state.preview.busy && !state.task_panel.is_busy();
    let secret = match &draft.secret {
        SyncSecretUpdate::Replace { value } => value.as_str(),
        _ => "",
    };
    let mut presets = row![].spacing(8);
    for preset in [S3Preset::Aws, S3Preset::Minio, S3Preset::R2, S3Preset::Oss, S3Preset::Custom] {
        let label =
            if preset == state.preset { format!("✓ {preset}") } else { preset.to_string() };
        let mut control = button(text(label)).padding([8, 12]).style(rounded_action_btn_style);
        if enabled {
            control = control.on_press(message(Event::Preset(preset)));
        }
        presets = presets.push(control);
    }
    let mut clear = button("清除密钥").padding([8, 12]).style(rounded_action_btn_style);
    if enabled {
        clear = clear.on_press(message(Event::ClearSecret));
    }
    let mut body = column![
        text("配置保存在当前网关设备；密钥不会回填到表单。").size(13),
        settings_field_label("服务预设"),
        presets,
        field("Region", &draft.region, Event::Region, enabled, false),
        field("Bucket", &draft.bucket, Event::Bucket, enabled, false),
        field(
            "Endpoint（AWS 可留空）",
            draft.endpoint.as_deref().unwrap_or(""),
            Event::Endpoint,
            enabled,
            false
        ),
        field("Access Key ID", &draft.access_key_id, Event::AccessKey, enabled, false),
        field("Secret Access Key", secret, Event::Secret, enabled, true),
        text(if state.secret_present {
            "已保存密钥；留空保留，修改连接信息后需重新填写。"
        } else {
            "尚未保存密钥，请填写实际凭据。"
        })
        .size(12),
        clear,
        field("远端根目录", &draft.remote_root, Event::Root, enabled, false),
        field("Profile", &draft.profile, Event::Profile, enabled, false),
        field("用户标识", &state.principal, Event::Principal, enabled, false),
        toggle("启用此连接", draft.enabled, Event::Enabled, enabled),
        toggle("自动上传", draft.auto_sync, Event::AutoSync, enabled),
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
    body = body
        .push(super::selection::view(app, &state.selection, enabled, |s| {
            message(Event::Selection(s))
        }))
        .push(state.preview.view().map(|event| message(Event::Preview(event))))
        .push(state.task_panel.view().map(|event| message(Event::TaskPanel(event))));
    let mut save = button(if state.busy { "处理中…" } else { "保存" })
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
    let header = settings_modal_header(
        text("S3 设备同步").size(22),
        settings_close_button(message(Event::Close)),
    );
    let card = settings_modal_layout(
        header,
        body,
        Some(row![refresh, test, cancel, save].spacing(8).into()),
    )
    .width(Length::Fixed(760.0))
    .height(Length::Fill)
    .max_height(780.0);
    settings_modal_overlay(None, message(Event::Close), card)
}
