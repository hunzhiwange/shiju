//! 系统设置中 query classification 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::assets::Icon;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, icon_btn, primary_action_btn_style,
    rounded_action_btn_style, settings_checkbox_style, settings_group_card, settings_group_heading,
    settings_muted_text_style, settings_page_heading, settings_panel, settings_row,
    settings_section_content, settings_select, settings_text_input_style,
};
use crate::app::message::settings::{
    ModelConfigTabMessage, QueryClassificationMessage, SettingsMessage,
};
use crate::app::state::ModelConfigTab;
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_row(label, description, 220.0, control)
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
    let mut category_options = Vec::new();
    for route in &app.model_routes_settings.routes {
        let pattern = route.pattern.trim();
        if !pattern.is_empty() && !category_options.iter().any(|item| item == pattern) {
            category_options.push(pattern.to_string());
        }
    }

    let header = row![
        container(settings_page_heading(
            crate::app::i18n::I18n::new(app.desktop_locale).__("查询模型分类"),
            crate::app::i18n::I18n::new(app.desktop_locale)
                .__("将请求 pattern 映射到已配置的模型路由。")
        ))
        .width(Length::Fill),
        button(text(crate::app::i18n::I18n::new(app.desktop_locale).__("前往模型路由")))
            .on_press(Message::Settings(SettingsMessage::ModelConfigTab(
                ModelConfigTabMessage::Selected(ModelConfigTab::ModelRoutes),
            )))
            .padding([8, 12])
            .style(rounded_action_btn_style),
        button(text(crate::app::i18n::I18n::new(app.desktop_locale).__("新增规则")))
            .on_press(Message::Settings(SettingsMessage::QueryClassification(
                QueryClassificationMessage::AddRule,
            )))
            .padding([8, 12])
            .style(primary_action_btn_style),
    ]
    .spacing(10)
    .align_y(Alignment::Start);

    let enabled_row = field_row(
        crate::app::i18n::I18n::new(app.desktop_locale).__("启用"),
        crate::app::i18n::I18n::new(app.desktop_locale).__("控制是否开启查询模型分类规则。"),
        checkbox(s.enabled)
            .label(crate::app::i18n::I18n::new(app.desktop_locale).__("启用查询模型分类"))
            .size(16)
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .on_toggle(|value| {
                Message::Settings(SettingsMessage::QueryClassification(
                    QueryClassificationMessage::EnabledToggled(value),
                ))
            })
            .style(settings_checkbox_style),
    );

    let mut list = column![
        header,
        settings_group_heading(
            crate::app::i18n::I18n::new(app.desktop_locale).__("基础行为"),
            crate::app::i18n::I18n::new(app.desktop_locale)
                .__("控制查询模型分类总开关以及规则优先级行为。")
        ),
        settings_group_card(enabled_row),
        settings_group_heading(
            crate::app::i18n::I18n::new(app.desktop_locale).__("模型分类规则"),
            crate::app::i18n::I18n::new(app.desktop_locale)
                .__("pattern 会匹配用户请求；模型分类从已配置的模型路由中选择。")
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    if s.rules.is_empty() {
        list = list.push(settings_panel(
            column![
                text(
                    crate::app::i18n::I18n::new(app.desktop_locale)
                        .__("暂无模型分类规则，点击右上角“新增规则”开始配置")
                )
                .size(13)
                .style(settings_muted_text_style)
            ]
            .spacing(0),
        ));
    }

    for (idx, rule) in s.rules.iter().enumerate() {
        let category_control: Element<'_, Message> = if category_options.is_empty() {
            text(
                crate::app::i18n::I18n::new(app.desktop_locale)
                    .__("暂无模型路由，请先点击“前往模型路由”添加。"),
            )
            .size(13)
            .style(settings_muted_text_style)
            .into()
        } else {
            let selected = category_options
                .iter()
                .find(|option| option.as_str() == rule.category.trim())
                .cloned();
            settings_select(category_options.clone(), selected, move |value| {
                Message::Settings(SettingsMessage::QueryClassification(
                    QueryClassificationMessage::CategoryChanged(idx, value),
                ))
            })
            .placeholder(crate::app::i18n::I18n::new(app.desktop_locale).__("请选择"))
            .width(Length::Fill)
            .into()
        };
        let card = settings_group_card(
            column![
                row![
                    text(
                        crate::__!(
                            crate::app::i18n::I18n::new(app.desktop_locale),
                            "规则 %u",
                            idx + 1
                        )
                        .expect("valid rule count template")
                    )
                    .size(14),
                    container(text(" ")).width(Length::Fill),
                    icon_btn(
                        Icon::Trash,
                        crate::app::i18n::I18n::new(app.desktop_locale).__("删除规则"),
                        Some(Message::Settings(SettingsMessage::QueryClassification(
                            QueryClassificationMessage::RemoveRule(idx),
                        ))),
                    ),
                ]
                .align_y(Alignment::Center),
                settings_section_content(
                    column![
                        field_row(
                            "pattern",
                            crate::app::i18n::I18n::new(app.desktop_locale)
                                .__("匹配关键字或模式，例如 bug / test / code。"),
                            text_input(
                                crate::app::i18n::I18n::new(app.desktop_locale)
                                    .__("例如 bug / test / code"),
                                &rule.pattern
                            )
                            .on_input(move |value| Message::Settings(
                                SettingsMessage::QueryClassification(
                                    QueryClassificationMessage::PatternChanged(idx, value),
                                )
                            ))
                            .padding(SETTINGS_CONTROL_PADDING)
                            .size(SETTINGS_CONTROL_TEXT_SIZE)
                            .style(settings_text_input_style)
                            .width(Length::Fill),
                        ),
                        field_row(
                            crate::app::i18n::I18n::new(app.desktop_locale).__("模型分类"),
                            crate::app::i18n::I18n::new(app.desktop_locale)
                                .__("选择命中后使用的模型路由；可在“模型路由”页新增选项。"),
                            category_control,
                        ),
                        field_row(
                            "priority",
                            crate::app::i18n::I18n::new(app.desktop_locale)
                                .__("越大优先级越高；匹配成功后使用所选模型分类对应的路由。"),
                            text_input("0", &rule.priority_input)
                                .on_input(move |value| Message::Settings(
                                    SettingsMessage::QueryClassification(
                                        QueryClassificationMessage::PriorityChanged(idx, value),
                                    )
                                ))
                                .padding(SETTINGS_CONTROL_PADDING)
                                .size(SETTINGS_CONTROL_TEXT_SIZE)
                                .style(settings_text_input_style)
                                .width(Length::Fill),
                        ),
                    ]
                    .spacing(0),
                ),
            ]
            .spacing(12),
        )
        .padding([10, 22]);

        list = list.push(card);
    }

    list.into()
}
#[cfg(test)]
#[path = "system_settings_query_classification_tests.rs"]
mod system_settings_query_classification_tests;
