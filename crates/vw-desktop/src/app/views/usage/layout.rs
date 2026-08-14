//! 用量视图布局模块
//!
//! 本模块负责构建用量（Usage）视图的整体布局和界面结构。
//! 它将各种用量统计数据组织成可视化的卡片布局，包括：
//! - 概览统计：显示会话和消息的基本统计信息
//! - 会话细节：显示当前会话的详细信息和模型配置
//! - 占比分析：显示不同类型消息的分布和 token 占比
//! - 步骤列表：显示会话中各个步骤的详细 token 使用情况

use iced::widget::{Space, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::app::state::UsageModelInfo;
use crate::app::{App, Message};

use super::data::UsageData;
use super::session_menu::kv_with_menu;
use super::steps::build_steps_panel;
use super::styles::card_style;
use super::utils::{fmt_ms, fmt_usd, kv, section_title};

/// 构建用量视图的主界面
///
/// 该函数创建一个完整的用量视图布局，包含多个信息卡片，
/// 用于展示会话的 token 使用情况、成本估算、模型配置等信息。
///
/// # 参数
///
/// * `app` - 应用状态引用，用于获取会话运行时配置和模型信息
/// * `usage_data` - 用量数据引用，包含当前会话的统计信息
///
/// # 返回值
///
/// 返回一个 `Element<'static, Message>`，表示可渲染的 UI 元素。
/// 该元素是一个可滚动的垂直布局，包含多个信息卡片。
///
/// # 布局结构
///
/// 视图从上到下依次包含以下卡片：
/// 1. **概览卡片** - 显示会话标题、消息数、token 总数等基本信息
/// 2. **会话细节卡片** - 显示模型配置、上下文限制、成本估算等详细信息
/// 3. **占比卡片** - 显示消息类型分布和 token 类型占比的可视化
/// 4. **步骤卡片** - 显示每个步骤的 token 使用详情
///
/// # 示例
///
/// ```ignore
/// let usage_data = UsageData::from_session(&app);
/// let view = build_usage_view(&app, &usage_data);
/// // view 可直接用于 iced 应用的渲染
/// ```
pub fn build_usage_view(app: &App, usage_data: &UsageData) -> Element<'static, Message> {
    // ===== 概览卡片：左侧统计信息 =====
    // 显示会话的基本统计，包括标题、消息数、原始记录数和 token 总数
    let stats_left = column![
        kv("会话", usage_data.session_title.clone()),
        kv("消息数", usage_data.message_count.to_string()),
        kv("原始记录", usage_data.call_count.to_string()),
        kv("总 token", usage_data.total_tokens.to_string()),
        kv("最近总 token", usage_data.last_step_total_tokens.to_string()),
    ]
    .spacing(10);

    // ===== 概览卡片：右侧统计信息 =====
    // 显示 token 的详细分类统计，包括累计值和最近步骤的值
    let stats_right = column![
        kv("总输入 token", app.usage.input_tokens.to_string()),
        kv("总输出 token", app.usage.output_tokens.to_string()),
        kv("总缓存 token", app.usage.cached_tokens.to_string()),
        kv("总推理 token", app.usage.reasoning_tokens.to_string()),
        kv("最近输入 token", usage_data.last_step_input_tokens.to_string()),
        kv("最近输出 token", usage_data.last_step_output_tokens.to_string()),
        kv("最近缓存 token", usage_data.last_step_cached_tokens.to_string()),
        kv("最近推理 token", usage_data.last_step_reasoning_tokens.to_string()),
    ]
    .spacing(10);

    // 组装概览卡片：包含标题和左右两列统计信息
    let stats = container(
        column![
            section_title("概览"),
            Space::new().height(Length::Fixed(8.0)),
            row![stats_left.width(Length::Fill), stats_right.width(Length::Fill)].spacing(24)
        ]
        .spacing(0),
    )
    .style(card_style)
    .padding(16)
    .width(Length::Fill);

    // ===== 模型信息处理 =====
    // 获取当前会话的运行时配置
    let runtime = app.current_session_runtime();

    // 构建模型显示行：优先显示模型信息，其次显示自动模型标识，最后显示配置的模型名
    let model_line = if let Some(info) = app.usage_model_info.as_ref() {
        format!("{} · {}", info.provider_name, info.model_name)
    } else if runtime.auto_model {
        "自动模型".to_string()
    } else {
        runtime.model.clone()
    };

    // 构建模型引用：用于标识具体的模型 ID
    let model_ref = if let Some(info) = app.usage_model_info.as_ref() {
        format!("{}/{}", info.provider_id, info.model_id)
    } else if runtime.auto_model {
        "自动".to_string()
    } else {
        runtime.model.clone()
    };

    // 计算上下文使用率：最近步骤输入 token 占当前输入窗口的百分比
    let usage_rate = app
        .usage_model_info
        .as_ref()
        .map(|info| {
            if info.input_limit == 0 {
                "0.0%".to_string()
            } else {
                format!(
                    "{:.1}%",
                    (usage_data.last_step_input_tokens as f64) * 100.0 / (info.input_limit as f64)
                )
            }
        })
        .unwrap_or_else(|| "暂无".to_string());

    // 构建成本价格字符串：显示每百万 token 的成本
    let cost_price = app.usage_model_info.as_ref().map(|info| {
        format!(
            "输入 {}/M · 输出 {}/M · 缓存读 {}/M · 缓存写 {}/M",
            fmt_usd(info.cost_input_per_million),
            fmt_usd(info.cost_output_per_million),
            fmt_usd(info.cost_cache_read_per_million),
            fmt_usd(info.cost_cache_write_per_million),
        )
    });

    // 估算成本的内部闭包
    //
    // 根据输入、输出和缓存的 token 数量，结合模型的价格信息计算总成本。
    //
    // # 参数
    // * `input_tokens` - 输入 token 数量
    // * `output_tokens` - 输出 token 数量
    // * `cached_tokens` - 缓存 token 数量
    // * `info` - 模型价格信息引用
    //
    // # 返回值
    // 返回估算的成本（美元）
    let estimate_cost =
        |input_tokens: i64, output_tokens: i64, cached_tokens: i64, info: &UsageModelInfo| {
            // 将 token 数量转换为百万单位
            let input = (input_tokens.max(0) as f64) / 1_000_000.0;
            let output = (output_tokens.max(0) as f64) / 1_000_000.0;
            let cached = (cached_tokens.max(0) as f64) / 1_000_000.0;

            // 计算总成本：输入成本 + 输出成本 + 缓存读取成本
            input * info.cost_input_per_million
                + output * info.cost_output_per_million
                + cached * info.cost_cache_read_per_million
        };

    // 计算累计总成本
    let total_cost = app.usage_model_info.as_ref().map(|info| {
        estimate_cost(
            app.usage.input_tokens,
            app.usage.output_tokens,
            app.usage.cached_tokens,
            info,
        )
    });

    // 计算最近步骤的成本
    let last_cost = app.usage_model_info.as_ref().map(|info| {
        estimate_cost(
            usage_data.last_step_input_tokens,
            usage_data.last_step_output_tokens,
            usage_data.last_step_cached_tokens,
            info,
        )
    });

    // ===== 会话细节卡片：左侧信息 =====
    // 显示会话的基本信息，包括标题、创建时间、最后活动时间和模型
    let session_detail_left = column![
        kv_with_menu(app, "标题", usage_data.session_title.clone(), app.active_session_id.clone(),),
        kv(
            "创建时间",
            usage_data
                .session
                .as_ref()
                .map(|s| fmt_ms(s.created_ms))
                .unwrap_or_else(|| "暂无".to_string()),
        ),
        kv(
            "最后活动",
            usage_data
                .session
                .as_ref()
                .map(|s| fmt_ms(s.updated_ms))
                .unwrap_or_else(|| "暂无".to_string()),
        ),
        kv("模型", model_line),
    ]
    .spacing(10);

    // ===== 会话细节卡片：右侧信息 =====
    // 显示模型的详细配置、限制和成本信息
    let session_detail_right = column![
        kv("模型引用", model_ref),
        kv(
            "上下文限制",
            app.usage_model_info
                .as_ref()
                .map(|i| i.context_limit.to_string())
                .unwrap_or_else(|| "暂无".to_string()),
        ),
        kv(
            "输出上限",
            app.usage_model_info
                .as_ref()
                .map(|i| i.output_limit.to_string())
                .unwrap_or_else(|| "暂无".to_string()),
        ),
        kv(
            "输入窗口",
            app.usage_model_info
                .as_ref()
                .map(|i| i.input_limit.to_string())
                .unwrap_or_else(|| "暂无".to_string()),
        ),
        kv(
            "限制说明",
            app.usage_model_info
                .as_ref()
                .filter(|i| i.limit_is_default)
                .map(|_| "使用保守默认值".to_string())
                .unwrap_or_else(|| "模型配置".to_string()),
        ),
        kv("输入 token", usage_data.last_step_input_tokens.to_string()),
        kv("使用率", usage_rate),
        kv("成本价", cost_price.unwrap_or_else(|| "暂无".to_string())),
        kv("最近成本(估算)", last_cost.map(fmt_usd).unwrap_or_else(|| "暂无".to_string()),),
        kv("总成本(估算)", total_cost.map(fmt_usd).unwrap_or_else(|| "暂无".to_string()),),
    ]
    .spacing(10);

    // 组装会话细节卡片：包含标题和左右两列详细信息
    let session_details = container(
        column![
            section_title("会话细节"),
            Space::new().height(Length::Fixed(8.0)),
            row![session_detail_left.width(Length::Fill), session_detail_right.width(Length::Fill)]
                .spacing(24)
        ]
        .spacing(0),
    )
    .style(card_style)
    .padding(16)
    .width(Length::Fill);

    // ===== 占比卡片：左侧 - 消息类型统计 =====
    // 显示用户、助手、系统和工具消息的数量
    let breakdown_left = column![
        kv("用户消息", usage_data.user_msgs.to_string()),
        kv("助手消息", usage_data.assistant_msgs.to_string()),
        kv("系统消息", usage_data.system_msgs.to_string()),
        kv("工具消息", usage_data.tool_msgs.to_string()),
    ]
    .spacing(10);

    // ===== 占比卡片：右侧 - Token 占比可视化 =====
    // 使用堆叠条形图展示不同类型 token 的占比
    let breakdown_right = {
        // 计算最近步骤的总 token 数（用于占比计算）
        let token_total = usage_data.last_step_input_tokens
            + usage_data.last_step_output_tokens
            + usage_data.last_step_cached_tokens
            + usage_data.last_step_reasoning_tokens;

        column![
            // 标题：显示 token 总数
            text(format!("Token 占比 · {}", token_total)).size(12).style(|theme: &iced::Theme| {
                iced::widget::text::Style {
                    color: Some(theme.extended_palette().background.weak.text.scale_alpha(0.9)),
                }
            }),
            // 堆叠条形图：可视化各类 token 的占比
            super::steps::stacked_bar(&[
                (usage_data.last_step_input_tokens as usize, super::steps::BarSeg::Prompt),
                (usage_data.last_step_output_tokens as usize, super::steps::BarSeg::Answer),
                (usage_data.last_step_cached_tokens as usize, super::steps::BarSeg::Tool),
                (usage_data.last_step_reasoning_tokens as usize, super::steps::BarSeg::Other),
            ]),
            // 图例：显示各类 token 的具体数值和占比
            column![
                super::steps::legend_item(
                    "输入",
                    usage_data.last_step_input_tokens as usize,
                    token_total as usize,
                    super::steps::BarSeg::Prompt
                ),
                super::steps::legend_item(
                    "输出",
                    usage_data.last_step_output_tokens as usize,
                    token_total as usize,
                    super::steps::BarSeg::Answer
                ),
                super::steps::legend_item(
                    "缓存",
                    usage_data.last_step_cached_tokens as usize,
                    token_total as usize,
                    super::steps::BarSeg::Tool
                ),
                super::steps::legend_item(
                    "推理",
                    usage_data.last_step_reasoning_tokens as usize,
                    token_total as usize,
                    super::steps::BarSeg::Other
                ),
            ]
            .spacing(6),
        ]
        .spacing(10)
    };

    // 组装占比卡片：包含标题和左右两列
    let breakdown = container(
        column![
            section_title("占比"),
            Space::new().height(Length::Fixed(8.0)),
            row![breakdown_left.width(Length::Fill), breakdown_right.width(Length::Fill)]
                .spacing(24)
        ]
        .spacing(0),
    )
    .style(card_style)
    .padding(16)
    .width(Length::Fill);

    // ===== 步骤卡片 =====
    // 显示每个步骤的详细 token 使用情况
    let steps_card = build_steps_panel(app, usage_data);

    // ===== 组装完整视图 =====
    // 将所有卡片按垂直方向排列，并设置最大宽度
    let root = column![
        container(text("用量").size(18)).padding([4, 2]),
        Space::new().height(Length::Fixed(6.0)),
        stats,
        Space::new().height(Length::Fixed(12.0)),
        session_details,
        Space::new().height(Length::Fixed(12.0)),
        breakdown,
        Space::new().height(Length::Fixed(12.0)),
        steps_card,
    ]
    .spacing(0)
    .max_width(920);

    // 返回可滚动的容器
    scrollable(container(root).padding([18, 18]).width(Length::Fill)).into()
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod layout_tests;
