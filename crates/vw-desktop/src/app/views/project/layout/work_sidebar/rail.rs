//! 收起工作侧栏后显示的图标导航。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::tooltip::{Position, Tooltip};
use iced::widget::{column, container, scrollable, svg, text};
use iced::{Element, Length, Radians};

use crate::app::assets::Icon;
use crate::app::components::input_panel::styles::tooltip_dark_style;
use crate::app::components::overlays::SideOverlay;
use crate::app::components::system_settings::SystemTab;
use crate::app::i18n::I18n;
use crate::app::message::ViewMessage;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::message::{ShijuAppsMessage, SkillHubMessage};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::skill_hub::ExpertHubSection;
use crate::app::state::WORK_SIDEBAR_RAIL_WIDTH;
use crate::app::{App, Message};

#[cfg(not(target_arch = "wasm32"))]
use super::expert_hub_message;
use super::{
    WORK_SIDEBAR_SCROLLBAR_GUTTER, WORK_SIDEBAR_SCROLLBAR_WIDTH, filter_control, icon_control,
    more_menu_action, more_menu_divider, more_menu_items, more_menu_style, navigation_icon_enabled,
    navigation_specs, project_create_message, project_navigation_message, refresh_all_control,
    search_control, sidebar_style, sidebar_toggle_control, task_pool_control,
};

const CONTROL_SIZE: f32 = 36.0;

pub(super) fn view(app: &App) -> Element<'static, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let navigation = column![control(
        i18n,
        "首页",
        Icon::Home,
        Some(Message::View(ViewMessage::WorkProjectHome)),
    )];
    #[cfg(not(target_arch = "wasm32"))]
    let navigation = navigation.push(control(
        i18n,
        "助手",
        Icon::Robot,
        Some(expert_hub_message(ExpertHubSection::Experts)),
    ));
    let controls = navigation
        .push(control(
            i18n,
            "定时任务",
            Icon::Clock,
            Some(Message::View(ViewMessage::OpenSystemSettingsTab(SystemTab::Cron))),
        ))
        .push(more_control(app, i18n))
        .push(container(more_menu_divider()).width(Length::Fixed(24.0)))
        .push(refresh_all_control(app))
        .push(task_pool_control(app))
        .push(search_control(i18n))
        .push(filter_control(app))
        .push(sidebar_toggle_control(app))
        .spacing(6)
        .align_x(iced::Alignment::Center)
        .width(Length::Fill);
    container(controls)
        .width(Length::Fixed(WORK_SIDEBAR_RAIL_WIDTH))
        .height(Length::Fill)
        .padding([10, 9])
        .style(sidebar_style)
        .into()
}

fn control(
    i18n: I18n,
    label: &'static str,
    icon: Icon,
    message: Option<Message>,
) -> Element<'static, Message> {
    let icon = navigation_icon_enabled(icon, message.is_some());
    with_tooltip(i18n, label, icon, message, false)
}

fn with_tooltip(
    i18n: I18n,
    label: &'static str,
    icon: svg::Svg<'static>,
    message: Option<Message>,
    active: bool,
) -> Element<'static, Message> {
    let button = icon_control(icon, message, active, CONTROL_SIZE);
    if active {
        return button.into();
    }
    Tooltip::new(
        button,
        container(text(i18n.__(label)).size(12)).padding([6, 8]).style(tooltip_dark_style),
        Position::Right,
    )
    .gap(8)
    .into()
}

fn more_control(app: &App, i18n: I18n) -> Element<'static, Message> {
    let icon = navigation_icon_enabled(Icon::DotsThreeVertical, true)
        .rotation(Radians(std::f32::consts::FRAC_PI_2));
    let trigger = with_tooltip(
        i18n,
        "更多",
        icon,
        Some(Message::View(ViewMessage::WorkSidebarMoreToggled)),
        app.work_sidebar_more_open,
    );
    SideOverlay::new(trigger, more_menu(i18n))
        .show(app.work_sidebar_more_open)
        .gap(12.0)
        .on_close(Message::View(ViewMessage::WorkSidebarMoreClosed))
        .into()
}

fn more_menu(i18n: I18n) -> Element<'static, Message> {
    let specs = navigation_specs();
    let content = column![
        more_menu_action(i18n, specs[2].0, specs[2].1, project_navigation_message()),
        more_menu_action(i18n, "新建项目", Icon::Plus, project_create_message()),
    ];
    #[cfg(not(target_arch = "wasm32"))]
    let content = content
        .push(more_menu_action(
            i18n,
            specs[3].0,
            specs[3].1,
            Message::ShijuApps(ShijuAppsMessage::Open),
        ))
        .push(more_menu_action(
            i18n,
            specs[4].0,
            specs[4].1,
            Message::SkillHub(SkillHubMessage::OpenShijuSkills),
        ))
        .push(more_menu_action(
            i18n,
            specs[5].0,
            specs[5].1,
            Message::SkillHub(SkillHubMessage::Open),
        ))
        .push(more_menu_action(
            i18n,
            specs[7].0,
            specs[7].1,
            expert_hub_message(ExpertHubSection::Knowledge),
        ));

    let items = content.push(more_menu_divider()).push(more_menu_items(i18n)).spacing(3);
    let scroll = scrollable(
        container(items).padding(iced::Padding::default().right(WORK_SIDEBAR_SCROLLBAR_GUTTER)),
    )
    .direction(Direction::Vertical(
        Scrollbar::new()
            .width(WORK_SIDEBAR_SCROLLBAR_WIDTH)
            .scroller_width(WORK_SIDEBAR_SCROLLBAR_WIDTH),
    ));

    container(scroll).padding([12, 14]).width(Length::Fixed(252.0)).style(more_menu_style).into()
}
