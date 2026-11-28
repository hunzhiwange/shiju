//! macOS controls query OS resources without requesting permissions during rendering.
use super::*;
use desktop_voice::permissions::{Permission, PermissionStatus};
use iced::widget::column;

pub(super) fn fields<'a>(i18n: I18n, state: &'a State, edit: &'a Edit) -> Element<'a, Event> {
    let resources = &state.resources;
    let mut languages = vec![OptionLabel { value: None, label: i18n.__("系统语言").into() }];
    languages.extend(
        resources
            .languages
            .iter()
            .map(|tag| OptionLabel { value: Some(tag.clone()), label: tag.clone() }),
    );
    if !languages.iter().any(|option| option.value == edit.local.local_language) {
        languages.push(OptionLabel {
            value: edit.local.local_language.clone(),
            label: desktop_i18n::__!(
                i18n,
                "%s（本机不可用，请重选）",
                edit.local.local_language.as_deref().unwrap_or_default()
            )
            .expect("valid language label"),
        });
    }
    let language =
        languages.iter().find(|option| option.value == edit.local.local_language).cloned();
    let mut voices = vec![OptionLabel { value: None, label: i18n.__("系统默认声音").into() }];
    voices.extend(resources.voices.iter().map(|voice| OptionLabel {
        value: Some(voice.id.clone()),
        label: format!("{} · {}", voice.name, voice.language),
    }));
    if !voices.iter().any(|option| option.value == edit.local.local_voice) {
        voices.push(OptionLabel {
            value: edit.local.local_voice.clone(),
            label: i18n.__("已选声音未安装，请重新选择").into(),
        });
    }
    let voice = voices.iter().find(|option| option.value == edit.local.local_voice).cloned();
    let mut fields = column![
        field(
            i18n.__("识别语言"),
            i18n.__("选择系统支持的本机识别语言。"),
            settings_select(languages, language, |option| Event::LocalLanguage(option.value))
                .width(Length::Fill),
        ),
        settings_divider(),
        field(
            i18n.__("朗读声音"),
            i18n.__("使用 macOS 上已安装的声音。"),
            column![
                settings_select(voices, voice, |option| Event::LocalVoice(option.value))
                    .width(Length::Fill),
                preview_button(state, i18n),
            ]
            .spacing(6)
            .align_x(Alignment::End),
        ),
    ];
    if let Some(warning) = &resources.warning {
        fields = fields.push(settings_error_banner(desktop_i18n::settings::message(i18n, warning)));
    } else if let Some(capability) = &resources.capability
        && (!capability.available || !capability.supports_on_device)
    {
        fields = fields.push(settings_error_banner(
            i18n.__("当前语言的本机识别不可用，请检查 macOS 听写设置或更换语言。"),
        ));
    }
    fields.into()
}

pub(super) fn permissions(i18n: I18n, state: &State) -> Element<'static, Event> {
    let resources = &state.resources;
    group(
        i18n.__("系统权限"),
        column![
            permission_field(
                i18n,
                "麦克风",
                Permission::Microphone,
                resources.microphone_permission
            ),
            settings_divider(),
            permission_field(
                i18n,
                "语音识别",
                Permission::SpeechRecognition,
                resources.speech_permission
            ),
            settings_divider(),
            field(
                i18n.__("本机语音资源"),
                "",
                action(i18n.__("刷新权限、语言与声音"), Event::RefreshLocal),
            ),
        ],
    )
}

fn permission_field(
    i18n: I18n,
    label: &'static str,
    permission: Permission,
    status: Option<PermissionStatus>,
) -> Element<'static, Event> {
    let description = match status {
        Some(PermissionStatus::Granted) => "已授权",
        Some(PermissionStatus::Denied) => "已拒绝，请在系统设置中允许",
        Some(PermissionStatus::Restricted) => "被系统限制，请检查设备管理策略",
        Some(PermissionStatus::NotDetermined) => "尚未请求",
        Some(PermissionStatus::Unsupported) => "当前平台不支持",
        _ => "尚未查询，请刷新",
    };
    let control: Element<'static, Event> = match status {
        Some(
            PermissionStatus::Granted
            | PermissionStatus::Restricted
            | PermissionStatus::Unsupported,
        ) => Space::new().into(),
        _ => action(i18n.__("检查 / 授权"), Event::Permission(permission)).into(),
    };
    field(i18n.__(label), i18n.__(description), control)
}
