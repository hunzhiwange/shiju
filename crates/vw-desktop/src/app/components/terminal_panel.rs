//! 终端面板组件模块
//!
//! 该模块提供终端界面的完整视图渲染功能，包括：
//! - 多标签页管理（添加、切换、关闭）
//! - 标签页右键上下文菜单（重命名、关闭）
//! - 终端内容显示区域与选区右键菜单
//! - 标签页重命名模态对话框
//!
//! 该模块是 VibeWindow 应用的终端 UI 层核心组件，
//! 负责将终端状态转换为 Iced 框架的可渲染元素。

use iced::widget::{
    Space, button, canvas, column, container, row, scrollable, svg, text, text_input,
};
use iced::{Background, Border, Color, Element, Font, Length, Point, Theme};

use crate::app::assets::{self, Icon};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::components::input_panel::styles::tooltip_dark_style;
use crate::app::components::overlays::point_below::PointBelowOverlay;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_close_button, settings_modal_card, settings_modal_header,
    settings_modal_overlay_with_backdrop_dismiss, settings_muted_text_style,
    settings_text_input_style,
};
use crate::app::components::widgets::{RightClickArea, SingleLineLabel};
use crate::app::i18n::I18n;
use crate::app::terminal::TerminalTab;
use crate::app::{App, Message, message};

#[cfg(not(target_arch = "wasm32"))]
#[path = "terminal_content_menu.rs"]
mod content_menu;
#[cfg(not(target_arch = "wasm32"))]
#[path = "terminal_pane_grid.rs"]
mod pane_grid;

#[cfg(not(target_arch = "wasm32"))]
#[path = "terminal_pane_drag.rs"]
mod pane_drag;
#[cfg(not(target_arch = "wasm32"))]
#[path = "terminal_split_menu.rs"]
mod split_menu;

const TAB_TITLE_ID: &str = "terminal-tab-title";
const TAB_STRIP_ID: &str = "terminal-tab-strip";

const TAB_HEADER_HEIGHT: f32 = 34.0;
const TAB_SCROLLBAR_WIDTH: f32 = 4.0;
const TAB_SCROLLBAR_GAP: f32 = 3.0;

struct TabCloseIcon {
    enabled: bool,
    active: bool,
}

impl canvas::Program<Message> for TabCloseIcon {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let palette = theme.palette();
        let extended = theme.extended_palette();
        let hovered = self.enabled && cursor.is_over(bounds);
        let color = if !self.enabled {
            palette.text.scale_alpha(0.28)
        } else if hovered {
            extended.danger.base.color
        } else if self.active {
            extended.primary.base.color
        } else if palette.background.r + palette.background.g + palette.background.b < 1.5 {
            Color::from_rgb8(238, 238, 238)
        } else {
            palette.text
        };
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let half_span = 3.4;
        let cross = canvas::Path::new(|path| {
            path.move_to(Point::new(center.x - half_span, center.y - half_span));
            path.line_to(Point::new(center.x + half_span, center.y + half_span));
            path.move_to(Point::new(center.x - half_span, center.y + half_span));
            path.line_to(Point::new(center.x + half_span, center.y - half_span));
        });
        frame.stroke(
            &cross,
            canvas::Stroke::default()
                .with_color(color)
                .with_width(if hovered { 1.6 } else { 0.7 })
                .with_line_cap(canvas::LineCap::Round),
        );
        vec![frame.into_geometry()]
    }
}

pub(super) fn terminal_tab_width(title: &str) -> f32 {
    (title.chars().count() as f32 * 8.4 + 62.0).clamp(92.0, 260.0)
}

pub(super) fn terminal_tab_is_active(active_id: Option<u64>, tab_id: u64, index: usize) -> bool {
    active_id.map_or(index == 0, |active_id| active_id == tab_id)
}

pub(super) fn active_terminal_tab_index(
    active_id: Option<u64>,
    tab_ids: impl IntoIterator<Item = u64>,
) -> Option<usize> {
    let mut first_index = None;

    for (index, tab_id) in tab_ids.into_iter().enumerate() {
        if first_index.is_none() {
            first_index = Some(index);
        }
        if active_id == Some(tab_id) {
            return Some(index);
        }
    }

    first_index
}

pub(super) fn terminal_surface_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(theme.extended_palette().background.base.color)),
        ..Default::default()
    }
}

pub(super) fn terminal_content_padding(tab_id: u64) -> iced::Padding {
    let left_offset = (tab_id % 2) as f32 * 0.01;
    iced::Padding { top: 8.0, right: 8.0, bottom: 4.0, left: 8.0 + left_offset }
}

#[cfg(not(target_arch = "wasm32"))]
fn fullscreen_button(app: &App, header_height: f32) -> Element<'_, Message> {
    use iced::widget::tooltip::{Position, Tooltip};

    let i18n = I18n::new(app.desktop_locale);
    let (icon, label) = if app.terminal.is_fullscreen {
        (Icon::FullscreenExit, i18n.__("退出全屏"))
    } else {
        (Icon::Fullscreen, i18n.__("全屏"))
    };
    let icon = svg::Svg::new(assets::get_icon(icon))
        .width(14.0)
        .height(14.0)
        .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) });
    let control = button(icon)
        .padding(5)
        .style(toolbar_button_style)
        .on_press(Message::Terminal(message::TerminalMessage::ToggleFullscreen));
    let tooltip = Tooltip::new(
        control,
        container(text(label).size(12)).padding([6, 8]).style(tooltip_dark_style),
        Position::Bottom,
    )
    .gap(6);

    container(tooltip).center_x(32.0).center_y(header_height).into()
}

fn toolbar_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let background = if status == button::Status::Hovered {
        theme.extended_palette().background.strong.color.scale_alpha(0.2)
    } else {
        Color::TRANSPARENT
    };
    button::Style {
        background: Some(Background::Color(background)),
        border: Border { radius: 12.0.into(), ..Default::default() },
        text_color: theme.palette().text,
        ..Default::default()
    }
}

fn toolbar_icon_button(icon: Icon, on_press: Message) -> Element<'static, Message> {
    let icon = svg::Svg::new(assets::get_icon(icon))
        .width(12.0)
        .height(12.0)
        .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) });
    let control = button(container(icon).center_x(24.0).center_y(24.0))
        .on_press(on_press)
        .padding(0)
        .style(toolbar_button_style);
    container(control).center_x(32.0).center_y(TAB_HEADER_HEIGHT).into()
}

fn hide_button() -> Element<'static, Message> {
    toolbar_icon_button(Icon::EyeSlash, Message::View(message::ViewMessage::ToggleTerminalPanel))
}

fn pane_view<'a>(
    app: &'a App,
    tabs: Vec<&'a TerminalTab>,
    active_id: Option<u64>,
    controls: iced::widget::Row<'a, Message>,
) -> (Element<'a, Message>, Element<'a, Message>) {
    let tab_header_height = TAB_HEADER_HEIGHT - TAB_SCROLLBAR_WIDTH - TAB_SCROLLBAR_GAP;

    // ==================== 构建标签页行 ====================

    let mut tabs_row = row![].align_y(iced::Alignment::Center);
    let can_close_tabs = app.terminal.tabs.len() > 1;

    // 遍历所有终端标签页，逐个构建 UI 元素
    for (index, t) in tabs.iter().enumerate() {
        // 判断当前标签页是否为激活状态
        // 如果没有指定激活 ID，则默认第一个标签页为激活
        let active = terminal_tab_is_active(active_id, t.id, index);
        let tab_id = t.id;

        // 计算标签页宽度：基于标题字符数量
        // 公式：字符数 * 8.4（每字符平均宽度）+ 62.0（内边距和图标预留）
        // 限制在 92.0-260.0 范围内
        let title = if t.default_title {
            crate::__!(crate::app::i18n::I18n::new(app.desktop_locale), "终端 %u", t.id)
                .expect("terminal title template")
        } else {
            t.title.clone()
        };
        let tab_width = terminal_tab_width(&title);

        // 关闭按钮尺寸
        let close_btn_size = 18.0;
        // 关闭按钮槽位宽度（包含按钮和周围间距）
        let close_slot_width = 26.0;
        // 选择按钮宽度 = 标签页宽度 - 关闭按钮槽位宽度
        let select_width = (tab_width - close_slot_width).max(0.0);

        let select_message = Message::Terminal(message::TerminalMessage::Select(tab_id));
        #[cfg(not(target_arch = "wasm32"))]
        let select_message = if active {
            app.terminal
                .panes
                .grid
                .iter()
                .find_map(|(pane, state)| {
                    state.tabs.contains(&tab_id).then(|| {
                        Message::Terminal(message::TerminalMessage::TogglePaneMaximized(
                            app.terminal.panes.instance.clone(),
                            *pane,
                        ))
                    })
                })
                .unwrap_or(select_message)
        } else {
            select_message
        };

        // ---------- 构建标签页选择按钮 ----------
        // 非激活标签切换终端；激活标签切换窗格放大状态。
        let select_btn = button(
            container(SingleLineLabel::new(title, 13.0).centered())
                .id(TAB_TITLE_ID)
                .padding([0, 10])
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .on_press(select_message)
        .height(Length::Fill)
        .width(Length::Fixed(select_width))
        .padding(0)
        .style(move |theme: &Theme, status: iced::widget::button::Status| {
            let palette = theme.extended_palette();
            // 激活标签使用主题色，非激活使用默认文本色
            let text_color = if active { palette.primary.base.color } else { theme.palette().text };
            // 非激活标签悬停时显示浅色背景
            let bg = if !active && status == iced::widget::button::Status::Hovered {
                palette.background.strong.color.scale_alpha(0.14)
            } else {
                Color::TRANSPARENT
            };
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
                text_color,
                ..Default::default()
            }
        });

        let close_btn = button(
            canvas::Canvas::new(TabCloseIcon { enabled: can_close_tabs, active })
                .width(close_btn_size)
                .height(close_btn_size),
        )
        .on_press_maybe(
            can_close_tabs.then_some(Message::Terminal(message::TerminalMessage::Close(tab_id))),
        )
        .height(Length::Fixed(close_btn_size))
        .width(Length::Fixed(close_btn_size))
        .padding(0)
        .style(|_, _| button::Style::default());

        // ---------- 组装单个标签页项 ----------
        // 结构：[选择按钮 | 关闭按钮] + 底部指示条
        let tab_item = container(
            column![
                // 标签页主体：选择区域 + 关闭按钮
                row![
                    select_btn,
                    container(close_btn)
                        .width(Length::Fixed(close_slot_width))
                        .height(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Center)
                        .align_y(iced::alignment::Vertical::Center)
                ]
                .spacing(0)
                .height(Length::Fill)
                .align_y(iced::Alignment::Center),
                // 底部指示条：激活标签显示主题色，否则透明
                container(Space::new().width(Length::Fill).height(Length::Fixed(3.0))).style(
                    move |theme: &Theme| {
                        let palette = theme.extended_palette();
                        iced::widget::container::Style {
                            background: Some(Background::Color(if active {
                                palette.primary.base.color
                            } else {
                                palette.background.base.color
                            })),
                            ..Default::default()
                        }
                    }
                )
            ]
            .spacing(0),
        )
        .width(Length::Fixed(tab_width))
        .height(Length::Fixed(tab_header_height))
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
            ..Default::default()
        });

        // ---------- 添加右键菜单支持 ----------
        // 包装标签页项以支持右键点击事件
        let right_click_tab: Element<'_, Message> = Element::new(RightClickArea::new(
            tab_item.into(),
            Box::new(move |p| {
                Message::Terminal(message::TerminalMessage::TabContextOpen(tab_id, p.x, p.y))
            }),
        ));

        // ---------- 条件渲染右键菜单 ----------
        // 当此标签页的上下文菜单打开时，显示菜单覆盖层
        let tab_with_overlay: Element<'_, Message> = if app.terminal.tab_context_menu_id
            == Some(tab_id)
        {
            // 构建右键菜单内容
            let context_menu = container(
                column![
                    // 重命名选项
                    button(
                        row![
                            svg::Svg::new(assets::get_icon(Icon::Pencil))
                                .width(Length::Fixed(12.0))
                                .height(Length::Fixed(12.0))
                                .style(|theme: &Theme, _| iced::widget::svg::Style {
                                    color: Some(theme.palette().text),
                                }),
                            text(crate::app::i18n::I18n::new(app.desktop_locale).__("重命名"))
                        ]
                        .spacing(6)
                        .align_y(iced::Alignment::Center),
                    )
                    .on_press(Message::Batch(vec![
                        Message::Terminal(message::TerminalMessage::RenameStart(tab_id)),
                        Message::Terminal(message::TerminalMessage::TabContextClose),
                    ]))
                    .padding([6, 12])
                    .width(Length::Fill)
                    .style(|theme: &Theme, status| {
                        let palette = theme.extended_palette();
                        iced::widget::button::Style {
                            background: match status {
                                iced::widget::button::Status::Hovered => Some(Background::Color(
                                    palette.background.strong.color.scale_alpha(0.12),
                                )),
                                _ => None,
                            },
                            text_color: theme.palette().text,
                            border: Border {
                                width: 0.0,
                                color: Color::TRANSPARENT,
                                radius: 4.0.into(),
                            },
                            ..Default::default()
                        }
                    }),
                    // 关闭选项
                    button(
                        row![
                            svg::Svg::new(assets::get_icon(Icon::Trash))
                                .width(Length::Fixed(12.0))
                                .height(Length::Fixed(12.0))
                                .style(|theme: &Theme, _| iced::widget::svg::Style {
                                    color: Some(theme.palette().text),
                                }),
                            text(crate::app::i18n::I18n::new(app.desktop_locale).__("关闭"))
                        ]
                        .spacing(6)
                        .align_y(iced::Alignment::Center),
                    )
                    .on_press_maybe(can_close_tabs.then_some(Message::Batch(vec![
                        Message::Terminal(message::TerminalMessage::Close(tab_id)),
                        Message::Terminal(message::TerminalMessage::TabContextClose),
                    ])))
                    .padding([6, 12])
                    .width(Length::Fill)
                    .style(move |theme: &Theme, status| {
                        let palette = theme.extended_palette();
                        iced::widget::button::Style {
                            // 关闭按钮悬停时使用危险色背景
                            background: match status {
                                iced::widget::button::Status::Hovered => Some(Background::Color(
                                    palette.danger.base.color.scale_alpha(0.12),
                                )),
                                _ => None,
                            },
                            text_color: if can_close_tabs {
                                theme.palette().text
                            } else {
                                theme.palette().text.scale_alpha(0.4)
                            },
                            border: Border {
                                width: 0.0,
                                color: Color::TRANSPARENT,
                                radius: 4.0.into(),
                            },
                            ..Default::default()
                        }
                    })
                ]
                .spacing(2),
            )
            .width(Length::Fixed(108.0))
            .padding([6, 6])
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                iced::widget::container::Style {
                    background: Some(Background::Color(palette.background.base.color)),
                    border: Border {
                        width: 1.0,
                        color: palette.background.strong.color.scale_alpha(0.6),
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }
            });

            // 获取菜单显示位置，默认在标签栏下方
            let (x, y) = app.terminal.tab_context_menu_pos.unwrap_or((0.0, tab_header_height));
            // 使用 PointBelowOverlay 组件创建菜单覆盖层
            PointBelowOverlay::new(right_click_tab, context_menu)
                .show(true)
                .anchor(Point::new(x, y))
                .gap(2.0)
                .on_close(Message::Terminal(message::TerminalMessage::TabContextClose))
                .into()
        } else {
            right_click_tab
        };

        tabs_row = tabs_row.push(tab_with_overlay);
    }

    // ==================== 构建可滚动的标签页栏 ====================

    let tabs_scroll = scrollable(tabs_row)
        .id(TAB_STRIP_ID)
        .direction(iced::widget::scrollable::Direction::Horizontal(
            iced::widget::scrollable::Scrollbar::new()
                .width(TAB_SCROLLBAR_WIDTH)
                .scroller_width(TAB_SCROLLBAR_WIDTH)
                .spacing(TAB_SCROLLBAR_GAP),
        ))
        .height(Length::Fixed(TAB_HEADER_HEIGHT))
        .width(Length::Fill);

    // ==================== 获取当前激活的标签页 ====================

    // 优先查找指定 ID 的标签页，否则使用第一个标签页
    let active_tab = active_terminal_tab_index(active_id, tabs.iter().map(|tab| tab.id))
        .and_then(|index| tabs.get(index).copied());

    // ==================== 构建终端内容区域 ====================

    let out: Element<'_, Message> = if let Some(_tab) = active_tab {
        // 非 WASM 平台：渲染真实的终端视图
        #[cfg(not(target_arch = "wasm32"))]
        {
            content_menu::terminal_surface(app, _tab)
        }
        // WASM 平台：显示不支持提示
        #[cfg(target_arch = "wasm32")]
        {
            container(text(crate::app::i18n::I18n::new(app.desktop_locale).__("Web 不支持终端")))
                .padding([10, 12])
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
    } else {
        // 无标签页时显示空白内容
        container(text("").font(Font::DEFAULT))
            .padding([10, 12])
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    // 包装终端内容区域，应用边框样式
    let out =
        container(out).width(Length::Fill).height(Length::Fill).padding(0).style(move |theme| {
            let mut style = terminal_surface_style(theme);
            style.border = Border {
                radius: iced::border::Radius {
                    top_left: 0.0,
                    top_right: 0.0,
                    bottom_right: 0.0,
                    bottom_left: 0.0,
                },
                width: 0.0,
                color: Color::TRANSPARENT,
            };
            style
        });

    let toolbar = row![tabs_scroll, controls].height(TAB_HEADER_HEIGHT);
    let header = container(toolbar).width(Length::Fill).style(terminal_surface_style);

    (header.into(), out.into())
}

/// 渲染终端窗格以及覆盖整个面板的重命名模态框。
pub fn view(app: &App) -> Element<'_, Message> {
    if !app.terminal.is_visible {
        return container(column![]).into();
    }

    #[cfg(not(target_arch = "wasm32"))]
    let content = pane_grid::view(app);
    #[cfg(target_arch = "wasm32")]
    let content = {
        let (header, out) = pane_view(
            app,
            app.terminal.tabs.iter().collect(),
            app.terminal.active_id,
            row![
                toolbar_icon_button(Icon::Plus, Message::Terminal(message::TerminalMessage::Add)),
                hide_button()
            ],
        );
        column![header, out].width(Length::Fill).height(Length::Fill)
    };
    let main_content =
        container(content).width(Length::Fill).height(Length::Fill).style(|theme: &Theme| {
            iced::widget::container::Style {
                background: Some(theme.extended_palette().background.weak.color.into()),
                ..Default::default()
            }
        });

    // ==================== 重命名模态框 ====================

    // 如果当前激活的标签页正在编辑标题，显示重命名模态框
    if let Some(active_id) = app.terminal.active_id
        && let Some(t) = app.terminal.tabs.iter().find(|t| t.id == active_id)
        && let Some(edit_title) = &t.edit_title
    {
        let i18n = I18n::new(app.desktop_locale);
        let cancel = Message::Terminal(message::TerminalMessage::RenameCancel(active_id));
        let save = Message::Terminal(message::TerminalMessage::RenameSave(active_id));
        let mut heading = column![text(i18n.__("重命名终端")).size(18)].spacing(4);
        // The terminal can be shortened independently of the window.
        let compact = app.terminal.height < 190.0;
        if !compact {
            heading = heading.push(
                text(i18n.__("输入一个便于识别的新名称")).size(13).style(settings_muted_text_style),
            );
        }
        let header = settings_modal_header(heading, settings_close_button(cancel.clone()));
        let rename_input = text_input(i18n.__("输入新名称"), edit_title)
            .on_input(move |value| {
                Message::Terminal(message::TerminalMessage::RenameChanged(active_id, value))
            })
            .on_submit(save.clone())
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .width(Length::Fill)
            .style(settings_text_input_style);

        let actions = row![
            button(text(i18n.__("取消")).size(14))
                .on_press(cancel.clone())
                .padding([8, 18])
                .height(38)
                .style(rounded_action_btn_style),
            button(text(i18n.__("确定")).size(14))
                .on_press(save)
                .padding([8, 18])
                .height(38)
                .style(primary_action_btn_style),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        let (content, vertical_padding) = if compact {
            let controls = row![rename_input, actions].spacing(10).align_y(iced::Alignment::Center);
            (column![header, controls].spacing(6), 5)
        } else {
            let footer = container(actions).align_right(Length::Fill);
            (column![header, rename_input, footer].spacing(10), 12)
        };
        let card = settings_modal_card(content)
            .padding([vertical_padding, 20])
            .width(Length::Fill)
            .max_width(420.0);
        return settings_modal_overlay_with_backdrop_dismiss(main_content.into(), cancel, card);
    }

    // 无模态框时直接返回主内容
    main_content.into()
}
