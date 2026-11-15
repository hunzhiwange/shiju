//! 输入面板公共控件与上下文卡片。
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{button, container, row, column, text};
use iced::{Element, Length, Alignment, Color, Theme};
use super::{surface_styles as styles, icons::{self, icon_svg}};
use desktop_chat_tools::state::SessionToolSelectorTab;
#[cfg(test)]
pub(super) fn popover_item_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    selected: bool,
) -> iced::widget::button::Style {
    styles::selectable_list_button_style(theme, status, selected)
}

pub(super) fn bottom_bar_round_toggle<'a, Message: Clone + 'a>(
    icon: Element<'a, Message>,
    tooltip_label: impl Into<String>,
    on_press: Message,
) -> Element<'a, Message> {
    let button_content = container(icon)
        .width(Length::Fixed(styles::BOTTOM_BAR_ICON_BUTTON_SIZE))
        .height(Length::Fixed(styles::BOTTOM_BAR_ICON_BUTTON_SIZE))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    let toggle = button(button_content)
        .padding(0)
        .style(|theme: &Theme, status| styles::round_icon_button_style(theme, status, true))
        .on_press(on_press);

    Tooltip::new(toggle, dark_tooltip_content(tooltip_label), TooltipPosition::Top).gap(6.0).into()
}

fn session_tool_tab_icon(tab: SessionToolSelectorTab) -> Icon {
    // 与 slash 面板语义对齐：技能=星标、工具=终端；代理用六边形身份标记，避免像聊天气泡。
    match tab {
        SessionToolSelectorTab::Agent => Icon::Hexagon,
        SessionToolSelectorTab::Tools => Icon::Terminal,
        SessionToolSelectorTab::Skills => Icon::Star,
    }
}

fn dark_tooltip_content<'a, Message: 'a>(label: impl Into<String>) -> Element<'a, Message> {
    container(
        text(label.into())
            .size(12)
            .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
    )
    .style(styles::tooltip_dark_style)
    .padding([6, 8])
    .into()
}

pub(super) fn summarize_names(names: &[String]) -> String {
    const MAX_VISIBLE_NAMES: usize = 3;
    let mut preview = names.iter().take(MAX_VISIBLE_NAMES).cloned().collect::<Vec<_>>().join("、");
    let hidden_count = names.len().saturating_sub(MAX_VISIBLE_NAMES);
    if hidden_count > 0 {
        preview.push_str(&format!(" +{hidden_count}"));
    }
    preview
}

pub fn selected_context_card<'a, Message: Clone + 'a>(
    i18n: I18n, summary: String, adjust: Message, cancel_expert: Option<Message>,
) -> Element<'a, Message> {

    let icon: Element<'_, Message> = icon_svg(Icon::Sliders, 14.0)
        .style(|theme: &Theme, _| iced::widget::svg::Style {
            color: Some(theme.palette().primary.scale_alpha(0.92)),
        })
        .into();
    let content = row![
        icon,
        column![
            text(i18n.__("已选工具与技能"))
                .size(12)
                .style(|theme: &Theme| {
                    iced::widget::text::Style {
                        color: Some(theme.palette().text.scale_alpha(0.92)),
                    }
                }),
            text(summary)
                .size(11)
                .width(Length::Fill)
                .wrapping(iced::widget::text::Wrapping::Word)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.palette().text.scale_alpha(0.68)),
                }),
        ]
        .spacing(2)
        .width(Length::Fill),
        text(i18n.__("调整")).size(11).style(
            |theme: &Theme| iced::widget::text::Style {
                color: Some(theme.palette().primary.scale_alpha(0.88)),
            }
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut controls = row![
        button(content)
            .padding([8, 10])
            .width(Length::Fill)
            .style(styles::manual_context_card_button_style)
            .on_press(adjust)
    ]
    .align_y(Alignment::Center);
    if let Some(action) = cancel_expert {
        controls = controls.push(
            button(text(i18n.__("取消专家")).size(11))
                .padding([8, 10])
                .style(styles::manual_context_card_button_style)
                .on_press(action),
        );
    }
    container(controls)
            .padding(0)
            .width(Length::Fill)
            .style(styles::manual_context_card_style)
            .into()
}

pub fn session_tool_tab_toggle<'a, Message: Clone + 'a>(tab: SessionToolSelectorTab, highlight: bool, agent_label: String, acp_agent: Option<&str>, on_press: Message) -> Element<'a, Message> {
    let icon: Element<'_, Message> =
        icon_svg(session_tool_tab_icon(tab), styles::BOTTOM_BAR_ICON_SIZE)
            .style(move |theme: &Theme, _| iced::widget::svg::Style {
                color: Some(styles::selector_text_color(theme, highlight)),
            })
            .into();
    if tab == SessionToolSelectorTab::Agent {
        let label = agent_label;
        let icon = acp_agent.map(|backend| icons::acp_agent_icon(backend, styles::BOTTOM_BAR_ICON_SIZE)).unwrap_or(icon);
        button(
            row![icon, text(label).size(styles::BOTTOM_BAR_LABEL_SIZE)]
                .spacing(6)
                .align_y(Alignment::Center),
        )
        .padding([4, 10])
        .style(move |theme: &Theme, status| {
            styles::selector_pill_button_style(theme, status, highlight)
        })
        .on_press(on_press)
        .into()
    } else {
        bottom_bar_round_toggle(icon, tab.label(), on_press)
    }
}

pub fn selected_context_summary(i18n: I18n, tool_names: &[String], skill_names: &[String], shiju_names: &[String]) -> Option<String> {
    let mut lines = Vec::new();
    if !tool_names.is_empty() {
        lines.push(
            desktop_i18n::__!(
                i18n,
                "工具 %u：%s",
                tool_names.len(),
                summarize_names(tool_names)
            )
            .expect("valid context template"),
        );
    }
    if !skill_names.is_empty() {
        lines.push(
            desktop_i18n::__!(
                i18n,
                "技能 %u：%s",
                skill_names.len(),
                summarize_names(skill_names)
            )
            .expect("valid context template"),
        );
    }
    if !shiju_names.is_empty() {
        lines.push(
            desktop_i18n::__!(
                i18n,
                "识矩 %u：%s",
                shiju_names.len(),
                summarize_names(shiju_names)
            )
            .expect("valid skill context template"),
        );
    }

    (!lines.is_empty()).then(|| lines.join("  /  "))
}
