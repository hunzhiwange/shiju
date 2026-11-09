//! 系统设置中 runtime 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use super::system_settings::SELECT_FOCUS_SCOPE;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_TEXT_SIZE, settings_checkbox_style, settings_divider, settings_field_label,
    settings_flexible_row_with_help, settings_group_card, settings_group_heading_with_help,
    settings_muted_text_style, settings_page_heading_with_help, settings_row_with_flexible_control,
    settings_select, settings_text_input,
};
use crate::app::i18n::I18n;
use crate::app::i18n::settings_runtime::text as runtime_text;
use crate::app::message::settings::{RuntimeMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{checkbox, column, container, text};
use iced::{Element, Length};

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

const CONTROL_WIDTH: f32 = 280.0;

fn text_row<'a>(
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    settings_row_with_flexible_control(
        label,
        description,
        CONTROL_WIDTH,
        settings_text_input(placeholder, value).on_input(on_input).width(Length::Fill),
    )
}

/// 渲染运行时类型、推理覆盖和 Docker 配置。
pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.runtime_settings;
    let locale = app.desktop_locale;
    let i18n = I18n::new(locale);
    let theme = &app.app_theme;

    let kind_pick = settings_select(
        ["native".to_string(), "docker".to_string()],
        Some(s.kind.clone()),
        |value| Message::Settings(SettingsMessage::Runtime(RuntimeMessage::KindChanged(value))),
    )
    .width(Length::Fill);

    let kind_row = settings_flexible_row_with_help(
        i18n,
        theme,
        runtime_text(locale, "运行时类型"),
        runtime_text(locale, "选择 native 或 docker 执行环境。"),
        CONTROL_WIDTH,
        kind_pick,
    );

    let reasoning_enabled_options = [
        LabeledOption { value: "auto", label: runtime_text(locale, "自动") },
        LabeledOption { value: "true", label: runtime_text(locale, "启用") },
        LabeledOption { value: "false", label: runtime_text(locale, "禁用") },
    ];
    let reasoning_enabled_selected = reasoning_enabled_options
        .iter()
        .find(|opt| opt.value == s.reasoning_enabled_input)
        .cloned()
        .or_else(|| reasoning_enabled_options.first().cloned());
    let reasoning_enabled_pick =
        settings_select(reasoning_enabled_options, reasoning_enabled_selected, |value| {
            Message::Settings(SettingsMessage::Runtime(RuntimeMessage::ReasoningEnabledChanged(
                value.value.to_string(),
            )))
        })
        .width(Length::Fill);

    let reasoning_enabled_row = settings_flexible_row_with_help(
        i18n,
        theme,
        runtime_text(locale, "启用推理"),
        runtime_text(locale, "覆盖 runtime.reasoning_enabled 的兼容配置。"),
        CONTROL_WIDTH,
        reasoning_enabled_pick,
    );

    let reasoning_level_options = [
        LabeledOption { value: "", label: runtime_text(locale, "默认") },
        LabeledOption { value: "minimal", label: runtime_text(locale, "最小") },
        LabeledOption { value: "low", label: runtime_text(locale, "低") },
        LabeledOption { value: "medium", label: runtime_text(locale, "中") },
        LabeledOption { value: "high", label: runtime_text(locale, "高") },
        LabeledOption { value: "xhigh", label: runtime_text(locale, "超高") },
    ];
    let reasoning_level_selected = reasoning_level_options
        .iter()
        .find(|opt| opt.value == s.reasoning_level_input)
        .cloned()
        .or_else(|| reasoning_level_options.first().cloned());
    let reasoning_level_pick =
        settings_select(reasoning_level_options, reasoning_level_selected, |value| {
            Message::Settings(SettingsMessage::Runtime(RuntimeMessage::ReasoningLevelChanged(
                value.value.to_string(),
            )))
        })
        .width(Length::Fill);

    let reasoning_level_row = settings_flexible_row_with_help(
        i18n,
        theme,
        runtime_text(locale, "推理级别"),
        runtime_text(locale, "兼容别名 runtime.reasoning_level。"),
        CONTROL_WIDTH,
        reasoning_level_pick,
    );

    let docker_section = column![
        settings_group_heading_with_help(
            i18n,
            theme,
            runtime_text(locale, "Docker 配置"),
            runtime_text(locale, "当 kind=docker 时生效，用于控制镜像、资源限制和工作区挂载。"),
        ),
        settings_group_card(
            column![
                settings_flexible_row_with_help(
                    i18n,
                    theme,
                    runtime_text(locale, "镜像 (image)"),
                    runtime_text(locale, "默认执行镜像。"),
                    CONTROL_WIDTH,
                    settings_text_input("alpine:3.20", &s.docker_image)
                        .on_input(|value| {
                            Message::Settings(SettingsMessage::Runtime(
                                RuntimeMessage::DockerImageChanged(value),
                            ))
                        })
                        .width(Length::Fill),
                ),
                settings_divider(),
                settings_flexible_row_with_help(
                    i18n,
                    theme,
                    runtime_text(locale, "网络模式 (network)"),
                    runtime_text(locale, "控制容器的网络模式。"),
                    CONTROL_WIDTH,
                    settings_text_input("none / bridge / host", &s.docker_network)
                        .on_input(|value| {
                            Message::Settings(SettingsMessage::Runtime(
                                RuntimeMessage::DockerNetworkChanged(value),
                            ))
                        })
                        .width(Length::Fill),
                ),
                settings_divider(),
                text_row(
                    runtime_text(locale, "内存限制 MB (memory_limit_mb)"),
                    runtime_text(locale, "留空表示不限制。"),
                    runtime_text(locale, "留空表示不限制"),
                    &s.docker_memory_limit_mb_input,
                    |value| {
                        Message::Settings(SettingsMessage::Runtime(
                            RuntimeMessage::DockerMemoryLimitMbChanged(value),
                        ))
                    },
                ),
                settings_divider(),
                text_row(
                    runtime_text(locale, "CPU 限制 (cpu_limit)"),
                    runtime_text(locale, "例如 1 / 1.5 / 2。"),
                    runtime_text(locale, "例如 1 / 1.5 / 2"),
                    &s.docker_cpu_limit_input,
                    |value| {
                        Message::Settings(SettingsMessage::Runtime(
                            RuntimeMessage::DockerCpuLimitChanged(value),
                        ))
                    }
                ),
                settings_divider(),
                settings_flexible_row_with_help(
                    i18n,
                    theme,
                    runtime_text(locale, "只读根文件系统 (read_only_rootfs)"),
                    runtime_text(locale, "限制容器根文件系统为只读。"),
                    CONTROL_WIDTH,
                    checkbox(s.docker_read_only_rootfs)
                        .label(runtime_text(locale, "启用"))
                        .text_size(SETTINGS_CONTROL_TEXT_SIZE)
                        .on_toggle(|value| {
                            Message::Settings(SettingsMessage::Runtime(
                                RuntimeMessage::DockerReadOnlyRootfsToggled(value),
                            ))
                        })
                        .style(settings_checkbox_style),
                ),
                settings_divider(),
                settings_flexible_row_with_help(
                    i18n,
                    theme,
                    runtime_text(locale, "挂载工作区 (mount_workspace)"),
                    runtime_text(locale, "决定是否把工作区挂载进容器。"),
                    CONTROL_WIDTH,
                    checkbox(s.docker_mount_workspace)
                        .label(runtime_text(locale, "启用"))
                        .text_size(SETTINGS_CONTROL_TEXT_SIZE)
                        .on_toggle(|value| {
                            Message::Settings(SettingsMessage::Runtime(
                                RuntimeMessage::DockerMountWorkspaceToggled(value),
                            ))
                        })
                        .style(settings_checkbox_style),
                ),
                settings_divider(),
                container(
                    column![
                        settings_field_label(runtime_text(
                            locale,
                            "允许的工作区根目录 (allowed_workspace_roots)",
                        )),
                        text(runtime_text(locale, "仅放行明确需要挂载的根目录。"))
                            .size(13)
                            .style(settings_muted_text_style),
                        settings_text_input(
                            runtime_text(locale, "逗号或换行分隔路径"),
                            &s.docker_allowed_workspace_roots_input,
                        )
                        .on_input(|value| Message::Settings(SettingsMessage::Runtime(
                            RuntimeMessage::DockerAllowedWorkspaceRootsChanged(value),
                        )))
                        .width(Length::Fill),
                    ]
                    .spacing(6),
                )
                .padding([20, 0]),
            ]
            .spacing(0)
        ),
    ]
    .spacing(16);

    let mut content = column![
        settings_page_heading_with_help(
            i18n,
            theme,
            runtime_text(locale, "运行时配置"),
            runtime_text(locale, "配置 native、docker 执行环境及推理覆盖项。")
        ),
        settings_group_heading_with_help(
            i18n,
            theme,
            runtime_text(locale, "基础行为"),
            runtime_text(locale, "运行时类型。")
        ),
        settings_group_card(kind_row),
        settings_group_heading_with_help(
            i18n,
            theme,
            runtime_text(locale, "推理覆盖"),
            runtime_text(
                locale,
                "该区域用于配置 runtime.reasoning_enabled 与兼容别名 runtime.reasoning_level。"
            ),
        ),
        settings_group_card(
            column![reasoning_enabled_row, settings_divider(), reasoning_level_row].spacing(0),
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    if s.kind == "docker" {
        content = content.push(docker_section);
    }
    container(content).id(SELECT_FOCUS_SCOPE).width(Length::Fill).into()
}
#[cfg(test)]
#[path = "system_settings_runtime_tests.rs"]
mod system_settings_runtime_tests;
