#[derive(Debug, Clone)]
pub struct HeartbeatSettingsState {
    /// 是否启用心跳
    pub enabled: bool,
    /// 心跳间隔（分钟）
    pub interval_minutes: u32,
    /// 心跳消息输入
    pub message_input: String,
    /// 目标输入
    pub target_input: String,
    /// 接收者输入
    pub to_input: String,
    /// 是否显示帮助对话框
    pub show_help_modal: bool,
    /// 保存错误信息
    pub save_error: Option<String>,
}

impl Default for HeartbeatSettingsState {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_minutes: 30,
            message_input: String::new(),
            target_input: String::new(),
            to_input: String::new(),
            show_help_modal: false,
            save_error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SchedulerSettingsState {
    /// 是否启用调度器
    pub enabled: bool,
    /// 最大任务数
    pub max_tasks: u32,
    /// 最大并发数
    pub max_concurrent: u32,
    /// 是否显示帮助对话框
    pub show_help_modal: bool,
    /// 保存错误信息
    pub save_error: Option<String>,
}

impl Default for SchedulerSettingsState {
    fn default() -> Self {
        Self {
            enabled: true,
            max_tasks: 64,
            max_concurrent: 4,
            show_help_modal: false,
            save_error: None,
        }
    }
}
