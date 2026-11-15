//! 智能体目录页面；不读取桌面 App。
use crate::{catalog, message::{Event, AgentsMessage}, agents_state::AgentsSettingsState};
use crate::profile_modal::AGENT_CONFIGURATION_HELP;
use desktop_ui_controls::system_settings_common::*;
use desktop_i18n::{I18n, settings_extensions::text as tr};
use iced::{Element, Length, Theme};
use iced::widget::{button, column, container, scrollable, text, scrollable::{Direction, Scrollbar}};
pub struct Input<'a> { pub state: &'a AgentsSettingsState, pub catalog: catalog::Input<'a>, pub theme: &'a Theme }
pub fn agents_view(input: Input<'_>) -> Element<'_, Event> {
    let settings = input.state;
    let title = settings_page_heading_with_help(
        I18n::new(input.catalog.locale),
        input.theme,
        tr(input.catalog.locale, "智能体配置"),
        tr(input.catalog.locale, AGENT_CONFIGURATION_HELP),
    );
    if !settings.config_ready {
        return column![
            title,
            settings_group_card(
                column![
                    settings_group_heading(
                        tr(input.catalog.locale, "配置尚未就绪"),
                        tr(
                            input.catalog.locale,
                            "为防止启动默认值覆盖已保存配置，加载成功前此页面保持只读。"
                        ),
                    ),
                    text(tr(
                        input.catalog.locale,
                        "正在等待 Gateway 返回当前作用域的智能体配置；完成前不会使用默认值保存。"
                    ))
                    .size(12)
                    .style(settings_muted_text_style),
                    button(
                        text(tr(input.catalog.locale, "重新加载")).size(SETTINGS_CONTROL_TEXT_SIZE)
                    )
                    .padding([8, 14])
                    .on_press(Event::Agents(AgentsMessage::Refresh))
                    .style(rounded_action_btn_style),
                ]
                .spacing(10),
            )
            .padding([16, 16]),
        ]
        .spacing(14)
        .width(Length::Fill)
        .into();
    }

    let mut content = column![
        title,
        catalog::controls(input.catalog),
        scrollable(container(catalog::grid(input.catalog)).padding([0, 7]))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill),
    ]
    .spacing(14)
    .width(Length::Fill)
    .height(Length::Fill);

    if settings.loading {
        content = content.push(
            text(tr(input.catalog.locale, "正在刷新提供商 / 模型 / 工具列表 ..."))
                .size(12)
                .style(settings_muted_text_style),
        );
    }

    content.into()
}

pub fn agent_project_options(current: Option<&str>, recent: &[String]) -> Vec<String> {
    let mut options = Vec::new();
    for path in current.into_iter().chain(recent.iter().map(String::as_str)) {
        let path = path.trim();
        if !path.is_empty() && !options.iter().any(|option: &String| option.as_str() == path) {
            options.push(path.to_string());
        }
    }
    options
}

#[cfg(test)]
#[path="agents_view_tests.rs"] mod tests;
