//! # 模型选择弹出框模块
//!
//! 本模块提供模型选择相关的 UI 组件，包括：
//! - 模型切换按钮：显示当前选中的模型，点击后弹出选择面板
//! - 模型弹出框内容：包含模型搜索、列表展示、自动模型切换等功能
//!
//! ## 主要功能
//!
//! - **模型展示**：显示当前选中的模型名称和图标
//! - **模型搜索**：支持按 provider 或 model 的 id/name 搜索
//! - **模型切换**：点击模型项切换当前使用的模型
//! - **自动模型**：启用后由系统自动选择最佳模型

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::svg;
use iced::widget::{
    Space, button, column, container, mouse_area, row, scrollable, stack, text, text_input,
};
use iced::{Alignment, Element, Length, Padding, Theme};

use desktop_assets::assets::Icon;
use super::composer_icons::ComposerIcon;
use super::surface_styles::{BOTTOM_BAR_ICON_BUTTON_SIZE, BOTTOM_BAR_ICON_SIZE, composer_selector_button_style};
use super::icons::{
    auto_icon, icon_svg, provider_logo_handle, themed_svg_handle,
};
use super::styles::{
    popover_style,
    selectable_list_button_style, selector_chevron_color, selector_label_font,
    square_icon_button_style,
};
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, SettingsSelectLabel, settings_divider,
    settings_field_label, settings_muted_text_style, settings_text_input_style, settings_toggle,
};
use desktop_i18n::I18n;
use super::session_lock_tooltip;
use super::model_popover::*;


#[derive(Debug, Clone)]
pub enum Action {
    ToggleModelPopover, ModelQueryChanged(String), ToggleInputModelProvider(String),
    ModelSelected(String), OpenSystemSettingsModelDetail(String, String),
    ManageModels, ConnectProvider, AutoModelToggled(bool),
}
pub struct ModelPopoverInput<'a> {
    pub settings: &'a desktop_settings_providers::state::ModelSettingsState,
    pub i18n: I18n, pub cli_local: bool, pub started: bool,
    pub can_select: bool, pub is_acp: bool,
}
pub fn model_toggle_button<Message: Clone + 'static>(
    i18n: I18n, providers: &[desktop_settings_providers::state::ProviderModelsSummary],
    cli_local: bool, auto_model: bool, model: &str, expanded: bool,
    reasoning_label: Option<&str>,
    emit: fn(Action) -> Message,
) -> Element<'static, Message> {
    const MODEL_LABEL_MAX_WIDTH: f32 = 148.0;
    const LEVEL_LABEL_MAX_WIDTH: f32 = 64.0;
    const CHEVRON_SIZE: f32 = 11.0;
    const GAP: f32 = 6.0;
    // 下拉箭头图标的样式：使用半透明的文字颜色
    let chevron_style = move |theme: &Theme, _| svg::Style {
        color: Some(selector_chevron_color(theme, expanded)),
    };
    let chevron_model = icon_svg(
        if expanded { Icon::ChevronUp } else { Icon::ChevronDown },
        CHEVRON_SIZE,
    )
    .style(chevron_style);

    // 根据是否启用自动模型确定显示的标签文本
    let toggle_label = if cli_local {
        i18n.__("智能体原生模型").to_string()
    } else if auto_model {
        i18n.__("自动模型").to_owned()
    } else {
        model_toggle_label(providers, auto_model, model)
    };

    // 根据当前状态确定按钮图标
    let model_icon_svg = if cli_local {
        ComposerIcon::Agent.svg(BOTTOM_BAR_ICON_SIZE)
    } else if auto_model {
        // 自动模型模式使用自动图标
        themed_svg_handle(auto_icon(), BOTTOM_BAR_ICON_SIZE)
    } else {
        // 手动模式根据 provider 确定图标
        let provider_id = model_provider_icon_id(providers, model);
        themed_svg_handle(provider_logo_handle(&provider_id), BOTTOM_BAR_ICON_SIZE)
    };

    // 给箭头预留空间，模型名和档位各自保持单行。
    let trailing_width = CHEVRON_SIZE + GAP;
    let model_name = SingleLineLabel::new(toggle_label, 12.0)
        .font(selector_label_font())
        .shrink();
    let (label_content, label_max_width): (Element<'static, Message>, f32) =
        if let Some(level) = reasoning_label {
            let level = container(SingleLineLabel::new(level.to_owned(), 11.0).shrink())
                .max_width(LEVEL_LABEL_MAX_WIDTH)
                .style(|theme: &Theme| container::Style {
                    text_color: Some(theme.palette().text.scale_alpha(0.62)),
                    ..Default::default()
                });
            (
                row![
                    container(model_name).max_width(MODEL_LABEL_MAX_WIDTH),
                    level,
                ]
                .spacing(GAP)
                .align_y(Alignment::Center)
                .into(),
                MODEL_LABEL_MAX_WIDTH + GAP + LEVEL_LABEL_MAX_WIDTH,
            )
        } else {
            (model_name.into(), MODEL_LABEL_MAX_WIDTH)
        };
    let label = stack![
        container(label_content)
            .max_width(label_max_width + trailing_width)
            .padding(Padding { right: trailing_width, ..Padding::ZERO }),
        container(chevron_model).align_right(Length::Fill).center_y(Length::Fill),
    ];

    button(container(row![model_icon_svg, label].spacing(GAP).align_y(Alignment::Center))
        .center_y(BOTTOM_BAR_ICON_BUTTON_SIZE))
        .style(move |theme: &Theme, status| composer_selector_button_style(theme, status, expanded))
        .padding([0, 10])
        .on_press(emit(Action::ToggleModelPopover))
        .into()
}

pub fn model_search_id() -> iced::widget::Id {
    iced::widget::Id::new("composer-model-search")
}

pub fn model_popover_content<'a, Message: Clone + 'static>(
    input: ModelPopoverInput<'a>, auto_model: bool, model: &str,
    source: Element<'a, Message>, reasoning: Option<Element<'a, Message>>, emit: fn(Action) -> Message,
) -> Element<'a, Message> {
    let popup_width = 320.0;
    let query = input.settings.query.trim().to_ascii_lowercase();
    if input.cli_local {
        let mut content = column![].spacing(10);
        if let Some(reasoning) = reasoning {
            content = content.push(reasoning);
        }
        let source = scrollable(container(source).padding([0, 7]))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)));
        return container(content.push(source))
            .style(popover_style).padding(10).width(Length::Fixed(popup_width)).max_height(420).into();
    }
    let i18n = input.i18n;
    let started = input.started;
    let can_select = input.can_select;
    let is_acp = input.is_acp;
    // 搜索输入框
    let search = text_input(
        i18n.__("搜索模型…"),
        &input.settings.query,
    )
    .id(model_search_id())
    .on_input(move |v| emit(Action::ModelQueryChanged(v)))
    .padding(SETTINGS_CONTROL_PADDING)
    .size(SETTINGS_CONTROL_TEXT_SIZE)
    .style(settings_text_input_style);

    // 模型列表容器
    let mut model_list = column![].spacing(8);
    let mut any_models = false;

    // 处理加载状态和空列表情况
    if input.settings.loading && input.settings.providers.is_empty() {
        // 加载中且无数据时显示加载提示
        model_list = model_list.push(
            text(i18n.__("加载中…"))
                .size(13)
                .style(settings_muted_text_style),
        );
    } else {
        // 遍历所有 provider 及其模型
        for p in &input.settings.providers {
            // 筛选启用且匹配搜索条件的模型
            let mut models = p
                .models
                .iter()
                .filter(|m| m.enabled)
                .filter(|m| {
                    model_matches_query(&query, &p.id, &p.name, &m.id, &m.name)
                        || model_media_labels(m).any(|label| {
                            let operation = match label {
                                "图片输出" => "生图",
                                "视频输出" => "生视频",
                                _ => label,
                            };
                            [label, operation].into_iter().any(|key| {
                                key.contains(query.as_str())
                                    || i18n.__(key).to_ascii_lowercase().contains(query.as_str())
                            })
                        })
                })
                .collect::<Vec<_>>();
            if models.is_empty() {
                continue;
            }
            // 按名称排序（名称相同时按 id 排序）
            models.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));

            any_models = true;
            // 搜索临时展开匹配项，不覆盖已保存的折叠偏好。
            let collapsed =
                query.is_empty() && input.settings.input_collapsed_provider_ids.contains(&p.id);
            // Provider 分组标题
            let header = row![
                icon_svg(if collapsed { Icon::ChevronRight } else { Icon::ChevronDown }, 14.0),
                themed_svg_handle(provider_logo_handle(&p.id), 16.0),
                text(&p.name).size(14).width(Length::Fill).wrapping(text::Wrapping::WordOrGlyph),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            let header = button(header)
                .padding([4, 8])
                .width(Length::Fill)
                .style(|theme, status| selectable_list_button_style(theme, status, false))
                .on_press_maybe(query.is_empty().then(|| {
                    emit(Action::ToggleInputModelProvider(p.id.clone()))
                }));
            model_list = model_list.push(header);
            if collapsed {
                continue;
            }

            // 渲染该 provider 下的每个模型
            for m in models {
                let provider_id = p.id.clone();
                let model_id = m.id.clone();
                let model_key = format!("{}/{}", provider_id, model_id);

                // 判断当前模型是否被选中
                let selected = !auto_model && model_is_selected(model, &provider_id, &model_id);

                // 选中状态显示勾选图标，否则显示占位空间
                let selected_badge: Element<'_, Message> = if selected {
                    icon_svg(Icon::Check, 14.0).into()
                } else {
                    Space::new().width(14).into()
                };

                let media_labels = model_media_labels(m)
                    .map(|label| i18n.__(label))
                    .collect::<Vec<_>>()
                    .join(" · ");
                let mut label = column![SettingsSelectLabel::new(truncated_model_name(&m.name))]
                    .spacing(2)
                    .width(Length::Fill);
                let height = if media_labels.is_empty() {
                    Length::Fixed(36.0)
                } else {
                    label = label.push(text(media_labels).size(11).style(settings_muted_text_style));
                    Length::Shrink
                };
                let select_btn = button(
                    row![label, selected_badge].spacing(8).align_y(Alignment::Center),
                )
                .padding([6, 8])
                .height(height)
                .width(Length::Fill)
                .style(move |theme: &Theme, status: iced::widget::button::Status| {
                    let mut style = selectable_list_button_style(theme, status, selected);
                    if !can_select && !selected {
                        style.text_color = theme.palette().text.scale_alpha(0.55);
                    }
                    style
                })
                .on_press_maybe(
                    can_select
                        .then_some(emit(Action::ModelSelected(model_key))),
                );

                // 模型详情按钮（问号图标）
                let detail_btn = mouse_area(
                    container(icon_svg(Icon::QuestionCircle, 14.0).style(|theme: &Theme, _| {
                        svg::Style { color: Some(theme.palette().text.scale_alpha(0.45)) }
                    }))
                    .width(Length::Fixed(22.0))
                    .height(Length::Fixed(22.0))
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center),
                )
                .on_press(emit(Action::OpenSystemSettingsModelDetail(provider_id.clone(), model_id.clone())));

                let select_btn = session_lock_tooltip(select_btn, i18n, started);
                model_list = model_list
                    .push(row![select_btn, detail_btn].spacing(4).align_y(Alignment::Center));
            }
        }

        // 如果没有任何可用模型，显示提示信息
        if !any_models {
            model_list = model_list.push(
                text(
                    i18n
                        .__("暂无可选模型（请先在系统设置里启用模型）"),
                )
                .size(13)
                .style(settings_muted_text_style),
            );
        }
    }

    let footer_button = |icon, label, action| {
        button(
            row![icon_svg(icon, 14.0), text(i18n.__(label)).size(12)]
                .spacing(6)
                .align_y(Alignment::Center),
        )
        .padding([5, 8])
        .style(|theme, status| square_icon_button_style(theme, status, true))
        .on_press(emit(action))
    };
    let footer = row![
        footer_button(Icon::Plus, "连接供应商", Action::ConnectProvider),
        footer_button(Icon::Sliders, "管理模型", Action::ManageModels),
    ]
    .spacing(6)
    .wrap();

    let auto = row![
        themed_svg_handle(auto_icon(), 18.0),
        settings_field_label(i18n.__("自动模型")),
        Space::new().width(Length::Fill),
        settings_toggle(auto_model).on_toggle_maybe(can_select.then_some(move |enabled| {
            emit(Action::AutoModelToggled(enabled))
        }),)
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let auto = session_lock_tooltip(auto, i18n, started);
    let mut controls = column![auto].spacing(10);
    if is_acp {
        controls = controls.push(settings_divider()).push(source);
    }
    let body = column![model_list, settings_divider(), controls].spacing(10);
    let body = scrollable(container(body).padding([0, 7]))
        .height(Length::Fill)
        .id(iced::widget::Id::new("input_panel_model_popover_scroll"))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)));
    let mut content = column![].spacing(10);
    if let Some(reasoning) = reasoning {
        content = content.push(container(reasoning).padding([0, 7]).width(Length::Fill));
    }
    container(content.push(search.width(Length::Fill)).push(body).push(footer))
    .style(popover_style)
    .padding(10)
    .width(Length::Fixed(popup_width))
    .height(Length::Fixed(500.0))
    .into()
}

/// 模型选择必须服从后端能力与当前请求状态。
pub fn can_select_model(runtime: &super::acp_options::OptionsState<'_>, started: bool) -> bool {
    use vw_api_acp_model::acp_model_source::{AcpModelSource, AcpModelSourceCapability};
    !started && !runtime.is_requesting && !runtime.acp_options_pending && !runtime.source_pending
        && (runtime.acp_agent.is_none() || runtime.acp_options.model_source_capabilities.iter().any(|c| matches!(c, AcpModelSourceCapability::Supported { source: AcpModelSource::SessionModel })))
}

pub fn with_model_popover<'a, Message: Clone + 'a>(toggle: Element<'a, Message>, content: Element<'a, Message>, expanded: bool, close: Message) -> Element<'a, Message> {
    desktop_ui_controls::overlays::AboveOverlay::new(toggle, content)
        .show(expanded)
        .align_x(iced::alignment::Horizontal::Right)
        .block_background_events(true)
        .flip_if_clipped()
        .gap(7.0)
        .on_close(close)
        .into()
}
