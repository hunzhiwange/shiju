//! 终端面板组件模块
//!
//! 该模块提供终端界面的完整视图渲染功能，包括：
//! - 多标签页管理（添加、切换、关闭）
//! - 标签页右键上下文菜单（重命名、关闭）
//! - 终端内容显示区域
//! - 标签页重命名模态对话框
//!
//! 该模块是 VibeWindow 应用的终端 UI 层核心组件，
//! 负责将终端状态转换为 Iced 框架的可渲染元素。

use iced::widget::{
    Space, button, center, column, container, mouse_area, row, scrollable, stack, svg, text,
    text_input,
};
use iced::{Background, Border, Color, Element, Font, Length, Point, Theme};

use crate::app::assets::{self, Icon};
use crate::app::components::overlays::point_below::PointBelowOverlay;
use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_modal_backdrop_style,
    settings_modal_card, settings_muted_text_style, settings_text_input_style,
};
use crate::app::components::widgets::RightClickArea;
use crate::app::{App, Message, message};

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

/// 渲染终端面板视图
///
/// 根据应用状态生成完整的终端界面元素，包括标签页栏和终端内容区。
/// 当终端面板不可见时返回空容器。
///
/// # 参数
///
/// * `app` - 应用状态引用，包含终端配置、窗口尺寸等信息
///
/// # 返回值
///
/// 返回可渲染的 Iced 元素，包含完整的终端 UI
///
/// # 布局结构
///
/// ```text
/// ┌─────────────────────────────────────────┐
/// │ [Tab1] [Tab2] [Tab3] ... [+]            │ ← 标签页栏（可水平滚动）
/// ├─────────────────────────────────────────┤
/// │                                         │
/// │         终端内容显示区域                 │
/// │                                         │
/// └─────────────────────────────────────────┘
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    // 终端面板不可见时直接返回空容器
    if !app.terminal.is_visible {
        return container(column![]).into();
    }

    // ==================== 尺寸常量 ====================

    // 元素间距
    let spacing = 0;
    // 标签页标题栏高度
    let tab_header_height = 34.0;

    // ==================== 构建标签页行 ====================

    let mut tabs_row = row![].spacing(spacing).align_y(iced::Alignment::Center);
    let can_close_tabs = app.terminal.tabs.len() > 1;

    // 遍历所有终端标签页，逐个构建 UI 元素
    for (index, t) in app.terminal.tabs.iter().enumerate() {
        // 判断当前标签页是否为激活状态
        // 如果没有指定激活 ID，则默认第一个标签页为激活
        let active = terminal_tab_is_active(app.terminal.active_id, t.id, index);
        let tab_id = t.id;

        // 计算标签页宽度：基于标题字符数量
        // 公式：字符数 * 8.4（每字符平均宽度）+ 62.0（内边距和图标预留）
        // 限制在 92.0-260.0 范围内
        let tab_width = terminal_tab_width(&t.title);

        // 关闭按钮尺寸
        let close_btn_size = 18.0;
        // 关闭按钮槽位宽度（包含按钮和周围间距）
        let close_slot_width = 26.0;
        // 选择按钮宽度 = 标签页宽度 - 关闭按钮槽位宽度
        let select_width = (tab_width - close_slot_width).max(0.0);

        // ---------- 构建标签页选择按钮 ----------
        // 点击此区域可切换到对应标签页
        let select_btn = button(
            container(
                text(t.title.clone())
                    .size(13)
                    .width(Length::Shrink)
                    .height(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
        )
        .on_press(Message::Terminal(message::TerminalMessage::Select(tab_id)))
        .height(Length::Fill)
        .width(Length::Fixed(select_width))
        .padding([0, 10])
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

        // ---------- 构建关闭按钮 ----------
        // 悬停时显示红色，用于关闭对应标签页
        let close_btn = button(
            container(
                svg::Svg::new(assets::get_icon(Icon::X))
                    .width(Length::Fixed(10.0))
                    .height(Length::Fixed(10.0))
                    .style(move |theme: &Theme, _| {
                        let palette = theme.extended_palette();
                        // 检测是否为深色主题：RGB 总和小于 1.5 判定为深色
                        let is_dark = theme.palette().background.r
                            + theme.palette().background.g
                            + theme.palette().background.b
                            < 1.5;
                        iced::widget::svg::Style {
                            color: Some(if !can_close_tabs {
                                theme.palette().text.scale_alpha(0.28)
                            } else if active {
                                palette.primary.base.color
                            } else if is_dark {
                                Color::from_rgb8(238, 238, 238)
                            } else {
                                theme.palette().text
                            }),
                        }
                    }),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
        )
        .on_press_maybe(
            can_close_tabs.then_some(Message::Terminal(message::TerminalMessage::Close(tab_id))),
        )
        .height(Length::Fixed(close_btn_size))
        .width(Length::Fixed(close_btn_size))
        .padding([0, 0])
        .style(move |theme: &Theme, status| {
            let palette = theme.extended_palette();
            // 检测是否为深色主题
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            // 非激活标签的图标颜色
            let inactive_color =
                if is_dark { Color::from_rgb8(238, 238, 238) } else { theme.palette().text };
            // 文本颜色：悬停时红色 > 激活时主题色 > 默认色
            let text_color = if !can_close_tabs {
                theme.palette().text.scale_alpha(0.28)
            } else if status == iced::widget::button::Status::Hovered {
                palette.danger.base.color
            } else if active {
                palette.primary.base.color
            } else {
                inactive_color
            };
            // 背景颜色：仅悬停时显示半透明红色背景
            let bg = if can_close_tabs && status == iced::widget::button::Status::Hovered {
                palette.danger.base.color.scale_alpha(0.24)
            } else {
                Color::TRANSPARENT
            };
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                border: Border {
                    width: 0.0,
                    color: Color::TRANSPARENT,
                    // 圆角半径为按钮尺寸的一半，形成圆形
                    radius: (close_btn_size * 0.5).into(),
                },
                text_color,
                ..Default::default()
            }
        });

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
                            text("重命名")
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
                            text("关闭")
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

    // ==================== 构建添加标签按钮 ====================

    let add_btn = button(
        container(
            svg::Svg::new(assets::get_icon(Icon::Plus))
                .width(Length::Fixed(12.0))
                .height(Length::Fixed(12.0))
                .style(|theme: &Theme, _| iced::widget::svg::Style {
                    color: Some(theme.palette().text),
                }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center),
    )
    .on_press(Message::Terminal(message::TerminalMessage::Add))
    .height(Length::Fixed(24.0))
    .width(Length::Fixed(24.0))
    .padding([0, 0])
    .style(|theme: &Theme, status| {
        let palette = theme.extended_palette();
        // 悬停时显示背景色
        let bg = if status == iced::widget::button::Status::Hovered {
            palette.background.strong.color.scale_alpha(0.2)
        } else {
            Color::TRANSPARENT
        };
        iced::widget::button::Style {
            background: Some(Background::Color(bg)),
            border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 12.0.into() },
            text_color: theme.palette().text,
            ..Default::default()
        }
    });

    let hide_btn = button(
        container(
            svg::Svg::new(assets::get_icon(Icon::EyeSlash))
                .width(Length::Fixed(12.0))
                .height(Length::Fixed(12.0))
                .style(|theme: &Theme, _| iced::widget::svg::Style {
                    color: Some(theme.palette().text),
                }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center),
    )
    .on_press(Message::View(message::ViewMessage::ToggleTerminalPanel))
    .height(Length::Fixed(24.0))
    .width(Length::Fixed(24.0))
    .padding([0, 0])
    .style(|theme: &Theme, status| {
        let palette = theme.extended_palette();
        let bg = if status == iced::widget::button::Status::Hovered {
            palette.background.strong.color.scale_alpha(0.2)
        } else {
            Color::TRANSPARENT
        };
        iced::widget::button::Style {
            background: Some(Background::Color(bg)),
            border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 12.0.into() },
            text_color: theme.palette().text,
            ..Default::default()
        }
    });

    // 将添加按钮添加到标签页行
    tabs_row = tabs_row.push(
        container(add_btn)
            .height(Length::Fixed(tab_header_height))
            .width(Length::Fixed(32.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    );

    tabs_row = tabs_row.push(
        container(hide_btn)
            .height(Length::Fixed(tab_header_height))
            .width(Length::Fixed(32.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    );

    // ==================== 构建可滚动的标签页栏 ====================

    let tabs_scroll = scrollable(tabs_row)
        .direction(iced::widget::scrollable::Direction::Horizontal(
            iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .height(Length::Fixed(tab_header_height))
        .width(Length::Fill);

    // ==================== 获取当前激活的标签页 ====================

    // 优先查找指定 ID 的标签页，否则使用第一个标签页
    let active_tab = active_terminal_tab_index(
        app.terminal.active_id,
        app.terminal.tabs.iter().map(|tab| tab.id),
    )
    .and_then(|index| app.terminal.tabs.get(index));

    // ==================== 构建终端内容区域 ====================

    let out: Element<'_, Message> = if let Some(_tab) = active_tab {
        // 非 WASM 平台：渲染真实的终端视图
        #[cfg(not(target_arch = "wasm32"))]
        {
            let term = &_tab.term;
            container(
                iced_term::TerminalView::show(term)
                    .map(|e| Message::Terminal(message::TerminalMessage::Event(e))),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(terminal_content_padding(_tab.id))
            .into()
        }
        // WASM 平台：显示不支持提示
        #[cfg(target_arch = "wasm32")]
        {
            container(text("Terminal not supported on Web"))
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

    // ==================== 组装标签页栏容器 ====================

    let tabs_container = container(tabs_scroll).width(Length::Fill).padding([0, 0]);

    // 构建头部容器，包含标签页栏，使用背景色
    let header = container(row![tabs_container].height(Length::Fixed(tab_header_height)))
        .width(Length::Fill)
        .padding([0, 0])
        .style(terminal_surface_style);

    // ==================== 组装主内容区域 ====================

    // 主内容 = 标签栏 + 终端内容区
    let main_content =
        container(column![header, out].spacing(0).height(Length::Fill).width(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(terminal_surface_style);

    // ==================== 重命名模态框 ====================

    // 如果当前激活的标签页正在编辑标题，显示重命名模态框
    if let Some(active_id) = app.terminal.active_id
        && let Some(t) = app.terminal.tabs.iter().find(|t| t.id == active_id)
        && let Some(edit_title) = &t.edit_title
    {
        let rename_input = text_input("输入新名称", edit_title)
            .on_input(move |v| {
                Message::Terminal(message::TerminalMessage::RenameChanged(active_id, v))
            })
            .on_submit(Message::Terminal(message::TerminalMessage::RenameSave(active_id)))
            .padding([10, 12])
            .size(14)
            .width(Length::Fill)
            .style(settings_text_input_style);

        let actions = row![
            Space::new().width(Length::Fill),
            button(text("取消").size(14))
                .on_press(Message::Terminal(message::TerminalMessage::RenameCancel(active_id)))
                .padding([8, 18])
                .height(Length::Fixed(38.0))
                .style(rounded_action_btn_style),
            button(text("确定").size(14))
                .on_press(Message::Terminal(message::TerminalMessage::RenameSave(active_id)))
                .padding([8, 18])
                .height(Length::Fixed(38.0))
                .style(primary_action_btn_style),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        // 与系统设置弹窗共用主题卡片、输入框和按钮样式。
        let modal = center(
            settings_modal_card(
                column![
                    text("重命名终端")
                        .size(18)
                        .font(Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT }),
                    text("输入一个便于识别的新名称").size(12).style(settings_muted_text_style),
                    rename_input,
                    actions,
                ]
                .spacing(14)
                .width(Length::Fill),
            )
            .width(Length::Fixed(420.0)),
        );

        // 构建半透明遮罩层，点击可取消重命名
        let overlay = mouse_area(
            container(Space::new().width(Length::Fill).height(Length::Fill))
                .style(settings_modal_backdrop_style),
        )
        .on_press(Message::Terminal(message::TerminalMessage::RenameCancel(active_id)));

        // 使用 stack 叠加：主内容 + 遮罩 + 模态框
        return stack![main_content, overlay, modal].into();
    }

    // 无模态框时直接返回主内容
    main_content.into()
}
