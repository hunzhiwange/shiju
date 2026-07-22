//! 系统设置中 sop 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::number_input::NumberInput;
use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, rounded_action_btn_style, settings_divider, settings_muted_text_style,
    settings_page_intro, settings_panel, settings_pick_list_menu_style, settings_pick_list_style,
    settings_section_card, settings_text_input_style,
};
use crate::app::message::settings::{SettingsMessage, SopMessage};
use crate::app::state::SopSettingsState;
use crate::app::{App, Message};
use iced::widget::{button, column, container, pick_list, row, text, text_input};
use iced::{Alignment, Element, Length};

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

fn hint_row<'a>(message: &'a str) -> Element<'a, Message> {
    row![
        container(text("")).width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
        text(message).size(12).style(settings_muted_text_style),
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .into()
}

fn number_row<'a>(
    label: &'static str,
    description: &'static str,
    value: u32,
    suffix: &'static str,
    min: f32,
    max: f32,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        row![
            NumberInput::new(value as f32, min, max, 1.0, 0, 1.0, on_change).settings_style(),
            text(suffix).size(12).style(settings_muted_text_style),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    )
}

pub(super) fn execution_mode_options() -> [String; 2] {
    ["supervised".to_string(), "autonomous".to_string()]
}

pub(super) fn rounded_u32(value: f32) -> u32 {
    value.round() as u32
}

fn sops_dir_changed(value: String) -> Message {
    Message::Settings(SettingsMessage::Sop(SopMessage::SopsDirChanged(value)))
}

fn sops_dir_pick_folder() -> Message {
    Message::Settings(SettingsMessage::Sop(SopMessage::SopsDirPickFolder))
}

fn default_execution_mode_changed(value: String) -> Message {
    Message::Settings(SettingsMessage::Sop(SopMessage::DefaultExecutionModeChanged(value)))
}

fn max_finished_runs_changed(value: f32) -> Message {
    Message::Settings(SettingsMessage::Sop(SopMessage::MaxFinishedRunsChanged(rounded_u32(value))))
}

fn max_concurrent_total_changed(value: f32) -> Message {
    Message::Settings(SettingsMessage::Sop(SopMessage::MaxConcurrentTotalChanged(rounded_u32(
        value,
    ))))
}

fn approval_timeout_secs_changed(value: f32) -> Message {
    Message::Settings(SettingsMessage::Sop(SopMessage::ApprovalTimeoutSecsChanged(rounded_u32(
        value,
    ))))
}

/// 根据无外部依赖的 SOP 设置状态构建页面内容。
fn settings_content(s: &SopSettingsState) -> Element<'_, Message> {
    let dir_row = field_row(
        "流程目录",
        "支持绝对路径或工作区相对路径。",
        row![
            text_input("留空时使用 <workspace>/sops", &s.sops_dir_input)
                .on_input(sops_dir_changed)
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style)
                .width(Length::Fill),
            button(text("选择目录"))
                .padding([6, 12])
                .style(rounded_action_btn_style)
                .on_press(sops_dir_pick_folder()),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    );

    let execution_mode_row = field_row(
        "默认执行模式",
        "没有显式 SOP.toml 配置时使用的执行策略。",
        pick_list(
            execution_mode_options(),
            Some(s.default_execution_mode.clone()),
            default_execution_mode_changed,
        )
        .padding([10, 14])
        .text_size(13)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fixed(220.0)),
    );

    let content = column![
        settings_page_intro("标准流程配置", "配置 SOP 目录来源、默认执行策略与运行队列限制。"),
        settings_section_card(
            "路径与执行策略",
            "控制 SOP 文件目录来源，以及缺省执行模式在没有显式 SOP.toml 配置时的行为。",
        ),
        settings_panel(column![dir_row, settings_divider(), execution_mode_row].spacing(0)),
        hint_row("支持绝对路径或工作区相对路径；留空时默认读取当前工作区下的 sops 目录。"),
        hint_row("自动执行会直接运行；人工审批会在开始前请求确认。"),
        settings_section_card(
            "运行队列限制",
            "控制完成记录保留上限、全局并发上限，以及审批等待超时时间。",
        ),
        settings_panel(
            column![
                number_row(
                    "已完成记录上限",
                    "控制保留的已完成执行记录数量。",
                    s.max_finished_runs,
                    "0 表示不限制",
                    0.0,
                    100_000.0,
                    max_finished_runs_changed,
                ),
                number_row(
                    "全局并发上限",
                    "限制所有 SOP 运行队列的总并发数。",
                    s.max_concurrent_total,
                    "全局并发运行数",
                    1.0,
                    1_000.0,
                    max_concurrent_total_changed,
                ),
                number_row(
                    "审批超时",
                    "等待人工审批的最长超时时间。",
                    s.approval_timeout_secs,
                    "0 表示禁用超时",
                    0.0,
                    86_400.0,
                    approval_timeout_secs_changed,
                ),
            ]
            .spacing(0),
        ),
        hint_row("建议先从较低并发开始，逐步观察执行吞吐、人工审批时延和队列堆积情况。"),
    ]
    .spacing(16)
    .width(Length::Fill);

    content.into()
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
    settings_content(&app.sop_settings)
}
#[cfg(test)]
#[path = "system_settings_sop_tests.rs"]
mod system_settings_sop_tests;
