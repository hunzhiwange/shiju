//! 任务看板模型选择器视图，集中处理草稿、编辑和批量任务的模型选择入口。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::svg;
use iced::widget::{
    Space, button, column, container, mouse_area, row, scrollable, stack, text,
    tooltip::{Position as TooltipPosition, Tooltip},
};
use iced::{Alignment, Color, Element, Length, Theme};

use crate::app::assets::{self, Icon};
use crate::app::components::overlays::BelowOverlay;
use crate::app::components::system_settings_common::{
    SettingsSelectLabel, settings_field_label, settings_muted_text_style, settings_text_input,
    settings_toggle,
};
use crate::app::message::{SettingsMessage, TaskBoardMessage, ViewMessage};
use crate::app::task::normalize_task_model_input;
use crate::app::{App, Message};

use super::super::common::{auto_icon, provider_logo_handle};
use super::styles::{
    popover_style, selector_option_style, selector_trigger_style, square_icon_button_style,
    tooltip_dark_style,
};

#[derive(Clone, Copy)]
enum TaskBoardModelSelectorKind {
    Draft,
    Edit,
    Bulk,
}

impl TaskBoardModelSelectorKind {
    fn current_model<'a>(&self, app: &'a App) -> &'a str {
        match self {
            Self::Draft | Self::Edit => app.task_board_draft.model.as_str(),
            Self::Bulk => app.task_board_bulk_model_input.as_str(),
        }
    }

    fn toggle_message(&self) -> TaskBoardMessage {
        match self {
            Self::Draft | Self::Edit => TaskBoardMessage::ToggleModelPopover,
            Self::Bulk => TaskBoardMessage::ToggleBulkModelPopover,
        }
    }

    fn close_message(&self) -> TaskBoardMessage {
        match self {
            Self::Draft | Self::Edit => TaskBoardMessage::CloseModelPopover,
            Self::Bulk => TaskBoardMessage::CloseBulkModelPopover,
        }
    }

    fn is_open(&self, app: &App) -> bool {
        match self {
            Self::Draft | Self::Edit => app.task_board_model_popover,
            Self::Bulk => app.task_board_bulk_model_popover,
        }
    }

    fn select_message(&self, model: String) -> TaskBoardMessage {
        match self {
            Self::Draft => TaskBoardMessage::ModelSelected(model),
            Self::Edit => TaskBoardMessage::UpdateEditingTaskModel(model),
            Self::Bulk => TaskBoardMessage::BulkModelSelected(model),
        }
    }

    fn input_message(&self, model: String) -> TaskBoardMessage {
        match self {
            Self::Draft => TaskBoardMessage::UpdateDraftModel(model),
            Self::Edit => TaskBoardMessage::UpdateEditingTaskModelInput(model),
            Self::Bulk => TaskBoardMessage::UpdateBulkModelInput(model),
        }
    }
}

/// 构建或更新 build model selector 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn build_model_selector<'a>(app: &'a App, is_edit_mode: bool) -> Element<'a, Message> {
    let kind = if is_edit_mode {
        TaskBoardModelSelectorKind::Edit
    } else {
        TaskBoardModelSelectorKind::Draft
    };
    build_selector(app, kind)
}

/// 构建或更新 build bulk model selector 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn build_bulk_model_selector<'a>(app: &'a App) -> Element<'a, Message> {
    build_selector(app, TaskBoardModelSelectorKind::Bulk)
}

fn build_selector<'a>(app: &'a App, kind: TaskBoardModelSelectorKind) -> Element<'a, Message> {
    let model_value = normalize_task_model_input(kind.current_model(app));
    let auto_model = model_value == "auto";
    let model = model_value.as_str();

    let chevron_style =
        |theme: &Theme, _| svg::Style { color: Some(theme.palette().text.scale_alpha(0.65)) };
    let chevron_model = svg::Svg::<iced::Theme>::new(assets::get_icon(Icon::ChevronDown))
        .width(Length::Fixed(12.0))
        .height(Length::Fixed(12.0))
        .style(chevron_style);

    let open = kind.is_open(app);
    let model_toggle = button(
        row![
            build_model_icon(auto_model, app, model),
            SettingsSelectLabel::new(build_toggle_label(app, auto_model, model)),
            chevron_model
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .style(move |theme, status| selector_trigger_style(theme, status, open))
    .padding([6, 8])
    .width(Length::Fill)
    .height(38)
    .on_press(Message::TaskBoard(kind.toggle_message()));

    let model_pop_content = build_model_popover(app, kind, auto_model, model);

    BelowOverlay::new(model_toggle, model_pop_content)
        .show(open)
        .gap(6.0)
        .on_close(Message::TaskBoard(kind.close_message()))
        .into()
}

fn build_toggle_label(app: &App, auto_model: bool, model: &str) -> String {
    if auto_model {
        return crate::app::i18n::I18n::new(app.desktop_locale).__("自动模型").to_owned();
    }

    let parsed = if model.contains('/') {
        let parts = model.splitn(2, '/').collect::<Vec<_>>();
        if parts.len() == 2 { Some((parts[0].to_string(), parts[1].to_string())) } else { None }
    } else {
        None
    };

    let display = match parsed.as_ref() {
        Some((provider_id, model_id)) => app
            .model_settings
            .providers
            .iter()
            .find(|p| &p.id == provider_id)
            .and_then(|p| p.models.iter().find(|m| &m.id == model_id))
            .map(|m| m.name.clone()),
        None => app
            .model_settings
            .providers
            .iter()
            .find_map(|p| p.models.iter().find(|m| m.id == model).map(|m| m.name.clone())),
    };

    display.unwrap_or_else(|| {
        parsed.as_ref().map(|(_, model_id)| model_id.clone()).unwrap_or_else(|| model.to_string())
    })
}

fn build_model_icon<'a>(auto_model: bool, app: &'a App, model: &str) -> svg::Svg<'a, Theme> {
    if auto_model {
        svg::Svg::<iced::Theme>::new(auto_icon())
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(14.0))
            .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
    } else {
        let provider_id = if model.contains('/') {
            model.split('/').next().unwrap_or("agent").to_string()
        } else {
            app.model_settings
                .providers
                .iter()
                .find_map(|p| p.models.iter().any(|m| m.id == model).then_some(p.id.clone()))
                .unwrap_or_else(|| "agent".to_string())
        };
        svg::Svg::<iced::Theme>::new(provider_logo_handle(&provider_id))
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(14.0))
            .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
    }
}

fn build_model_popover<'a>(
    app: &'a App,
    kind: TaskBoardModelSelectorKind,
    auto_model: bool,
    model: &str,
) -> Element<'a, Message> {
    let popup_width = 320.0;
    let query = app.model_settings.query.trim().to_ascii_lowercase();

    let matches_query =
        |provider_id: &str, provider_name: &str, model_id: &str, model_name: &str| -> bool {
            if query.is_empty() {
                return true;
            }
            provider_id.to_ascii_lowercase().contains(&query)
                || provider_name.to_ascii_lowercase().contains(&query)
                || model_id.to_ascii_lowercase().contains(&query)
                || model_name.to_ascii_lowercase().contains(&query)
        };

    let is_selected = |current: &str, provider_id: &str, model_id: &str| -> bool {
        if current.contains('/') {
            current == format!("{}/{}", provider_id, model_id)
        } else {
            current == model_id
        }
    };

    let search = settings_text_input(
        crate::app::i18n::I18n::new(app.desktop_locale).__("搜索模型…"),
        &app.model_settings.query,
    )
    .on_input(|v| Message::Settings(SettingsMessage::ModelQueryChanged(v)));

    let mut model_list = column![].spacing(2);
    let mut any_models = false;

    if app.model_settings.loading && app.model_settings.providers.is_empty() {
        model_list = model_list.push(
            text(crate::app::i18n::I18n::new(app.desktop_locale).__("加载中…")).size(13).style(
                |t: &Theme| iced::widget::text::Style {
                    color: Some(t.extended_palette().background.base.text),
                },
            ),
        );
    } else {
        for p in &app.model_settings.providers {
            let mut models = p
                .models
                .iter()
                .filter(|m| m.enabled)
                .filter(|m| matches_query(&p.id, &p.name, &m.id, &m.name))
                .collect::<Vec<_>>();
            if models.is_empty() {
                continue;
            }
            models.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));

            let header = row![
                svg::Svg::<iced::Theme>::new(provider_logo_handle(&p.id))
                    .width(Length::Fixed(16.0))
                    .height(Length::Fixed(16.0))
                    .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) }),
                text(p.name.clone()).size(13).wrapping(text::Wrapping::WordOrGlyph),
                container(text("")).width(Length::Fill),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            model_list = model_list.push(container(header).padding([4, 0]));
            any_models = true;

            for m in models {
                let provider_id = p.id.clone();
                let model_id = m.id.clone();
                let model_key = format!("{}/{}", provider_id, model_id);

                let selected = !auto_model && is_selected(model, &provider_id, &model_id);
                let selected_badge: Element<'_, Message> = if selected {
                    svg::Svg::<iced::Theme>::new(assets::get_icon(Icon::Check))
                        .width(14)
                        .height(14)
                        .style(|theme: &Theme, _| svg::Style {
                            color: Some(theme.palette().primary),
                        })
                        .into()
                } else {
                    Space::new().width(14).height(14).into()
                };

                let select_btn = button(
                    row![SettingsSelectLabel::new(m.name.clone()), selected_badge]
                        .spacing(8)
                        .align_y(Alignment::Center),
                )
                .padding(iced::Padding { top: 6.0, right: 36.0, bottom: 6.0, left: 10.0 })
                .width(Length::Fill)
                .height(36)
                .style(move |theme, status| selector_option_style(theme, status, selected))
                .on_press(Message::TaskBoard(kind.select_message(model_key)));

                let detail_btn = mouse_area(
                    container(
                        svg::Svg::<iced::Theme>::new(assets::get_icon(Icon::QuestionCircle))
                            .width(Length::Fixed(14.0))
                            .height(Length::Fixed(14.0))
                            .style(|theme: &Theme, _| svg::Style {
                                color: Some(theme.palette().text.scale_alpha(0.45)),
                            }),
                    )
                    .width(Length::Fixed(22.0))
                    .height(Length::Fixed(22.0))
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center),
                )
                .on_press(Message::View(
                    ViewMessage::OpenSystemSettingsModelDetail(provider_id, model_id),
                ));

                let select_btn = stack![
                    select_btn,
                    container(detail_btn)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Right)
                        .align_y(iced::alignment::Vertical::Center)
                        .padding([0, 6])
                ];

                let model_tip = container(
                    column![
                        text(m.name.clone()).size(12).wrapping(text::Wrapping::WordOrGlyph).style(
                            |_t: &Theme| {
                                iced::widget::text::Style { color: Some(Color::WHITE) }
                            }
                        ),
                        text(format!("{} / {}", p.name, p.id))
                            .size(11)
                            .wrapping(iced::widget::text::Wrapping::Word)
                            .style(|_t: &Theme| iced::widget::text::Style {
                                color: Some(Color::WHITE.scale_alpha(0.72)),
                            }),
                        text(
                            crate::app::i18n::I18n::new(app.desktop_locale)
                                .format(
                                    "工具 %s · 图片输入 %s · 上下文限制 %u",
                                    &[
                                        if m.toolcall { "✓" } else { "✕" }.into(),
                                        if m.attachment { "✓" } else { "✕" }.into(),
                                        m.context_limit.into()
                                    ]
                                )
                                .expect("valid model capabilities template")
                        )
                        .size(11)
                        .wrapping(iced::widget::text::Wrapping::Word)
                        .style(|_t: &Theme| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(255, 225, 80)),
                        }),
                    ]
                    .spacing(2),
                )
                .style(tooltip_dark_style)
                .padding([6, 8])
                .width(Length::Fixed(240.0));

                let item = Tooltip::new(select_btn, model_tip, TooltipPosition::Right).gap(8);

                model_list = model_list.push(item);
                any_models = true;
            }
        }

        if !any_models {
            model_list = model_list.push(
                text(
                    crate::app::i18n::I18n::new(app.desktop_locale)
                        .__("暂无可选模型（请先在系统设置里启用模型）"),
                )
                .size(13)
                .style(|t: &Theme| iced::widget::text::Style {
                    color: Some(t.extended_palette().background.base.text),
                }),
            );
        }
    }

    let manage_btn: Element<'_, Message> = {
        let btn = button(
            container(
                svg::Svg::<iced::Theme>::new(assets::get_icon(Icon::Sliders))
                    .width(Length::Fixed(16.0))
                    .height(Length::Fixed(16.0))
                    .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) }),
            )
            .width(Length::Fixed(28.0))
            .height(Length::Fixed(28.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
        )
        .padding(0)
        .style(|theme: &Theme, status| square_icon_button_style(theme, status, true))
        .on_press(Message::View(ViewMessage::OpenSystemSettingsTab(
            crate::app::components::system_settings::SystemTab::Models,
        )));
        let tip = container(
            text(crate::app::i18n::I18n::new(app.desktop_locale).__("管理模型")).size(12),
        )
        .style(tooltip_dark_style)
        .padding([6, 8]);
        Tooltip::new(btn, tip, TooltipPosition::Right).gap(8).into()
    };

    let providers_btn: Element<'_, Message> = {
        let btn = button(
            container(
                svg::Svg::<iced::Theme>::new(assets::get_icon(Icon::Plus))
                    .width(Length::Fixed(16.0))
                    .height(Length::Fixed(16.0))
                    .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) }),
            )
            .width(Length::Fixed(28.0))
            .height(Length::Fixed(28.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
        )
        .padding(0)
        .style(|theme: &Theme, status| square_icon_button_style(theme, status, true))
        .on_press(Message::View(ViewMessage::OpenSystemSettingsTab(
            crate::app::components::system_settings::SystemTab::Providers,
        )));
        let tip = container(
            text(crate::app::i18n::I18n::new(app.desktop_locale).__("连接供应商")).size(12),
        )
        .style(tooltip_dark_style)
        .padding([6, 8]);
        Tooltip::new(btn, tip, TooltipPosition::Right).gap(8).into()
    };

    let boxed_content = column![
        container(
            column![
                row![
                    Tooltip::new(
                        mouse_area(
                            svg::Svg::<iced::Theme>::new(auto_icon())
                                .width(Length::Fixed(18.0))
                                .height(Length::Fixed(18.0))
                                .style(|theme: &Theme, _| svg::Style {
                                    color: Some(theme.palette().text),
                                }),
                        )
                        .on_press(Message::TaskBoard(kind.select_message("auto".to_string()))),
                        container(
                            text(
                                crate::app::i18n::I18n::new(app.desktop_locale)
                                    .__("自动模型自动选择可用模型")
                            )
                            .size(12)
                        )
                        .style(tooltip_dark_style)
                        .padding([6, 8]),
                        TooltipPosition::Right,
                    )
                    .gap(8),
                    text(crate::app::i18n::I18n::new(app.desktop_locale).__("自动模型")),
                    container(text("")).width(Length::Fill),
                    settings_toggle(auto_model).on_toggle({
                        let kind = kind;
                        move |_| Message::TaskBoard(kind.select_message("auto".to_string()))
                    })
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(10)
        )
        .padding([6, 8]),
        container(
            column![
                settings_field_label(
                    crate::app::i18n::I18n::new(app.desktop_locale).__("手工填写模型")
                ),
                settings_text_input(
                    crate::app::i18n::I18n::new(app.desktop_locale)
                        .__("auto / provider/model / 自定义模型"),
                    model
                )
                .on_input({
                    let kind = kind;
                    move |value| {
                        Message::TaskBoard(kind.input_message(normalize_task_model_input(&value)))
                    }
                }),
                text(
                    crate::app::i18n::I18n::new(app.desktop_locale)
                        .__("兼容特殊调度器或未出现在列表中的模型 ID")
                        .to_owned()
                )
                .size(12)
                .style(settings_muted_text_style)
            ]
            .spacing(6)
        )
        .padding(iced::Padding { top: 2.0, right: 8.0, bottom: 6.0, left: 8.0 }),
        container(
            column![
                row![search.width(Length::Fill), providers_btn, manage_btn]
                    .spacing(8)
                    .align_y(Alignment::Center),
                container(
                    scrollable(container(model_list).padding([0, 7]))
                        .id("task_board_model_selector_scroll")
                        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                        .height(Length::Shrink)
                        .width(Length::Fill)
                )
                .max_height(240),
            ]
            .spacing(8)
        )
        .padding(iced::Padding { top: 2.0, right: 8.0, bottom: 8.0, left: 8.0 }),
    ]
    .spacing(6);

    container(boxed_content)
        .style(popover_style)
        .padding(4)
        .width(Length::Fixed(popup_width))
        .into()
}

#[cfg(test)]
#[path = "model_selector_tests.rs"]
mod model_selector_tests;
