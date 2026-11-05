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
    Space, button, column, container, mouse_area, row, scrollable, stack, text, text_input, toggler,
};
use iced::{Alignment, Element, Length, Theme};

use crate::app::assets::Icon;
use crate::app::components::input_panel::icons::{
    auto_icon, icon_svg, provider_logo_handle, themed_svg_handle,
};
use crate::app::components::input_panel::styles::{
    BOTTOM_BAR_CHEVRON_ICON_SIZE, BOTTOM_BAR_ICON_SIZE, BOTTOM_BAR_LABEL_SIZE, popover_style,
    selectable_list_button_style, selector_chevron_color, selector_label_font,
    selector_pill_button_style, selector_text_color, square_icon_button_style,
};
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, settings_text_input_style,
};
use crate::app::state::ProviderModelsSummary;
use crate::app::{App, Message, message};
use vw_gateway_client::vw_api_types::acp_model_source::AcpModelSource;

pub(super) fn normalize_model_input(model: &str) -> String {
    let trimmed = model.trim();
    if trimmed.is_empty() { "auto".to_string() } else { trimmed.to_string() }
}

pub(super) fn model_matches_query(
    query: &str,
    provider_id: &str,
    provider_name: &str,
    model_id: &str,
    model_name: &str,
) -> bool {
    if query.is_empty() {
        return true;
    }
    provider_id.to_ascii_lowercase().contains(query)
        || provider_name.to_ascii_lowercase().contains(query)
        || model_id.to_ascii_lowercase().contains(query)
        || model_name.to_ascii_lowercase().contains(query)
}

pub(super) fn model_is_selected(current: &str, provider_id: &str, model_id: &str) -> bool {
    if current.contains('/') {
        current == format!("{}/{}", provider_id, model_id)
    } else {
        current == model_id
    }
}

pub(super) fn truncated_model_name(name: &str) -> String {
    let max_chars = 26usize;
    if name.chars().count() > max_chars {
        name.chars().take(max_chars.saturating_sub(1)).collect::<String>() + "…"
    } else {
        name.to_string()
    }
}

pub(super) fn model_toggle_label(
    providers: &[ProviderModelsSummary],
    auto_model: bool,
    model: &str,
) -> String {
    if auto_model {
        return "自动模型".to_string();
    }

    let parsed = model.split_once('/');
    let display = match parsed {
        Some((provider_id, model_id)) => providers
            .iter()
            .find(|p| p.id.as_str() == provider_id)
            .and_then(|p| p.models.iter().find(|m| m.id.as_str() == model_id))
            .map(|m| m.name.clone()),
        None => providers
            .iter()
            .find_map(|p| p.models.iter().find(|m| m.id.as_str() == model).map(|m| m.name.clone())),
    };

    display.unwrap_or_else(|| match parsed {
        Some((_, model_id)) => model_id.to_string(),
        None => model.to_string(),
    })
}

pub(super) fn model_provider_icon_id(providers: &[ProviderModelsSummary], model: &str) -> String {
    if model.contains('/') {
        model.split('/').next().unwrap_or("agent").to_string()
    } else {
        providers
            .iter()
            .find_map(|p| p.models.iter().any(|m| m.id.as_str() == model).then_some(p.id.clone()))
            .unwrap_or_else(|| "agent".to_string())
    }
}

/// 创建模型切换按钮
///
/// 该按钮显示当前选中的模型名称和对应的 provider 图标，
/// 点击后触发模型弹出框的显示/隐藏切换。
///
/// # 参数
///
/// - `app`: 应用状态引用，用于获取模型配置信息
/// - `auto_model`: 是否启用了自动模型选择模式
/// - `model`: 当前选中的模型标识符，格式为 `{provider_id}/{model_id}` 或纯 `model_id`
///
/// # 返回值
///
/// 返回一个可点击的按钮组件，显示模型图标、名称和下拉箭头
///
/// # 示例
///
/// ```ignore
/// let button = model_toggle_button(&app, false, "openai/gpt-4");
/// ```
pub fn model_toggle_button(
    app: &App,
    auto_model: bool,
    model: &str,
    expanded: bool,
) -> Element<'static, Message> {
    let cli_local = app.current_session_runtime_ref().is_some_and(|runtime| {
        runtime.acp_agent.is_some() && runtime.acp_model_source == AcpModelSource::CliLocal
    });
    let highlight_toggle = expanded || (auto_model && !cli_local);
    // 下拉箭头图标的样式：使用半透明的文字颜色
    let chevron_style = move |theme: &Theme, _| svg::Style {
        color: Some(selector_chevron_color(theme, highlight_toggle)),
    };
    let chevron_model = icon_svg(
        if expanded { Icon::ChevronUp } else { Icon::ChevronDown },
        BOTTOM_BAR_CHEVRON_ICON_SIZE,
    )
    .style(chevron_style);

    // 根据是否启用自动模型确定显示的标签文本
    let toggle_label = if cli_local {
        "智能体原生模型".to_string()
    } else if auto_model {
        crate::app::i18n::I18n::new(app.desktop_locale).__("自动模型").to_owned()
    } else {
        model_toggle_label(&app.model_settings.providers, auto_model, model)
    };

    // 根据当前状态确定按钮图标
    let model_icon_svg = if cli_local {
        icon_svg(Icon::Hexagon, BOTTOM_BAR_ICON_SIZE)
    } else if auto_model {
        // 自动模型模式使用自动图标
        themed_svg_handle(auto_icon(), BOTTOM_BAR_ICON_SIZE)
    } else {
        // 手动模式根据 provider 确定图标
        let provider_id = model_provider_icon_id(&app.model_settings.providers, model);
        themed_svg_handle(provider_logo_handle(&provider_id), BOTTOM_BAR_ICON_SIZE)
    };

    // 构建按钮：图标 + 文本 + 下拉箭头
    button(
        row![
            model_icon_svg,
            text(toggle_label).size(BOTTOM_BAR_LABEL_SIZE).font(selector_label_font()).style(
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
    .on_press(Message::View(message::ViewMessage::ToggleModelPopover))
    .into()
}

/// 创建模型弹出框内容
///
/// 该组件包含完整的模型选择界面，包括：
/// - 自动模型切换开关
/// - 模型搜索框
/// - 按 provider 分组的模型列表
/// - 管理模型和添加供应商的快捷按钮
/// - 悬停提示信息
///
/// # 参数
///
/// - `app`: 应用状态引用，包含模型配置、搜索查询等
/// - `auto_model`: 是否启用了自动模型选择模式
/// - `model`: 当前选中的模型标识符
///
/// # 返回值
///
/// 返回一个包含完整模型选择界面的组件
///
/// # 布局结构
///
/// ```text
/// ┌────────────────────────────────┐
/// │ ○ 自动模型 [开关]              │
/// ├────────────────────────────────┤
/// │ [搜索框] [+] [⚙]               │
/// │ ┌──────────────────────────┐  │
/// │ │ Provider A               │  │
/// │ │   Model 1         ✓      │  │
/// │ │   Model 2               │  │
/// │ │ Provider B               │  │
/// │ │   Model 3               │  │
/// │ └──────────────────────────┘  │
/// └────────────────────────────────┘
/// ```
pub fn model_popover_content<'a>(
    app: &'a App,
    auto_model: bool,
    model: &str,
) -> Element<'a, Message> {
    // 弹出框宽度
    let popup_width = 320.0;
    // 获取搜索关键词（转为小写用于不区分大小写的匹配）
    let query = app.model_settings.query.trim().to_ascii_lowercase();
    let runtime = app.current_session_runtime();
    let cli_local =
        runtime.acp_agent.is_some() && runtime.acp_model_source == AcpModelSource::CliLocal;
    let can_select = !runtime.is_requesting
        && !runtime.acp_options_pending
        && runtime.acp_source_pending.is_none()
        && (runtime.acp_agent.is_none() || runtime.acp_options.model_source_capabilities.iter().any(|capability| {
            matches!(capability, vw_gateway_client::vw_api_types::acp_model_source::AcpModelSourceCapability::Supported { source: AcpModelSource::SessionModel })
        }));
    let is_acp = runtime.acp_agent.is_some();
    let input_model = runtime.acp_model_input_draft.as_deref().unwrap_or(model).to_string();

    // 搜索输入框
    let search = text_input(
        crate::app::i18n::I18n::new(app.desktop_locale).__("搜索模型…"),
        &app.model_settings.query,
    )
    .on_input(|v| Message::Settings(message::SettingsMessage::ModelQueryChanged(v)))
    .padding(SETTINGS_CONTROL_PADDING)
    .size(SETTINGS_CONTROL_TEXT_SIZE)
    .style(settings_text_input_style);

    // 模型列表容器
    let mut model_list = column![].spacing(8);
    let mut any_models = false;
    if runtime.acp_agent.is_some() {
        model_list = model_list.push(super::acp_options::source_controls(app));
        model_list = model_list.push(text("客户端模型 · 供应商模型").size(12));
    }

    // 处理加载状态和空列表情况
    if app.model_settings.loading && app.model_settings.providers.is_empty() {
        // 加载中且无数据时显示加载提示
        model_list = model_list.push(
            text(crate::app::i18n::I18n::new(app.desktop_locale).__("加载中…")).size(13).style(
                |t: &Theme| iced::widget::text::Style {
                    color: Some(t.extended_palette().secondary.base.text),
                },
            ),
        );
    } else {
        // 遍历所有 provider 及其模型
        for p in &app.model_settings.providers {
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

            // Provider 分组标题
            let header = row![
                themed_svg_handle(provider_logo_handle(&p.id), 16.0),
                text(p.name.clone()).size(13),
                container(text("")).width(Length::Fill),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            model_list = model_list.push(container(header).padding([4, 0]));
            any_models = true;

            // 渲染该 provider 下的每个模型
            for m in models {
                let provider_id = p.id.clone();
                let model_id = m.id.clone();
                let model_key = format!("{}/{}", provider_id, model_id);

                // 截断过长的模型名称（最多 26 个字符）
                let display_name = truncated_model_name(&m.name);

                // 判断当前模型是否被选中
                let selected =
                    !cli_local && !auto_model && model_is_selected(model, &provider_id, &model_id);

                // 选中状态显示勾选图标，否则显示占位空间
                let selected_badge: Element<'_, Message> = if selected {
                    row![
                        text(crate::app::i18n::I18n::new(app.desktop_locale).__("已选择")).size(11),
                        icon_svg(Icon::Check, 14.0)
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center)
                    .into()
                } else {
                    Space::new().width(Length::Fixed(44.0)).into()
                };

                // 模型选择按钮
                let select_btn = button(
                    row![text(display_name).size(13).width(Length::Fill), selected_badge]
                        .spacing(8)
                        .align_y(Alignment::Center),
                )
                .padding(iced::Padding { top: 6.0, right: 52.0, bottom: 6.0, left: 8.0 })
                .width(Length::Fill)
                .style(move |theme: &Theme, status: iced::widget::button::Status| {
                    selectable_list_button_style(theme, status, selected)
                })
                .on_press_maybe(
                    can_select
                        .then_some(Message::Chat(message::ChatMessage::ModelSelected(model_key))),
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
                .on_press(Message::View(
                    message::ViewMessage::OpenSystemSettingsModelDetail(
                        provider_id.clone(),
                        model_id.clone(),
                    ),
                ));

                // 将详情按钮叠加在选择按钮右侧
                let select_btn = stack![
                    select_btn,
                    container(detail_btn)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Right)
                        .align_y(iced::alignment::Vertical::Center)
                        .padding([0, 6])
                ];

                model_list = model_list.push(select_btn);
                any_models = true;
            }
        }

        // 如果没有任何可用模型，显示提示信息
        if !any_models {
            model_list = model_list.push(
                text(
                    crate::app::i18n::I18n::new(app.desktop_locale)
                        .__("暂无可选模型（请先在系统设置里启用模型）"),
                )
                .size(13)
                .style(|t: &Theme| iced::widget::text::Style {
                    color: Some(t.extended_palette().secondary.base.text),
                }),
            );
        }
    }

    // "管理模型" 按钮
    let manage_btn: Element<'_, Message> = {
        let btn = button(
            container(icon_svg(Icon::Sliders, 16.0))
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(28.0))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .padding(0)
        .style(|theme: &Theme, status| square_icon_button_style(theme, status, true))
        .on_press(Message::View(message::ViewMessage::OpenSystemSettingsTab(
            crate::app::components::system_settings::SystemTab::Models,
        )));
        btn.into()
    };

    // "连接供应商" 按钮
    let providers_btn: Element<'_, Message> = {
        let btn = button(
            container(icon_svg(Icon::Plus, 16.0))
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(28.0))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .padding(0)
        .style(|theme: &Theme, status| square_icon_button_style(theme, status, true))
        .on_press(Message::View(message::ViewMessage::OpenSystemSettingsTab(
            crate::app::components::system_settings::SystemTab::Providers,
        )));
        btn.into()
    };

    // 主内容区域：模式切换 + 模型列表
    let boxed_content = column![
        // 模式切换区域
        container(
            row![
                themed_svg_handle(auto_icon(), 18.0),
                text(crate::app::i18n::I18n::new(app.desktop_locale).__("自动模型")),
                container(text("")).width(Length::Fill),
                toggler(auto_model && !cli_local).on_toggle_maybe(
                    can_select
                        .then_some(|b| Message::Chat(message::ChatMessage::AutoModelToggled(b)))
                )
            ]
            .spacing(8)
        )
        .padding([6, 8]),
        container(
            column![
                text(crate::app::i18n::I18n::new(app.desktop_locale).__("手工填写模型")).size(12),
                text_input(
                    crate::app::i18n::I18n::new(app.desktop_locale)
                        .__("auto / provider/model / 自定义模型"),
                    &input_model
                )
                .on_input_maybe(can_select.then_some(move |value: String| {
                    Message::Chat(if is_acp {
                        message::ChatMessage::AcpModelInputDraftChanged(value)
                    } else {
                        message::ChatMessage::ModelInputChanged(normalize_model_input(&value))
                    })
                }))
                .on_submit_maybe(can_select.then_some(Message::Chat(
                    message::ChatMessage::ModelSelected(normalize_model_input(&input_model)),
                )))
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_text_input_style),
                text(if runtime.acp_agent.is_some() {
                    "输入 Provider/model 后按回车确认"
                } else {
                    crate::app::i18n::I18n::new(app.desktop_locale)
                        .__("兼容特殊调度器或未出现在列表中的模型 ID")
                })
                .size(11)
                .style(|theme: &Theme| {
                    let palette = theme.palette();
                    let is_dark =
                        palette.background.r + palette.background.g + palette.background.b < 1.5;
                    iced::widget::text::Style {
                        color: Some(if is_dark {
                            palette.text.scale_alpha(0.78)
                        } else {
                            theme.extended_palette().secondary.base.text.scale_alpha(0.9)
                        }),
                    }
                },)
            ]
            .spacing(6)
        )
        .padding(iced::Padding { top: 2.0, right: 8.0, bottom: 6.0, left: 8.0 }),
        // 搜索和列表区域
        container(
            column![
                // 搜索框 + 快捷按钮
                row![search.width(Length::Fill), providers_btn, manage_btn]
                    .spacing(8)
                    .align_y(Alignment::Center),
                // 可滚动的模型列表
                scrollable(container(model_list).padding(iced::Padding {
                    top: 0.0,
                    right: 12.0,
                    bottom: 0.0,
                    left: 0.0,
                }))
                .id(iced::widget::Id::new("input_panel_model_popover_scroll"))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fixed(260.0)),
            ]
            .spacing(8)
        )
        .padding(iced::Padding { top: 2.0, right: 8.0, bottom: 8.0, left: 8.0 }),
    ]
    .spacing(6);

    let content =
        container(boxed_content).style(popover_style).padding(4).width(Length::Fixed(popup_width));

    content.into()
}
