//! macOS controls query OS resources without requesting permissions during rendering.
use super::*;
use desktop_voice::permissions::{Permission, PermissionStatus};
use iced::widget::column;

pub(super) fn form<'a>(i18n: desktop_i18n::I18n, state: &'a State) -> Element<'a, Message> {
    let edit = state.edit.as_ref().expect("open voice edit");
    let resources = &state.resources;
    let mut languages =
        vec![OptionLabel { value: None, label: i18n.__("系统语言（不是自动检测）").into() }];
    languages.extend(
        resources
            .languages
            .iter()
            .map(|tag| OptionLabel { value: Some(tag.clone()), label: tag.clone() }),
    );
    if !languages.iter().any(|v| v.value == edit.local.local_language) {
        languages.push(OptionLabel {
            value: edit.local.local_language.clone(),
            label: desktop_i18n::__!(i18n, "%s（本机不可用，请重选）", (edit.local.local_language.as_deref().unwrap_or_default()).to_string()).expect("valid settings message"),
        });
    }
    let language = languages.iter().find(|v| v.value == edit.local.local_language).cloned();
    let mut voices = vec![OptionLabel { value: None, label: i18n.__("系统默认声音").into() }];
    voices.extend(resources.voices.iter().map(|voice| OptionLabel {
        value: Some(voice.id.clone()),
        label: format!("{} · {}", voice.name, voice.language),
    }));
    if !voices.iter().any(|v| v.value == edit.local.local_voice) {
        voices.push(OptionLabel {
            value: edit.local.local_voice.clone(),
            label: i18n.__("已选声音未安装，请重新选择").into(),
        });
    }
    let voice = voices.iter().find(|v| v.value == edit.local.local_voice).cloned();
    let mut body = column![
        text(i18n.__("使用 macOS 本机识别与朗读，无需语音 API Key。对话识别文字仍交给当前聊天模型。"))
            .style(settings_muted_text_style),
        settings_field_label(i18n.__("本机识别语言")),
        settings_select(languages, language, |v| msg(Event::LocalLanguage(v.value))).width(Length::Fill),
        text(i18n.__("列表仅包含系统报告支持本机识别的语言；缺资源时请在 macOS 键盘／听写设置管理，不会联网回退或自动下载。"))
            .style(settings_muted_text_style),
        settings_field_label(i18n.__("已安装的 Apple 朗读声音")),
        settings_select(voices, voice, |v| msg(Event::LocalVoice(v.value))).width(Length::Fill),
        text(i18n.__("试听只合成固定短句，不使用麦克风、不要求语音识别授权、不发送模型或工具请求。"))
            .style(settings_muted_text_style),
        if state.preview.is_some() { action(i18n.__("停止试听").into(), Event::StopPreview) } else { action(i18n.__("试听本机声音").into(), Event::Preview) },
        text(i18n.__("本机识别当前不支持自定义词典；云端词典保持在原网关，不会应用于本机模式。"))
            .style(settings_muted_text_style),
        settings_field_label(i18n.__("系统权限（分别授权）")),
        text(desktop_i18n::__!(i18n, "语音识别：%s", (permission(i18n, resources.speech_permission)).to_string()).expect("valid settings message")),
        action(i18n.__("授权／打开语音识别设置").into(), Event::Permission(Permission::SpeechRecognition)),
        text(desktop_i18n::__!(i18n, "麦克风：%s", (permission(i18n, resources.microphone_permission)).to_string()).expect("valid settings message")),
        action(i18n.__("授权／打开麦克风设置").into(), Event::Permission(Permission::Microphone)),
        action(i18n.__("刷新权限、语言与声音").into(), Event::RefreshLocal),
        text(i18n.__("请本人处理系统弹窗，返回应用后刷新。若开发进程缺用途说明，请使用新版 ShiJu.app。"))
            .style(settings_muted_text_style),
    ].spacing(12);
    if let Some(capability) = &resources.capability {
        body = body.push(
            text(desktop_i18n::__!(i18n, "系统解析语言：%s；本机识别支持：%s；服务当前可用：%s", (capability.language).to_string(), i18n.__(if capability.supports_on_device { "是" } else { "否" }), i18n.__(if capability.available { "是" } else { "否" })).expect("valid settings message"))
            .style(settings_muted_text_style),
        );
    }
    if let Some(warning) = &resources.warning {
        body = body.push(text(desktop_i18n::settings::message(i18n, warning)).style(settings_muted_text_style));
    }
    body.into()
}
fn permission(i18n: desktop_i18n::I18n, status: Option<PermissionStatus>) -> &'static str {
    match status {
        Some(PermissionStatus::Granted) => i18n.__("已授权"),
        Some(PermissionStatus::Denied) => i18n.__("已拒绝，请在系统设置中允许"),
        Some(PermissionStatus::Restricted) => i18n.__("被系统限制，请检查设备管理策略"),
        Some(PermissionStatus::NotDetermined) => i18n.__("尚未请求"),
        Some(PermissionStatus::Unsupported) => i18n.__("当前平台不支持"),
        _ => i18n.__("尚未查询，请刷新"),
    }
}
