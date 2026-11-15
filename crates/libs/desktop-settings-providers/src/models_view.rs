use desktop_ui_controls::system_settings_common::*;
use desktop_assets::assets::Icon;
use desktop_i18n::settings_models::{self, text as model_text};
use crate::view_message::{Message, SettingsMessage};
use crate::view_message as message;
use crate::view_style::ViewStyle;
use iced::widget::{button, column, container, row, text, text_input};
use iced::widget::text::Wrapping;
use iced::{Alignment, Background, Element, Length};
pub fn main_view<'a>(s: &'a crate::state::ModelSettingsState, style: ViewStyle<'a>) -> Element<'a, Message> {
    let q = s.query.trim().to_ascii_lowercase();
    let busy = s.loading || s.saving;
    let enabled_model_count = s
        .providers
        .iter()
        .flat_map(|provider| &provider.models)
        .filter(|model| model.enabled)
        .count();
    let total_model_count = s.providers.iter().map(|provider| provider.models.len()).sum::<usize>();
    let mut list = column![].spacing(14).width(Length::Fill);
    let mut any = false;
    let mut matched_provider_count = 0usize;
    let mut matched_model_count = 0usize;

    for p in &s.providers {
        let pid_lower = p.id.to_ascii_lowercase();
        let pname_lower = p.name.to_ascii_lowercase();
        let provider_match = !q.is_empty() && (pid_lower.contains(&q) || pname_lower.contains(&q));

        let models = if q.is_empty() {
            p.models.iter().collect::<Vec<_>>()
        } else if provider_match {
            p.models.iter().collect::<Vec<_>>()
        } else {
            p.models
                .iter()
                .filter(|m| {
                    m.id.to_ascii_lowercase().contains(&q)
                        || m.name.to_ascii_lowercase().contains(&q)
                })
                .collect::<Vec<_>>()
        };

        if models.is_empty() {
            continue;
        }

        any = true;
        matched_provider_count += 1;
        matched_model_count += models.len();

        let expanded = s.expanded_provider_ids.contains(&p.id);
        let provider_enabled_count = p.models.iter().filter(|model| model.enabled).count();
        let provider_model_count = p.models.len();
        let mut enable_provider_btn =
            button(text(model_text(style.locale, "全部启用")).size(12))
                .padding([6, 10])
                .style(rounded_action_btn_style);
        if !busy && provider_enabled_count < provider_model_count {
            enable_provider_btn = enable_provider_btn.on_press(Message::Settings(
                message::SettingsMessage::ProviderModelsSetAll(p.id.clone(), true),
            ));
        }
        let mut disable_provider_btn =
            button(text(model_text(style.locale, "全部禁用")).size(12))
                .padding([6, 10])
                .style(rounded_action_btn_style);
        if !busy && provider_enabled_count > 0 {
            disable_provider_btn = disable_provider_btn.on_press(Message::Settings(
                message::SettingsMessage::ProviderModelsSetAll(p.id.clone(), false),
            ));
        }
        let mut reset_provider_btn =
            button(text(model_text(style.locale, "恢复默认")).size(12))
                .padding([6, 10])
                .style(rounded_action_btn_style);
        if !busy && provider_model_count > 0 {
            reset_provider_btn = reset_provider_btn.on_press(Message::Settings(
                message::SettingsMessage::ProviderModelsReset(p.id.clone()),
            ));
        }
        let expand_provider_btn = button(
            text(if expanded {
                model_text(style.locale, "收起")
            } else {
                model_text(style.locale, "展开")
            })
            .size(12),
        )
        .on_press(Message::Settings(message::SettingsMessage::ProviderModelsExpansionToggled(
            p.id.clone(),
        )))
        .padding([6, 10])
        .style(rounded_action_btn_style);

        let provider_header = container(
            column![
                row![
                    container(provider_logo_svg(&p.id, 18.0)).center_x(24).center_y(24),
                    column![
                        text(&p.name).size(15).wrapping(Wrapping::WordOrGlyph),
                        text(&p.id)
                            .size(13)
                            .wrapping(Wrapping::WordOrGlyph)
                            .style(settings_muted_text_style),
                    ]
                    .spacing(4)
                    .width(Length::Fill),
                    settings_value_badge(settings_models::enabled_count(
                        style.locale,
                        provider_enabled_count,
                        provider_model_count,
                    )),
                    expand_provider_btn,
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                row![enable_provider_btn, disable_provider_btn, reset_provider_btn]
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .wrap(),
            ]
            .spacing(12),
        )
        .padding([14, 0])
        .width(Length::Fill);

        let mut card_rows = column![provider_header];
        if expanded {
            for m in models {
                let mut model_toggle = settings_toggle(m.enabled);
                if !busy {
                    let provider_id = p.id.clone();
                    let model_id = m.id.clone();
                    model_toggle = model_toggle.on_toggle(move |enabled| {
                        Message::Settings(SettingsMessage::ModelToggle(
                            provider_id.clone(),
                            model_id.clone(),
                            enabled,
                        ))
                    });
                }
                let metadata = settings_models::model_metadata(
                    style.locale,
                    m.toolcall,
                    m.attachment,
                    &format_context_limit(m.context_limit),
                );
                let item = container(
                    row![
                        column![
                            text(&m.name).size(15).wrapping(Wrapping::WordOrGlyph),
                            text(&m.id)
                                .size(13)
                                .wrapping(Wrapping::WordOrGlyph)
                                .style(settings_muted_text_style),
                            text(metadata).size(12).style(settings_muted_text_style),
                        ]
                        .spacing(4)
                        .width(Length::Fill),
                        icon_btn(
                            Icon::QuestionCircle,
                            model_text(style.locale, "更多"),
                            Some(Message::Settings(SettingsMessage::ModelDetailOpen(
                                p.id.clone(),
                                m.id.clone(),
                            ))),
                        ),
                        model_toggle,
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                )
                .padding([12, 0])
                .width(Length::Fill);
                card_rows = card_rows.push(settings_divider()).push(item);
            }
        }
        list = list.push(settings_group_card(card_rows));
    }

    if !any {
        let empty_title = if s.loading {
            model_text(style.locale, "正在刷新模型目录…")
        } else {
            model_text(style.locale, "暂无已连接的提供商或模型")
        };
        let empty_description = if s.loading {
            model_text(style.locale, "稍后会展示当前已发现的 provider 与模型。")
        } else {
            model_text(style.locale, "请先完成 provider 连接，或调整搜索关键字后重试。")
        };

        list = list.push(settings_group_card(
            container(settings_group_heading(empty_title, empty_description)).padding([16, 0]),
        ));
    }

    let query_input = text_input(model_text(style.locale, "搜索模型"), &s.query)
        .on_input(|v| Message::Settings(message::SettingsMessage::ModelQueryChanged(v)))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .width(Length::Fill);

    let refresh_btn = button(text(model_text(
        style.locale,
        if s.loading {
            "刷新中…"
        } else if s.saving {
            "保存中…"
        } else {
            "刷新"
        },
    )))
    .on_press_maybe((!busy).then_some(Message::Settings(SettingsMessage::ModelsRefresh)))
    .padding([8, 14])
    .style(rounded_action_btn_style);

    let filter_content = settings_group_card(
        container(
            row![
                query_input,
                row![
                    settings_value_badge(settings_models::provider_count(
                        style.locale,
                        matched_provider_count,
                    )),
                    settings_value_badge(settings_models::model_count(
                        style.locale,
                        matched_model_count,
                    )),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .padding([14, 0]),
    );

    let mut content = column![
        row![
            container(style.page_heading(
                model_text(style.locale, "模型配置"),
                model_text(
                    style.locale,
                    "浏览已连接 provider 的模型目录，搜索、筛选并切换模型可用状态。",
                ),
            ))
            .width(Length::Fill),
            refresh_btn,
        ]
        .spacing(12)
        .align_y(Alignment::Center),
        style.group_heading(
            model_text(style.locale, "搜索与过滤"),
            model_text(
                style.locale,
                "按 provider 名称、provider ID、模型名称或模型 ID 过滤结果。",
            ),
        ),
        filter_content,
    ]
    .spacing(16)
    .width(Length::Fill);

    let mut enable_all_btn = button(text(model_text(style.locale, "全部启用")).size(12))
        .padding([7, 12])
        .style(rounded_action_btn_style);
    if !busy && enabled_model_count < total_model_count {
        enable_all_btn = enable_all_btn
            .on_press(Message::Settings(message::SettingsMessage::ModelsSetAll(true)));
    }

    let mut disable_all_btn = button(text(model_text(style.locale, "全部禁用")).size(12))
        .padding([7, 12])
        .style(rounded_action_btn_style);
    if !busy && enabled_model_count > 0 {
        disable_all_btn = disable_all_btn
            .on_press(Message::Settings(message::SettingsMessage::ModelsSetAll(false)));
    }

    let mut connected_heading = column![style.group_heading(
        model_text(style.locale, "已连接模型"),
        model_text(
            style.locale,
            "按 provider 分组展示可用模型，可单独或一次性切换启用状态。",
        ),
    )]
    .spacing(6);
    if s.saving {
        connected_heading = connected_heading.push(
            text(model_text(style.locale, "正在保存模型状态，完成后即可继续操作。"))
                .size(13)
                .style(settings_muted_text_style),
        );
    }
    content = content.push(
        row![
            container(connected_heading).width(Length::Fill),
            row![enable_all_btn, disable_all_btn].spacing(8).align_y(Alignment::Center),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    );

    content = content.push(list);

    content.into()
}

/// 渲染模型设置视图的覆盖层
///
/// 该函数用于渲染模型设置页面上方的弹窗层，目前支持：
/// - 模型详情弹窗
///
/// # 参数
///
/// * `app` - 应用状态引用，从中读取 `model_settings.detail_modal` 数据
/// * `dialog` - 基础对话框元素，覆盖层将在此之上渲染
///
/// # 返回值
///
/// 返回一个 `Element`，包含基础对话框和可能的覆盖层
///
/// # 模型详情弹窗结构
///
/// 弹窗包含：
/// - 固定标题栏：弹窗标题、切换按钮（字段列表/原始 JSON）、关闭按钮
/// - 滚动内容区：模型和提供商标识，以及字段列表或原始 JSON
///
/// # UI 交互
///
/// - 关闭按钮：触发 `SettingsMessage::ModelDetailClose` 消息
/// - 切换按钮：触发 `SettingsMessage::ModelDetailToggleRaw` 消息
pub fn detail_card<'a>(mm: &'a crate::state::ModelDetailModalState, style: ViewStyle<'a>) -> (Message, Element<'a, Message>) {
    let locale = style.locale;
    let close_message = Message::Settings(SettingsMessage::ModelDetailClose);
    let toggle_raw =
        button(text(model_text(locale, if mm.show_raw { "字段列表" } else { "原始 JSON" })))
            .on_press(Message::Settings(SettingsMessage::ModelDetailToggleRaw))
            .padding([8, 14])
            .style(rounded_action_btn_style);

    let header = settings_modal_header(
        text(model_text(locale, "模型详情")).size(20),
        row![toggle_raw, settings_close_button(close_message.clone())]
            .spacing(8)
            .align_y(Alignment::Center),
    );
    let identity = column![
        text(&mm.model_name).size(17).wrapping(Wrapping::WordOrGlyph),
        text(&mm.provider_name)
            .size(13)
            .wrapping(Wrapping::WordOrGlyph)
            .style(settings_muted_text_style),
        detail_row(model_text(locale, "提供商").into(), mm.provider_id.clone()),
        settings_divider(),
        detail_row(
            settings_models::connection_text::text(locale, "模型").into(),
            mm.model_id.clone(),
        ),
    ]
    .spacing(6);

    let details = if mm.show_raw {
        settings_group_card(
            container(text(&mm.raw_json).size(13).wrapping(Wrapping::WordOrGlyph)).padding([14, 0]),
        )
    } else {
        let mut rows = column![];
        for (index, field) in mm.rows.iter().enumerate() {
            if index != 0 {
                rows = rows.push(settings_divider());
            }
            rows = rows.push(detail_row(field.label.display(locale), field.value.display(locale)));
        }
        settings_group_card(rows)
    };

    let body = column![identity, details].spacing(14);
    let card = settings_modal_layout(header, body, None)
        .style(|theme| {
            let mut style = settings_modal_card_style(theme);
            // Keep catalog text from showing through the detail card.
            if let Some(Background::Color(color)) = &mut style.background {
                color.a = 1.0;
            }
            style
        })
        .width(780)
        .height(520);
    (close_message, container(card).padding(12).into())
}

fn detail_row(label: String, value: String) -> Element<'static, Message> {
    container(
        row![
            text(label)
                .size(13)
                .width(Length::FillPortion(2))
                .wrapping(Wrapping::WordOrGlyph)
                .style(settings_muted_text_style),
            text(value).size(13).width(Length::FillPortion(3)).wrapping(Wrapping::WordOrGlyph),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([8, 0])
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
mod models_view_tests;
