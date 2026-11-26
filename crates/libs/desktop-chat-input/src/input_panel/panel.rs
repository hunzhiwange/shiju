//! 输入面板公共控件与上下文卡片。
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{button, container, row, column, text};
use iced::{Element, Length, Alignment, Color, Theme};
use super::{surface_styles as styles, icons::{self, icon_svg}};
use super::composer_icons::ComposerIcon;
use desktop_chat_tools::state::SessionToolSelectorTab;
pub use desktop_chat_tools::selector_style::{agent_option, popover_style as agent_popover_style};
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

fn session_tool_tab_icon(tab: SessionToolSelectorTab) -> ComposerIcon {
    match tab {
        SessionToolSelectorTab::Agent => ComposerIcon::Agent,
        SessionToolSelectorTab::Tools => ComposerIcon::Tools,
        SessionToolSelectorTab::Skills => ComposerIcon::Skill,
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
    i18n: I18n,
    summary: String,
    adjust: Message,
    clear: Message,
) -> Element<'a, Message> {
    let content = column![
        text(i18n.__("已选工具与技能"))
            .size(12)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.palette().text.scale_alpha(0.92)),
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
    .width(Length::Fill);

    let adjust_icon = icon_svg(Icon::Sliders, styles::BOTTOM_BAR_ICON_SIZE)
        .style(|theme: &Theme, _| iced::widget::svg::Style {
            color: Some(theme.palette().primary.scale_alpha(0.92)),
        });
    let clear_icon = icon_svg(Icon::X, styles::BOTTOM_BAR_ICON_SIZE)
        .style(|theme: &Theme, _| iced::widget::svg::Style {
            color: Some(theme.palette().text.scale_alpha(0.88)),
        });
    let controls = row![
        button(content)
            .padding([8, 10])
            .width(Length::Fill)
            .style(styles::manual_context_card_button_style)
            .on_press(adjust.clone()),
        bottom_bar_round_toggle(adjust_icon.into(), i18n.__("调整"), adjust),
        bottom_bar_round_toggle(clear_icon.into(), i18n.__("清除已选工具与技能"), clear),
    ]
    .spacing(4)
    .align_y(Alignment::Center);
    container(controls)
        .padding(iced::Padding::ZERO.right(8))
        .width(Length::Fill)
        .style(styles::manual_context_card_style)
        .into()
}

pub fn session_tool_tab_toggle<'a, Message: Clone + 'a>(
    i18n: I18n,
    tab: SessionToolSelectorTab,
    highlight: bool,
    agent_label: String,
    acp_agent: Option<&str>,
    portrait: Option<iced::widget::image::Handle>,
    on_press: Message,
) -> Element<'a, Message> {
    if tab == SessionToolSelectorTab::Agent {
        return agent_toggle(highlight, agent_label, acp_agent, portrait, on_press);
    }
    let icon: Element<'_, Message> =
        session_tool_tab_icon(tab).svg(styles::BOTTOM_BAR_ICON_SIZE)
            .style(move |theme: &Theme, _| iced::widget::svg::Style {
                color: Some(styles::selector_text_color(theme, highlight)),
            })
            .into();
    let toggle = button(container(icon).center(styles::BOTTOM_BAR_ICON_BUTTON_SIZE))
        .padding(0)
        .style(move |theme, status| styles::composer_selector_button_style(theme, status, highlight))
        .on_press(on_press);
    Tooltip::new(toggle, dark_tooltip_content(i18n.__(tab.label())), TooltipPosition::Top)
        .gap(6.0)
        .into()
}

pub fn agent_toggle<'a, Message: Clone + 'a>(
    highlight: bool,
    label: String,
    acp_agent: Option<&str>,
    portrait: Option<iced::widget::image::Handle>,
    on_press: Message,
) -> Element<'a, Message> {
    let icon: Element<'_, Message> = if let Some(portrait) = portrait {
        iced::widget::image(portrait).width(18).height(18).into()
    } else if let Some(backend) = acp_agent {
        icons::acp_agent_icon(backend, styles::BOTTOM_BAR_ICON_SIZE)
    } else {
        ComposerIcon::Agent.svg(styles::BOTTOM_BAR_ICON_SIZE)
            .style(move |theme: &Theme, _| iced::widget::svg::Style {
                color: Some(styles::selector_text_color(theme, highlight)),
            })
            .into()
    };
    button(
        container(row![icon, text(label).size(styles::BOTTOM_BAR_LABEL_SIZE)]
            .spacing(6)
            .align_y(Alignment::Center))
            .center_y(styles::BOTTOM_BAR_ICON_BUTTON_SIZE),
    )
    .padding([0, 10])
    .style(move |theme, status| styles::composer_selector_button_style(theme, status, highlight))
    .on_press(on_press)
    .into()
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

/// 窄屏主操作始终独立成行，次级操作由宿主放入已有菜单。
pub fn mobile_controls<'a, Message: 'a>(
    menu: Element<'a, Message>,
    attachment: Element<'a, Message>,
    pool: Option<Element<'a, Message>>,
    cancel: Option<Element<'a, Message>>,
    send: Element<'a, Message>,
) -> Element<'a, Message> {
    let mut actions = row![menu, iced::widget::Space::new().width(Length::Fill), attachment]
        .spacing(4).align_y(Alignment::Center);
    if let Some(pool) = pool { actions = actions.push(pool); }
    if let Some(cancel) = cancel { actions = actions.push(container(cancel).padding(6)); }
    actions = actions.push(container(send).padding(6));
    actions.padding([4, 7]).width(Length::Fill).into()
}

/// 先采用已配置目录的标题，再回退市场目录和原始标识。
pub fn skill_title<'a>(skill_id: &str, mut titles: impl Iterator<Item = (&'a str, &'a str)>) -> String {
    titles.find(|(id, _)| *id == skill_id).map(|(_, title)| title.to_owned()).unwrap_or_else(|| skill_id.to_owned())
}

pub fn session_tool_tab_highlight(
    tab: SessionToolSelectorTab,
    popover_open: bool,
    active_tab: SessionToolSelectorTab,
    selector: &desktop_chat_tools::state::SessionToolSelectorState,
) -> bool {
    (popover_open && active_tab == tab) || match tab {
        SessionToolSelectorTab::Agent => false,
        SessionToolSelectorTab::Tools => !selector.selected_manual_tools().is_empty(),
        SessionToolSelectorTab::Skills => !selector.selected_manual_skills().is_empty() || !selector.selected_shiju_skills().is_empty(),
    }
}
pub fn agent_label<'a>(key: &str, selected_label: Option<&str>, mut labels: impl Iterator<Item = (&'a str, &'a str)>) -> String {
    selected_label.map(str::to_owned).or_else(|| labels.find(|(id, _)| *id == key).map(|(_, label)| label.to_owned())).unwrap_or_else(|| key.to_owned())
}
