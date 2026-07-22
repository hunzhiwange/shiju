//! 系统设置中 query classification 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::assets::Icon;
use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, icon_btn, primary_action_btn_style, settings_checkbox_style,
    settings_muted_text_style, settings_page_intro, settings_panel, settings_panel_style,
    settings_section_card, settings_text_input_style,
};
use crate::app::message::settings::{QueryClassificationMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, container, row, text, text_input};
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
    let s = &app.query_classification_settings;

    let header = row![
        container(settings_page_intro(
            "查询分类",
            "将 pattern 映射为 category，并同步回 query_classification 配置。"
        ))
        .width(Length::Fill),
        button(text("新增规则"))
            .on_press(Message::Settings(SettingsMessage::QueryClassification(
                QueryClassificationMessage::AddRule,
            )))
            .padding([8, 12])
            .style(primary_action_btn_style),
    ]
    .spacing(10)
    .align_y(Alignment::Start);

    let enabled_row = field_row(
        "启用",
        "控制是否开启查询分类规则。",
        checkbox(s.enabled)
            .label("启用查询分类")
            .on_toggle(|value| {
                Message::Settings(SettingsMessage::QueryClassification(
                    QueryClassificationMessage::EnabledToggled(value),
                ))
            })
            .style(settings_checkbox_style),
    );

    let mut list = column![
        header,
        settings_section_card("基础行为", "控制查询分类总开关以及规则优先级行为。"),
        settings_panel(column![enabled_row].spacing(0)),
        settings_section_card(
            "分类规则",
            "pattern 会同步到 patterns/keywords，category 会写入 hint。"
        ),
    ]
    .spacing(16);

    if s.rules.is_empty() {
        list = list.push(settings_panel(
            column![
                text("暂无分类规则，点击右上角“新增规则”开始配置")
                    .size(13)
                    .style(settings_muted_text_style)
            ]
            .spacing(0),
        ));
    }

    for (idx, rule) in s.rules.iter().enumerate() {
        let card = container(
            column![
                row![
                    text(format!("规则 {}", idx + 1)).size(14),
                    container(text(" ")).width(Length::Fill),
                    icon_btn(
                        Icon::Trash,
                        "删除规则",
                        Some(Message::Settings(SettingsMessage::QueryClassification(
                            QueryClassificationMessage::RemoveRule(idx),
                        ))),
                    ),
                ]
                .align_y(Alignment::Center),
                settings_panel(
                    column![
                        field_row(
                            "pattern",
                            "匹配关键字或模式，例如 bug / test / code。",
                            text_input("例如 bug / test / code", &rule.pattern)
                                .on_input(move |value| Message::Settings(
                                    SettingsMessage::QueryClassification(
                                        QueryClassificationMessage::PatternChanged(idx, value),
                                    )
                                ))
                                .padding([10, 12])
                                .size(13)
                                .style(settings_text_input_style)
                                .width(Length::Fill),
                        ),
                        field_row(
                            "category",
                            "命中后写入的分类标签，例如 reasoning / fast。",
                            text_input("例如 reasoning / fast", &rule.category)
                                .on_input(move |value| Message::Settings(
                                    SettingsMessage::QueryClassification(
                                        QueryClassificationMessage::CategoryChanged(idx, value),
                                    )
                                ))
                                .padding([10, 12])
                                .size(13)
                                .style(settings_text_input_style)
                                .width(Length::Fill),
                        ),
                        field_row(
                            "priority",
                            "越大优先级越高；pattern 会同步到 patterns/keywords，category 会写入 hint。",
                            text_input("0", &rule.priority_input)
                                .on_input(move |value| Message::Settings(
                                    SettingsMessage::QueryClassification(
                                        QueryClassificationMessage::PriorityChanged(idx, value),
                                    )
                                ))
                                .padding([10, 12])
                                .size(13)
                                .style(settings_text_input_style)
                                .width(Length::Fixed(180.0)),
                        ),
                    ]
                    .spacing(0),
                ),
            ]
            .spacing(12),
        )
        .padding(14)
        .width(Length::Fill)
        .style(settings_panel_style);

        list = list.push(card);
    }

    list.into()
}
#[cfg(test)]
#[path = "system_settings_query_classification_tests.rs"]
mod system_settings_query_classification_tests;
