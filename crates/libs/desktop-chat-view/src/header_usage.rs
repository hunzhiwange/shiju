use std::f32::consts::{FRAC_PI_2, TAU};
use iced::widget::canvas::path::Arc;
use iced::widget::canvas::{Frame, Geometry, Path as CanvasPath, Program, Stroke};
use iced::{Color, Radians, Theme};
#[derive(Debug, Clone, Copy)]
pub struct UsageRing {
    /// 使用率百分比（0.0 - 100.0）
    pub percent: f32,
}

impl<Message> Program<Message> for UsageRing {
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
        let percent = percent_val / 100.0;
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

        // 沿容量圆环从正上方顺时针绘制已用比例。
        if percent > 0.0 {
            let progress_path = if percent == 1.0 {
                bg_path
            } else {
                CanvasPath::new(|path| {
                    path.arc(Arc {
                        center,
                        radius,
                        start_angle: Radians(-FRAC_PI_2),
                        end_angle: Radians(-FRAC_PI_2 + TAU * percent),
                    });
                })
            };

            frame.stroke(
                &progress_path,
                Stroke { width: stroke_width, style: progress_color.into(), ..Stroke::default() },
            );
        }

        vec![frame.into_geometry()]
    }
}
