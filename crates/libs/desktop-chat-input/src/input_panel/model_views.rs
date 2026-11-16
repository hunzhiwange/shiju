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
    Space, button, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Alignment, Element, Length, Theme};

use desktop_assets::assets::Icon;
use super::icons::{
    auto_icon, icon_svg, provider_logo_handle, themed_svg_handle,
};
use super::styles::{
    popover_style,
    selectable_list_button_style, selector_chevron_color, selector_label_font,
    selector_pill_button_style, selector_text_color, square_icon_button_style,
};
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
    AcpModelInputDraftChanged(String), ModelInputChanged(String),
}
pub struct ModelPopoverInput<'a> {
    pub settings: &'a desktop_settings_providers::state::ModelSettingsState,
    pub i18n: I18n, pub cli_local: bool, pub started: bool,
    pub can_select: bool, pub is_acp: bool, pub input_model: String,
}
pub fn model_toggle_button<Message: Clone + 'static>(
    i18n: I18n, providers: &[desktop_settings_providers::state::ProviderModelsSummary],
    cli_local: bool, auto_model: bool, model: &str, expanded: bool,
    emit: fn(Action) -> Message,
) -> Element<'static, Message> {
    let highlight_toggle = expanded || (auto_model && !cli_local);
    // 下拉箭头图标的样式：使用半透明的文字颜色
    let chevron_style = move |theme: &Theme, _| svg::Style {
        color: Some(selector_chevron_color(theme, highlight_toggle)),
    };
    let chevron_model = icon_svg(
        if expanded { Icon::ChevronUp } else { Icon::ChevronDown },
        11.0,
    )
    .style(chevron_style);

    // 根据是否启用自动模型确定显示的标签文本
    let toggle_label = if cli_local {
        "智能体原生模型".to_string()
    } else if auto_model {
        i18n.__("自动模型").to_owned()
    } else {
        model_toggle_label(providers, auto_model, model)
    };

    // 根据当前状态确定按钮图标
    let model_icon_svg = if cli_local {
        icon_svg(Icon::Hexagon, 12.0)
    } else if auto_model {
        // 自动模型模式使用自动图标
        themed_svg_handle(auto_icon(), 12.0)
    } else {
        // 手动模式根据 provider 确定图标
        let provider_id = model_provider_icon_id(providers, model);
        themed_svg_handle(provider_logo_handle(&provider_id), 12.0)
    };

    // 构建按钮：图标 + 文本 + 下拉箭头
    button(
        row![
            model_icon_svg,
            text(toggle_label).size(12.0).font(selector_label_font()).style(
                move |theme: &Theme| iced::widget::text::Style {
                    color: Some(selector_text_color(theme, highlight_toggle))
                }
            ),
            chevron_model
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .style(move |theme: &Theme, status| selector_pill_button_style(theme, status, highlight_toggle))
    .padding([4, 10])
    .on_press(emit(Action::ToggleModelPopover))
    .into()
}

pub fn model_popover_content<'a, Message: Clone + 'static>(
    input: ModelPopoverInput<'a>, auto_model: bool, model: &str,
    source: Element<'a, Message>, emit: fn(Action) -> Message,
) -> Element<'a, Message> {
    let popup_width = 320.0;
    let query = input.settings.query.trim().to_ascii_lowercase();
    if input.cli_local {
        return container(scrollable(container(source).padding([0, 7]))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))))
            .style(popover_style).padding(10).width(Length::Fixed(popup_width)).max_height(420).into();
    }
    let i18n = input.i18n;
    let started = input.started;
    let can_select = input.can_select;
    let is_acp = input.is_acp;
    let input_model = input.input_model;
    // 搜索输入框
    let search = text_input(
        i18n.__("搜索模型…"),
        &input.settings.query,
    )
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
                .filter(|m| model_matches_query(&query, &p.id, &p.name, &m.id, &m.name))
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

                // 模型选择按钮
                let select_btn = button(
                    row![SettingsSelectLabel::new(truncated_model_name(&m.name)), selected_badge]
                        .spacing(8)
                        .align_y(Alignment::Center),
                )
                .padding([6, 8])
                .height(36)
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

    // "管理模型" 按钮
    let manage_btn: Element<'_, Message> = {
        let btn = button(
            container(icon_svg(Icon::Sliders, 16.0))
                .width(Length::Fixed(36.0))
                .height(Length::Fixed(36.0))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .padding(0)
        .style(|theme: &Theme, status| square_icon_button_style(theme, status, true))
        .on_press(emit(Action::ManageModels));
        btn.into()
    };

    // "连接供应商" 按钮
    let providers_btn: Element<'_, Message> = {
        let btn = button(
            container(icon_svg(Icon::Plus, 16.0))
                .width(Length::Fixed(36.0))
                .height(Length::Fixed(36.0))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .padding(0)
        .style(|theme: &Theme, status| square_icon_button_style(theme, status, true))
        .on_press(emit(Action::ConnectProvider));
        btn.into()
    };

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

    let manual_input = text_input(i18n.__("auto / provider/model / 自定义模型"), &input_model)
        .on_input_maybe(can_select.then_some(move |value: String| {
            emit(if is_acp {
                Action::AcpModelInputDraftChanged(value)
            } else {
                Action::ModelInputChanged(normalize_model_input(&value))
            })
        }))
        .on_submit_maybe(can_select.then_some(emit(Action::ModelSelected(
            normalize_model_input(&input_model),
        ))))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style);
    let manual = column![
        settings_field_label(i18n.__("手工填写模型")),
        session_lock_tooltip(manual_input, i18n, started),
        text(if started && !is_acp {
            i18n.__("会话开始后不能切换模型")
        } else if is_acp {
            "输入 Provider/model 后按回车确认"
        } else {
            i18n.__("兼容特殊调度器或未出现在列表中的模型 ID")
        })
        .size(12)
        .style(settings_muted_text_style),
    ]
    .spacing(6);

    let auto = session_lock_tooltip(auto, i18n, started);
    let mut controls = column![auto, manual].spacing(10);
    if is_acp {
        controls = controls.push(settings_divider()).push(source);
    }
    let body = if query.is_empty() {
        column![controls, settings_divider(), model_list]
    } else {
        column![model_list, settings_divider(), controls]
    }
    .spacing(10);
    let body = scrollable(container(body).padding([0, 7]))
        .id(iced::widget::Id::new("input_panel_model_popover_scroll"))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)));
    container(
        column![
            row![search.width(Length::Fill), providers_btn, manage_btn]
                .spacing(8)
                .align_y(Alignment::Center),
            body,
        ]
        .spacing(10),
    )
    .style(popover_style)
    .padding(10)
    .width(Length::Fixed(popup_width))
    .max_height(520)
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
        .show(expanded).block_background_events(true).gap(6.0).on_close(close).into()
}
