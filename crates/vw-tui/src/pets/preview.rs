//! Shared preview-state model for the `/pets` side pane.
//!
//! The preview pane is intentionally small and stateful: the selection popup
//! renders it synchronously, while async preview loading updates this state
//! from outside the widget tree. Keeping the state in a mutex-backed object lets
//! the picker remember the last preview area for out-of-band image rendering
//! without requiring the rest of the popup machinery to know about pet images.
//!
//! 预览面板保持一个小型共享状态：弹窗同步渲染它，异步加载任务从控件树外更新它。
//! 互斥锁让选择器记录最近一次预览区域并支持带外图像绘制，而无需让通用弹窗逻辑理解宠物协议。

use std::sync::Arc;
use std::sync::Mutex;

use ratatui::buffer::Buffer;
use ratatui::layout::HorizontalAlignment;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

use crate::render::renderable::Renderable;

#[derive(Debug, Clone, Default)]
pub(crate) struct PetPickerPreviewState {
    inner: Arc<Mutex<PetPickerPreviewInner>>,
}

impl PetPickerPreviewState {
    /// Return a renderable wrapper for the picker side pane.
    ///
    /// The wrapper is cheap to clone and intentionally shares interior state
    /// with the controller so selection-change callbacks can update the visible
    /// loading/error/ready state without rebuilding the popup.
    /// 返回共享内部状态的轻量可渲染包装器，使回调无需重建弹窗即可更新加载、错误或就绪状态。
    pub(crate) fn renderable(&self) -> PetPickerPreviewRenderable {
        PetPickerPreviewRenderable {
            inner: Arc::clone(&self.inner),
        }
    }

    pub(crate) fn set_loading(&self) {
        self.update(|inner| {
            inner.status = PetPickerPreviewStatus::Loading;
        });
    }

    pub(crate) fn set_disabled(&self) {
        self.update(|inner| {
            inner.status = PetPickerPreviewStatus::Disabled;
        });
    }

    pub(crate) fn set_ready(&self) {
        self.update(|inner| {
            inner.status = PetPickerPreviewStatus::Ready;
        });
    }

    pub(crate) fn set_error(&self, message: String) {
        self.update(|inner| {
            inner.status = PetPickerPreviewStatus::Error { message };
        });
    }

    pub(crate) fn clear(&self) {
        self.update(|inner| {
            inner.status = PetPickerPreviewStatus::Hidden;
            inner.last_area = None;
        });
    }

    pub(crate) fn area(&self) -> Option<Rect> {
        self.inner.lock().ok().and_then(|inner| inner.last_area)
    }

    /// 在持锁期间执行一次短小状态更新；锁中毒时保留最后可见状态。
    fn update(&self, f: impl FnOnce(&mut PetPickerPreviewInner)) {
        if let Ok(mut inner) = self.inner.lock() {
            f(&mut inner);
        }
    }
}

/// 预览的展示状态以及最近一次实际渲染区域。
#[derive(Debug, Default)]
struct PetPickerPreviewInner {
    status: PetPickerPreviewStatus,
    last_area: Option<Rect>,
}

/// 预览加载生命周期；就绪图像由带外协议绘制，文本面板只显示状态。
#[derive(Debug, Default)]
enum PetPickerPreviewStatus {
    #[default]
    Hidden,
    Loading,
    Disabled,
    Ready,
    Error {
        message: String,
    },
}

pub(crate) struct PetPickerPreviewRenderable {
    inner: Arc<Mutex<PetPickerPreviewInner>>,
}

impl Renderable for PetPickerPreviewRenderable {
    // 在锁内只复制展示所需数据，随后释放锁再写入 ratatui 缓冲区。
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let (title, body) = {
            let Ok(mut inner) = self.inner.lock() else {
                return;
            };
            inner.last_area = Some(area);
            match &inner.status {
                PetPickerPreviewStatus::Hidden => return,
                PetPickerPreviewStatus::Loading => ("Loading preview...", None),
                PetPickerPreviewStatus::Disabled => (
                    "Terminal pets disabled",
                    Some("No pet will be shown.".to_string()),
                ),
                PetPickerPreviewStatus::Ready => return,
                PetPickerPreviewStatus::Error { message } => {
                    ("Preview unavailable", Some(message.clone()))
                }
            }
        };

        let text_height = if body.is_some() { 2 } else { 1 };
        let text_area = centered_text_area(area, text_height);
        let mut lines = vec![Line::from(title.bold())];
        if let Some(body) = body {
            lines.push(Line::from(body.dim()));
        }
        Paragraph::new(lines)
            .alignment(HorizontalAlignment::Center)
            .render(text_area, buf);
    }

    // 预览面板使用固定高度，图像协议按记录的区域另行定位。
    fn desired_height(&self, _width: u16) -> u16 {
        4
    }
}

/// 在给定矩形内垂直居中指定高度，并以区域边界为上限。
fn centered_text_area(area: Rect, height: u16) -> Rect {
    let height = height.min(area.height);
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(area.x, y, area.width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // 验证居中计算同时保留原始横向范围并正确移动纵坐标。
    fn centered_text_area_centers_vertically() {
        assert_eq!(
            centered_text_area(
                Rect::new(
                    /*x*/ 5, /*y*/ 10, /*width*/ 20, /*height*/ 8
                ),
                /*height*/ 2
            ),
            Rect::new(
                /*x*/ 5, /*y*/ 13, /*width*/ 20, /*height*/ 2
            )
        );
    }
}
