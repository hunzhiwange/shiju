//! 使用率可视化模块
//!
//! 本模块提供 token 使用率的可视化组件，包括：
//! - 圆环进度条，显示当前上下文使用百分比
//! - 使用率计算函数，从应用状态中提取并计算使用统计
//!
//! 主要用于输入面板中显示当前会话的 token 消耗情况。

use crate::app::{App, Message};
use iced::widget::canvas::{Frame, Geometry, Path as CanvasPath, Program, Stroke};
use iced::{Color, Theme};

/// 计算当前上下文使用率百分比
///
/// 根据活跃会话的最后一个步骤的输入 token 数与模型输入窗口限制，
/// 计算当前上下文使用百分比。
///
/// # 参数
///
/// - `app`: 应用状态引用，包含会话信息和模型使用配置
///
/// # 返回
///
/// 返回使用率百分比（0.0 - 100.0），如果无法计算则返回 0.0
///
/// # 计算逻辑
///
/// 1. 从活跃会话的最后一个步骤获取输入 token 数
/// 2. 获取模型的有效输入窗口
/// 3. 计算 (input_tokens / input_limit) * 100
pub fn get_usage_rate_percent(app: &App) -> f32 {
    // 尝试从活跃会话获取最后一个步骤的输入 token 数
    if let Some(info) = &app.usage_model_info {
        let last_step_input_tokens = app
            .active_session_view_state
            .steps
            .last()
            .map(|step| step.usage.input_tokens)
            .unwrap_or(0);
        // 防止除以零
        if info.input_limit == 0 {
            return 0.0;
        }
        // 计算使用率百分比
        ((last_step_input_tokens as f64) * 100.0 / (info.input_limit as f64)) as f32
    } else {
        0.0
    }
}

/// 获取详细的使用统计信息
///
/// 从应用状态中提取完整的使用数据，包括 token 数量和估算费用。
///
/// # 参数
///
/// - `app`: 应用状态引用，包含会话和模型使用信息
///
/// # 返回
///
/// 返回元组 (最后步骤输入tokens, 上下文限制, 估算费用, 总token数):
/// - `last_step_input_tokens`: 活跃会话最后一步的输入 token 数
/// - `context_limit`: 模型的上下文窗口限制
/// - `estimated_cost`: 基于 token 使用量的估算费用（美元）
/// - `total_tokens`: 所有类型 token 的总和（输入+输出+缓存+推理）
///
/// # 费用计算
///
/// 估算费用使用输入和输出 token 价格的平均值计算：
/// `estimated_cost = (total_tokens / 1,000,000) * ((input_price + output_price) / 2)`
pub fn get_usage_details(app: &App) -> (i64, i64, f64, i64) {
    // 获取活跃会话最后一步的输入 token 数
    let last_step_input_tokens =
        app.active_session_view_state.steps.last().map(|step| step.usage.input_tokens).unwrap_or(0);

    // 获取模型的上下文限制
    let context_limit = app.usage_model_info.as_ref().map(|i| i.context_limit as i64).unwrap_or(0);

    // 获取模型的每百万 token 价格（输入和输出）
    let (cost_input, cost_output) = app
        .usage_model_info
        .as_ref()
        .map(|info| (info.cost_input_per_million, info.cost_output_per_million))
        .unwrap_or((0.0, 0.0));

    // 计算所有类型 token 的总和
    let total_tokens = app.usage.input_tokens
        + app.usage.output_tokens
        + app.usage.cached_tokens
        + app.usage.reasoning_tokens;

    // 估算费用：使用平均价格
    let estimated_cost = (total_tokens as f64 / 1_000_000.0) * ((cost_input + cost_output) / 2.0);

    (last_step_input_tokens, context_limit, estimated_cost, total_tokens)
}

/// 使用率圆环组件
///
/// 一个基于 Canvas 的圆环进度条，用于可视化显示使用率百分比。
/// 根据使用率的不同区间显示不同的颜色：
/// - 0% - 70%: 绿色 (#2EC27E) - 正常
/// - 70% - 90%: 橙色 (#F5A623) - 警告
/// - 90% - 100%: 红色 (#E53E3E) - 危险
///
/// # 示例
///
/// ```ignore
/// let ring = UsageRing { percent: 75.0 };
/// // 在 Canvas 中使用
/// ```
#[derive(Debug, Clone, Copy)]
pub struct UsageRing {
    /// 使用率百分比（0.0 - 100.0）
    pub percent: f32,
}

impl Program<Message> for UsageRing {
    /// 画布程序状态（本组件无需状态管理）
    type State = ();

    /// 绘制圆环进度条
    ///
    /// # 参数
    ///
    /// - `_state`: 画布状态（未使用）
    /// - `renderer`: Iced 渲染器
    /// - `theme`: 当前主题，用于获取配色
    /// - `bounds`: 绘制区域的边界矩形
    /// - `_cursor`: 鼠标光标位置（未使用）
    ///
    /// # 返回
    ///
    /// 返回包含圆环和百分比文本的几何图形向量
    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // 边界检查：无效尺寸时返回空图形
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return vec![frame.into_geometry()];
        }

        // 计算圆环的几何参数
        let size = bounds.size();
        let center = iced::Point::new(size.width / 2.0, size.height / 2.0);
        let radius = (size.width.min(size.height) / 2.0) - 2.0;
        let stroke_width = 2.4;

        // 绘制百分比文本
        let percent_val = self.percent.clamp(0.0, 100.0);
        let text_color = theme.extended_palette().secondary.base.text;
        let content = format!("{:.0}", percent_val);
        // 估算文本宽度以居中显示
        let estimated_width = content.len() as f32 * 8.0 * 0.6;
        let estimated_height = 8.0;
        let offset = iced::Vector::new(estimated_width / 2.0, estimated_height / 1.6);

        frame.fill_text(iced::widget::canvas::Text {
            content,
            position: center - offset,
            color: text_color,
            size: 8.0.into(),
            ..Default::default()
        });

        // 绘制背景圆环（灰色）
        let bg_color = theme.extended_palette().background.strong.color;
        let bg_path = CanvasPath::circle(center, radius);
        frame.stroke(
            &bg_path,
            Stroke { width: stroke_width, style: bg_color.into(), ..Stroke::default() },
        );

        // 根据使用率确定进度颜色
        let percent = self.percent.clamp(0.0, 100.0) / 100.0;
        let progress_color = if percent < 0.7 {
            // 0% - 70%: 绿色 - 正常状态
            Color::from_rgb8(0x2E, 0xC2, 0x7E)
        } else if percent < 0.9 {
            // 70% - 90%: 橙色 - 警告状态
            Color::from_rgb8(0xF5, 0xA6, 0x23)
        } else {
            // 90% - 100%: 红色 - 危险状态
            Color::from_rgb8(0xE5, 0x3E, 0x3E)
        };

        // 使用虚线绘制进度弧（显示使用百分比）
        if percent > 0.001 {
            let circumference = 2.0 * std::f32::consts::PI * radius;
            let dash_length = circumference * percent;
            let gap_length = circumference * (1.0 - percent);

            frame.stroke(
                &bg_path,
                Stroke {
                    width: stroke_width,
                    style: progress_color.into(),
                    line_dash: iced::widget::canvas::LineDash {
                        segments: &[dash_length, gap_length],
                        // 从顶部开始绘制（1/4 周长偏移）
                        offset: (circumference / 4.0) as usize,
                    },
                    ..Stroke::default()
                },
            );
        }

        vec![frame.into_geometry()]
    }
}
