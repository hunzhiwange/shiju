use desktop_ui_controls::system_settings_common::*;
use desktop_assets::assets::Icon;
use desktop_i18n::settings_models::{self, text as model_text};
use crate::view_message::{Message, SettingsMessage, ModelRoutesMessage};
use crate::view_style::ViewStyle;
use iced::widget::{button,column,container,row,text};
use iced::{Alignment, Element, Length};
fn field_row<'a>(
    style: ViewStyle<'_>,
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    style.field(
        label,
        description,
        220.0,
        control,
    )
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
pub fn view<'a>(routes: &'a [crate::state::ModelRoute], style: ViewStyle<'a>) -> Element<'a, Message> {

    let header = row![
        container(style.page_heading(
            model_text(style.locale, "模型路由"),
            model_text(
                style.locale,
                "将匹配模式映射到指定 provider/model，并与查询分类优先级保持一致。"
            )
        ))
        .width(Length::Fill),
        button(text(model_text(style.locale, "新增路由")))
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
        style.group_heading(
            model_text(style.locale, "路由规则"),
            model_text(
                style.locale,
                "优先级越高的规则越先命中，并会同步到 query classification。"
            )
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    if routes.is_empty() {
        list = list.push(settings_panel(
            column![
                text(model_text(style.locale, "暂无模型路由，点击右上角“新增路由”开始配置"))
                    .size(13)
                    .style(settings_muted_text_style)
            ]
            .spacing(0),
        ));
    }

    for (idx, route) in routes.iter().enumerate() {
        let card = settings_group_card(
            column![
                row![
                    text(settings_models::route_title(style.locale, idx + 1)).size(14),
                    container(text(" ")).width(Length::Fill),
                    icon_btn(
                        Icon::Trash,
                        model_text(style.locale, "删除路由"),
                        Some(Message::Settings(SettingsMessage::ModelRoutes(
                            ModelRoutesMessage::RemoveRoute(idx),
                        ))),
                    ),
                ]
                .align_y(Alignment::Center),
                settings_section_content(
                    column![
                        field_row(
                            style,
                            model_text(style.locale, "匹配模式"),
                            model_text(
                                style.locale,
                                "定义命中的任务场景，例如 reasoning / fast / code。"
                            ),
                            settings_text_input(
                                model_text(style.locale, "如：reasoning / fast / code"),
                                &route.pattern
                            )
                            .on_input(move |value| Message::Settings(SettingsMessage::ModelRoutes(
                                ModelRoutesMessage::PatternChanged(idx, value),
                            )))
                            .width(Length::Fill),
                        ),
                        field_row(
                            style,
                            model_text(style.locale, "提供商"),
                            model_text(style.locale, "命中后要切换到的 provider。"),
                            settings_text_input(
                                model_text(style.locale, "如：openai"),
                                &route.provider
                            )
                            .on_input(move |value| Message::Settings(SettingsMessage::ModelRoutes(
                                ModelRoutesMessage::ProviderChanged(idx, value),
                            )))
                            .width(Length::Fill),
                        ),
                        field_row(
                            style,
                            model_text(style.locale, "模型"),
                            model_text(style.locale, "命中后优先使用的模型。"),
                            settings_text_input(
                                model_text(style.locale, "如：gpt-5"),
                                &route.model
                            )
                            .on_input(move |value| Message::Settings(SettingsMessage::ModelRoutes(
                                ModelRoutesMessage::ModelChanged(idx, value),
                            )))
                            .width(Length::Fill),
                        ),
                        field_row(
                            style,
                            model_text(style.locale, "优先级"),
                            model_text(
                                style.locale,
                                "越大优先级越高；保存时同步生成 query_classification 规则。"
                            ),
                            settings_text_input("0", &route.priority_input)
                                .on_input(move |value| Message::Settings(
                                    SettingsMessage::ModelRoutes(
                                        ModelRoutesMessage::PriorityChanged(idx, value),
                                    )
                                ))
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
#[path = "model_routes_view_tests.rs"]
mod model_routes_view_tests;
