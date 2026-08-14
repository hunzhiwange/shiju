//! 研究配置设置界面组件
//!
//! 本模块提供研究（Research）功能的配置界面，允许用户在图形界面中调整
//! 代理的研究预检行为配置项。
//!
//! # 功能概述
//!
//! - 启用/禁用研究阶段
//! - 选择触发方式（从不/总是/关键词/长度/问句）
//! - 配置关键词列表（用于“关键词”触发模式）
//! - 设置消息长度阈值（用于“长度”触发模式）
//! - 配置最大迭代次数
//! - 显示/隐藏研究过程日志
//! - 自定义系统提示前缀
//!
//! # 配置持久化
//!
//! 所有配置项的修改会实时更新到应用状态，并通过 Gateway 保存到
//! 共享数据库的 `research` 字段中。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, icon_btn, settings_checkbox_style, settings_close_button,
    settings_divider, settings_help_button, settings_help_copy_state, settings_muted_text_style,
    settings_page_intro, settings_panel, settings_pick_list_menu_style, settings_pick_list_style,
    settings_section_card, settings_text_input_style, settings_value_badge,
};
use crate::app::{App, Message, message};
use iced::widget::{Space, checkbox, column, container, pick_list, row, slider, text, text_input};
use iced::{Alignment, Element, Length};

const RESEARCH_HELP: &str = r#"研究配置说明

一、作用
- research 用于在正式回复前做一轮预检：先调用工具收集上下文，再生成主回复。
- 典型用于代码搜索、资料补全、复杂问题分解。

二、字段含义
1) enabled
- 是否启用 research 阶段。

2) trigger
- 从不：永不触发。
- 总是：每次都触发。
- 关键词：命中关键词时触发。
- 长度：消息长度超过 min_message_length 时触发。
- 问句：包含问号时触发。

3) keywords
- trigger 为“关键词”时使用，支持多个关键词。

4) min_message_length
- trigger 为“长度”时使用，消息长度阈值。

5) max_iterations
- 单次 research 允许的最大工具调用轮次。

6) show_progress
- 是否在控制台显示 research 过程日志。

7) system_prompt_prefix
- 可选。用于追加自定义研究指令前缀。

三、示例
{
  "research": {
    "enabled": true,
    "trigger": "keywords",
    "keywords": ["find", "search", "investigate"],
    "min_message_length": 50,
    "max_iterations": 5,
    "show_progress": true,
    "system_prompt_prefix": ""
  }
  }
"#;

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
use vw_config_types::automation::ResearchTrigger;

fn trigger_label(trigger: ResearchTrigger) -> &'static str {
    match trigger {
        ResearchTrigger::Never => "从不",
        ResearchTrigger::Always => "总是",
        ResearchTrigger::Keywords => "关键词",
        ResearchTrigger::Length => "长度",
        ResearchTrigger::Question => "问句",
    }
}

fn parse_trigger_label(value: &str) -> ResearchTrigger {
    match value {
        "总是" => ResearchTrigger::Always,
        "关键词" => ResearchTrigger::Keywords,
        "长度" => ResearchTrigger::Length,
        "问句" => ResearchTrigger::Question,
        _ => ResearchTrigger::Never,
    }
}

fn inline_help_panel(app: &App) -> Option<Element<'static, Message>> {
    app.research_settings.show_help_modal.then(|| {
        let close_message = Message::Settings(message::SettingsMessage::ResearchHelpClose);
        let close_btn = settings_close_button(close_message);
        let (copy_icon, copy_tip) =
            settings_help_copy_state(app.last_copied_code_hash, RESEARCH_HELP);
        let copy_btn =
            icon_btn(copy_icon, copy_tip, Some(Message::CopyCode(RESEARCH_HELP.to_string())));

        settings_panel(
            column![
                row![
                    text("Research 配置帮助").size(16),
                    Space::new().width(Length::Fill),
                    copy_btn,
                    close_btn
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                text(RESEARCH_HELP).size(13).style(settings_muted_text_style),
            ]
            .spacing(12)
            .width(Length::Fill),
        )
        .into()
    })
}

/// 构建研究配置设置界面的视图
///
/// # 参数
///
/// * `app` - 应用状态引用，从中读取 `research_settings` 获取当前配置
///
/// # 返回值
///
/// 返回一个 iced Element，包含完整的配置界面，包括：
/// - 标题和帮助按钮
/// - 各配置项的输入控件
/// - 错误信息显示（如有）
/// - 帮助卡片（当 `show_help_modal` 为 true 时）
///
/// # 界面布局
///
/// ```text
/// ┌─────────────────────────────────────────────┐
/// │ 研究配置                            [?]     │
/// │ 启用       [x] 开启 Research 预检阶段       │
/// │ 触发方式   [下拉选择框]                     │
/// │ 关键词     [文本输入框]                     │
/// │ 最短消息   [滑块] N 字                      │
/// │ 最大迭代   [滑块] N 次                      │
/// │ 进度输出   [x] 显示 Research 过程日志       │
/// │ 提示前缀   [文本输入框]                     │
/// └─────────────────────────────────────────────┘
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.research_settings;
    let help_btn =
        settings_help_button(Message::Settings(message::SettingsMessage::ResearchHelpOpen));

    let enabled_row = field_row(
        "启用",
        "控制是否开启 Research 预检阶段。",
        checkbox(s.enabled)
            .label("开启 Research 预检阶段")
            .on_toggle(|v| Message::Settings(message::SettingsMessage::ResearchEnabledToggled(v)))
            .style(settings_checkbox_style),
    );

    let trigger_pick = pick_list(
        ["从不", "总是", "关键词", "长度", "问句"],
        Some(trigger_label(s.trigger)),
        |v| {
            Message::Settings(message::SettingsMessage::ResearchTriggerChanged(
                parse_trigger_label(v),
            ))
        },
    )
    .padding([10, 14])
    .text_size(13)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fixed(220.0));

    let trigger_row = field_row("触发方式", "定义进入 Research 阶段的条件。", trigger_pick);

    let keywords_row = text_row(
        "关键词",
        "仅在触发方式为关键词时生效。",
        "find, search, check",
        &s.keywords_input,
        |v| Message::Settings(message::SettingsMessage::ResearchKeywordsChanged(v)),
    );

    let keywords_hint = row![
        container(text("")).width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
        text("仅在触发方式为“关键词”时生效，支持逗号分隔。")
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let min_len_slider = slider(1.0..=10_000.0, s.min_message_length as f32, |v| {
        Message::Settings(message::SettingsMessage::ResearchMinMessageLengthChanged(
            v.round() as u32
        ))
    })
    .width(Length::Fixed(280.0));

    let min_len_row = field_row(
        "最短消息",
        "触发方式为长度时的最小消息阈值。",
        row![min_len_slider, settings_value_badge(format!("{} 字", s.min_message_length)),]
            .spacing(16)
            .align_y(Alignment::Center),
    );

    let max_iter_slider = slider(1.0..=100.0, s.max_iterations as f32, |v| {
        Message::Settings(message::SettingsMessage::ResearchMaxIterationsChanged(v.round() as u32))
    })
    .width(Length::Fixed(280.0));

    let max_iter_row = field_row(
        "最大迭代",
        "限制单次 Research 的工具调用轮次。",
        row![max_iter_slider, settings_value_badge(format!("{} 次", s.max_iterations)),]
            .spacing(16)
            .align_y(Alignment::Center),
    );

    let progress_row = field_row(
        "进度输出",
        "控制是否输出 Research 过程日志。",
        checkbox(s.show_progress)
            .label("显示 Research 过程日志")
            .on_toggle(|v| {
                Message::Settings(message::SettingsMessage::ResearchShowProgressToggled(v))
            })
            .style(settings_checkbox_style),
    );

    let prompt_row = text_row(
        "提示前缀",
        "可选的自定义 research 指令前缀。",
        "可选，自定义 research 指令前缀",
        &s.system_prompt_prefix,
        |v| Message::Settings(message::SettingsMessage::ResearchSystemPromptPrefixChanged(v)),
    );

    let mut col = column![
        row![
            container(settings_page_intro(
                "研究配置",
                "配置 Research 预检阶段的触发条件、预算与日志输出。"
            ))
            .width(Length::Fill),
            help_btn
        ]
        .align_y(Alignment::Start),
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(help_panel) = inline_help_panel(app) {
        col = col.push(help_panel);
    }

    col = col
        .push(settings_section_card("基础行为", "控制研究阶段启用状态与触发方式。"))
        .push(settings_panel(
            column![enabled_row, settings_divider(), trigger_row, settings_divider(), keywords_row]
                .spacing(0),
        ))
        .push(keywords_hint)
        .push(settings_section_card("预算与输出", "配置长度阈值、最大迭代和过程输出。"))
        .push(settings_panel(
            column![
                min_len_row,
                settings_divider(),
                max_iter_row,
                settings_divider(),
                progress_row,
                settings_divider(),
                prompt_row,
            ]
            .spacing(0),
        ));

    col.into()
}

#[cfg(test)]
#[path = "system_settings_research_tests.rs"]
mod system_settings_research_tests;
