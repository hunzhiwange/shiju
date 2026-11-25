//! 独立工作台中的变更文件导航。
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::{
    icon_svg, rounded_action_btn_style, settings_checkbox_style, settings_muted_text_style,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, checkbox, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use super::{DiffFileMeta, Message, ViewContext};
use crate::git_panel::utils::FileStatus;
use crate::message_git::messages::GitMessage;

pub(super) fn change_list<'a>(
    app: &ViewContext<'a>,
    files: &[DiffFileMeta],
    commit_form: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let mut items = column![].spacing(2).width(Length::Fill);
    if app.git.git_diff_file_metas_loading && files.is_empty() {
        items = items.push(text(i18n.__("加载中")).size(12).style(settings_muted_text_style));
    }
    for file in files {
        let (parent, name) = file.path.rsplit_once('/').unwrap_or(("", &file.path));
        let selected = app.git.git_focused_file.as_deref() == Some(file.path.as_str());
        let name = crate::git_panel::repository::ellipsize(name, 24);
        let parent = crate::git_panel::repository::ellipsize(parent, 29);
        let mut label = column![text(name).size(13)].spacing(3).width(Length::Fill);
        if !parent.is_empty() {
            label = label.push(text(parent).size(11).style(settings_muted_text_style));
        }
        let status = match file.status {
            FileStatus::Added => "A",
            FileStatus::Deleted => "D",
            FileStatus::Modified => "M",
            FileStatus::Renamed => "R",
            FileStatus::Untracked => "U",
            FileStatus::Unknown => "?",
        };
        let item = button(
            row![
                icon_svg(Icon::FileText, 14.0),
                label,
                text(status).size(12).style(settings_muted_text_style)
            ]
            .spacing(7)
            .align_y(Alignment::Center),
        )
        .padding([6, 8])
        .width(Length::Fill)
        .style(move |theme, status| {
            let mut style = rounded_action_btn_style(theme, status);
            if selected && status == button::Status::Active {
                style.background =
                    Some(theme.extended_palette().primary.base.color.scale_alpha(0.16).into());
            }
            style
        })
        .on_press(Message::Git(GitMessage::FocusFile(file.path.clone())));
        let path = file.path.clone();
        let selection = checkbox(app.git.staged_files_selected.contains(&path))
            .size(14)
            .style(settings_checkbox_style)
            .on_toggle(move |checked| {
                Message::Git(GitMessage::ToggleStageFile(path.clone(), checked))
            });
        items =
            items.push(row![container(selection).padding([0, 6]), item].align_y(Alignment::Center));
    }

    let header = row![
        text(i18n.__("变更文件")).size(12).width(Length::Fill),
        text(files.len().to_string()).size(12).style(settings_muted_text_style),
    ]
    .spacing(6);
    container(column![
        container(header).padding([7, 8]),
        scrollable(container(items).padding(iced::Padding { right: 7.0, ..iced::Padding::ZERO }))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill),
        commit_form,
    ])
    .width(282)
    .height(Length::Fill)
    .style(|theme| container::Style {
        background: Some(theme.extended_palette().background.weak.color.scale_alpha(0.33).into()),
        ..Default::default()
    })
    .into()
}
