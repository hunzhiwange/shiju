//! Git 差异视图选区处理模块
//!
//! 本模块提供了 Git 差异视图中行选区和悬停状态的可视化判断功能。
//! 支持拖拽范围选择、单行选择以及悬停高亮等交互模式。
//!
//! # 主要功能
//!
//! - **选区边框样式**: 提供统一的选中状态边框视觉样式
//! - **颜色混合**: 支持两个颜色之间的渐变混合计算
//! - **范围判断**: 判断指定行是否在当前激活的选区范围内
//! - **选择状态**: 判断行是否被选中（包括范围选择和单行选择）
//! - **悬停状态**: 判断行是否处于鼠标悬停状态

use iced::{Border, Color};

use super::DiffRenderCtx;
use super::DiffViewState;
use super::GitDiffLineRange;

/// 创建选中状态的边框样式
///
/// 返回一个带有金黄色边框和高圆角半径的样式，用于突出显示选中的内容。
///
/// # 返回值
///
/// 返回配置好的 `Border` 实例，包含以下特征：
/// - 宽度: 1.0 像素
/// - 颜色: 金黄色 (RGBA: 255, 210, 0, 0.95)
/// - 圆角半径: 2.0
///
/// # 示例
///
/// ```ignore
/// let border = selected_border();
/// // 在渲染选中元素时应用此边框样式
/// ```
pub fn selected_border() -> Border {
    Border { width: 1.0, color: Color::from_rgba8(255, 210, 0, 0.95), radius: 2.0.into() }
}

/// 在两个颜色之间进行线性混合
///
/// 根据参数 `t` 在颜色 `a` 和颜色 `b` 之间进行插值计算。
/// 当 `t = 0.0` 时返回颜色 `a`，当 `t = 1.0` 时返回颜色 `b`。
///
/// # 参数
///
/// - `a`: 起始颜色
/// - `b`: 目标颜色
/// - `t`: 混合比例因子，会被自动限制在 [0.0, 1.0] 范围内
///
/// # 返回值
///
/// 返回混合后的颜色，alpha 通道固定为 1.0（完全不透明）
///
/// # 示例
///
/// ```ignore
/// let red = Color::from_rgb(1.0, 0.0, 0.0);
/// let blue = Color::from_rgb(0.0, 0.0, 1.0);
/// let purple = mix_color(red, blue, 0.5); // 红蓝各半混合
/// ```
pub fn mix_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::from_rgba(
        a.r * (1.0 - t) + b.r * t,
        a.g * (1.0 - t) + b.g * t,
        a.b * (1.0 - t) + b.b * t,
        1.0,
    )
}

/// 获取当前激活的行范围选区
///
/// 检查应用状态中的激活选区，按优先级返回：
/// 1. 如果存在拖拽选区（`git_diff_drag_range`），返回该范围
/// 2. 否则，如果存在评论草稿（`git_diff_comment_draft`），返回其关联的范围
/// 3. 都不存在则返回 `None`
///
/// # 参数
///
/// - `app`: 应用状态引用
///
/// # 返回值
///
/// 返回当前激活选区的引用，若无激活选区则返回 `None`
fn active_range<'a>(app: &'a DiffViewState<'_>) -> Option<&'a GitDiffLineRange> { app.active_range }

/// 判断指定的行是否在当前激活的范围内
///
/// 检查给定文件、行号和版本类型（旧版/新版）是否匹配当前激活的选区范围。
/// 范围判断会自动处理起止点倒序的情况（即 start > end 时也能正确判断）。
///
/// # 参数
///
/// - `app`: 应用状态引用
/// - `file`: 文件路径
/// - `line`: 要检查的行号
/// - `is_old`: 是否为旧版本文件（true 表示旧版，false 表示新版）
///
/// # 返回值
///
/// 如果该行在激活范围内返回 `true`，否则返回 `false`
///
/// # 示例
///
/// ```ignore
/// // 检查第 10 行是否在选区中
/// if is_range_selected(&app, "src/main.rs", 10, false) {
///     // 第 10 行在选区中，进行高亮渲染
/// }
/// ```
pub fn is_range_selected(app: &DiffViewState<'_>, file: &str, line: usize, is_old: bool) -> bool {
    // 尝试获取当前激活的范围
    let Some(r) = active_range(app) else {
        return false;
    };
    // 检查文件名和版本类型是否匹配
    if r.file != file || r.is_old != is_old {
        return false;
    }
    // 处理范围起止点可能倒序的情况，确保 a <= b
    let (a, b) = if r.start <= r.end { (r.start, r.end) } else { (r.end, r.start) };
    // 判断行号是否在 [a, b] 闭区间内
    line >= a && line <= b
}

/// 判断指定的行是否被选中
///
/// 综合判断某行是否处于选中状态，包括两种情况：
/// 1. 该行在当前激活的范围选区内
/// 2. 该行在单行选择列表中（`git_diff_selected_lines`）
///
/// # 参数
///
/// - `app`: 应用状态引用
/// - `file`: 文件路径
/// - `line`: 要检查的行号
/// - `is_old`: 是否为旧版本文件
///
/// # 返回值
///
/// 如果该行被选中（范围或单行）返回 `true`，否则返回 `false`
///
/// # 示例
///
/// ```ignore
/// // 检查第 15 行是否被选中（包括范围选中和单行选中）
/// if is_diff_selected(&app, "src/lib.rs", 15, false) {
///     // 渲染选中样式
/// }
/// ```
pub fn is_diff_selected(
    app: &DiffViewState<'_>,
    render_ctx: &DiffRenderCtx<'_>,
    file: &str,
    line: usize,
    is_old: bool,
) -> bool {
    // 首先检查是否在范围选区内
    is_range_selected(app, file, line, is_old)
        || render_ctx.is_diff_line_selected(file, line, is_old)
}

/// 判断指定的行是否处于悬停状态
///
/// 检查给定文件、行号和版本类型是否与当前悬停状态匹配。
/// 悬停状态由 `git_diff_hovered_line` 字段维护。
///
/// # 参数
///
/// - `app`: 应用状态引用
/// - `file`: 文件路径
/// - `line`: 要检查的行号
/// - `is_old`: 是否为旧版本文件
///
/// # 返回值
///
/// 如果该行处于悬停状态返回 `true`，否则返回 `false`
///
/// # 示例
///
/// ```ignore
/// // 检查第 20 行是否被悬停
/// if is_diff_hovered(&app, "src/utils.rs", 20, false) {
///     // 渲染悬停样式
/// }
/// ```
pub fn is_diff_hovered(app: &DiffViewState<'_>, file: &str, line: usize, is_old: bool) -> bool {
    app.hovered_line
        .is_some_and(|(f, l, o)| f == file && *l == line && *o == is_old)
}
