//! 文件树原位重命名，保留图标和层级，不接入行选择或拖拽。
use iced::widget::{container, text_input};
use iced::{Background, Color, Element, Length, Theme};

use super::icons::{file_icon_for, static_icon_svg, themed_icon_svg};
use super::{row_layout, tree_gutter};
use crate::app::components::system_settings_common::{
    settings_text_input, settings_text_input_style,
};
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

fn input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut style = settings_text_input_style(theme, status);
    style.background = Background::Color(Color::TRANSPARENT);
    style.border = iced::Border::default();
    style
}

pub(super) fn view(
    app: &App,
    depth: usize,
    directory: bool,
    expanded: bool,
) -> Element<'_, Message> {
    let icon = if directory {
        themed_icon_svg(row_layout::folder_icon(expanded))
    } else {
        static_icon_svg(file_icon_for(&app.file_tree_rename_value))
    };
    let content = row_layout::content(
        depth,
        icon,
        settings_text_input("", &app.file_tree_rename_value)
            // 行内编辑沿用文件名字号与行高，不使用表单控件的额外留白。
            .padding(0)
            .size(13)
            .style(input_style)
            .id(INPUT_ID)
            .on_input(|value| Message::Project(ProjectMessage::FileTreeRenameChanged(value)))
            .on_submit(Message::Project(ProjectMessage::FileTreeRenameSave))
            .width(Length::Fill),
    );
    tree_gutter::view(
        depth,
        container(content).padding(row_layout::ROW_PADDING).width(Length::Fill).into(),
    )
}

#[cfg(test)]
#[path = "inline_rename_tests.rs"]
mod inline_rename_tests;
