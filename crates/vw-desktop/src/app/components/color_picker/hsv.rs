//! HSV 颜色空间模块
//!
//! 本模块提供 HSV（色相-饱和度-明度）颜色空间的表示与转换功能。
//! HSV 颜色模型在颜色选择器中广泛应用，因为它更符合人类对颜色的直觉感知。
//!
//! # 主要功能
//!
//! - HSV 与 RGB 颜色空间之间的双向转换
//! - 支持与 iced 库的 Color 类型互操作
//!
//! # 颜色空间说明
//!
//! - **H（Hue/色相）**：0-360 度，表示颜色类型（红、橙、黄、绿、青、蓝、紫）
//! - **S（Saturation/饱和度）**：0.0-1.0，表示颜色的纯度
//! - **V（Value/明度）**：0.0-1.0，表示颜色的明亮程度

use iced::Color;

/// HSV 颜色表示结构体
///
/// 使用 HSV（色相-饱和度-明度）模型表示颜色。
/// 相比 RGB 模型，HSV 更适合颜色选择和编辑操作。
///
/// # 字段范围
///
/// - `h`: 色相，范围 [0.0, 360.0) 度
/// - `s`: 饱和度，范围 [0.0, 1.0]
/// - `v`: 明度，范围 [0.0, 1.0]
///
/// # 示例
///
/// ```ignore
/// use crate::app::components::color_picker::hsv::Hsv;
/// use iced::Color;
///
/// // 从 RGB 转换为 HSV
/// let red = Color::from_rgb(1.0, 0.0, 0.0);
/// let hsv = Hsv::from_color(red);
/// assert_eq!(hsv.h, 0.0);  // 红色色相为 0 度
///
/// // 从 HSV 转换回 RGB
/// let color = hsv.to_color();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Hsv {
    /// 色相（Hue），单位为度
    ///
    /// 表示颜色在色轮上的位置：
    /// - 0° / 360°：红色
    /// - 60°：黄色
    /// - 120°：绿色
    /// - 180°：青色
    /// - 240°：蓝色
    /// - 300°：品红/洋红
    pub h: f32,

    /// 饱和度（Saturation）
    ///
    /// 表示颜色的纯度：
    /// - 0.0：完全灰度（无色彩）
    /// - 1.0：完全饱和（纯色）
    pub s: f32,

    /// 明度（Value/Brightness）
    ///
    /// 表示颜色的明亮程度：
    /// - 0.0：完全黑暗（黑色）
    /// - 1.0：最大亮度
    pub v: f32,
}

impl Hsv {
    /// 从 RGB 颜色转换为 HSV 颜色
    ///
    /// 将 iced 的 `Color`（RGBA 格式）转换为 HSV 表示。
    /// 转换算法遵循标准 RGB 到 HSV 的数学公式。
    ///
    /// # 参数
    ///
    /// - `color`: iced 库的 Color 实例，包含 r、g、b、a 分量
    ///
    /// # 返回值
    ///
    /// 返回对应的 `Hsv` 实例，其中：
    /// - `h` 范围为 [0.0, 360.0)
    /// - `s` 范围为 [0.0, 1.0]
    /// - `v` 范围为 [0.0, 1.0]
    ///
    /// # 算法说明
    ///
    /// 1. 计算 RGB 中的最大值（max）、最小值（min）和差值（delta）
    /// 2. 明度 V 直接取 max 值
    /// 3. 饱和度 S = delta / max（当 max > 0 时）
    /// 4. 色相 H 根据 max 出现在哪个通道来计算
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let red = Color::from_rgb(1.0, 0.0, 0.0);
    /// let hsv = Hsv::from_color(red);
    /// // hsv.h ≈ 0.0, hsv.s = 1.0, hsv.v = 1.0
    /// ```
    pub fn from_color(color: Color) -> Self {
        let r = color.r;
        let g = color.g;
        let b = color.b;

        // 计算最大值、最小值和差值
        // 这些值用于确定色相和饱和度
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        // 色相计算：根据最大值所在的通道确定基础色相
        // 不同通道对应色轮上不同的 60 度扇区
        let h = if delta == 0.0 {
            // 当 R=G=B 时，颜色为灰度，色相无意义，设为 0
            0.0
        } else if max == r {
            // 红色通道最大：色相在 0-60 度（红到黄）或 300-360 度（红到洋红）
            60.0 * ((g - b) / delta % 6.0)
        } else if max == g {
            // 绿色通道最大：色相在 60-180 度（黄到青）
            60.0 * ((b - r) / delta + 2.0)
        } else {
            // 蓝色通道最大：色相在 180-300 度（青到洋红）
            60.0 * ((r - g) / delta + 4.0)
        };

        // 确保色相在 [0, 360) 范围内
        // 当计算结果为负数时（如洋红色），加上 360 度
        let h = if h < 0.0 { h + 360.0 } else { h };

        // 饱和度：颜色的纯度
        // 当 max=0 时（黑色），饱和度无意义，设为 0
        let s = if max == 0.0 { 0.0 } else { delta / max };

        // 明度：直接取 RGB 最大值
        let v = max;

        Self { h, s, v }
    }

    /// 从 HSV 颜色转换为 RGB 颜色
    ///
    /// 将 HSV 表示转换回 iced 的 `Color`（RGB 格式）。
    /// 转换算法遵循标准 HSV 到 RGB 的数学公式。
    ///
    /// # 返回值
    ///
    /// 返回对应的 iced `Color` 实例，alpha 通道默认为 1.0（完全不透明）
    ///
    /// # 算法说明
    ///
    /// HSV 转 RGB 的核心思想是将色相分为 6 个 60 度的扇区，
    /// 每个扇区内 R、G、B 的变化规律不同。
    ///
    /// 1. 计算色度 C = V × S（饱和度越高，颜色越纯）
    /// 2. 计算中间值 X = C × (1 - |H/60 mod 2 - 1|)
    /// 3. 计算明度偏移 M = V - C
    /// 4. 根据色相所在扇区确定 (R', G', B') 的值
    /// 5. 最终 RGB = (R' + M, G' + M, B' + M)
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let hsv = Hsv { h: 0.0, s: 1.0, v: 1.0 };
    /// let color = hsv.to_color();
    /// // color.r = 1.0, color.g = 0.0, color.b = 0.0 (纯红色)
    /// ```
    pub fn to_color(self) -> Color {
        // 色度（Chroma）：表示颜色的"彩色"部分
        // 当饱和度 S=0 时，C=0，颜色为灰度
        let c = self.v * self.s;

        // 中间值 X：用于计算 R'、G'、B' 中的一个分量
        // 根据 H 在 60 度区间内的位置，X 在 0 到 C 之间变化
        let x = c * (1.0 - ((self.h / 60.0) % 2.0 - 1.0).abs());

        // 明度偏移量：将颜色从色度空间映射到亮度空间
        // 加上 M 后确保最小值为 V-C，保持整体亮度
        let m = self.v - c;

        // 根据色相所在的 60 度扇区确定 R'、G'、B' 的值
        // 每个扇区内，一个通道为 C，一个为 X，一个为 0
        let (r, g, b) = if self.h < 60.0 {
            // 0-60 度：红色扇区，R'=C, G'=X, B'=0
            (c, x, 0.0)
        } else if self.h < 120.0 {
            // 60-120 度：黄色扇区，R'=X, G'=C, B'=0
            (x, c, 0.0)
        } else if self.h < 180.0 {
            // 120-180 度：绿色扇区，R'=0, G'=C, B'=X
            (0.0, c, x)
        } else if self.h < 240.0 {
            // 180-240 度：青色扇区，R'=0, G'=X, B'=C
            (0.0, x, c)
        } else if self.h < 300.0 {
            // 240-300 度：蓝色扇区，R'=X, G'=0, B'=C
            (x, 0.0, c)
        } else {
            // 300-360 度：洋红扇区，R'=C, G'=0, B'=X
            (c, 0.0, x)
        };

        // 最终 RGB 值 = R'/G'/B' + 明度偏移
        // 这样可以保持颜色的整体明度
        Color::from_rgb(r + m, g + m, b + m)
    }
}

#[cfg(test)]
#[path = "hsv_tests.rs"]
mod tests;
