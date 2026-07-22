//! 颜色选择器属性模块，负责颜色解析、格式转换和拾色控件渲染。

use iced::Color;

#[cfg(test)]
#[path = "conversion_tests.rs"]
mod tests;

/// 处理颜色值的解析、格式化或空间转换。
///
/// 无法识别的颜色返回空结果，避免把错误颜色静默写入设计元素。
pub fn format_rgba_to_hex(r: f32, g: f32, b: f32, a: f32) -> String {
    let r_u8 = (r * 255.0).round() as u8;
    let g_u8 = (g * 255.0).round() as u8;
    let b_u8 = (b * 255.0).round() as u8;
    let a_u8 = (a * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}{:02X}", r_u8, g_u8, b_u8, a_u8)
}

/// 处理颜色值的解析、格式化或空间转换。
///
/// 无法识别的颜色返回空结果，避免把错误颜色静默写入设计元素。
pub fn format_rgba_to_css(r: f32, g: f32, b: f32, a: f32) -> String {
    let r_u8 = (r * 255.0).round() as u8;
    let g_u8 = (g * 255.0).round() as u8;
    let b_u8 = (b * 255.0).round() as u8;
    format!("rgba({}, {}, {}, {:.2})", r_u8, g_u8, b_u8, a)
}

/// 解析外部输入并转换为内部设计模型。
///
/// 不支持或格式不完整的输入通过 `Option`/`Result` 显式表达。
pub fn parse_css_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if s.starts_with("rgba(") && s.ends_with(')') {
        let content = &s[5..s.len() - 1];
        let parts: Vec<&str> = content.split(',').map(|p| p.trim()).collect();
        if parts.len() == 4 {
            let r = parts[0].parse::<u8>().ok()?;
            let g = parts[1].parse::<u8>().ok()?;
            let b = parts[2].parse::<u8>().ok()?;
            let a = parts[3].parse::<f32>().ok()?;
            Some(Color::from_rgba8(r, g, b, a))
        } else {
            None
        }
    } else if s.starts_with("rgb(") && s.ends_with(')') {
        let content = &s[4..s.len() - 1];
        let parts: Vec<&str> = content.split(',').map(|p| p.trim()).collect();
        if parts.len() == 3 {
            let r = parts[0].parse::<u8>().ok()?;
            let g = parts[1].parse::<u8>().ok()?;
            let b = parts[2].parse::<u8>().ok()?;
            Some(Color::from_rgb8(r, g, b))
        } else {
            None
        }
    } else {
        None
    }
}

/// 解析外部输入并转换为内部设计模型。
///
/// 不支持或格式不完整的输入通过 `Option`/`Result` 显式表达。
pub fn parse_color(s: &str) -> Option<Color> {
    if s.len() >= 7 && s.starts_with('#') {
        let r = u8::from_str_radix(&s[1..3], 16).ok()?;
        let g = u8::from_str_radix(&s[3..5], 16).ok()?;
        let b = u8::from_str_radix(&s[5..7], 16).ok()?;
        let a = if s.len() == 9 { u8::from_str_radix(&s[7..9], 16).ok()? } else { 255 };
        Some(Color::from_rgba8(r, g, b, a as f32 / 255.0))
    } else {
        None
    }
}

/// 处理颜色值的解析、格式化或空间转换。
///
/// 无法识别的颜色返回空结果，避免把错误颜色静默写入设计元素。
pub fn rgba_to_hsla(color: Color) -> (f32, f32, f32, f32) {
    let r = color.r;
    let g = color.g;
    let b = color.b;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    let (h, s) = if max == min {
        (0.0, 0.0)
    } else {
        let d = max - min;
        let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
        let h = if max == r {
            (g - b) / d + (if g < b { 6.0 } else { 0.0 })
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        (h * 60.0, s)
    };

    let h = if h < 0.0 { h + 360.0 } else { h };

    (h, s, l, color.a)
}

/// 处理颜色值的解析、格式化或空间转换。
///
/// 无法识别的颜色返回空结果，避免把错误颜色静默写入设计元素。
pub fn hsla_to_rgba(h: f32, s: f32, l: f32, a: f32) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Color::from_rgba(r + m, g + m, b + m, a)
}

/// 执行 format_percent 对应的设计辅助逻辑。
///
/// 返回值直接交给调用方继续渲染、导入或属性更新。
pub fn format_percent(value: f32) -> String {
    let mut s = format!("{:.2}", value);
    while s.contains('.') && s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}
