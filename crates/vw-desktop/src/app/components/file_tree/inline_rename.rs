//! 文件树原位重命名，保留图标和层级，不接入行选择或拖拽。
use iced::widget::{container, row, text, text_input};
use iced::{Alignment, Element, Length};

use super::icons::{file_icon_for, static_icon_svg, themed_icon_svg};
use crate::app::assets::Icon;
use crate::app::components::system_settings_common::settings_text_input;
use crate::app::message::ProjectMessage;
use crate::app::{App, Message};

pub(crate) const INPUT_ID: &str = "file-tree-inline-rename";

pub(crate) fn selection_end(name: &str, directory: bool) -> usize {
    let selected = if directory {
        name
    } else {
        std::path::Path::new(name).file_stem().and_then(|stem| stem.to_str()).unwrap_or(name)
    };
    text_input::Value::new(selected).len()
}

pub(super) fn view(
    app: &App,
    depth: usize,
    directory: bool,
    expanded: bool,
) -> Element<'_, Message> {
    let mut content = row![text("  ".repeat(depth)).size(13)].spacing(4).align_y(Alignment::Center);
    if directory {
        content = content.push(themed_icon_svg(if expanded {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        }));
    }
    let icon = if directory {
        themed_icon_svg(Icon::FolderOpen)
    } else {
        static_icon_svg(file_icon_for(&app.file_tree_rename_value))
    };
    content = content.push(container(icon).width(Length::Fixed(16.0))).push(
        settings_text_input("", &app.file_tree_rename_value)
            .id(INPUT_ID)
            .on_input(|value| Message::Project(ProjectMessage::FileTreeRenameChanged(value)))
            .on_submit(Message::Project(ProjectMessage::FileTreeRenameSave))
            .width(Length::Fill),
    );
    container(content).padding([2, 5]).width(Length::Fill).into()
}

#[cfg(test)]
#[path = "inline_rename_tests.rs"]
mod inline_rename_tests;
