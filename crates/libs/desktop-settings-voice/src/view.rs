//! Native voice settings and their transactional editor.
use config_voice::{RecognitionLanguage, VoiceName};
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::*;
use desktop_voice::{
    hotkeys::{Runtime, Shortcut},
    preferences::{Edit, VoiceEngine},
};
use iced::widget::{Space, button, column, row, text};
use iced::{Alignment, Element, Length};

use crate::{Event, State};

#[path = "system_settings_voice_local.rs"]
mod local;

fn action(label: &str, event: Event) -> iced::widget::Button<'_, Event> {
    button(text(label).size(14)).on_press(event).style(rounded_action_btn_style).padding([8, 14])
}

fn field<'a>(
    label: &'a str,
    description: &'a str,
    control: impl Into<Element<'a, Event>>,
) -> Element<'a, Event> {
    settings_flexible_row(label, description, 248.0, control).padding([12, 0]).into()
}

fn group<'a>(title: &'a str, rows: impl Into<Element<'a, Event>>) -> Element<'a, Event> {
    column![text(title).size(17), settings_group_card(rows)].spacing(8).into()
}

pub fn view<'a>(state: &'a State, i18n: I18n, hotkeys: &Runtime) -> Element<'a, Event> {
    let configure_label = if state.busy { "正在读取…" } else { "编辑语音设置" };
    let configure = button(text(i18n.__(configure_label)).size(14))
        .padding([8, 14])
        .style(primary_action_btn_style)
        .on_press_maybe((!state.busy).then_some(Event::Open));
    let mut page = column![
        settings_page_heading(i18n.__("语音设置"), i18n.__("管理听写、语音聊天与本机录音。")),
        group(
            i18n.__("设备与服务"),
            field(
                i18n.__("麦克风与语音引擎"),
                i18n.__("选择输入设备、识别语言和朗读声音。"),
                configure,
            ),
        ),
    ]
    .spacing(20)
    .width(Length::Fill);

    if cfg!(target_os = "macos") {
        let registered = hotkeys.registered_config();
        let status = if registered.is_some() { i18n.__("已启用") } else { i18n.__("已关闭") };
        let mut controls = row![settings_value_badge(status)].spacing(8).align_y(Alignment::Center);
        if registered.is_some() || hotkeys.error.is_some() {
            controls = controls.push(action(i18n.__("停用快捷键"), Event::DisableHotkeys));
        }
        let mut shortcuts = column![field(
            i18n.__("全局语音快捷键"),
            if registered.is_some() {
                ""
            } else {
                i18n.__("在语音设置中录制快捷键，保存后生效。")
            },
            controls.wrap().align_x(Alignment::End),
        )];
        if let Some(config) = registered {
            for (label, shortcut) in [("听写", config.dictation), ("语音聊天", config.chat)] {
                if let Some(shortcut) = shortcut {
                    shortcuts = shortcuts.push(settings_divider()).push(field(
                        i18n.__(label),
                        "",
                        settings_value_badge(shortcut.label()),
                    ));
                }
            }
        }
        if let Some(error) = &hotkeys.error {
            shortcuts =
                shortcuts.push(settings_error_banner(desktop_i18n::settings::message(i18n, error)));
        }
        page = page.push(group(
            i18n.__("听写与快捷键"),
            shortcuts.push(settings_divider()).push(field(
                i18n.__("跨应用听写"),
                i18n.__("用听写快捷键在其他应用的文本框中输入文字。"),
                action(i18n.__("权限与试用"), Event::Onboarding),
            )),
        ));
    }
    page.into()
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

pub fn modal<'a, Message: Clone + 'a>(
    state: &'a State,
    i18n: I18n,
    base: Element<'a, Message>,
    map: fn(Event) -> Message,
) -> Element<'a, Message> {
    let Some(edit) = &state.edit else {
        return base;
    };
    let form = if state.busy {
        text(i18n.__("正在读取或保存设置…")).style(settings_muted_text_style).into()
    } else {
        settings_form(state, edit, i18n)
    };
    let close = if state.busy { Event::Noop } else { Event::Cancel };
    let header = settings_modal_header(
        text(i18n.__("编辑语音设置")).size(20),
        settings_close_button(close.clone()),
    );
    let save = button(text(i18n.__("保存")).size(14))
        .style(primary_action_btn_style)
        .padding([8, 14])
        .on_press_maybe((!state.busy).then_some(Event::Save));
    let cancel = action(i18n.__("取消"), Event::Cancel)
        .on_press_maybe((!state.busy).then_some(Event::Cancel));
    let footer =
        row![Space::new().width(Length::Fill), cancel, save].spacing(8).align_y(Alignment::Center);
    let card: Element<'_, Event> = settings_modal_layout_with_scroll_id(
        header,
        form,
        Some(footer.into()),
        Some("voice-settings-body"),
    )
    .width(660)
    .height(Length::Fill)
    .into();
    let modal = settings_modal_overlay(Some(base), map(close), card.map(map));
    if let Some(chat) = state.recording_hotkey {
        let cancel = Event::RecordHotkey(None);
        let card: Element<'_, Event> = settings_modal_card(
            column![
                settings_modal_header(
                    text(if chat {
                        i18n.__("录制语音聊天快捷键")
                    } else {
                        i18n.__("录制听写快捷键")
                    })
                    .size(20),
                    settings_close_button(cancel.clone()),
                ),
                text(i18n.__("请使用 Control、Option 或 Command 加字母/空格；不支持 Fn。"))
                    .style(settings_muted_text_style),
                action(i18n.__("取消录制"), cancel.clone()),
            ]
            .spacing(16)
            .width(Length::Fill),
        )
        .width(460)
        .into();
        settings_modal_overlay(Some(modal), map(cancel), card.map(map))
    } else {
        modal
    }
}

fn settings_form<'a>(state: &'a State, edit: &'a Edit, i18n: I18n) -> Element<'a, Event> {
    let mut devices =
        vec![OptionLabel { value: None, label: i18n.__("系统默认麦克风").to_owned() }];
    devices.extend(
        state.devices.iter().map(|device| OptionLabel {
            value: Some(device.id.clone()),
            label: device.name.clone(),
        }),
    );
    let selected = devices
        .iter()
        .find(|device| device.value == edit.local.microphone)
        .cloned()
        .unwrap_or_else(|| {
            let entry = OptionLabel {
                value: edit.local.microphone.clone(),
                label: i18n.__("已选麦克风不可用，请重新选择").to_owned(),
            };
            devices.push(entry.clone());
            entry
        });
    let mut engines =
        vec![OptionLabel { value: VoiceEngine::Cloud, label: i18n.__("云端语音").into() }];
    if cfg!(target_os = "macos") {
        engines.insert(
            0,
            OptionLabel { value: VoiceEngine::Local, label: i18n.__("macOS 本机").into() },
        );
    }
    let engine = engines.iter().find(|option| option.value == edit.local.engine).cloned();
    let mut general = column![
        field(
            i18n.__("麦克风"),
            "",
            settings_select(devices, Some(selected), |option| Event::Microphone(option.value))
                .width(Length::Fill),
        ),
        settings_divider(),
        field(
            i18n.__("语音引擎"),
            if edit.local.engine == VoiceEngine::Local {
                i18n.__("macOS 负责识别和朗读。语音聊天仍使用当前会话的模型。")
            } else {
                i18n.__("通过当前网关调用云端语音服务。")
            },
            settings_select(engines, engine, |option| Event::Engine(option.value))
                .width(Length::Fill),
        ),
    ];
    if edit.local.engine == VoiceEngine::Local {
        general = general.push(settings_divider()).push(local::fields(i18n, state, edit));
    }
    let mut form = column![group(i18n.__("设备与服务"), general)].spacing(18);
    form = if edit.local.engine == VoiceEngine::Local {
        form.push(local::permissions(i18n, state))
    } else {
        form.push(cloud_form(state, edit, i18n))
    };
    if cfg!(target_os = "macos") {
        form = form.push(hotkey_form(edit, i18n));
    }
    form = form.push(group(
        i18n.__("最近本机录音"),
        field(
            i18n.__("保存听写录音"),
            i18n.__("仅在本机保存最近 20 段成功听写，最多 64 MiB。关闭后停止保存新录音。"),
            settings_toggle(edit.local.retain_recordings).on_toggle(Event::RetainRecordings),
        ),
    ));
    if edit.local.engine == VoiceEngine::Cloud && state.cloud_ready {
        form = form.push(dictionary_form(state, edit, i18n));
    }
    form.into()
}

fn hotkey_form(edit: &Edit, i18n: I18n) -> Element<'_, Event> {
    let config = &edit.local.hotkeys;
    group(
        i18n.__("听写与快捷键"),
        column![
            field(
                i18n.__("启用全局快捷键"),
                i18n.__("至少录制一个快捷键，保存后生效。"),
                settings_toggle(config.enabled).on_toggle(Event::HotkeysEnabled),
            ),
            settings_divider(),
            field(
                i18n.__("听写"),
                i18n.__("按住说话，松开完成。双击进入免提，再按一次结束。"),
                shortcut_control(config.dictation, false, i18n),
            ),
            settings_divider(),
            field(i18n.__("语音聊天"), "", shortcut_control(config.chat, true, i18n)),
            settings_divider(),
            field(
                i18n.__("输入监控"),
                i18n.__("全局快捷键需要 macOS 输入监控权限。"),
                action(i18n.__("检查 / 授权"), Event::HotkeyPermission),
            ),
        ],
    )
}

fn shortcut_control(shortcut: Option<Shortcut>, chat: bool, i18n: I18n) -> Element<'static, Event> {
    let label = shortcut.map(Shortcut::label).unwrap_or_else(|| i18n.__("未设置").into());
    let mut controls = row![
        settings_value_badge(label),
        action(i18n.__("录制快捷键"), Event::RecordHotkey(Some(chat))),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if shortcut.is_some() {
        controls = controls.push(action(i18n.__("移除"), Event::ClearHotkey(chat)));
    }
    controls.wrap().align_x(Alignment::End).into()
}

fn preview_button(state: &State, i18n: I18n) -> iced::widget::Button<'static, Event> {
    if state.preview.is_some() {
        action(i18n.__("停止试听"), Event::StopPreview)
    } else {
        action(i18n.__("试听音色"), Event::Preview)
    }
}

fn cloud_form<'a>(state: &'a State, edit: &'a Edit, i18n: I18n) -> Element<'a, Event> {
    if !state.cloud_ready {
        let mut rows = column![field(
            i18n.__("云端语音设置未加载"),
            i18n.__("检查网关连接与配对后重试。"),
            action(i18n.__("重新读取"), Event::Engine(VoiceEngine::Cloud)),
        )];
        if let Some(error) = &state.cloud_error {
            rows = rows.push(settings_error_banner(desktop_i18n::settings::message(i18n, error)));
        }
        return group(i18n.__("云端语音"), rows);
    }
    let languages = [
        (RecognitionLanguage::Auto, "自动检测"),
        (RecognitionLanguage::Zh, "中文"),
        (RecognitionLanguage::En, "英语"),
        (RecognitionLanguage::Es, "西班牙语"),
        (RecognitionLanguage::Fr, "法语"),
    ]
    .map(|(value, label)| OptionLabel { value, label: i18n.__(label).to_owned() });
    let selected_language =
        languages.iter().find(|option| option.value == edit.config.language).cloned();
    let language =
        settings_select(languages, selected_language, |option| Event::Language(option.value))
            .width(Length::Fill);
    let voices = [
        VoiceName::Marin,
        VoiceName::Cedar,
        VoiceName::Alloy,
        VoiceName::Ash,
        VoiceName::Ballad,
        VoiceName::Coral,
        VoiceName::Echo,
        VoiceName::Sage,
        VoiceName::Shimmer,
        VoiceName::Verse,
    ]
    .map(|value| OptionLabel {
        value,
        label: serde_json::to_value(value)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default(),
    });
    let selected_voice = voices.iter().find(|option| option.value == edit.config.voice).cloned();
    let provider = edit.config.provider_id.as_deref().unwrap_or("");
    group(
        i18n.__("云端语音"),
        column![
            field(i18n.__("识别语言"), "", language),
            settings_divider(),
            field(
                i18n.__("朗读声音"),
                i18n.__("音色只影响新语音聊天。试听使用当前网关服务，不录音。"),
                column![
                    settings_select(voices, selected_voice, |option| Event::Voice(option.value))
                        .width(Length::Fill),
                    preview_button(state, i18n),
                ]
                .spacing(6)
                .align_x(Alignment::End),
            ),
            settings_divider(),
            field(
                i18n.__("网关提供商 ID"),
                i18n.__("服务凭据请在提供商设置中配置。"),
                settings_text_input("openai", provider).on_input(Event::Provider),
            ),
            settings_divider(),
            field(
                i18n.__("启用听写"),
                "",
                settings_toggle(edit.config.dictation_enabled).on_toggle(Event::Dictation),
            ),
            settings_divider(),
            field(
                i18n.__("启用语音聊天"),
                "",
                settings_toggle(edit.config.realtime_enabled).on_toggle(Event::Realtime),
            ),
        ],
    )
}

fn dictionary_form<'a>(state: &'a State, edit: &'a Edit, i18n: I18n) -> Element<'a, Event> {
    let mut dictionary = column![
        row![
            settings_text_input(i18n.__("添加专有名词"), &state.dictionary)
                .on_input(Event::Dictionary)
                .on_submit(Event::AddWord),
            action(i18n.__("添加"), Event::AddWord),
        ]
        .align_y(Alignment::Center)
        .spacing(8)
    ]
    .padding([12, 0])
    .spacing(8);
    for (index, word) in edit.config.dictionary.iter().enumerate() {
        dictionary = dictionary.push(
            row![
                text(word).size(14).width(Length::Fill),
                action(i18n.__("删除"), Event::RemoveWord(index)),
            ]
            .align_y(Alignment::Center)
            .spacing(8),
        );
    }
    group(i18n.__("识别词典"), dictionary)
}

#[cfg(test)]
#[path = "system_settings_voice_tests.rs"]
mod tests;
