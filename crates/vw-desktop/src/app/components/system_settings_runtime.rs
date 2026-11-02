//! 系统设置中 runtime 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_control_with_hint, settings_divider,
    settings_muted_text_style, settings_page_intro, settings_pick_list_menu_style,
    settings_pick_list_style, settings_section_card, settings_section_content,
    settings_text_input_style,
};
use crate::app::i18n::settings_runtime::{Text, text as runtime_text};
use crate::app::message::settings::{RuntimeMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{checkbox, column, container, pick_list, row, text, text_input};
use iced::{Alignment, Element, Length};

#[derive(Clone, PartialEq)]
struct LabeledOption {
    value: &'static str,
    label: &'static str,
}

impl std::fmt::Display for LabeledOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

fn text_row<'a>(
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        text_input(placeholder, value)
            .on_input(on_input)
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    )
}

fn text_row_with_hint<'a>(
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    hint: &'static str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        settings_control_with_hint(
            text_input(placeholder, value)
                .on_input(on_input)
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style)
                .width(Length::Fill),
            hint,
        ),
    )
}

/// 构建或处理 `view` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.runtime_settings;
    let locale = app.desktop_locale;

    let kind_pick =
        pick_list(["native".to_string(), "docker".to_string()], Some(s.kind.clone()), |value| {
            Message::Settings(SettingsMessage::Runtime(RuntimeMessage::KindChanged(value)))
        })
        .padding([10, 14])
        .text_size(13)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fixed(280.0));

    let kind_row = field_row(
        runtime_text(locale, Text::RuntimeKind),
        runtime_text(locale, Text::EnvironmentHelp),
        kind_pick,
    );

    let reasoning_enabled_options = [
        LabeledOption { value: "auto", label: runtime_text(locale, Text::Auto) },
        LabeledOption { value: "true", label: runtime_text(locale, Text::Enabled) },
        LabeledOption { value: "false", label: runtime_text(locale, Text::Disabled) },
    ];
    let reasoning_enabled_selected = reasoning_enabled_options
        .iter()
        .find(|opt| opt.value == s.reasoning_enabled_input)
        .cloned()
        .or_else(|| reasoning_enabled_options.first().cloned());
    let reasoning_enabled_pick =
        pick_list(reasoning_enabled_options, reasoning_enabled_selected, |value| {
            Message::Settings(SettingsMessage::Runtime(RuntimeMessage::ReasoningEnabledChanged(
                value.value.to_string(),
            )))
        })
        .padding([10, 14])
        .text_size(13)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fixed(280.0));

    let reasoning_enabled_row = field_row(
        runtime_text(locale, Text::EnableReasoning),
        runtime_text(locale, Text::ReasoningHelp),
        reasoning_enabled_pick,
    );

    let reasoning_level_options = [
        LabeledOption { value: "", label: runtime_text(locale, Text::Default) },
        LabeledOption { value: "minimal", label: runtime_text(locale, Text::Minimal) },
        LabeledOption { value: "low", label: runtime_text(locale, Text::Low) },
        LabeledOption { value: "medium", label: runtime_text(locale, Text::Medium) },
        LabeledOption { value: "high", label: runtime_text(locale, Text::High) },
        LabeledOption { value: "xhigh", label: runtime_text(locale, Text::ExtraHigh) },
    ];
    let reasoning_level_selected = reasoning_level_options
        .iter()
        .find(|opt| opt.value == s.reasoning_level_input)
        .cloned()
        .or_else(|| reasoning_level_options.first().cloned());
    let reasoning_level_pick =
        pick_list(reasoning_level_options, reasoning_level_selected, |value| {
            Message::Settings(SettingsMessage::Runtime(RuntimeMessage::ReasoningLevelChanged(
                value.value.to_string(),
            )))
        })
        .padding([10, 14])
        .text_size(13)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fixed(280.0));

    let reasoning_level_row = field_row(
        runtime_text(locale, Text::ReasoningLevel),
        runtime_text(locale, Text::LevelHelp),
        reasoning_level_pick,
    );

    let docker_section = column![
        settings_section_card(
            runtime_text(locale, Text::DockerConfig),
            runtime_text(locale, Text::DockerHelp),
        ),
        settings_section_content(
            column![
                text_row(
                    runtime_text(locale, Text::Image),
                    runtime_text(locale, Text::ImageHelp),
                    "alpine:3.20",
                    &s.docker_image,
                    |value| {
                        Message::Settings(SettingsMessage::Runtime(
                            RuntimeMessage::DockerImageChanged(value),
                        ))
                    }
                ),
                settings_divider(),
                text_row(
                    runtime_text(locale, Text::Network),
                    runtime_text(locale, Text::NetworkHelp),
                    "none / bridge / host",
                    &s.docker_network,
                    |value| {
                        Message::Settings(SettingsMessage::Runtime(
                            RuntimeMessage::DockerNetworkChanged(value),
                        ))
                    }
                ),
                settings_divider(),
                text_row(
                    runtime_text(locale, Text::MemoryLimit),
                    runtime_text(locale, Text::UnlimitedHelp),
                    runtime_text(locale, Text::Unlimited),
                    &s.docker_memory_limit_mb_input,
                    |value| {
                        Message::Settings(SettingsMessage::Runtime(
                            RuntimeMessage::DockerMemoryLimitMbChanged(value),
                        ))
                    },
                ),
                settings_divider(),
                text_row(
                    runtime_text(locale, Text::CpuLimit),
                    runtime_text(locale, Text::CpuExampleHelp),
                    runtime_text(locale, Text::CpuExample),
                    &s.docker_cpu_limit_input,
                    |value| {
                        Message::Settings(SettingsMessage::Runtime(
                            RuntimeMessage::DockerCpuLimitChanged(value),
                        ))
                    }
                ),
                settings_divider(),
                field_row(
                    runtime_text(locale, Text::ReadOnly),
                    runtime_text(locale, Text::ReadOnlyHelp),
                    checkbox(s.docker_read_only_rootfs)
                        .label(runtime_text(locale, Text::Enabled))
                        .on_toggle(|value| {
                            Message::Settings(SettingsMessage::Runtime(
                                RuntimeMessage::DockerReadOnlyRootfsToggled(value),
                            ))
                        })
                        .style(settings_checkbox_style),
                ),
                settings_divider(),
                field_row(
                    runtime_text(locale, Text::Mount),
                    runtime_text(locale, Text::MountHelp),
                    checkbox(s.docker_mount_workspace)
                        .label(runtime_text(locale, Text::Enabled))
                        .on_toggle(|value| {
                            Message::Settings(SettingsMessage::Runtime(
                                RuntimeMessage::DockerMountWorkspaceToggled(value),
                            ))
                        })
                        .style(settings_checkbox_style),
                ),
                settings_divider(),
                text_row_with_hint(
                    runtime_text(locale, Text::AllowedRoots),
                    runtime_text(locale, Text::RootsHelp),
                    runtime_text(locale, Text::PathsHint),
                    &s.docker_allowed_workspace_roots_input,
                    runtime_text(locale, Text::RootsHint),
                    |value| {
                        Message::Settings(SettingsMessage::Runtime(
                            RuntimeMessage::DockerAllowedWorkspaceRootsChanged(value),
                        ))
                    },
                ),
            ]
            .spacing(0)
        ),
    ]
    .spacing(16);

    let mut content = column![
        settings_page_intro(
            runtime_text(locale, Text::RuntimeConfig),
            runtime_text(locale, Text::RuntimeIntro)
        ),
        settings_section_card(
            runtime_text(locale, Text::Basic),
            runtime_text(locale, Text::KindHelp)
        ),
        settings_section_content(column![kind_row].spacing(0)),
        settings_section_card(
            runtime_text(locale, Text::Overrides),
            runtime_text(locale, Text::OverridesHelp),
        ),
        settings_section_content(
            column![reasoning_enabled_row, settings_divider(), reasoning_level_row].spacing(0),
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    if s.kind == "docker" {
        content = content.push(docker_section);
    }
    container(content).width(Length::Fill).into()
}
#[cfg(test)]
#[path = "system_settings_runtime_tests.rs"]
mod system_settings_runtime_tests;
