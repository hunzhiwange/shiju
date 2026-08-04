//! fullscreen layout slots 计算。
//!
//! 主屏只保留可滚动 transcript 与固定底部输入；header/sidebar 字段暂留在反馈结构
//! 中以保持 controller 边界稳定，但正常布局中恒为空。

use ratatui::layout::Rect;

/// fullscreen 布局的稳定 slots。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct FullscreenLayoutSlots {
    pub(crate) header: Rect,
    pub(crate) scrollable: Rect,
    pub(crate) scrollable_body: Rect,
    pub(crate) scrollbar_track: Option<Rect>,
    pub(crate) project_context: Option<Rect>,
    pub(crate) modified_files: Option<Rect>,
    pub(crate) bottom: Rect,
    pub(crate) bottom_float: Option<Rect>,
    pub(crate) modal: Option<Rect>,
}

impl FullscreenLayoutSlots {
    /// 按当前 scrollable 容器可用高度粗略估算消息视口容量。
    ///
    /// 当前 fullscreen skeleton 仍按“每条消息至少占一行”近似，
    /// 后续虚拟列表和高度缓存会在 Phase 4 继续替换这层估算。
    pub(crate) fn message_viewport_capacity(&self) -> usize {
        self.scrollable_body.height as usize
    }
}

/// fullscreen 布局的静态参数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FullscreenLayoutConfig {
    pub(crate) bottom_height: u16,
    pub(crate) modal_width_pct: u16,
    pub(crate) modal_height_pct: u16,
}

impl Default for FullscreenLayoutConfig {
    fn default() -> Self {
        Self { bottom_height: 8, modal_width_pct: 70, modal_height_pct: 45 }
    }
}

/// 计算 fullscreen slots。
pub(crate) fn compute_fullscreen_layout(
    area: Rect,
    _has_bottom_float: bool,
    has_modal: bool,
    prompt_extra_rows: u16,
) -> FullscreenLayoutSlots {
    let config = FullscreenLayoutConfig::default();
    if area.width == 0 || area.height == 0 {
        return FullscreenLayoutSlots::default();
    }

    let desired_bottom = config.bottom_height.saturating_add(prompt_extra_rows);
    let max_bottom = (area.height / 2).max(1);
    let bottom_height = desired_bottom.min(max_bottom).min(area.height);
    let scroll_height = area.height.saturating_sub(bottom_height);
    let header = Rect::new(area.x, area.y, area.width, 0);
    let scrollable = Rect::new(area.x, area.y, area.width, scroll_height);
    let (scrollable_body, scrollbar_track) = split_scrollbar_area(scrollable);
    let bottom = Rect::new(area.x, area.y.saturating_add(scroll_height), area.width, bottom_height);

    FullscreenLayoutSlots {
        header,
        scrollable,
        scrollable_body,
        scrollbar_track,
        project_context: None,
        modified_files: None,
        bottom,
        bottom_float: None,
        modal: has_modal
            .then(|| centered_rect(area, config.modal_width_pct, config.modal_height_pct, 28, 8)),
    }
}

pub(crate) fn split_scrollbar_area(area: Rect) -> (Rect, Option<Rect>) {
    if area.width < 4 || area.height < 2 {
        return (area, None);
    }
    let body = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    let track = Rect::new(
        area.x.saturating_add(area.width.saturating_sub(1)),
        area.y,
        1,
        area.height,
    );
    (body, Some(track))
}

fn centered_rect(
    area: Rect,
    width_pct: u16,
    height_pct: u16,
    min_width: u16,
    min_height: u16,
) -> Rect {
    if area.width == 0 || area.height == 0 {
        return Rect::default();
    }

    let target_width =
        u16_from_u32_saturating((u32::from(area.width) * u32::from(width_pct)) / 100);
    let target_height =
        u16_from_u32_saturating((u32::from(area.height) * u32::from(height_pct)) / 100);
    let width = target_width.max(min_width).min(area.width.saturating_sub(2).max(1));
    let height = target_height.max(min_height).min(area.height.saturating_sub(2).max(1));
    let x = area.x.saturating_add(area.width.saturating_sub(width) / 2);
    let y = area.y.saturating_add(area.height.saturating_sub(height) / 2);

    Rect::new(x, y, width, height)
}

fn u16_from_u32_saturating(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}
