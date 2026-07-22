//! 系统设置中 model routes 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::assets::Icon;
use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, icon_btn, primary_action_btn_style, settings_muted_text_style,
    settings_page_intro, settings_panel, settings_panel_style, settings_section_card,
    settings_text_input_style,
};
use crate::app::message::settings::{ModelRoutesMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{button, column, container, row, text, text_input};
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
    let s = &app.model_routes_settings;

    let header = row![
        container(settings_page_intro(
            "模型路由",
            "将匹配模式映射到指定 provider/model，并与查询分类优先级保持一致。"
        ))
        .width(Length::Fill),
        button(text("新增路由"))
            .on_press(Message::Settings(
                SettingsMessage::ModelRoutes(ModelRoutesMessage::AddRoute,)
            ))
            .padding([8, 12])
            .style(primary_action_btn_style),
    ]
    .spacing(10)
    .align_y(Alignment::Start);

    let mut list = column![
        header,
        settings_section_card(
            "路由规则",
            "优先级越高的规则越先命中，并会同步到 query classification。"
        ),
    ]
    .spacing(16);

    if s.routes.is_empty() {
        list = list.push(settings_panel(
            column![
                text("暂无模型路由，点击右上角“新增路由”开始配置")
                    .size(13)
                    .style(settings_muted_text_style)
            ]
            .spacing(0),
        ));
    }

    for (idx, route) in s.routes.iter().enumerate() {
        let card = container(
            column![
                row![
                    text(format!("路由 {}", idx + 1)).size(14),
                    container(text(" ")).width(Length::Fill),
                    icon_btn(
                        Icon::Trash,
                        "删除路由",
                        Some(Message::Settings(SettingsMessage::ModelRoutes(
                            ModelRoutesMessage::RemoveRoute(idx),
                        ))),
                    ),
                ]
                .align_y(Alignment::Center),
                settings_panel(
                    column![
                        field_row(
                            "匹配模式",
                            "定义命中的任务场景，例如 reasoning / fast / code。",
                            text_input("如：reasoning / fast / code", &route.pattern)
                                .on_input(move |value| Message::Settings(
                                    SettingsMessage::ModelRoutes(
                                        ModelRoutesMessage::PatternChanged(idx, value),
                                    )
                                ))
                                .padding([10, 12])
                                .size(13)
                                .style(settings_text_input_style)
                                .width(Length::Fill),
                        ),
                        field_row(
                            "提供商",
                            "命中后要切换到的 provider。",
                            text_input("如：openai", &route.provider)
                                .on_input(move |value| Message::Settings(
                                    SettingsMessage::ModelRoutes(
                                        ModelRoutesMessage::ProviderChanged(idx, value),
                                    )
                                ))
                                .padding([10, 12])
                                .size(13)
                                .style(settings_text_input_style)
                                .width(Length::Fill),
                        ),
                        field_row(
                            "模型",
                            "命中后优先使用的模型。",
                            text_input("如：gpt-5", &route.model)
                                .on_input(move |value| Message::Settings(
                                    SettingsMessage::ModelRoutes(ModelRoutesMessage::ModelChanged(
                                        idx, value
                                    ),)
                                ))
                                .padding([10, 12])
                                .size(13)
                                .style(settings_text_input_style)
                                .width(Length::Fill),
                        ),
                        field_row(
                            "优先级",
                            "越大优先级越高；保存时同步生成 query_classification 规则。",
                            text_input("0", &route.priority_input)
                                .on_input(move |value| Message::Settings(
                                    SettingsMessage::ModelRoutes(
                                        ModelRoutesMessage::PriorityChanged(idx, value),
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
#[path = "system_settings_model_routes_tests.rs"]
mod system_settings_model_routes_tests;
