//! macOS controls query OS resources without requesting permissions during rendering.
use super::*;
use crate::app::voice::permissions::{Permission, PermissionStatus};
use iced::widget::column;

pub(super) fn form<'a>(state: &'a State) -> Element<'a, Message> {
    let edit = state.edit.as_ref().expect("open voice edit");
    let resources = &state.resources;
    let mut languages =
        vec![OptionLabel { value: None, label: "系统语言（不是自动检测）".into() }];
    languages.extend(
        resources
            .languages
            .iter()
            .map(|tag| OptionLabel { value: Some(tag.clone()), label: tag.clone() }),
    );
    if !languages.iter().any(|v| v.value == edit.local.local_language) {
        languages.push(OptionLabel {
            value: edit.local.local_language.clone(),
            label: format!(
                "{}（本机不可用，请重选）",
                edit.local.local_language.as_deref().unwrap_or_default()
            ),
        });
    }
    let language = languages.iter().find(|v| v.value == edit.local.local_language).cloned();
    let mut voices = vec![OptionLabel { value: None, label: "系统默认声音".into() }];
    voices.extend(resources.voices.iter().map(|voice| OptionLabel {
        value: Some(voice.id.clone()),
        label: format!("{} · {}", voice.name, voice.language),
    }));
    if !voices.iter().any(|v| v.value == edit.local.local_voice) {
        voices.push(OptionLabel {
            value: edit.local.local_voice.clone(),
            label: "已选声音未安装，请重新选择".into(),
        });
    }
    let voice = voices.iter().find(|v| v.value == edit.local.local_voice).cloned();
    let mut body = column![
        text("使用 macOS 本机识别与朗读，无需语音 API Key。对话识别文字仍交给当前聊天模型。")
            .style(settings_muted_text_style),
        settings_field_label("本机识别语言"),
        settings_select(languages, language, |v| msg(Event::LocalLanguage(v.value))).width(Length::Fill),
        text("列表仅包含系统报告支持本机识别的语言；缺资源时请在 macOS 键盘／听写设置管理，不会联网回退或自动下载。")
            .style(settings_muted_text_style),
        settings_field_label("已安装的 Apple 朗读声音"),
        settings_select(voices, voice, |v| msg(Event::LocalVoice(v.value))).width(Length::Fill),
        text("试听只合成固定短句，不使用麦克风、不要求语音识别授权、不发送模型或工具请求。")
            .style(settings_muted_text_style),
        if state.preview.is_some() { action("停止试听".into(), Event::StopPreview) } else { action("试听本机声音".into(), Event::Preview) },
        text("本机识别当前不支持自定义词典；云端词典保持在原网关，不会应用于本机模式。")
            .style(settings_muted_text_style),
        settings_field_label("系统权限（分别授权）"),
        text(format!("语音识别：{}", permission(resources.speech_permission))),
        action("授权／打开语音识别设置".into(), Event::Permission(Permission::SpeechRecognition)),
        text(format!("麦克风：{}", permission(resources.microphone_permission))),
        action("授权／打开麦克风设置".into(), Event::Permission(Permission::Microphone)),
        action("刷新权限、语言与声音".into(), Event::RefreshLocal),
        text("请本人处理系统弹窗，返回应用后刷新。若开发进程缺用途说明，请使用新版 ShiJu.app。")
            .style(settings_muted_text_style),
    ].spacing(12);
    if let Some(capability) = &resources.capability {
        body = body.push(
            text(format!(
                "系统解析语言：{}；本机识别支持：{}；服务当前可用：{}",
                capability.language,
                if capability.supports_on_device { "是" } else { "否" },
                if capability.available { "是" } else { "否" }
            ))
            .style(settings_muted_text_style),
        );
    }
    if let Some(warning) = &resources.warning {
        body = body.push(text(warning).style(settings_muted_text_style));
    }
    body.into()
}
fn permission(status: Option<PermissionStatus>) -> &'static str {
    match status {
        Some(PermissionStatus::Granted) => "已授权",
        Some(PermissionStatus::Denied) => "已拒绝，请在系统设置中允许",
        Some(PermissionStatus::Restricted) => "被系统限制，请检查设备管理策略",
        Some(PermissionStatus::NotDetermined) => "尚未请求",
        Some(PermissionStatus::Unsupported) => "当前平台不支持",
        _ => "尚未查询，请刷新",
    }
}
