//! CLI 渲染辅助模块
//!
//! 本模块提供 CLI 界面的底层渲染功能，用于创建视觉元素和动画效果。
//!
//! # 主要功能
//!
//! - **扫描线背景渲染**：创建交替条纹的背景效果，增强视觉层次感
//! - **执行状态指示器**：渲染动画化的执行进度指示器，显示代理的运行状态
//!
//! # 使用场景
//!
//! 这些渲染函数在 CLI 事件循环中被调用，用于：
//! - 在代理执行任务时提供视觉反馈
//! - 创建专业的终端 UI 外观
//! - 增强用户对系统状态的感知

use super::theme::{ACCENT_CYAN, EXECUTION_DOT, SCANLINE_DARK, SCANLINE_LIGHT, SURFACE_BASE};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;

/// 渲染扫描线背景效果
///
/// 创建交替颜色的条纹背景，模拟老式 CRT 显示器的扫描线效果。
/// 这种视觉效果能够增加终端界面的层次感和科技感。
///
/// # 参数
///
/// - `f`：ratatui 的 Frame 实例，用于绑定渲染目标
/// - `area`：渲染区域，定义背景的尺寸和位置
///
/// # 渲染逻辑
///
/// 1. **边界检查**：如果区域宽度或高度为 0，直接返回
/// 2. **条纹生成**：偶数行使用较深颜色，奇数行使用较浅颜色
/// 3. **颜色配置**：
///    - 偶数行：RGB(7, 16, 25) - 深蓝色
///    - 奇数行：RGB(10, 20, 32) - 略浅的深蓝色
///
/// # 示例
///
/// ```ignore
/// use ratatui::Frame;
/// use ratatui::layout::Rect;
///
/// fn render_ui(f: &mut Frame, area: Rect) {
///     // 在指定区域渲染扫描线背景
///     render_scanline_background(f, area);
/// }
/// ```
pub(crate) fn render_scanline_background(f: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
    // 边界检查：区域无效时直接返回
    if area.width == 0 || area.height == 0 {
        return;
    }

    // 预分配行向量以提高性能
    let mut lines: Vec<Line> = Vec::with_capacity(area.height as usize);

    // 生成与区域等宽的空白字符串作为条纹模板
    let stripe = " ".repeat(area.width as usize);

    // 逐行生成条纹
    for row in 0..area.height {
        // 交替使用两种深度的蓝色背景
        // 偶数行更深，奇数行略浅，形成扫描线效果
        let bg = if row % 2 == 0 { SCANLINE_DARK } else { SCANLINE_LIGHT };

        // 将条纹添加到行集合中
        lines.push(Line::from(Span::styled(stripe.clone(), Style::default().bg(bg))));
    }

    // 使用 Paragraph 组件渲染背景
    f.render_widget(Paragraph::new(Text::from(lines)), area);
}

/// 渲染执行状态指示器
///
/// 创建一个动画化的进度指示器，通过两个来回移动的方块（■）展示代理的
/// 执行状态。这种设计既美观又能清晰传达"正在处理中"的状态。
///
/// # 参数
///
/// - `f`：ratatui 的 Frame 实例，用于绑定渲染目标
/// - `area`：渲染区域，定义指示器的尺寸和位置
/// - `busy`：是否处于繁忙（执行中）状态
/// - `spinner_idx`：动画帧索引，用于计算方块位置
///
/// # 渲染逻辑
///
/// ## 非繁忙状态
/// - 渲染一个纯色背景的空白行，不显示动画
///
/// ## 繁忙状态
/// - 背景由点（.）组成，使用灰色
/// - 两个方块（■）在行内来回移动，使用亮青色
/// - 方块位置根据 `spinner_idx` 计算，形成往返动画效果
///
/// # 动画算法
///
/// 1. 计算移动范围：`travel = width - 1`
/// 2. 计算偏移量：`offset = spinner_idx % travel`
/// 3. 左侧方块位置：`pos_left = offset`
/// 4. 右侧方块位置：`pos_right = travel - offset`
/// 5. 处理两方块相遇的情况，避免重叠
///
/// # 颜色配置
///
/// - 背景：RGB(8, 18, 28) - 深色背景
/// - 点字符（.）：RGB(70, 82, 98) - 灰色前景
/// - 方块字符（■）：RGB(120, 210, 255) - 亮青色前景，加粗
///
/// # 示例
///
/// ```ignore
/// use ratatui::Frame;
/// use ratatui::layout::Rect;
///
/// fn render_status(f: &mut Frame, area: Rect, frame_count: usize) {
///     let is_busy = agent_is_running();
///     render_execution_indicator(f, area, is_busy, frame_count);
/// }
/// ```
pub(crate) fn render_execution_indicator(
    f: &mut ratatui::Frame<'_>,
    area: ratatui::layout::Rect,
    busy: bool,
    spinner_idx: usize,
) {
    // 边界检查：区域无效时直接返回
    if area.width == 0 || area.height == 0 {
        return;
    }

    // 计算单行渲染区域（垂直居中）
    let line_area = ratatui::layout::Rect {
        x: area.x,
        y: area.y.saturating_add(area.height / 2), // 垂直居中
        width: area.width,
        height: 1, // 只占用一行
    };

    // 定义颜色方案
    let bg = SURFACE_BASE;
    let dot_style = Style::default().fg(EXECUTION_DOT).bg(bg);
    let block_style = Style::default().fg(ACCENT_CYAN).bg(bg).add_modifier(Modifier::BOLD); // 加粗效果

    let width = line_area.width as usize;

    // 生成空白字符串（用于非繁忙状态）
    let blank = " ".repeat(width);

    // 非繁忙状态：渲染纯色背景的空白行
    if !busy {
        let lines = vec![Line::from(Span::styled(blank.clone(), Style::default().bg(bg)))];
        f.render_widget(Paragraph::new(Text::from(lines)), line_area);
        return;
    }

    // 繁忙状态：渲染动画指示器

    // 生成点字符填充的行
    let _dots = ".".repeat(width);

    // 计算动画移动范围
    // travel 是方块可以移动的最大偏移量
    let travel = width.saturating_sub(1).max(1);

    // 使用 spinner_idx 计算当前偏移量（循环）
    let offset = spinner_idx % travel;

    // 计算两个方块的位置
    // 左侧方块从左向右移动
    let pos_left = offset;
    // 右侧方块从右向左移动
    let mut pos_right = travel.saturating_sub(offset);

    // 处理两方块相遇的情况
    // 当它们位置相同时，将右侧方块向后移一位，避免重叠
    if pos_left == pos_right && width > 1 {
        pos_right = (pos_right + 1) % width;
    }

    // 收集并去重方块位置
    let mut positions = vec![pos_left, pos_right];
    positions.sort_unstable();
    positions.dedup();

    // 构建 Span 序列
    // 将点字符和方块字符按照位置组合成完整的行
    let mut spans: Vec<Span> = Vec::new();
    let mut last = 0usize; // 上一个处理位置

    for pos in positions {
        // 添加当前位置之前的点字符
        if pos > last {
            spans.push(Span::styled(".".repeat(pos - last), dot_style));
        }

        // 在当前位置添加方块字符
        spans.push(Span::styled("■", block_style));

        // 更新处理位置（方块占用 1 个字符）
        last = pos + 1;
    }

    // 添加剩余的点字符（行尾部分）
    if last < width {
        spans.push(Span::styled(".".repeat(width - last), dot_style));
    }

    // 渲染最终的行
    let lines = vec![Line::from(spans)];
    f.render_widget(Paragraph::new(Text::from(lines)), line_area);
}
