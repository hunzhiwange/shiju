//! 待办事项面板组件模块
//!
//! 本模块提供待办事项面板的 UI 渲染和数据计算功能。该面板显示在聊天界面中，
//! 用于实时展示任务进度、执行状态和任务列表。
//!
//! # 主要功能
//!
//! - 显示待办事项总数、已完成数量和当前执行任务
//! - 作为当前会话 Todo 的唯一面板实现，供输入区和聊天工具视图复用
//! - 运行时默认展开，全部完成后自动折叠
//! - 使用灰色卡片背景、状态圆点和脉冲动画展示任务状态
//!
//! # 核心组件
//!
//! - [`TodoPanelData`]: 待办面板数据结构
//! - [`compute_todo_data`]: 计算待办数据的函数
//! - [`read_todos_for_panel`]: 读取并排序当前会话任务
//! - [`todo_id_display`]: 格式化任务 ID 供界面展示
//! - [`todo_panel`]: 渲染待办面板的函数

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::svg;
use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::assets::Icon;
use crate::app::components::input_panel::icons::icon_svg;
use crate::app::{App, Message, TodoPanelPlacement, message};
use vw_shared::todo::Todo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TodoPanelSurface {
    InputBottom,
    ChatTopRight,
}

/// 待办面板数据结构
///
/// 该结构体包含待办面板显示所需的所有数据，包括任务统计信息和当前执行的任务。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TodoPanelData {
    /// 待办事项总数
    pub total: usize,

    /// 已完成的待办事项数量
    pub completed: usize,

    /// 当前正在执行的任务描述
    /// 如果没有执行中的任务，则显示默认文本"无执行中任务"
    pub running_task: String,
}

fn todo_panel_is_dark(theme: &Theme) -> bool {
    let background = theme.palette().background;
    background.r + background.g + background.b < 1.5
}

pub(super) fn todo_status_pill_colors(
    theme: &Theme,
    is_completed: bool,
    is_running: bool,
) -> (Color, Color) {
    if is_completed {
        return (Color::from_rgb8(46, 194, 126), Color::WHITE);
    }

    if is_running {
        return (theme.palette().primary, Color::WHITE);
    }

    if todo_panel_is_dark(theme) {
        (Color::from_rgb8(96, 165, 250), Color::WHITE)
    } else {
        (Color::from_rgb8(71, 85, 105), Color::from_rgb8(248, 250, 252))
    }
}

pub(super) fn todo_task_text_color(theme: &Theme, is_completed: bool) -> Color {
    if is_completed {
        if todo_panel_is_dark(theme) {
            theme.palette().text.scale_alpha(0.78)
        } else {
            theme.extended_palette().secondary.base.text.scale_alpha(0.72)
        }
    } else {
        theme.palette().text
    }
}

/// 计算待办面板数据
///
/// 根据应用状态和待办事项列表，计算面板所需显示的数据。
/// 该函数会优先从待办列表中查找正在执行的任务，如果没有找到，
/// 则从当前会话的运行时引用中获取活跃的代理请求。
///
/// # 参数
///
/// * `app` - 应用状态引用，用于获取当前会话信息
/// * `todo_items` - 待办事项列表切片，包含所有待办任务
///
/// # 返回值
///
/// 返回 [`TodoPanelData`] 实例，包含：
/// - `total`: 待办事项总数
/// - `completed`: 已完成的待办事项数量
/// - `running_task`: 当前执行中的任务描述（可能为默认文本）
///
/// # 示例
///
/// ```ignore
/// let data = compute_todo_data(&app, &todo_items);
/// println!("已完成 {}/{} 项任务", data.completed, data.total);
/// println!("当前任务：{}", data.running_task);
/// ```
pub fn compute_todo_data(app: &App, todo_items: &[Todo]) -> TodoPanelData {
    // 计算待办事项总数
    let total = todo_items.len();

    // 统计已完成的待办事项数量
    let completed = todo_items.iter().filter(|t| t.status == "completed").count();

    // 查找当前正在执行的任务
    // 优先从待办列表中查找状态为 in_progress 的任务
    let running_task = todo_items
        .iter()
        .find(|t| t.status == "in_progress")
        .map(|t| t.content.clone())
        // 如果待办列表中没有执行中的任务，则从当前会话中获取活跃请求
        .or_else(|| {
            app.current_session_runtime_ref().and_then(|r| {
                r.active_agent_request
                    .as_ref()
                    .map(|request| request.query.lines().next().unwrap_or("").trim().to_string())
            })
        })
        // 过滤掉空字符串
        .filter(|s| !s.is_empty())
        // 如果都没有，使用默认文本
        .unwrap_or_else(|| "无执行中任务".to_string());

    TodoPanelData { total, completed, running_task }
}

/// 读取当前会话的 Todo 列表并按显示顺序排序。
///
/// 排序规则与聊天工具视图保持一致：可解析的数字 ID 优先按数值升序排列，
/// 其余 ID 按字符串字典序排列。
pub fn read_todos_for_panel(app: &App) -> Option<(Vec<Todo>, Option<String>)> {
    if app.chat_todo_session_id != app.active_session_id {
        return Some((Vec::new(), None));
    }
    let mut todos = app.chat_todo_items.clone();

    todos.sort_by(|a, b| match (a.id.parse::<u64>().ok(), b.id.parse::<u64>().ok()) {
        (Some(ai), Some(bi)) => ai.cmp(&bi),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.id.cmp(&b.id),
    });

    Some((todos, None))
}

/// 将任务 ID 转换为更紧凑的展示文本。
///
/// 如果 ID 末尾的下划线分段是纯数字，则仅显示该数字；否则保留原始文本。
#[allow(dead_code)]
pub fn todo_id_display(raw: &str) -> String {
    let s = raw.trim();
    let tail = s.rsplit('_').next().unwrap_or(s).trim();

    if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) {
        return tail.to_string();
    }

    s.to_string()
}

fn placement_button(placement: TodoPanelPlacement, selected: bool) -> Element<'static, Message> {
    button(text(placement.label()).size(11))
        .padding([3, 8])
        .style(move |theme: &Theme, status| {
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            let selected_bg = if is_dark {
                Color::from_rgba8(70, 74, 83, 0.92)
            } else {
                Color::from_rgba8(223, 229, 238, 1.0)
            };
            let idle_bg = if is_dark {
                Color::from_rgba8(35, 37, 42, 0.72)
            } else {
                Color::from_rgba8(244, 246, 249, 1.0)
            };
            let hover_bg = if selected {
                selected_bg
            } else if is_dark {
                Color::from_rgba8(44, 47, 54, 0.86)
            } else {
                Color::from_rgba8(235, 239, 245, 1.0)
            };
            let background = match status {
                iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed => {
                    hover_bg
                }
                _ if selected => selected_bg,
                _ => idle_bg,
            };

            iced::widget::button::Style {
                background: Some(Background::Color(background)),
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
                text_color: if selected {
                    theme.palette().text
                } else {
                    theme.palette().text.scale_alpha(0.72)
                },
                ..Default::default()
            }
        })
        .on_press(Message::Chat(message::ChatMessage::SetTodoPanelPlacement(placement)))
        .into()
}

fn open_git_panel_button(changed_files: usize) -> Element<'static, Message> {
    let git_icon: Element<'_, Message> = icon_svg(Icon::GitBranch, 13.0)
        .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
        .into();
    let label = if changed_files > 0 {
        format!("打开 Git 面板 · {changed_files}")
    } else {
        "打开 Git 面板".to_string()
    };

    button(row![git_icon, text(label).size(12)].spacing(7).align_y(Alignment::Center))
        .padding([5, 9])
        .width(Length::Fill)
        .style(|theme: &Theme, status| {
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            let background = match status {
                iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed => {
                    if is_dark {
                        Color::from_rgba8(70, 70, 70, 0.88)
                    } else {
                        Color::from_rgba8(231, 235, 241, 1.0)
                    }
                }
                _ => {
                    if is_dark {
                        Color::from_rgba8(58, 58, 58, 0.76)
                    } else {
                        Color::from_rgba8(242, 244, 247, 1.0)
                    }
                }
            };
            iced::widget::button::Style {
                background: Some(Background::Color(background)),
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 8.0.into() },
                text_color: theme.palette().text,
                ..Default::default()
            }
        })
        .on_press(Message::Batch(vec![
            Message::Project(message::ProjectMessage::FileManagerShowChanges(true)),
            Message::Git(message::GitMessage::RefreshGitPanelData),
        ]))
        .into()
}

fn floating_collapsed_badge(total: usize) -> Element<'static, Message> {
    let count = total.min(99).to_string();
    let badge = container(text(count).size(14))
        .width(Length::Fixed(26.0))
        .height(Length::Fixed(26.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(|theme: &Theme| {
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            let bg = if is_dark {
                Color::from_rgba8(34, 36, 41, 0.96)
            } else {
                Color::from_rgba8(252, 252, 253, 1.0)
            };
            let border = if is_dark {
                Color::from_rgba8(72, 76, 84, 0.95)
            } else {
                Color::from_rgba8(214, 220, 229, 1.0)
            };
            iced::widget::container::Style {
                background: Some(Background::Color(bg)),
                border: Border { width: 1.0, color: border, radius: 999.0.into() },
                text_color: Some(theme.palette().text),
                shadow: iced::Shadow {
                    color: Color::BLACK.scale_alpha(if is_dark { 0.22 } else { 0.10 }),
                    offset: iced::Vector::new(0.0, 8.0),
                    blur_radius: 18.0,
                },
                ..Default::default()
            }
        });

    button(badge)
        .padding(0)
        .width(Length::Fixed(26.0))
        .height(Length::Fixed(26.0))
        .style(iced::widget::button::text)
        .on_press(Message::Chat(message::ChatMessage::ToggleTodoPanel))
        .into()
}

/// 渲染待办事项面板
///
/// 根据应用状态和待办事项列表创建当前唯一的 Todo 面板 UI。
///
/// 交互行为与聊天中的运行态工具块保持一致：
/// - 初始进入时默认展开
/// - 用户可点击头部手动展开或折叠
/// - 当当前会话任务全部完成后，面板会自动折叠
/// - 折叠状态仅显示摘要和当前执行任务
/// - 展开状态显示完整任务列表，执行中任务带脉冲动画
///
/// # 参数
///
/// * `app` - 应用状态引用，用于获取面板展开状态和当前运行时信息
/// * `todo_items` - 待办事项列表切片，包含所有待办任务
/// * `submit_anim` - 提交动画帧计数器（0-255），用于驱动执行中任务的脉冲动画
///
/// # 返回值
///
/// 返回 Iced Element，可直接嵌入到 UI 树中
///
/// # UI 结构
///
/// ## 展开状态
/// ```text
/// ┌─────────────────────────────────────┐
/// │ 2 of 5 待办事项 completed        ▼ │  <- 可点击头部
/// ├─────────────────────────────────────┤
/// │ ✓ 任务1已完成            [任务已完成] │
/// │ ● 任务2执行中            [任务执行中] │  <- 灰底卡片 + 脉冲动画
/// │ ○ 任务3待执行            [待执行]     │
/// └─────────────────────────────────────┘
/// ```
///
/// ## 折叠状态
/// ```text
/// ┌─────────────────────────────────────┐
/// │ 2 of 5 待办事项 completed        ▶ │  <- 任务完成后自动回到此状态
/// │ 执行中：正在处理文件...              │
/// └─────────────────────────────────────┘
/// ```
///
/// # 示例
///
/// ```ignore
/// let panel = todo_panel(&app, &todo_items, anim_frame);
/// // 将 panel 添加到 UI 布局中
/// ```
pub fn todo_panel(
    app: &App,
    todo_items: &[Todo],
    submit_anim: u8,
    surface: TodoPanelSurface,
) -> Element<'static, Message> {
    // 计算面板所需的数据
    let data = compute_todo_data(app, todo_items);

    if surface == TodoPanelSurface::ChatTopRight && !app.chat_todo_expanded {
        return floating_collapsed_badge(data.total);
    }

    // 构建摘要文本：显示已完成数量和总数
    let summary_text = format!("已完成 {} / {} 项待办", data.completed, data.total);

    // 根据展开状态选择不同的箭头图标
    let header_icon = if app.chat_todo_expanded { Icon::ChevronUp } else { Icon::ChevronDown };

    // 创建箭头图标，使用主题文本颜色
    let header_chevron = icon_svg(header_icon, 14.0)
        .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) });

    // 构建头部行：包含摘要文本和箭头图标
    let header_title: Element<'static, Message> = if surface == TodoPanelSurface::ChatTopRight {
        column![
            text("进度").size(13),
            text(summary_text).size(11).style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.palette().text.scale_alpha(0.66)),
            })
        ]
        .spacing(1)
        .into()
    } else {
        text(summary_text).size(12).into()
    };

    let header_row =
        row![header_title, container(Space::new()).width(Length::Fill), header_chevron]
            .spacing(8)
            .align_y(Alignment::Center);

    // 将头部包装在可点击区域内，点击时切换展开/折叠状态
    let header = mouse_area(container(header_row).width(Length::Fill))
        .on_press(Message::Chat(message::ChatMessage::ToggleTodoPanel));

    // 根据展开状态渲染不同的内容
    if app.chat_todo_expanded {
        // ========== 展开状态：显示完整的任务列表 ==========

        // 根据动画帧计算脉冲点的大小（4-8像素之间循环）
        // 这创建了执行中任务的"呼吸"动画效果
        let pulse_size = match submit_anim % 4 {
            0 => 4.0,
            1 => 6.0,
            2 => 8.0,
            _ => 6.0,
        };

        // 创建任务列表容器
        let mut todo_rows = column![].spacing(6);

        // 遍历所有待办事项，创建对应的 UI 行
        for todo in todo_items.iter().cloned() {
            // 判断任务状态
            let is_completed = todo.status == "completed";
            let is_running = todo.status == "in_progress";

            // 根据状态设置标签文本
            let status_badge = if is_completed {
                "任务已完成"
            } else if is_running {
                "任务执行中"
            } else {
                "待执行"
            };

            // 创建状态标记图标（左侧的圆形标记）
            let marker: Element<'_, Message> = if is_completed {
                // 已完成任务：显示绿色圆角背景的勾号
                container(text("✓").size(10))
                    .width(Length::Fixed(16.0))
                    .height(Length::Fixed(16.0))
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center)
                    .style(|_theme: &Theme| iced::widget::container::Style {
                        background: Some(Background::Color(Color::from_rgb8(46, 194, 126))),
                        border: Border {
                            width: 0.0,
                            color: Color::TRANSPARENT,
                            radius: 4.0.into(),
                        },
                        text_color: Some(Color::WHITE),
                        ..Default::default()
                    })
                    .into()
            } else if is_running {
                // 执行中任务：显示带脉冲动画的圆点
                container(
                    container(
                        Space::new()
                            .width(Length::Fixed(pulse_size))
                            .height(Length::Fixed(pulse_size)),
                    )
                    .style(|theme: &Theme| iced::widget::container::Style {
                        background: Some(Background::Color(theme.palette().primary)),
                        border: Border {
                            width: 0.0,
                            color: Color::TRANSPARENT,
                            radius: 999.0.into(), // 完全圆形
                        },
                        ..Default::default()
                    }),
                )
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .into()
            } else {
                // 待执行任务：显示空心圆角边框
                container(Space::new())
                    .width(Length::Fixed(16.0))
                    .height(Length::Fixed(16.0))
                    .style(|theme: &Theme| iced::widget::container::Style {
                        background: Some(Background::Color(theme.palette().background)),
                        border: Border {
                            width: 1.0,
                            color: theme.extended_palette().background.strong.color,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    })
                    .into()
            };

            // 创建任务文本，已完成任务使用半透明颜色
            let task_text = text(todo.content).size(12).style(move |theme: &Theme| {
                iced::widget::text::Style { color: Some(todo_task_text_color(theme, is_completed)) }
            });

            // 创建状态标签（右侧的彩色标签）
            let status_label = text(status_badge).size(10).style(move |theme: &Theme| {
                let (_, text_color) = todo_status_pill_colors(theme, is_completed, is_running);
                iced::widget::text::Style { color: Some(text_color) }
            });
            let status_pill = container(status_label)
                .padding([0, 8])
                .height(Length::Fixed(18.0))
                .align_y(iced::alignment::Vertical::Center)
                .style(move |theme: &Theme| {
                    let (background_color, text_color) =
                        todo_status_pill_colors(theme, is_completed, is_running);
                    iced::widget::container::Style {
                        background: Some(Background::Color(background_color.scale_alpha(0.16))),
                        border: Border {
                            width: 0.0,
                            color: Color::TRANSPARENT,
                            radius: 999.0.into(), // 胶囊形状
                        },
                        text_color: Some(text_color),
                        ..Default::default()
                    }
                });

            // 组合任务行：标记 + 任务文本 + 状态标签
            let todo_row = row![marker, container(task_text).width(Length::Fill), status_pill]
                .spacing(8)
                .align_y(Alignment::Center);

            // 为任务行添加容器样式（圆角卡片效果）
            let todo_item = container(todo_row).padding([5, 8]).style(|theme: &Theme| {
                let ext = theme.extended_palette();
                let item_border_color =
                    theme.extended_palette().background.strong.color.scale_alpha(0.45);
                iced::widget::container::Style {
                    // 使用更浅的弱背景色，减轻待办项卡片重量感
                    background: Some(Background::Color(
                        ext.background.weak.color.scale_alpha(0.18),
                    )),
                    border: Border { width: 1.0, color: item_border_color, radius: 10.0.into() },
                    ..Default::default()
                }
            });

            // 将任务项添加到列表中
            todo_rows = todo_rows.push(todo_item);
        }

        // ========== 计算任务列表的高度 ==========

        /// 单个任务行的高度（像素）
        const TODO_ROW_HEIGHT: f32 = 30.0;

        /// 任务行之间的间距（像素）
        const TODO_ROW_SPACING: f32 = 6.0;

        /// 额外的高度余量（像素），用于底部内边距
        const TODO_HEIGHT_EXTRA: f32 = 8.0;

        /// 列表最大高度，较原先上限降低约 1/3
        const TODO_MAX_HEIGHT: f32 = 188.0;

        // 最多显示 5 行，其余内容通过滚动查看
        let visible_count = todo_items.len().min(5) as f32;

        // 计算任务行的总高度：所有行高度 + 行间距
        let rows_height = if visible_count > 0.0 {
            visible_count * TODO_ROW_HEIGHT + (visible_count - 1.0) * TODO_ROW_SPACING
        } else {
            0.0
        };

        // 最终列表高度受最大高度限制
        let list_height = (rows_height + TODO_HEIGHT_EXTRA).min(TODO_MAX_HEIGHT);

        // 创建可滚动任务列表
        let task_list =
            scrollable(container(todo_rows).width(Length::Fill).padding(iced::Padding {
                top: 0.0,
                right: 8.0,
                bottom: 0.0,
                left: 0.0,
            }))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .width(Length::Fill)
            .height(Length::Fixed(list_height));

        let placement_controls = row![
            placement_button(
                TodoPanelPlacement::ChatTopRight,
                app.chat_todo_placement == TodoPanelPlacement::ChatTopRight,
            ),
            placement_button(
                TodoPanelPlacement::InputBottom,
                app.chat_todo_placement == TodoPanelPlacement::InputBottom,
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        // 组合最终的面板：头部 + 位置设置 + Git 操作 + 任务列表
        let body = column![
            header,
            placement_controls,
            open_git_panel_button(app.git_changed_files.len()),
            task_list
        ]
        .spacing(6);

        let width = if surface == TodoPanelSurface::ChatTopRight {
            Length::Fixed(340.0)
        } else {
            Length::Fill
        };
        let radius = if surface == TodoPanelSurface::ChatTopRight { 18.0 } else { 0.0 };
        let border_width = if surface == TodoPanelSurface::ChatTopRight { 1.0 } else { 0.0 };
        let padding = if surface == TodoPanelSurface::ChatTopRight { [12, 14] } else { [6, 10] };

        container(body)
            .width(width)
            .padding(padding)
            .style(move |theme: &Theme| {
                let ext = theme.extended_palette();
                let is_dark = theme.palette().background.r
                    + theme.palette().background.g
                    + theme.palette().background.b
                    < 1.5;
                let background = if surface == TodoPanelSurface::ChatTopRight {
                    if is_dark {
                        Color::from_rgba8(37, 38, 41, 0.96)
                    } else {
                        Color::from_rgba8(252, 252, 253, 0.98)
                    }
                } else {
                    ext.background.base.color.scale_alpha(0.50)
                };
                let border_color = if is_dark {
                    Color::from_rgba8(78, 80, 86, 0.86)
                } else {
                    Color::from_rgba8(220, 225, 233, 1.0)
                };
                iced::widget::container::Style {
                    background: Some(Background::Color(background)),
                    border: Border {
                        width: border_width,
                        color: border_color,
                        radius: radius.into(),
                    },
                    shadow: if surface == TodoPanelSurface::ChatTopRight {
                        iced::Shadow {
                            color: Color::BLACK.scale_alpha(if is_dark { 0.22 } else { 0.10 }),
                            offset: iced::Vector::new(0.0, 10.0),
                            blur_radius: 24.0,
                        }
                    } else {
                        iced::Shadow::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    } else {
        // ========== 折叠状态：仅显示摘要和当前任务 ==========

        // 折叠态仅渲染头部，待展开后再渲染详细内容
        container(header)
            .width(Length::Fill)
            .padding([4, 10])
            .style(|theme: &Theme| {
                let ext = theme.extended_palette();
                iced::widget::container::Style {
                    background: Some(Background::Color(
                        ext.background.base.color.scale_alpha(0.50),
                    )),
                    border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
                    ..Default::default()
                }
            })
            .into()
    }
}
