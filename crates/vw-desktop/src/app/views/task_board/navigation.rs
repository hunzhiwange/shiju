use crate::app::components::system_settings::SystemTab;
use crate::app::components::system_settings_common::{
    icon_svg, primary_action_btn_style, settings_divider,
};
use crate::app::i18n::I18n;
use crate::app::message::{ProjectMessage, TaskBoardMessage, ViewMessage};
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{
    icons::icon_btn_svg_style, settings_divider_style,
};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let create = entry(
        Icon::Plus,
        i18n.__("新建"),
        app.navigation_tabs.project_path.as_ref().map(|_| TaskBoardMessage::CreateTask.into()),
        false,
        true,
    );
    let rail = column![
        create,
        entry(
            Icon::Search,
            i18n.__("搜索"),
            Some(TaskBoardMessage::SearchWorkTasks(Some(String::new())).into()),
            false,
            false,
        ),
        container(settings_divider()).padding([4, 5]),
        entry(
            Icon::Clipboard,
            i18n.__("工作"),
            Some(if app.task_board.task_board_worktree_pixel_office {
                TaskBoardMessage::ToggleWorktreePixelOffice(false).into()
            } else {
                TaskBoardMessage::CloseTaskLogs.into()
            }),
            true,
            false,
        ),
        entry(Icon::ChatTextFill, i18n.__("聊天"), Some(open_chat(app)), false, false,),
        entry(
            Icon::Clock,
            i18n.__("自动化"),
            Some(Message::View(ViewMessage::OpenCron)),
            false,
            false,
        ),
        entry(
            Icon::Robot,
            i18n.__("团队"),
            Some(Message::View(ViewMessage::OpenSystemSettingsTab(SystemTab::Agents))),
            false,
            false,
        ),
    ]
    .spacing(5)
    .width(Length::Fill)
    .height(Length::Fill);
    row![
        container(rail)
            .width(78)
            .height(Length::Fill)
            .padding([12, 7])
            .style(super::style::rail_surface),
        container(Space::new()).width(1).height(Length::Fill).style(settings_divider_style),
    ]
    .into()
}

fn open_chat(app: &App) -> Message {
    let Some(path) = app.navigation_tabs.project_path.as_deref() else {
        return Message::View(ViewMessage::GoHome);
    };
    let (tab_id, _) = desktop_workspace::projects::project_tab(path);
    if app.navigation_tabs.open_tabs.iter().any(|tab| tab.id == tab_id) {
        Message::View(ViewMessage::TabSelected(tab_id))
    } else {
        Message::Project(ProjectMessage::OpenRecentPressed(path.to_owned()))
    }
}

fn entry(
    icon: Icon,
    label: &str,
    message: Option<Message>,
    selected: bool,
    primary: bool,
) -> Element<'_, Message> {
    let mut glyph = icon_svg(icon, 17.0);
    if primary {
        let enabled = message.is_some();
        glyph = glyph.style(move |theme, _| iced::widget::svg::Style {
            color: Some(
                primary_action_btn_style(
                    theme,
                    if enabled { button::Status::Active } else { button::Status::Disabled },
                )
                .text_color,
            ),
        });
    } else {
        glyph = glyph.style(icon_btn_svg_style);
    }
    button(
        column![glyph, text(label).size(10)]
            .spacing(4)
            .width(Length::Fill)
            .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(48)
    .padding([6, 3])
    .style(move |theme, status| {
        if primary {
            let mut style = primary_action_btn_style(theme, status);
            style.border = style.border.rounded(8);
            style
        } else {
            super::style::navigation_button(theme, status, selected)
        }
    })
    .on_press_maybe(message)
    .into()
}
