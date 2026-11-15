//! 颜色选择器图像生成模块
//!
//! 本模块提供了颜色选择器所需的各种渐变图像的生成功能。
//! 主要生成三种类型的图像：
//! - 饱和度-明度（S-V）选择器图像：2D 方形渐变，用于选择颜色的饱和度和明度
//! - 色相（Hue）选择器图像：1D 横向渐变，用于选择基础色相
//! - 透明度（Alpha）选择器图像：基于选定颜色的透明度渐变
//!
//! 所有图像均使用 HSV 颜色空间计算，并转换为 RGBA 像素数据。

use iced::Color;
use iced::widget::image;

use super::hsv::Hsv;

/// 生成饱和度-明度选择器的图像句柄
///
/// 创建一个 256x256 像素的 2D 渐变图像，其中：
/// - 横轴（X 轴）表示饱和度（S），从 0（灰色）到 1（全彩）
/// - 纵轴（Y 轴）表示明度（V），从 0（黑色）到 1（最亮）
/// - 色相（H）由参数 `h` 固定
///
/// # 参数
///
/// * `h` - 色相值，范围 0.0 到 360.0，表示色轮上的角度
///
/// # 返回值
///
/// 返回一个 `image::Handle`，可用于在 iced UI 中显示该渐变图像
///
/// # 示例
///
/// ```rust,ignore
/// // 创建色相为 180°（青色）的 S-V 选择器图像
/// let sv_image = sv_image_handle(180.0);
/// ```
///
/// # 实现细节
///
/// - 使用 HSV 到 RGB 的转换计算每个像素的颜色
/// - 图像使用 RGBA 格式，每个像素 4 个字节
/// - Alpha 通道始终为 255（完全不透明）
/// - 使用 `saturating_sub(1)` 防止除零错误
pub fn sv_image_handle(h: f32) -> image::Handle {
    // 定义图像宽度（饱和度轴）
    let w = 256u32;
    // 定义图像高度（明度轴）
    let hpx = 256u32;

    // 预分配像素数据缓冲区：宽度 * 高度 * 4（RGBA）
    let mut pixels = vec![0u8; (w * hpx * 4) as usize];

    // 逐行遍历像素
    for y in 0..hpx {
        // 计算明度值（V）：Y 轴从上到下，明度从 1.0（顶部）到 0.0（底部）
        // 使用 saturating_sub 防止除零，max(1) 确保分母不为零
        let v = 1.0 - (y as f32 / (hpx.saturating_sub(1)).max(1) as f32);

        // 逐列遍历像素
        for x in 0..w {
            // 计算饱和度值（S）：X 轴从左到右，饱和度从 0.0（左侧）到 1.0（右侧）
            let s = x as f32 / (w.saturating_sub(1)).max(1) as f32;

            // 使用固定的色相 H 和计算得到的 S、V 创建 HSV 颜色，并转换为 RGB
            let c = Hsv { h, s, v }.to_color();

            // 计算当前像素在缓冲区中的起始索引（每像素 4 字节）
            let idx = ((y * w + x) * 4) as usize;

            // 写入 RGBA 值到像素缓冲区
            pixels[idx] = (c.r * 255.0).round() as u8; // 红色通道
            pixels[idx + 1] = (c.g * 255.0).round() as u8; // 绿色通道
            pixels[idx + 2] = (c.b * 255.0).round() as u8; // 蓝色通道
            pixels[idx + 3] = 255; // Alpha 通道（完全不透明）
        }
    }

    // 从 RGBA 像素数据创建图像句柄
    image::Handle::from_rgba(w, hpx, pixels)
}

/// 生成色相选择器的图像句柄
///
/// 创建一个 256x16 像素的横向渐变条，显示完整的色相光谱（0° 到 360°）。
/// 饱和度和明度固定为 1.0，仅展示色相变化。
///
/// # 参数
///
/// 无参数
///
/// # 返回值
///
/// 返回一个 `image::Handle`，可用于在 iced UI 中显示色相渐变条
///
/// # 示例
///
/// ```rust,ignore
/// // 创建色相选择器图像
/// let hue_image = hue_image_handle();
/// ```
///
/// # 实现细节
///
/// - 色相从左到右线性分布：0°（红）-> 120°（绿）-> 240°（蓝）-> 360°（红）
/// - S 和 V 固定为 1.0，显示最饱和、最明亮的颜色
/// - 图像高度为 16 像素，适合作为横向选择条
pub fn hue_image_handle() -> image::Handle {
    // 定义图像宽度（色相轴）
    let w = 256u32;
    // 定义图像高度（显示高度）
    let hpx = 16u32;

    // 预分配像素数据缓冲区
    let mut pixels = vec![0u8; (w * hpx * 4) as usize];

    // 逐行遍历像素（所有行的颜色相同，仅为了视觉效果增加高度）
    for y in 0..hpx {
        // 逐列遍历像素
        for x in 0..w {
            // 计算色相值：X 轴从左到右，色相从 0° 到 360°
            let hue = (x as f32 / (w.saturating_sub(1)).max(1) as f32) * 360.0;

            // 创建 HSV 颜色：色相渐变，饱和度和明度固定为 1.0
            let c = Hsv { h: hue, s: 1.0, v: 1.0 }.to_color();

            // 计算当前像素在缓冲区中的起始索引
            let idx = ((y * w + x) * 4) as usize;

            // 写入 RGBA 值到像素缓冲区
            pixels[idx] = (c.r * 255.0).round() as u8; // 红色通道
            pixels[idx + 1] = (c.g * 255.0).round() as u8; // 绿色通道
            pixels[idx + 2] = (c.b * 255.0).round() as u8; // 蓝色通道
            pixels[idx + 3] = 255; // Alpha 通道（完全不透明）
        }
    }

    // 从 RGBA 像素数据创建图像句柄
    image::Handle::from_rgba(w, hpx, pixels)
}

/// 生成透明度选择器的图像句柄
///
/// 创建一个 256x16 像素的横向渐变条，基于选定的 RGB 颜色展示透明度变化。
/// 从完全透明（alpha = 0.0）到完全不透明（alpha = 1.0）。
///
/// # 参数
///
/// * `rgb` - 基础 RGB 颜色，透明度将应用于此颜色
///
/// # 返回值
///
/// 返回一个 `image::Handle`，可用于在 iced UI 中显示透明度渐变条
///
/// # 示例
///
/// ```rust,ignore
/// // 为红色创建透明度选择器
/// let alpha_image = alpha_image_handle(Color::from_rgb(1.0, 0.0, 0.0));
/// ```
///
/// # 实现细节
///
/// - RGB 分量保持不变，仅修改 alpha 通道
/// - Alpha 从左到右线性增加：0.0（完全透明）-> 1.0（完全不透明）
/// - 图像高度为 16 像素，适合作为横向选择条
pub fn alpha_image_handle(rgb: Color) -> image::Handle {
    // 定义图像宽度（透明度轴）
    let w = 256u32;
    // 定义图像高度（显示高度）
    let hpx = 16u32;

    // 预分配像素数据缓冲区
    let mut pixels = vec![0u8; (w * hpx * 4) as usize];

    // 逐行遍历像素（所有行的颜色相同，仅为了视觉效果增加高度）
    for y in 0..hpx {
        // 逐列遍历像素
        for x in 0..w {
            // 计算透明度值：X 轴从左到右，alpha 从 0.0 到 1.0
            let a = x as f32 / (w.saturating_sub(1)).max(1) as f32;

            // 创建颜色：保持 RGB 不变，仅修改 alpha
            let c = Color { a, ..rgb };

            // 计算当前像素在缓冲区中的起始索引
            let idx = ((y * w + x) * 4) as usize;

            // 写入 RGBA 值到像素缓冲区
            pixels[idx] = (c.r * 255.0).round() as u8; // 红色通道
            pixels[idx + 1] = (c.g * 255.0).round() as u8; // 绿色通道
            pixels[idx + 2] = (c.b * 255.0).round() as u8; // 蓝色通道
            pixels[idx + 3] = (c.a * 255.0).round() as u8; // Alpha 通道（渐变）
        }
    }

    // 从 RGBA 像素数据创建图像句柄
    image::Handle::from_rgba(w, hpx, pixels)
}

#[cfg(test)]
#[path = "images_tests.rs"]
mod images_tests;
