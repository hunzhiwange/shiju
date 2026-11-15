//! 颜色处理工具模块
//!
//! 本模块提供颜色格式验证与转换的实用函数，主要用于处理十六进制颜色代码
//! 并将其转换为终端可识别的 ANSI 转义序列。
//!
//! # 主要功能
//!
//! - 验证十六进制颜色格式的有效性
//! - 将十六进制颜色转换为 RGB 值
//! - 生成带粗体样式的 ANSI 24位真彩色转义序列

/// 验证十六进制颜色字符串是否为有效格式
///
/// 检查给定的颜色字符串是否符合标准十六进制颜色格式（#RRGGBB）。
/// 有效格式必须满足以下条件：
/// - 总长度为 7 个字符
/// - 以 '#' 字符开头
/// - 后续 6 个字符均为有效的十六进制数字（0-9, a-f, A-F）
///
/// # 参数
///
/// * `hex` - 可选的十六进制颜色字符串引用，格式应为 `#RRGGBB`
///
/// # 返回值
///
/// 如果输入为 `None` 或格式无效，返回 `false`；否则返回 `true`
///
/// # 示例
///
/// ```
/// use agent_text::color::is_valid_hex;
///
/// assert!(is_valid_hex(Some("#FF5733")));
/// assert!(is_valid_hex(Some("#ffffff")));
/// assert!(is_valid_hex(Some("#ABCDEF")));
/// assert!(!is_valid_hex(Some("FF5733")));   // 缺少 '#' 前缀
/// assert!(!is_valid_hex(Some("#FFF")));     // 长度不足
/// assert!(!is_valid_hex(Some("#GGGGGG")));  // 非法十六进制字符
/// assert!(!is_valid_hex(None));             // None 输入
/// ```
pub fn is_valid_hex(hex: Option<&str>) -> bool {
    // 如果 hex 为 None，直接返回 false
    let Some(hex) = hex else { return false };

    let bytes = hex.as_bytes();

    // 验证长度必须为 7（# + 6位十六进制字符）且首字符为 '#'
    if bytes.len() != 7 || bytes[0] != b'#' {
        return false;
    }

    // 检查后续 6 个字符是否均为有效的十六进制数字
    bytes[1..].iter().all(|b| b.is_ascii_hexdigit())
}

/// 将十六进制颜色字符串转换为 RGB 值
///
/// 解析标准格式的十六进制颜色字符串（#RRGGBB），提取其中的红、绿、蓝分量。
///
/// # 参数
///
/// * `hex` - 十六进制颜色字符串引用，格式必须为 `#RRGGBB`
///
/// # 返回值
///
/// - 成功时返回 `Some((r, g, b))`，其中 r、g、b 为 0-255 范围的 u8 值
/// - 如果输入格式无效，返回 `None`
///
/// # 示例
///
/// ```
/// use agent_text::color::hex_to_rgb;
///
/// assert_eq!(hex_to_rgb("#FF0000"), Some((255, 0, 0)));    // 纯红
/// assert_eq!(hex_to_rgb("#00FF00"), Some((0, 255, 0)));    // 纯绿
/// assert_eq!(hex_to_rgb("#0000FF"), Some((0, 0, 255)));    // 纯蓝
/// assert_eq!(hex_to_rgb("#FFFFFF"), Some((255, 255, 255))); // 白色
/// assert_eq!(hex_to_rgb("#000000"), Some((0, 0, 0)));      // 黑色
/// assert_eq!(hex_to_rgb("invalid"), None);                 // 无效格式
/// ```
pub fn hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    // 首先验证十六进制格式的有效性
    if !is_valid_hex(Some(hex)) {
        return None;
    }

    // 分别解析 R、G、B 三个分量（每个分量占 2 个十六进制字符）
    let r = u8::from_str_radix(&hex[1..3], 16).ok()?;
    let g = u8::from_str_radix(&hex[3..5], 16).ok()?;
    let b = u8::from_str_radix(&hex[5..7], 16).ok()?;

    Some((r, g, b))
}

/// 将十六进制颜色转换为带粗体样式的 ANSI 转义序列
///
/// 生成支持 24 位真彩色（True Color）的 ANSI 转义序列，同时启用粗体样式。
/// 适用于支持真彩色的现代终端模拟器。
///
/// # ANSI 转义序列说明
///
/// - `\x1b[38;2;R;G;Bm` - 设置 24 位真彩色前景色
/// - `\x1b[1m` - 启用粗体样式
///
/// # 参数
///
/// * `hex` - 可选的十六进制颜色字符串引用，格式应为 `#RRGGBB`
///
/// # 返回值
///
/// - 成功时返回 `Some(String)`，包含完整的 ANSI 转义序列
/// - 如果输入为 `None` 或格式无效，返回 `None`
///
/// # 示例
///
/// ```
/// use agent_text::color::hex_to_ansi_bold;
///
/// let seq = hex_to_ansi_bold(Some("#FF5733"));
/// assert!(seq.is_some());
/// assert!(seq.unwrap().contains("\x1b[38;2;255;87;51m"));
/// assert!(seq.unwrap().contains("\x1b[1m"));
///
/// assert!(hex_to_ansi_bold(None).is_none());
/// assert!(hex_to_ansi_bold(Some("invalid")).is_none());
/// ```
///
/// # 注意事项
///
/// 生成的转义序列需要终端支持 24 位真彩色。在不支持的终端中，
/// 颜色可能无法正确显示或显示为默认颜色。
pub fn hex_to_ansi_bold(hex: Option<&str>) -> Option<String> {
    // 如果 hex 为 None，直接返回 None
    let hex = hex?;

    // 将十六进制颜色转换为 RGB 值
    let (r, g, b) = hex_to_rgb(hex)?;

    // 生成 ANSI 转义序列：设置真彩色前景色 + 启用粗体
    Some(format!("\x1b[38;2;{};{};{}m\x1b[1m", r, g, b))
}
