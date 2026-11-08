//! 项目视图工具模块
//!
//! 本模块提供项目视图相关的辅助工具函数，包括：
//! - 徽章标签生成
//! - 颜色计算与处理
//! - 字符串显示宽度处理
//! - 路径格式化
//!
//! 这些函数主要用于 UI 渲染辅助，确保界面元素的一致性和可读性。

use iced::Color;
use unicode_width::UnicodeWidthChar;

/// 从项目标题生成徽章标签
///
/// 该函数从项目标题中提取一个用于徽章显示的短标签。
/// 提取逻辑如下：
/// 1. 跳过前导空白字符
/// 2. 优先使用第一个字母数字字符
/// 3. 如果没有字母数字字符，则使用第一个非空白字符
/// 4. 如果标题全为空白或为空，返回 "?"
///
/// # 参数
///
/// * `title` - 项目标题字符串
///
/// # 返回值
///
/// 返回单字符徽章标签：
/// - 优先返回首个字母数字字符
/// - ASCII 字母会转换为大写
/// - 没有字母数字时回退到首个非空白字符
/// - 全空白时返回 `?`
pub(super) fn project_badge_label(title: &str) -> String {
    let mut first_non_ws = None;

    for ch in title.chars() {
        if ch.is_whitespace() {
            continue;
        }

        if first_non_ws.is_none() {
            first_non_ws = Some(ch);
        }

        if ch.is_alphanumeric() {
            return if ch.is_ascii_alphabetic() {
                ch.to_ascii_uppercase().to_string()
            } else {
                ch.to_string()
            };
        }
    }

    first_non_ws
        .map(|ch| {
            if ch.is_ascii_alphabetic() {
                ch.to_ascii_uppercase().to_string()
            } else {
                ch.to_string()
            }
        })
        .unwrap_or_else(|| "?".to_string())
}
///
/// 计算字符串的稳定 32 位哈希值
///
/// 使用 FNV-1a 哈希算法计算字符串的 32 位哈希值。
/// 该算法具有良好的分布性和性能，适合用于颜色选择等场景。
///
/// # 参数
///
/// * `s` - 要计算哈希的字符串
///
/// # 返回值
///
/// 返回 32 位无符号整数哈希值
///
/// # 示例
///
/// ```ignore
/// let hash1 = stable_hash32("project-alpha");
/// let hash2 = stable_hash32("project-alpha");
/// assert_eq!(hash1, hash2); // 相同输入产生相同输出
/// ```
pub(super) fn stable_hash32(s: &str) -> u32 {
    // FNV 偏移基数
    let mut hash: u32 = 2166136261;

    // 对每个字节进行哈希计算
    for b in s.as_bytes() {
        // 异或操作
        hash ^= *b as u32;
        // 乘以 FNV 质数（使用 wrapping_mul 处理溢出）
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

/// 根据种子字符串生成项目强调色
///
/// 该函数根据输入的种子字符串（通常是项目名称）从预定义的调色板中
/// 选择一个颜色。使用哈希函数确保相同种子总是产生相同颜色，
/// 而不同种子倾向于产生不同颜色。
///
/// # 参数
///
/// * `seed` - 种子字符串，通常使用项目名称或路径
///
/// # 返回值
///
/// 返回调色板中的一个颜色
///
/// # 调色板
///
/// 包含 10 种精选颜色，涵盖紫色、粉红、蓝色、橙色、绿色、青色等
///
/// # 示例
///
/// ```ignore
/// let color1 = project_accent_color("my-project");
/// let color2 = project_accent_color("my-project");
/// assert_eq!(color1, color2); // 相同种子产生相同颜色
/// ```
pub(super) fn project_accent_color(seed: &str) -> Color {
    // 预定义的项目强调色调色板
    let palette = [
        Color::from_rgb8(0x8A, 0x3F, 0xFF), // 紫色
        Color::from_rgb8(0xFF, 0x4D, 0x7D), // 粉红
        Color::from_rgb8(0x00, 0xA3, 0xFF), // 亮蓝
        Color::from_rgb8(0xF2, 0xA9, 0x00), // 金黄
        Color::from_rgb8(0x2E, 0xB8, 0x72), // 翠绿
        Color::from_rgb8(0xFF, 0x7A, 0x00), // 橙色
        Color::from_rgb8(0x00, 0xC2, 0xB8), // 青色
        Color::from_rgb8(0xEF, 0x44, 0x44), // 红色
        Color::from_rgb8(0x3B, 0x82, 0xF6), // 蓝色
        Color::from_rgb8(0xA8, 0x55, 0xF7), // 紫罗兰
    ];

    // 使用哈希值选择颜色
    let idx = (stable_hash32(seed) as usize) % palette.len();
    palette[idx]
}

/// 根据背景色计算对比度合适的文本颜色
///
/// 使用相对亮度公式计算背景色的亮度，然后选择黑色或白色作为文本颜色，
/// 以确保文本在背景上清晰可读。
///
/// # 参数
///
/// * `bg` - 背景颜色
///
/// # 返回值
///
/// 返回适合在指定背景上显示的文本颜色：
/// - 亮度 > 0.62：返回深色 (#121212)
/// - 亮度 ≤ 0.62：返回白色
///
/// # 亮度公式
///
/// 使用 ITU-R BT.709 标准的相对亮度公式：
/// L = 0.2126 * R + 0.7152 * G + 0.0722 * B
///
/// # 示例
///
/// ```ignore
/// let dark_bg = Color::from_rgb8(0x20, 0x20, 0x20);
/// let text_color = contrast_text_color(dark_bg);
/// assert_eq!(text_color, Color::WHITE);
/// ```
pub(super) fn contrast_text_color(bg: Color) -> Color {
    // 使用 ITU-R BT.709 标准计算相对亮度
    let lum = 0.2126 * bg.r + 0.7152 * bg.g + 0.0722 * bg.b;

    // 亮度阈值 0.62，高于此值使用深色文本，否则使用白色文本
    if lum > 0.62 { Color::from_rgb8(18, 18, 18) } else { Color::WHITE }
}

/// 使颜色变亮
///
/// 将输入颜色向白色方向提亮，通过将颜色分量与白色混合实现。
/// 混合比例约为 1:3（原色:白色）。
///
/// # 参数
///
/// * `color` - 要变亮的颜色
///
/// # 返回值
///
/// 返回变亮后的颜色
///
/// # 算法
///
/// 使用公式：new = (old + 3.0) / 4.0
/// 这相当于将颜色与白色以 1:3 的比例混合
///
/// # 示例
///
/// ```ignore
/// let dark = Color::from_rgb8(0x40, 0x40, 0x40);
/// let lighter = lighten_color(dark);
/// // lighter 会比 dark 更亮
/// ```
pub(super) fn lighten_color(color: Color) -> Color {
    Color::from_rgb((color.r + 3.0) / 4.0, (color.g + 3.0) / 4.0, (color.b + 3.0) / 4.0)
}

/// 根据显示宽度截断字符串
///
/// 考虑 Unicode 字符的实际显示宽度进行截断，在末尾添加省略号。
/// 不同字符的显示宽度不同（例如中文通常占 2 个宽度单位，ASCII 占 1 个）。
///
/// # 参数
///
/// * `s` - 要截断的字符串
/// * `max_width` - 最大显示宽度（以等宽字符为单位）
///
/// # 返回值
///
/// 返回截断后的字符串，如果原字符串宽度超过 max_width，
/// 则在末尾添加省略号 "…"
///
/// # 边界情况
///
/// - 如果 `max_width` 为 0，返回空字符串
/// - 如果 `max_width` 为 1，返回 "…"
/// - 如果字符串宽度 <= max_width，返回原字符串
///
/// # 示例
///
/// ```ignore
/// let result = truncate_display_width("Hello World", 8);
/// assert_eq!(result, "Hello W…");
///
/// let result = truncate_display_width("你好世界", 5);
/// assert_eq!(result, "你好…"); // 每个中文字符宽度为 2
/// ```
pub fn truncate_display_width(s: &str, max_width: usize) -> String {
    // 计算字符串的总显示宽度
    let total_width: usize = s.chars().map(|ch| ch.width().unwrap_or(0)).sum();

    // 如果总宽度不超过限制，直接返回原字符串
    if total_width <= max_width {
        return s.to_string();
    }

    // 处理边界情况
    if max_width == 0 {
        return String::new();
    }
    if max_width == 1 {
        return "…".to_string();
    }

    let mut out = String::new();
    let mut used = 0usize;

    // 预留 1 个宽度单位给省略号
    let keep_width = max_width - 1;

    // 逐字符添加，直到达到保留宽度
    for ch in s.chars() {
        let w = ch.width().unwrap_or(0);

        // 如果添加当前字符会超出限制，停止
        if used + w > keep_width {
            break;
        }

        out.push(ch);
        used += w;
    }

    // 添加省略号
    out.push('…');
    out
}

/// 为工具提示格式化工作空间路径
///
/// 将工作空间目录路径转换为相对于项目根目录的相对路径，
/// 使工具提示更加简洁易读。
///
/// # 参数
///
/// * `workspace_dir` - 工作空间目录的完整路径
/// * `project_root` - 项目根目录的完整路径
///
/// # 返回值
///
/// 返回格式化后的路径字符串：
/// - 如果工作空间是项目根目录，返回 "."
/// - 如果工作空间在项目内，返回相对路径
/// - 如果工作空间不在项目内，返回原路径
///
/// # 示例
///
/// ```ignore
/// let path = workspace_path_for_tooltip(
///     "/home/user/project/src",
///     "/home/user/project"
/// );
/// assert_eq!(path, "src");
///
/// let path = workspace_path_for_tooltip(
///     "/home/user/project",
///     "/home/user/project"
/// );
/// assert_eq!(path, ".");
/// ```
#[allow(dead_code)]
pub(super) fn workspace_path_for_tooltip(workspace_dir: &str, project_root: &str) -> String {
    let workspace_path = std::path::Path::new(workspace_dir);
    let project_root_path = std::path::Path::new(project_root);

    // 尝试将工作空间路径转换为相对于项目根目录的相对路径
    if let Ok(relative) = workspace_path.strip_prefix(project_root_path) {
        // 如果相对路径为空，说明工作空间就是项目根目录
        if relative.as_os_str().is_empty() {
            return ".".to_string();
        }
        // 返回相对路径
        return relative.to_string_lossy().to_string();
    }

    // 如果不在项目内，返回原路径
    workspace_dir.to_string()
}

/// 线性插值混合两种颜色
///
/// 根据参数 t 在两种颜色之间进行线性插值，
/// t=0 时返回颜色 a，t=1 时返回颜色 b。
///
/// # 参数
///
/// * `a` - 起始颜色
/// * `b` - 目标颜色
/// * `t` - 插值参数，范围 [0.0, 1.0]，超出范围会被截断
///
/// # 返回值
///
/// 返回混合后的颜色
///
/// # 算法
///
/// 对每个颜色分量（R、G、B、A）使用公式：
/// result = a + (b - a) * t
///
/// # 示例
///
/// ```ignore
/// let red = Color::from_rgb8(255, 0, 0);
/// let blue = Color::from_rgb8(0, 0, 255);
/// let purple = mix_color(red, blue, 0.5);
/// // purple 大约是 (127, 0, 255)
/// ```
pub(super) fn mix_color(a: Color, b: Color, t: f32) -> Color {
    // 确保 t 在有效范围内
    let t = t.clamp(0.0, 1.0);

    // 对每个分量进行线性插值
    Color::from_rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

#[cfg(test)]
#[path = "utils_tests.rs"]
mod utils_tests;
