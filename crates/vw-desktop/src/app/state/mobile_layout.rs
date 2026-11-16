//! 会话内的呈现策略；尺寸使用 Iced 已转换的逻辑像素，无需再次应用缩放。

const MOBILE_BREAKPOINT: f32 = 768.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LayoutPreference {
    #[default]
    Auto,
    Desktop,
    Mobile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Desktop,
    Mobile,
}

/// 仅保存布局选择与最后有效宽度，不持有业务模式或桌面分栏状态。
#[derive(Debug, Clone, Copy)]
pub struct MobileLayoutState {
    preference: LayoutPreference,
    logical_width: Option<f32>,
    pub drawer_open: bool,
    pub settings_detail: bool,
}

impl Default for MobileLayoutState {
    fn default() -> Self {
        Self { preference: LayoutPreference::Auto, logical_width: None, drawer_open: false, settings_detail: false }
    }
}

impl MobileLayoutState {
    pub fn new(width: f32, height: f32) -> Self {
        let mut state = Self::default();
        state.update_size(width, height);
        state
    }

    pub fn preference(&self) -> LayoutPreference {
        self.preference
    }

    pub fn select(&mut self, preference: LayoutPreference) {
        self.preference = preference;
    }

    /// 无有效初始尺寸时暂用桌面布局；无效事件保留最后有效宽度。
    pub fn mode(&self) -> LayoutMode {
        match self.preference {
            LayoutPreference::Desktop => LayoutMode::Desktop,
            LayoutPreference::Mobile => LayoutMode::Mobile,
            LayoutPreference::Auto => match self.logical_width {
                Some(width) if width < MOBILE_BREAKPOINT => LayoutMode::Mobile,
                _ => LayoutMode::Desktop,
            },
        }
    }

    /// 高度只用于拒绝无效尺寸，软键盘引发的有效高度变化不影响布局。
    pub fn update_size(&mut self, width: f32, height: f32) {
        if width.is_finite() && width > 0.0 && height.is_finite() && height > 0.0 {
            self.logical_width = Some(width);
        }
    }
}

#[cfg(test)]
#[path = "mobile_layout_tests.rs"]
mod mobile_layout_tests;
