//! 动画文本组件模块
//!
//! 本模块提供用于生成动画文本颜色的工具函数，主要用于创建具有渐变和闪烁效果的文本显示。
//! 核心功能包括：
//! - 根据主题和当前时间戳生成动态变化的文本颜色
//! - 支持明暗主题的自动适配
//! - 提供颜色混合算法以实现平滑的颜色过渡效果

use iced::{Color, Theme};

/// 左到右照亮扫光动画的周期（毫秒）。
pub const LEFT_TO_RIGHT_HIGHLIGHT_CYCLE_MS: u64 = 2_000;

/// 生成动画渐变文本颜色
///
/// 该函数根据当前主题、时间戳和激活状态，生成一个动态变化的文本颜色。
/// 主要用于创建视觉上吸引人的动画文本效果，例如闪烁或脉冲效果。
///
/// # 参数
///
/// * `theme` - 当前应用主题的引用，用于提取基础颜色和判断明暗模式
/// * `now_ms` - 当前时间戳（毫秒），用于驱动动画的时间变化
/// * `is_active` - 文本是否处于激活状态，`false` 时返回静态基础颜色
///
/// # 返回值
///
/// * `Some(Color)` - 计算后的文本颜色，包含动画效果
/// * `None` - 理论上不会返回 `None`，但保持 Option 类型以与其他颜色接口一致
///
/// # 算法说明
///
/// 1. 当 `is_active` 为 `false` 时，直接返回主题的次要基础文本颜色
/// 2. 根据主题背景色判断当前是明亮还是暗黑模式
/// 3. 在暗黑模式下使用较低的透明度值（0.28/0.98），明亮模式使用较高值（0.45/1.0）
/// 4. 使用时间戳 `now_ms` 实现 240ms 周期的闪烁效果
/// 5. 将选中的颜色与白色进行轻微混合，提升视觉亮度
///
/// # 示例
///
/// ```ignore
/// use iced::Theme;
/// use crate::animated_text::animated_gradient_text_color;
///
/// let theme = Theme::default();
/// let now_ms = 1000; // 当前时间戳
/// let is_active = true;
///
/// if let Some(color) = animated_gradient_text_color(&theme, now_ms, is_active) {
///     // 使用计算后的颜色渲染文本
/// }
/// ```
pub fn animated_gradient_text_color(theme: &Theme, now_ms: u64, is_active: bool) -> Option<Color> {
    // 获取主题的次要基础文本颜色作为基准
    let base = theme.extended_palette().secondary.base.text;
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;

    // 如果未激活，直接返回静态基础颜色，不应用动画效果
    if !is_active {
        return Some(if is_dark { theme.palette().text } else { base });
    }

    // 判断当前主题是否为暗色模式
    // 通过 RGB 值的和来判断：小于 1.5 认为是暗色背景
    // 根据明暗模式创建两个颜色端点
    // a: 较低透明度的基础颜色（暗色模式使用白色）
    // b: 主色调颜色（通常是强调色，蓝色）
    let a = if is_dark { Color::WHITE.scale_alpha(0.35) } else { base.scale_alpha(0.45) };
    let b = theme.palette().primary.scale_alpha(if is_dark { 0.98 } else { 1.0 });

    // 实现闪烁效果：每 240ms 切换一次颜色
    // (now_ms / 240) % 2 == 0 判断当前时间点处于闪烁周期的哪个阶段
    let blink = (now_ms / 240).is_multiple_of(2);

    // 根据 blink 状态选择使用哪个颜色端点
    let primary = if blink { b } else { a };

    // 定义与白色混合的比例，提升颜色亮度
    // 暗色模式下使用较小的提升值（0.14），亮色模式使用较大值（0.16）
    let lift = if is_dark { 0.14 } else { 0.16 };

    // 将选中的颜色与白色进行线性混合，生成最终颜色
    Some(mix_color(primary, Color::WHITE, lift))
}

/// 计算左到右照亮动画在当前字符上的混合强度。
pub fn left_to_right_sweep_mix(
    now_ms: u64,
    char_idx: usize,
    char_count: usize,
    is_active: bool,
) -> f32 {
    if !is_active || char_count == 0 {
        return 0.0;
    }

    let position =
        if char_count <= 1 { 0.5 } else { char_idx as f32 / char_count.saturating_sub(1) as f32 };
    let sweep_half_width = (0.78 / char_count as f32).clamp(0.16, 0.28);
    let cycle = (now_ms % LEFT_TO_RIGHT_HIGHLIGHT_CYCLE_MS) as f32
        / LEFT_TO_RIGHT_HIGHLIGHT_CYCLE_MS as f32;
    let center = cycle * (1.0 + sweep_half_width * 2.0) - sweep_half_width;
    let distance = (position - center).abs();
    let mix = (1.0 - distance / sweep_half_width).clamp(0.0, 1.0);
    let focused_mix = mix * mix;

    focused_mix * (3.0 - 2.0 * focused_mix)
}

/// 生成不偏色的明暗扫光文本颜色。
pub fn neutral_sweep_text_color(
    theme: &Theme,
    base: Color,
    now_ms: u64,
    char_idx: usize,
    char_count: usize,
    is_active: bool,
) -> Color {
    if !is_active {
        return base;
    }

    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;
    let highlight = if is_dark {
        mix_color(base, Color::WHITE, 0.42)
    } else {
        mix_color(base, Color::BLACK, 0.18)
    };

    mix_color(base, highlight, left_to_right_sweep_mix(now_ms, char_idx, char_count, true))
}

/// 混合两个颜色（线性插值）
///
/// 使用线性插值算法在两个颜色之间进行混合，可以创建平滑的颜色过渡效果。
/// 当 `t` 为 0 时返回颜色 `a`，当 `t` 为 1 时返回颜色 `b`，介于两者之间时返回混合颜色。
///
/// # 参数
///
/// * `a` - 起始颜色
/// * `b` - 目标颜色
/// * `t` - 混合比例，范围 [0.0, 1.0]，超出范围会被自动裁剪
///
/// # 返回值
///
/// 返回混合后的颜色，包含 RGBA 四个通道的插值结果
///
/// # 算法
///
/// 对每个颜色通道（R、G、B、A）应用公式：
/// `result = a + (b - a) * t`
///
/// # 示例
///
/// ```ignore
/// use iced::Color;
/// use crate::animated_text::mix_color;
///
/// let red = Color::from_rgb(1.0, 0.0, 0.0);
/// let blue = Color::from_rgb(0.0, 0.0, 1.0);
///
/// // 50% 混合，得到紫色
/// let purple = mix_color(red, blue, 0.5);
/// ```
pub fn mix_color(a: Color, b: Color, t: f32) -> Color {
    // 确保 t 值在有效范围内 [0.0, 1.0]
    let t = t.clamp(0.0, 1.0);

    // 对 RGBA 四个通道分别进行线性插值
    Color::from_rgba(
        a.r + (b.r - a.r) * t, // 红色通道插值
        a.g + (b.g - a.g) * t, // 绿色通道插值
        a.b + (b.b - a.b) * t, // 蓝色通道插值
        a.a + (b.a - a.a) * t, // 透明度通道插值
    )
}
#[cfg(test)]
#[path = "animated_text_tests.rs"]
mod animated_text_tests;
