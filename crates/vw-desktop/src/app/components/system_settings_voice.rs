//! Native voice preferences use the shared form and modal controls.
use super::system_settings_common::*;
use crate::app::i18n::I18n;
use crate::app::message::{
    SettingsMessage,
    settings::voice::{Event, State},
};
use crate::app::{App, Message};
use iced::widget::{Space, button, checkbox, column, row, text};
use iced::{Alignment, Element, Length};
use vw_config_types::voice::{RecognitionLanguage, VoiceName};
fn msg(event: Event) -> Message {
    Message::Settings(SettingsMessage::Voice(event))
}
fn action<'a>(label: String, event: Event) -> iced::widget::Button<'a, Message> {
    button(text(label)).on_press(msg(event)).style(rounded_action_btn_style).padding([8, 14])
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    column![
        text(i18n.__("语音设置")).size(22),
        text(i18n.__("设备保存在本机；服务、语言、词典与音色保存在当前网关。"))
            .style(settings_muted_text_style),
        {
            let b = button(text(i18n.__("编辑语音设置")))
                .padding([8, 14])
                .style(primary_action_btn_style);
            if app.voice.settings.busy { b } else { b.on_press(msg(Event::Open)) }
        },
        text(app.voice.hotkeys.status()).style(settings_muted_text_style),
        action("立即停用全局热键（本机）".into(), Event::DisableHotkeys),
        super::voice_recordings::view(app),
        text(i18n.__("屏幕上下文：未启用")).style(settings_muted_text_style),
    ]
    .spacing(14)
    .into()
}
#[derive(Clone, PartialEq)]
struct OptionLabel<T> {
    value: T,
    label: String,
}
impl<T> std::fmt::Display for OptionLabel<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}
pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    modal(&app.voice.settings, I18n::new(app.desktop_locale), base)
}
pub(super) fn modal<'a>(
    state: &'a State,
    i18n: I18n,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(edit) = &state.edit else {
        return base;
    };
    let mut devices =
        vec![OptionLabel { value: None, label: i18n.__("系统默认麦克风").to_owned() }];
    devices.extend(
        state.devices.iter().map(|device| OptionLabel {
            value: Some(device.id.clone()),
            label: device.name.clone(),
        }),
    );
    let selected =
        devices.iter().find(|d| d.value == edit.local.microphone).cloned().unwrap_or_else(|| {
            let entry = OptionLabel {
                value: edit.local.microphone.clone(),
                label: i18n.__("已选麦克风不可用，请重新选择").to_owned(),
            };
            devices.push(entry.clone());
            entry
        });
    let languages = [
        (RecognitionLanguage::Auto, "自动检测"),
        (RecognitionLanguage::Zh, "中文"),
        (RecognitionLanguage::En, "英语"),
        (RecognitionLanguage::Es, "西班牙语"),
        (RecognitionLanguage::Fr, "法语"),
    ]
    .map(|(value, label)| OptionLabel { value, label: i18n.__(label).to_owned() });
    let selected_language = languages.iter().find(|l| l.value == edit.config.language).cloned();
    let provider = edit.config.provider_id.as_deref().unwrap_or("");
    let form = column![
        settings_field_label(i18n.__("麦克风")),
        settings_select(devices, Some(selected), |v| msg(Event::Microphone(v.value)))
            .width(Length::Fill),
        settings_field_label(i18n.__("识别语言")),
        settings_select(languages, selected_language, |v| msg(Event::Language(v.value)))
            .width(Length::Fill),
        settings_field_label(i18n.__("网关提供商 ID")),
        settings_text_input("openai", provider).on_input(|v| msg(Event::Provider(v))),
        text("OpenAI · gpt-transcribe · gpt-realtime-2.1").size(13),
        text(i18n.__("服务凭据请在提供商设置中配置。")).style(settings_muted_text_style),
        checkbox(edit.config.dictation_enabled)
            .label(i18n.__("启用听写"))
            .on_toggle(|v| msg(Event::Dictation(v)))
            .style(settings_checkbox_style),
        checkbox(edit.config.realtime_enabled)
            .label(i18n.__("启用语音聊天"))
            .on_toggle(|v| msg(Event::Realtime(v)))
            .style(settings_checkbox_style),
        checkbox(edit.local.retain_recordings)
            .label("仅在本机留存最近二十段听写录音（最多 64 MiB）")
            .on_toggle(|v| msg(Event::RetainRecordings(v)))
            .style(settings_checkbox_style),
        text("默认关闭；保存位置见语音设置页。关闭停止新增，已有录音请在最近本机录音中删除。")
            .style(settings_muted_text_style),
        settings_field_label("全局语音快捷键（本机）"),
        checkbox(edit.local.hotkeys.enabled)
            .label(if cfg!(target_os = "macos") { "启用全局快捷键" } else { "当前平台不支持全局快捷键" })
            .on_toggle(|v| msg(Event::HotkeysEnabled(v)))
            .style(settings_checkbox_style),
        row![
            text(format!("听写：{}", edit.local.hotkeys.dictation.map(|v| v.label()).unwrap_or_else(|| "未设置".into()))).width(Length::Fill),
            action("录制听写快捷键".into(), Event::RecordHotkey(Some(false))),
        ].spacing(8).align_y(Alignment::Center),
        row![
            text(format!("语音聊天：{}", edit.local.hotkeys.chat.map(|v| v.label()).unwrap_or_else(|| "未设置".into()))).width(Length::Fill),
            action("录制聊天快捷键".into(), Event::RecordHotkey(Some(true))),
        ].spacing(8).align_y(Alignment::Center),
        text(if state.recording_hotkey.is_some() {
            "请按修饰键加字母/空格；Escape 取消录制。不支持 Fn。"
        } else {
            "按住听写，松开完成；280 毫秒内双击进入免提，再按完成，Escape 取消。独立快捷键启动语音聊天。保存后生效。"
        }).style(settings_muted_text_style),
        action("检查/打开输入监控授权".into(), Event::HotkeyPermission),
        settings_field_label(i18n.__("音色：推荐")),
        voices(&[VoiceName::Marin, VoiceName::Cedar], edit.config.voice),
        settings_field_label(i18n.__("音色：其他")),
        voices(
            &[
                VoiceName::Alloy,
                VoiceName::Ash,
                VoiceName::Ballad,
                VoiceName::Coral,
                VoiceName::Echo,
                VoiceName::Sage,
                VoiceName::Shimmer,
                VoiceName::Verse
            ],
            edit.config.voice
        ),
        text(i18n.__("音色只影响新语音聊天。试听使用当前网关服务，不录音。"))
            .style(settings_muted_text_style),
        if state.preview.is_some() {
            action(i18n.__("停止试听").into(), Event::StopPreview)
        } else {
            action(i18n.__("试听音色").into(), Event::Preview)
        },
        settings_field_label(i18n.__("识别词典")),
        row![
            settings_text_input(i18n.__("添加专有名词"), &state.dictionary)
                .on_input(|v| msg(Event::Dictionary(v))),
            action(i18n.__("添加").into(), Event::AddWord)
        ]
        .spacing(8),
        column(edit.config.dictionary.iter().enumerate().map(|(index, word)| {
            row![
                text(word).width(Length::Fill),
                action(i18n.__("删除").into(), Event::RemoveWord(index))
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        }))
        .spacing(6),
    ]
    .spacing(12);
    let form: Element<'_, Message> =
        if state.busy { text(i18n.__("正在保存语音设置…")).into() } else { form.into() };
    let close = if state.busy { Message::None } else { msg(Event::Cancel) };
    let header = settings_modal_header(
        text(i18n.__("语音设置")).size(20),
        settings_close_button(close.clone()),
    );
    let mut save = button(text(i18n.__("保存"))).style(primary_action_btn_style).padding([8, 14]);
    let mut cancel = button(text(i18n.__("取消"))).style(rounded_action_btn_style).padding([8, 14]);
    if !state.busy {
        save = save.on_press(msg(Event::Save));
        cancel = cancel.on_press(msg(Event::Cancel));
    }
    let footer =
        row![Space::new().width(Length::Fill), cancel, save].spacing(8).align_y(Alignment::Center);
    let modal = settings_modal_overlay(
        Some(base),
        close,
        settings_modal_layout(header, form, Some(footer.into())).width(660).height(Length::Fill),
    );
    if let Some(chat) = state.recording_hotkey {
        let cancel = msg(Event::RecordHotkey(None));
        settings_modal_overlay(
            Some(modal),
            cancel.clone(),
            settings_modal_layout(
                settings_modal_header(
                    text(if chat {
                        "录制语音聊天快捷键"
                    } else {
                        "录制听写快捷键"
                    })
                    .size(20),
                    settings_close_button(cancel),
                ),
                text("请按修饰键加字母或空格。Escape 取消；不支持 Fn。"),
                Some(action("取消录制".into(), Event::RecordHotkey(None)).into()),
            )
            .width(500),
        )
    } else {
        modal
    }
}
fn voices(options: &[VoiceName], selected: VoiceName) -> Element<'static, Message> {
    row(options.iter().map(|&value| {
        let label = serde_json::to_value(value)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        button(text(label))
            .on_press(msg(Event::Voice(value)))
            .padding([6, 9])
            .style(move |theme, status| {
                settings_segment_button_style(theme, status, selected == value)
            })
            .into()
    }))
    .spacing(5)
    .wrap()
    .into()
}
#[cfg(test)]
#[path = "system_settings_voice_tests.rs"]
mod tests;
