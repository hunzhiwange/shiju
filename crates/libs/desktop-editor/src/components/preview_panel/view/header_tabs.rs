//! 文件编辑器标签栏。空间不足时，右侧下拉列出其余打开的文件。

use std::{collections::HashMap, fmt};

use desktop_assets::assets::Icon;
use desktop_ui_controls::overlays::PointBelowOverlay;
use desktop_ui_controls::right_click_area::RightClickArea;
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_ui_controls::system_settings_common::settings_select;
use iced::widget::{button, container, responsive, row};
use iced::{Background, Color, Element, Length, Point, alignment};

use super::super::styles::{
    estimate_tab_width_px, file_icon_for, file_tab_icon_svg, small_icon_svg,
};
use super::super::tabs::build_preview_tab_menu;
use super::super::widgets::DraggableArea;
use super::HeaderMessage as Message;

const TAB_HEIGHT: f32 = 28.0;
const TAB_SPACING: f32 = 4.0;
const CONTROL_WIDTH: f32 = 28.0;
const MIN_TAB_WIDTH: f32 = 64.0;
const MAX_TAB_WIDTH: f32 = 200.0;

pub fn tab_is_selected(active_path: Option<&str>, tab_path: &str) -> bool {
    active_path == Some(tab_path)
}

pub fn close_button_style(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let p = theme.palette().primary;
    let hover_bg = Color::from_rgba(p.r, p.g, p.b, 0.10);
    let pressed_bg = Color::from_rgba(p.r, p.g, p.b, 0.18);
    let bg = match status {
        iced::widget::button::Status::Hovered => Some(Background::Color(hover_bg)),
        iced::widget::button::Status::Pressed => Some(Background::Color(pressed_bg)),
        _ => None,
    };

    iced::widget::button::Style {
        background: bg,
        text_color: theme.palette().text,
        border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 4.0.into() },
        ..Default::default()
    }
}

pub fn tab_button_style(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
    selected: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let base = palette.background.base.color;
    let luma = 0.2126 * base.r + 0.7152 * base.g + 0.0722 * base.b;
    let is_dark = luma < 0.5;
    let primary = palette.primary.base.color;
    let hover_bg = if is_dark {
        palette.background.strong.color.scale_alpha(0.8)
    } else {
        Color::from_rgba(primary.r, primary.g, primary.b, 0.10)
    };
    let selected_bg = if is_dark {
        palette.background.strong.color.scale_alpha(0.95)
    } else {
        Color::from_rgba(primary.r, primary.g, primary.b, 0.18)
    };
    let base_bg = if selected { Some(Background::Color(selected_bg)) } else { None };
    let bg = match status {
        iced::widget::button::Status::Hovered => Some(Background::Color(hover_bg)),
        iced::widget::button::Status::Pressed => Some(Background::Color(selected_bg)),
        _ => base_bg,
    };

    iced::widget::button::Style {
        background: bg,
        text_color: theme.palette().text,
        border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 4.0.into() },
        ..Default::default()
    }
}

pub fn tab_drag_start_message(path: &str) -> Message {
    Message::DragStart(path.to_string())
}

pub fn tab_drag_end_message() -> Message {
    Message::DragEnd
}

pub fn tab_close_message(path: &str) -> Message {
    Message::Close(path.to_string())
}

pub fn tab_select_message(path: &str) -> Message {
    Message::Select(path.to_string())
}

pub fn open_file_message() -> Message {
    Message::OpenFile
}

pub fn tab_right_click_message(path: &str, pos: Point) -> Message {
    Message::RightClick(path.to_string(), pos.x, pos.y)
}

pub fn tab_context_menu_matches(menu_path: Option<&str>, tab_path: &str) -> bool {
    menu_path == Some(tab_path)
}

pub fn tab_menu_anchor(menu_pos: Option<Point>) -> Point {
    menu_pos.unwrap_or(Point::ORIGIN)
}

fn preview_tab<'a, M: Clone + 'a>(
    title: &str,
    selected: bool,
    select: M,
    close: M,
) -> Element<'a, M> {
    let icon = file_icon_for(title);
    let close =
        button(small_icon_svg(Icon::X)).on_press(close).padding(2).style(close_button_style);
    let content = row![
        container(file_tab_icon_svg(icon)).width(Length::Fixed(16.0)),
        container(SingleLineLabel::new(title.to_owned(), 13.0))
            .width(Length::Fill)
            .align_y(iced::alignment::Vertical::Center),
        container(close).width(Length::Shrink),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center)
    .width(Length::Fill);

    button(container(content).padding([4, 6]).height(TAB_HEIGHT).width(Length::Fill))
        .on_press(select)
        .style(move |theme, status| tab_button_style(theme, status, selected))
        .padding(0)
        .width(Length::Fill)
        .into()
}

pub struct FileTab<'a, Key, M> {
    pub key: Key,
    pub title: &'a str,
    pub path: &'a str,
    pub selected: bool,
    pub select: M,
    pub close: M,
}

#[derive(Clone)]
struct OverflowTab<Key, M> {
    key: Key,
    label: String,
    select: M,
}

impl<Key: PartialEq, M> PartialEq for OverflowTab<Key, M> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.label == other.label
    }
}

impl<Key, M> fmt::Display for OverflowTab<Key, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

pub(super) fn visible_tab_widths(
    widths: &[f32],
    active: Option<usize>,
    available: f32,
    has_open_file: bool,
) -> (Vec<Option<f32>>, bool) {
    let control_space = CONTROL_WIDTH + TAB_SPACING;
    let tabs_space = available - if has_open_file { control_space } else { 0.0 };
    let total = widths.iter().sum::<f32>() + widths.len().saturating_sub(1) as f32 * TAB_SPACING;
    if widths.is_empty() || total <= tabs_space {
        return (widths.iter().copied().map(Some).collect(), has_open_file);
    }
    if widths.len() == 1 && tabs_space >= MIN_TAB_WIDTH {
        return (vec![Some(tabs_space)], has_open_file);
    }

    let show_open_file = has_open_file && available >= CONTROL_WIDTH + control_space;
    let mut remaining = available - CONTROL_WIDTH;
    if show_open_file {
        remaining -= control_space;
    }

    // 为当前文件预留位置，切换隐藏文件后可直接看到对应标签。
    let mut visible = vec![None; widths.len()];
    if let Some(active) = active {
        let width = widths[active].min(remaining - TAB_SPACING);
        if width >= MIN_TAB_WIDTH {
            visible[active] = Some(width);
            remaining -= width + TAB_SPACING;
        }
    }

    for (index, &width) in widths.iter().enumerate() {
        if Some(index) == active {
            continue;
        }
        let occupied = width + TAB_SPACING;
        if occupied > remaining {
            break;
        }
        visible[index] = Some(width);
        remaining -= occupied;
    }

    (visible, show_open_file)
}

pub fn file_tab_strip<'a, Key, M>(
    tabs: Vec<FileTab<'a, Key, M>>,
    open_file: Option<M>,
    decorate: impl Fn(&Key, Element<'a, M>) -> Element<'a, M> + 'a,
) -> Element<'a, M>
where
    Key: Clone + PartialEq + 'static,
    M: Clone + 'static,
{
    let widths: Vec<_> =
        tabs.iter().map(|tab| estimate_tab_width_px(tab.title).min(MAX_TAB_WIDTH)).collect();
    let active = tabs.iter().position(|tab| tab.selected);
    responsive(move |size| {
        let (visible, show_open_file) =
            visible_tab_widths(&widths, active, size.width, open_file.is_some());
        let mut strip = row(tabs.iter().zip(&visible).filter_map(|(tab, width)| {
            let width = (*width)?;
            let button =
                preview_tab(tab.title, tab.selected, tab.select.clone(), tab.close.clone());
            Some(container(decorate(&tab.key, button)).width(width).into())
        }))
        .spacing(TAB_SPACING)
        .align_y(iced::Alignment::Center)
        .width(Length::Fill);
        let open_button = open_file.as_ref().filter(|_| show_open_file).map(|action| {
            button(container(small_icon_svg(Icon::Plus)).center(Length::Fill))
                .width(CONTROL_WIDTH)
                .height(TAB_HEIGHT)
                .on_press(action.clone())
                .padding(0)
                .style(close_button_style)
        });

        if visible.iter().any(Option::is_none) {
            let dropdown = overflow_dropdown(&tabs, &visible);
            let actions =
                row![open_button, dropdown].spacing(TAB_SPACING).align_y(iced::Alignment::Center);
            strip = strip
                .push(container(actions).width(Length::Fill).align_x(alignment::Horizontal::Right));
        } else if let Some(open_button) = open_button {
            strip = strip.push(open_button);
        }

        strip.into()
    })
    .height(Length::Fixed(30.0))
    .into()
}

fn overflow_dropdown<'a, Key, M>(
    tabs: &[FileTab<'a, Key, M>],
    visible: &[Option<f32>],
) -> Element<'a, M>
where
    Key: Clone + PartialEq + 'static,
    M: Clone + 'static,
{
    let mut counts = HashMap::new();
    for tab in tabs {
        *counts.entry(tab.title).or_insert(0) += 1;
    }

    let mut selected = None;
    let options = tabs
        .iter()
        .zip(visible)
        .filter_map(|(tab, width)| {
            if width.is_some() {
                return None;
            }

            let label = if counts[tab.title] > 1 && tab.path != tab.title {
                format!("{} · {}", tab.title, tab.path)
            } else {
                tab.title.to_owned()
            };
            let option = OverflowTab { key: tab.key.clone(), label, select: tab.select.clone() };
            if tab.selected {
                selected = Some(option.clone());
            }
            Some(option)
        })
        .collect::<Vec<_>>();

    settings_select(options, selected, |tab| tab.select)
        .menu_icon(Icon::ChevronDown, 260.0)
        .width(Length::Fixed(CONTROL_WIDTH))
        .height(TAB_HEIGHT)
        .roomy_menu(true)
        .align_menu_right()
        .open_on_right_click()
        .into()
}

pub fn build_header_tabs<'a>(
    tabs_data: impl IntoIterator<Item = (&'a str, &'a str)>,
    active_path: Option<&str>,
    menu_path: Option<&'a str>,
    menu_pos: Option<Point>,
    i18n: desktop_i18n::I18n,
) -> Element<'a, Message> {
    let tabs = tabs_data
        .into_iter()
        .map(|(path, title)| FileTab {
            key: path.to_owned(),
            title,
            path,
            selected: tab_is_selected(active_path, path),
            select: tab_select_message(path),
            close: tab_close_message(path),
        })
        .collect();

    file_tab_strip(tabs, Some(open_file_message()), move |path, tab| {
        let draggable = Element::new(DraggableArea::new(
            tab,
            tab_drag_start_message(path),
            tab_drag_end_message(),
        ));
        let menu_target = path.clone();
        let tab = RightClickArea::new(
            draggable,
            Box::new(move |position| tab_right_click_message(&menu_target, position)),
        );

        if tab_context_menu_matches(menu_path, path) {
            PointBelowOverlay::new(tab, build_preview_tab_menu(i18n, path).map(Message::TabMenu))
                .show(true)
                .anchor(tab_menu_anchor(menu_pos))
                .on_close(Message::MenuClose)
                .capture_outside_click(false)
                .into()
        } else {
            tab.into()
        }
    })
}
