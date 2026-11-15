/// 公开的 FocusArea 枚举，描述该模块支持的一组离散状态或事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusArea {
    None,
    Preview,
    Terminal,
}

/// Todo 面板在会话界面中的承载位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoPanelPlacement {
    InputBottom,
    ChatTopRight,
}

/// 编辑菜单剪切/复制/粘贴的上下文路由种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardShortcutKind {
    Copy,
    Cut,
    Paste,
}

impl TodoPanelPlacement {
    pub fn label(self) -> &'static str {
        match self {
            TodoPanelPlacement::InputBottom => "输入底部",
            TodoPanelPlacement::ChatTopRight => "右上角",
        }
    }
}

/// 公开的 SettingsTab 枚举，描述该模块支持的一组离散状态或事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Settings,
    SettingsJson,
    Files,
    Projects,
    Sessions,
}

impl SettingsTab {
    /// 公开的 all 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    #[allow(dead_code)]
    pub(crate) fn all() -> [SettingsTab; 3] {
        [SettingsTab::Settings, SettingsTab::Files, SettingsTab::Projects]
    }
}

impl std::fmt::Display for SettingsTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingsTab::Settings => write!(f, "设置"),
            SettingsTab::SettingsJson => write!(f, "JSON配置"),
            SettingsTab::Files => write!(f, "文件管理器"),
            SettingsTab::Projects => write!(f, "项目管理器"),
            SettingsTab::Sessions => write!(f, "历史会话"),
        }
    }
}

/// 公开的 Screen 枚举，描述该模块支持的一组离散状态或事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Project,
    Preview,
    Apps,
    Usage,
    Knowledge,
    WorkflowTool,
    WorkflowManager,
    TaskBoard,
    #[cfg(not(target_arch = "wasm32"))]
    ShijuApps,
    #[cfg(not(target_arch = "wasm32"))]
    ShijuSkills,
    #[cfg(not(target_arch = "wasm32"))]
    ExpertHub,
}


#[cfg(test)] #[path="ui_types_tests.rs"] mod tests;
