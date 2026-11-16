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
        Self {
            preference: LayoutPreference::Auto,
            logical_width: None,
            drawer_open: false,
            settings_detail: false,
        }
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

use desktop_ui_controls::settings_navigation::SystemTab;
pub enum Message {
    Selected(LayoutPreference),
    DrawerToggled,
    Back,
    Chat,
    Settings,
    SettingsTab(SystemTab),
    ProjectSelected(String),
    SessionSelected(String, String),
    NewSession(String),
}
pub enum Effect {
    OpenSettingsTab(SystemTab),
    OpenProject(String),
    OpenSession(String, String),
    CreateSession(String),
}
pub struct Context<'a> {
    pub mobile: &'a mut MobileLayoutState,
    pub panels: &'a mut crate::panel::settings::State,
    pub tabs: &'a mut crate::panel::tabs::State,
    pub notifications_expanded: &'a mut bool,
}
pub fn update(app: Context<'_>, blocked: bool, message: Message) -> Option<Effect> {
    match message {
        Message::Selected(preference) => app.mobile.select(preference),
        Message::DrawerToggled => app.mobile.drawer_open = !app.mobile.drawer_open,
        Message::Back => {
            if blocked {
                return None;
            }
            if *app.notifications_expanded {
                *app.notifications_expanded = false;
            } else if app.mobile.drawer_open {
                app.mobile.drawer_open = false;
            } else if app.panels.show_system_settings && app.mobile.settings_detail {
                app.mobile.settings_detail = false;
            } else if app.panels.show_system_settings {
                app.panels.show_system_settings = false;
            } else {
                app.mobile.drawer_open = true;
            }
        }
        Message::Chat => {
            app.panels.show_system_settings = false;
            app.mobile.drawer_open = false;
            app.tabs.show_project();
        }
        Message::Settings => {
            app.panels.show_system_settings = true;
            app.mobile.settings_detail = false;
            app.mobile.drawer_open = false;
        }
        Message::SettingsTab(tab) => {
            app.mobile.settings_detail = true;
            return Some(Effect::OpenSettingsTab(tab));
        }
        Message::ProjectSelected(path) => return Some(Effect::OpenProject(path)),
        Message::SessionSelected(path, id) => {
            app.mobile.drawer_open = false;
            app.panels.show_system_settings = false;
            return Some(Effect::OpenSession(path, id));
        }
        Message::NewSession(path) => {
            app.mobile.drawer_open = false;
            app.panels.show_system_settings = false;
            return Some(Effect::CreateSession(path));
        }
    }
    None
}
