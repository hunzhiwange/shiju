//! 用量统计步骤面板视图模块
//!
//! 本模块提供了用量统计界面中步骤列表面板的 UI 组件实现，主要用于：
//! - 显示会话中各个步骤的详细执行信息
//! - 可视化每个步骤的 token 使用情况
//! - 提供步骤快照的回放和查看功能
//!
//! # 主要组件
//!
//! - [`build_steps_panel`] - 构建步骤面板的主入口函数
//! - [`BarSeg`] - 条形图分段类型枚举
//! - [`stacked_bar`] - 堆叠条形图组件
//! - [`legend_item`] - 图例项组件

use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::message::view::ViewMessage;
use crate::app::{App, Message};

use super::data::UsageData;
use super::styles::card_style;
use super::utils::{fmt_ms, icon_btn, kv};
use crate::app::assets::Icon;
use crate::app::components::system_settings_common::rounded_action_btn_style;

/// 条形图分段类型枚举
///
/// 定义了堆叠条形图中不同类型的数据分段，用于可视化 token 使用分布。
/// 每种类型对应不同的颜色，以区分不同的数据来源。
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum BarSeg {
    /// 用户输入相关
    User,
    /// 助手回复相关
    Assistant,
    /// 工具调用相关
    Tool,
    /// 其他类型
    Other,
    /// 提示词 token
    Prompt,
    /// 回答 token
    Answer,
}

/// 获取分段对应的颜色
///
/// 根据当前主题和分段类型，返回对应的显示颜色。
/// 不同类型的分段使用不同的主题色系以增强可辨识度。
///
/// # 参数
///
/// - `theme` - 当前 Iced 主题引用
/// - `seg` - 分段类型
///
/// # 返回
///
/// 返回该分段类型对应的颜色（带 0.90 透明度）
fn seg_color(theme: &Theme, seg: BarSeg) -> Color {
    let p = theme.extended_palette();
    match seg {
        BarSeg::User => theme.palette().primary.scale_alpha(0.90),
        BarSeg::Assistant => p.success.base.color.scale_alpha(0.90),
        BarSeg::Tool => p.danger.base.color.scale_alpha(0.90),
        BarSeg::Other => p.background.strong.color.scale_alpha(0.90),
        BarSeg::Prompt => theme.palette().primary.scale_alpha(0.90),
        BarSeg::Answer => p.success.base.color.scale_alpha(0.90),
    }
}

/// 创建堆叠条形图组件
///
/// 根据给定的数值和分段类型数组，创建一个横向堆叠的条形图。
/// 每个分段按比例占据条形图的宽度，不同分段使用不同颜色区分。
///
/// # 参数
///
/// - `values` - 元组数组，包含 (数值, 分段类型)
///
/// # 返回
///
/// 返回一个 Iced Element，渲染为带边框的堆叠条形图
///
/// # 算法说明
///
/// 为了避免渲染问题，采用以下策略：
/// 1. 将总宽度划分为 1000 份
/// 2. 按比例计算每个分段的宽度
/// 3. 确保有数值的分段至少占 1 份（避免消失）
/// 4. 调整最后一个分段以确保总和为 1000 份
///
/// # 示例
///
/// ```ignore
/// let bar = stacked_bar(&[
///     (100, BarSeg::User),
///     (200, BarSeg::Assistant),
/// ]);
/// ```
pub fn stacked_bar(values: &[(usize, BarSeg)]) -> Element<'static, Message> {
    // 计算总值
    let total = values.iter().map(|(n, _)| *n).sum::<usize>();

    // 如果总值为 0，返回空条形图
    if total == 0 {
        return container(Space::new())
            .height(Length::Fixed(10.0))
            .width(Length::Fill)
            .style(|theme: &Theme| {
                let p = theme.extended_palette();
                iced::widget::container::Style {
                    background: Some(Background::Color(p.background.weak.color)),
                    border: Border {
                        width: 1.0,
                        color: p.background.strong.color,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                }
            })
            .into();
    }

    // 将数值转换为 1000 份中的比例
    // 使用 1000 而非 100 来获得更好的精度
    let mut portions = values
        .iter()
        .map(|(n, _)| ((*n as f32) * 1000.0 / (total as f32)).round() as i32)
        .collect::<Vec<_>>();

    // 确保非零值的分段至少占 1 份，避免小比例分段完全消失
    for (i, (n, _)) in values.iter().enumerate() {
        if *n > 0 && portions[i] == 0 {
            portions[i] = 1;
        }
    }

    // 计算当前总和
    let sum = portions.iter().sum::<i32>();

    // 调整最后一个分段，确保总份数精确为 1000
    // 这样可以避免因舍入误差导致的视觉偏差
    if let Some(last) = portions.last_mut() {
        *last += 1000 - sum;
        if *last < 0 {
            *last = 0;
        }
    }

    // 构建堆叠条形图的各个分段
    let mut segs: iced::widget::Row<'static, Message> =
        row![].spacing(0).height(Length::Fill).width(Length::Fill);

    for (i, (_n, color)) in values.iter().enumerate() {
        let portion = portions.get(i).copied().unwrap_or(0).max(0) as u16;
        // 跳过宽度为 0 的分段
        if portion == 0 {
            continue;
        }
        let seg = *color;
        segs = segs.push(
            container(Space::new()).width(Length::FillPortion(portion)).height(Length::Fill).style(
                move |theme: &Theme| iced::widget::container::Style {
                    background: Some(Background::Color(seg_color(theme, seg))),
                    ..Default::default()
                },
            ),
        );
    }

    // 包装在容器中，添加边框和背景
    container(segs)
        .height(Length::Fixed(10.0))
        .width(Length::Fill)
        .style(|theme: &Theme| {
            let p = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(Background::Color(p.background.weak.color)),
                border: Border { width: 1.0, color: p.background.strong.color, radius: 6.0.into() },
                ..Default::default()
            }
        })
        .into()
}

/// 创建图例项组件
///
/// 生成一个包含颜色标识、标签和数值的图例项，
/// 用于说明条形图中各颜色分段的含义。
///
/// # 参数
///
/// - `label` - 图例标签文本
/// - `value` - 该分段的数值
/// - `total` - 总数值（用于计算百分比）
/// - `seg` - 分段类型（决定颜色）
///
/// # 返回
///
/// 返回一个水平布局的 Iced Element，包含：
/// - 颜色方块（10x10）
/// - 标签文本
/// - 弹性空间
/// - 数值和百分比文本
///
/// # 示例
///
/// ```ignore
/// let legend = legend_item("用户", 150, 500, BarSeg::User);
/// // 显示为: [蓝色方块] 用户 · 150 · 30.0%
/// ```
pub fn legend_item<'a>(
    label: &'a str,
    value: usize,
    total: usize,
    seg: BarSeg,
) -> Element<'a, Message> {
    // 计算百分比，避免除零
    let pct = if total == 0 {
        "0.0%".to_string()
    } else {
        format!("{:.1}%", (value as f64) * 100.0 / (total as f64))
    };

    row![
        // 颜色标识方块
        container(Space::new()).width(Length::Fixed(10.0)).height(Length::Fixed(10.0)).style(
            move |theme: &Theme| iced::widget::container::Style {
                background: Some(Background::Color(seg_color(theme, seg))),
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 2.0.into() },
                ..Default::default()
            }
        ),
        // 标签
        text(label).size(12),
        // 弹性空间，将数值推到右侧
        Space::new().width(Length::Fill),
        // 数值和百分比
        text(format!("{} · {}", value, pct)).size(12).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(theme.palette().text) }
        }),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn raw_payload_summary(value: &serde_json::Value) -> String {
    let message_count =
        value.get("messages").and_then(serde_json::Value::as_array).map_or(0, Vec::len);
    let tool_call_count = value
        .get("output")
        .and_then(|output| output.get("tool_calls"))
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    format!("历史消息 {message_count} 条 · 工具调用 {tool_call_count} 条")
}

fn vertical_scrollbar() -> iced::widget::scrollable::Direction {
    iced::widget::scrollable::Direction::Vertical(
        iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
    )
}

fn raw_payload_status_for_turn(
    app: &App,
    session_id: &str,
    turn_id: &str,
    step_index: u32,
) -> String {
    let key = (session_id.to_string(), turn_id.to_string(), step_index);
    if app.usage_step_raw_loading.contains(&key) {
        return "原始记录：加载中...".to_string();
    }
    if let Some(value) = app.active_session_view_state.step_llm_raw.get(&key) {
        return format!("原始记录：{}", raw_payload_summary(value));
    }
    match app.usage_step_raw_payloads.get(&key) {
        Some(Ok(Some(value))) => format!("原始记录：{}", raw_payload_summary(value)),
        Some(Ok(None)) => "原始记录：暂无记录".to_string(),
        Some(Err(err)) => format!("原始记录：加载失败（{err}）"),
        None => "原始记录：点击后按需加载".to_string(),
    }
}

fn raw_payload_panel_for_turn(
    app: &App,
    session_id: &str,
    turn_id: &str,
    step_index: u32,
) -> Element<'static, Message> {
    let key = (session_id.to_string(), turn_id.to_string(), step_index);
    let status = text(raw_payload_status_for_turn(app, session_id, turn_id, step_index)).size(12);
    let unavailable = matches!(app.usage_step_raw_payloads.get(&key), Some(Ok(None)));
    let loading = app.usage_step_raw_loading.contains(&key);
    let label = if loading {
        "加载中..."
    } else if unavailable {
        "暂无原始记录"
    } else if matches!(app.usage_step_raw_payloads.get(&key), Some(Err(_))) {
        "重试加载"
    } else {
        "查看原始记录"
    };
    let mut open_btn =
        button(text(label).size(12)).padding([6, 12]).style(rounded_action_btn_style);
    if !loading && !unavailable {
        open_btn = open_btn.on_press(Message::View(ViewMessage::UsageTurnStepRawOpen(
            turn_id.to_string(),
            step_index,
        )));
    }

    container(row![status.width(Length::Fill), open_btn].spacing(10).align_y(Alignment::Center))
        .width(Length::Fill)
        .padding(10)
        .style(|theme: &Theme| {
            let p = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(Background::Color(p.background.weak.color)),
                border: Border { width: 1.0, color: p.background.strong.color, radius: 8.0.into() },
                ..Default::default()
            }
        })
        .into()
}

#[cfg(test)]
fn raw_payload_status(app: &App, session_id: &str, step_index: u32) -> String {
    let turn_id = app
        .active_session_view_state
        .steps
        .iter()
        .find(|step| step.index == step_index)
        .and_then(|step| step.turn_id.as_deref())
        .unwrap_or_default();
    raw_payload_status_for_turn(app, session_id, turn_id, step_index)
}

#[cfg(test)]
fn raw_payload_panel(app: &App, session_id: &str, step_index: u32) -> Element<'static, Message> {
    let turn_id = app
        .active_session_view_state
        .steps
        .iter()
        .find(|step| step.index == step_index)
        .and_then(|step| step.turn_id.as_deref())
        .unwrap_or_default();
    raw_payload_panel_for_turn(app, session_id, turn_id, step_index)
}

/// 构建步骤面板
///
/// 创建完整的步骤列表面板，显示会话中所有步骤的详细信息。
/// 每个步骤可以展开/折叠，查看详细的 token 使用情况和快照操作。
///
/// # 参数
///
/// - `app` - 应用状态引用，包含展开状态等
/// - `usage_data` - 用量数据引用，包含步骤列表
///
/// # 返回
///
/// 返回一个可滚动的面板 Element，包含：
/// - 标题区域
/// - 步骤列表（倒序显示，最新在前）
/// - 每个步骤可展开显示：
///   - 模型名称
///   - token 使用详情（输入/输出/缓存/推理）
///   - 成本估算
///   - 完成原因
///   - 快照回放和打开按钮
///
/// # 功能特性
///
/// - 响应式展开/折叠交互
/// - 自动滚动支持（固定高度 320px）
/// - 卡片式布局设计
/// - 时间范围显示（开始 → 结束）
///
/// # 示例
///
/// ```ignore
/// let panel = build_steps_panel(&app, &usage_data);
/// // 返回一个包含所有步骤信息的可滚动面板
/// ```
pub fn build_steps_panel(app: &App, usage_data: &UsageData) -> Element<'static, Message> {
    /// 滚动区域左右内边距（像素）
    const SCROLL_SIDE_PAD: u16 = 10;

    // 获取步骤列表，如果无会话则使用空列表
    let steps = usage_data.session.as_ref().map(|s| s.steps.clone()).unwrap_or_default();

    // 构建步骤列表主体
    let steps_body: Element<'_, Message> = if steps.is_empty() {
        // 无步骤时显示提示文本
        container(text("暂无步骤记录").size(12)).padding(10).into()
    } else {
        // 有步骤时构建可滚动列表
        let mut col = column![].spacing(12);

        // 倒序遍历步骤，最新的显示在最上面
        for s in steps.iter().rev() {
            let turn_id = s.turn_id.clone().unwrap_or_default();
            // 计算该步骤的总 token 数
            let total = s.usage.input_tokens
                + s.usage.output_tokens
                + s.usage.cached_tokens
                + s.usage.reasoning_tokens;

            // 检查该步骤是否处于展开状态
            let expanded = app.usage_step_expanded.contains(&s.index);

            // 格式化时间范围
            let time_range = match s.finished_ms {
                Some(end) => format!("{} → {}", fmt_ms(s.started_ms), fmt_ms(end)),
                None => format!("{} → …", fmt_ms(s.started_ms)),
            };

            // 格式化成本，保留 4 位小数
            let cost =
                s.cost_usd.map(|c| format!("US${:.4}", c)).unwrap_or_else(|| "暂无".to_string());

            // 根据展开状态选择图标
            let toggle_icon = if expanded { Icon::ChevronUp } else { Icon::ChevronDown };

            // 构建步骤头部：索引、时间范围、展开按钮
            let header = row![
                text(format!("步骤 {}", s.index)).size(12),
                Space::new().width(Length::Fill),
                text(time_range).size(12).style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().background.weak.text.scale_alpha(0.9)),
                }),
                icon_btn(
                    toggle_icon,
                    if expanded { "折叠" } else { "展开" },
                    Message::View(ViewMessage::UsageTurnStepToggled(turn_id.clone(), s.index)),
                ),
            ]
            .align_y(Alignment::Center);

            // 构建详细信息区域（仅在展开时显示）
            let info: Element<'_, Message> = if expanded {
                let session_id = usage_data.session_id.clone();
                column![
                    kv("模型", s.model.clone().unwrap_or_else(|| "暂无".to_string())),
                    kv("输入 token", s.usage.input_tokens.to_string()),
                    kv("输出 token", s.usage.output_tokens.to_string()),
                    kv("缓存 token", s.usage.cached_tokens.to_string()),
                    kv("推理 token", s.usage.reasoning_tokens.to_string()),
                    kv("总 token", total.to_string()),
                    kv("成本(估算)", cost),
                    kv("完成原因", s.finish_reason.clone().unwrap_or_else(|| "暂无".to_string())),
                    raw_payload_panel_for_turn(app, &session_id, &turn_id, s.index),
                ]
                .spacing(8)
                .into()
            } else {
                Space::new().height(Length::Fixed(0.0)).into()
            };

            // 构建操作按钮区域（仅在展开时显示）
            let actions: Element<'_, Message> = if expanded {
                let mut actions = row![].spacing(8).align_y(Alignment::Center);

                // 如果有开始快照，添加回放和打开按钮
                if let Some(p) = s.start_snapshot_path.as_ref() {
                    actions = actions.push(icon_btn(
                        Icon::ArrowRepeat,
                        "回放(开始)",
                        Message::View(ViewMessage::ReplaySessionFromSnapshot(p.clone())),
                    ));
                    actions = actions.push(icon_btn(
                        Icon::FolderOpen,
                        "打开(开始)",
                        Message::View(ViewMessage::OpenPathInFinder(p.clone())),
                    ));
                }

                // 如果有结束快照，添加回放和打开按钮
                if let Some(p) = s.finish_snapshot_path.as_ref() {
                    actions = actions.push(icon_btn(
                        Icon::ArrowRepeat,
                        "回放(结束)",
                        Message::View(ViewMessage::ReplaySessionFromSnapshot(p.clone())),
                    ));
                    actions = actions.push(icon_btn(
                        Icon::FolderOpen,
                        "打开(结束)",
                        Message::View(ViewMessage::OpenPathInFinder(p.clone())),
                    ));
                }
                actions.into()
            } else {
                Space::new().height(Length::Fixed(0.0)).into()
            };

            // 将步骤卡片添加到列中
            col = col.push(
                container(column![
                    header,
                    // 展开时的间距
                    if expanded {
                        Space::new().height(Length::Fixed(8.0))
                    } else {
                        Space::new().height(Length::Fixed(0.0))
                    },
                    info,
                    // 展开时的间距
                    if expanded {
                        Space::new().height(Length::Fixed(8.0))
                    } else {
                        Space::new().height(Length::Fixed(0.0))
                    },
                    actions,
                ])
                .padding(12)
                .style(|theme: &Theme| {
                    let p = theme.extended_palette();
                    iced::widget::container::Style {
                        background: Some(Background::Color(p.background.base.color)),
                        border: Border {
                            width: 1.0,
                            color: p.background.strong.color,
                            radius: 10.0.into(),
                        },
                        ..Default::default()
                    }
                }),
            );
        }

        // 包装为可滚动容器，固定高度 320px
        scrollable(container(col).width(Length::Fill).padding([0u16, SCROLL_SIDE_PAD]))
            .direction(vertical_scrollbar())
            .height(Length::Fixed(320.0))
            .into()
    };

    // 包装整个面板，应用卡片样式
    container(
        column![
            super::utils::section_title("步骤"),
            Space::new().height(Length::Fixed(8.0)),
            steps_body
        ]
        .spacing(0),
    )
    .style(card_style)
    .padding(16)
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
#[path = "steps_tests.rs"]
mod steps_tests;
