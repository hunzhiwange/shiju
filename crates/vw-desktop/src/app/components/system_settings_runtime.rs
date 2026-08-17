//! 系统设置中 runtime 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_control_with_hint, settings_divider,
    settings_muted_text_style, settings_page_intro, settings_panel, settings_pick_list_menu_style,
    settings_pick_list_style, settings_section_card, settings_text_input_style,
};
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

    let kind_pick =
        pick_list(["native".to_string(), "docker".to_string()], Some(s.kind.clone()), |value| {
            Message::Settings(SettingsMessage::Runtime(RuntimeMessage::KindChanged(value)))
        })
        .padding([10, 14])
        .text_size(13)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fixed(280.0));

    let kind_row = field_row("运行时类型", "选择 native 或 docker 执行环境。", kind_pick);

    let reasoning_enabled_options = [
        LabeledOption { value: "auto", label: "自动" },
        LabeledOption { value: "true", label: "启用" },
        LabeledOption { value: "false", label: "禁用" },
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
        "启用推理",
        "覆盖 runtime.reasoning_enabled 的兼容配置。",
        reasoning_enabled_pick,
    );

    let reasoning_level_options = [
        LabeledOption { value: "", label: "默认" },
        LabeledOption { value: "minimal", label: "最小" },
        LabeledOption { value: "low", label: "低" },
        LabeledOption { value: "medium", label: "中" },
        LabeledOption { value: "high", label: "高" },
        LabeledOption { value: "xhigh", label: "超高" },
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

    let reasoning_level_row =
        field_row("推理级别", "兼容别名 runtime.reasoning_level。", reasoning_level_pick);

    let docker_section = column![
        settings_section_card(
            "Docker 配置",
            "当 kind=docker 时生效，用于控制镜像、资源限制和工作区挂载。",
        ),
        settings_panel(
            column![
                text_row(
                    "镜像 (image)",
                    "默认执行镜像。",
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
                    "网络模式 (network)",
                    "控制容器的网络模式。",
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
                    "内存限制 MB (memory_limit_mb)",
                    "留空表示不限制。",
                    "留空表示不限制",
                    &s.docker_memory_limit_mb_input,
                    |value| {
                        Message::Settings(SettingsMessage::Runtime(
                            RuntimeMessage::DockerMemoryLimitMbChanged(value),
                        ))
                    },
                ),
                settings_divider(),
                text_row(
                    "CPU 限制 (cpu_limit)",
                    "例如 1 / 1.5 / 2。",
                    "例如 1 / 1.5 / 2",
                    &s.docker_cpu_limit_input,
                    |value| {
                        Message::Settings(SettingsMessage::Runtime(
                            RuntimeMessage::DockerCpuLimitChanged(value),
                        ))
                    }
                ),
                settings_divider(),
                field_row(
                    "只读根文件系统 (read_only_rootfs)",
                    "限制容器根文件系统为只读。",
                    checkbox(s.docker_read_only_rootfs)
                        .label("启用")
                        .on_toggle(|value| {
                            Message::Settings(SettingsMessage::Runtime(
                                RuntimeMessage::DockerReadOnlyRootfsToggled(value),
                            ))
                        })
                        .style(settings_checkbox_style),
                ),
                settings_divider(),
                field_row(
                    "挂载工作区 (mount_workspace)",
                    "决定是否把工作区挂载进容器。",
                    checkbox(s.docker_mount_workspace)
                        .label("启用")
                        .on_toggle(|value| {
                            Message::Settings(SettingsMessage::Runtime(
                                RuntimeMessage::DockerMountWorkspaceToggled(value),
                            ))
                        })
                        .style(settings_checkbox_style),
                ),
                settings_divider(),
                text_row_with_hint(
                    "允许的工作区根目录 (allowed_workspace_roots)",
                    "仅放行明确需要挂载的根目录。",
                    "逗号或换行分隔路径",
                    &s.docker_allowed_workspace_roots_input,
                    "建议仅放行明确需要挂载的根目录。",
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
        settings_page_intro("运行时配置", "配置 native、docker 执行环境及推理覆盖项。"),
        settings_section_card("基础行为", "运行时类型。"),
        settings_panel(column![kind_row].spacing(0)),
        settings_section_card(
            "推理覆盖",
            "该区域用于配置 runtime.reasoning_enabled 与兼容别名 runtime.reasoning_level。",
        ),
        settings_panel(
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
