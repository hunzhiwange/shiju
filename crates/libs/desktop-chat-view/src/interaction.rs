//! Transient chat interaction state, separate from the movable session snapshot.
use iced::widget::Id;
use std::collections::{HashMap, HashSet};

pub struct State {
    pub chat_markdown_selection: Option<crate::chat_ui::ChatMarkdownSelection>,
    /// 鼠标悬停的思考块消息索引
    pub chat_think_hovered_idx: Option<u64>,
    /// 鼠标悬停的工具内文件项键
    pub chat_tool_file_hovered: Option<String>,
    /// 鼠标悬停的工具调用块消息索引
    pub chat_tool_hovered_idx: Option<u64>,
    /// 已探索摘要数字翻转动画状态
    pub chat_explore_summary_animations:
        HashMap<u128, desktop_chat_tools::tools::explore_input::ExploreSummaryAnimationState>,
    /// 工具详情对话框状态
    pub tool_detail_dialog:
        Option<desktop_session::message::chat::input::tool_detail::ToolDetailDialog>,
    /// 思考块滚动容器 ID 映射
    pub chat_think_scroll_ids: HashMap<u64, Id>,
    /// 流式响应中当前思考块的消息索引
    pub chat_stream_think_msg_idx: Option<usize>,
    /// 流式响应中的思考块计数
    pub chat_stream_think_count: usize,
    /// 流式响应中自动展开的思考块索引
    pub chat_stream_think_open_idx: Option<usize>,
    /// 轨迹账本搜索文本。
    pub trajectory_search_text: String,
    /// 轨迹详情选择与标签状态。
    pub trajectory_details: crate::trajectory::model::TrajectoryDetailsState,
    /// Duration、Turns 与 Calls 的瞬时轨迹控制状态。
    pub trajectory_controls: crate::trajectory::model::TrajectoryControlsState,
    /// 正在补载的轨迹原始步骤载荷。
    pub trajectory_raw_loading:
        HashSet<desktop_session::message::chat::trajectory::SessionStepRawKey>,
    /// 轨迹原始步骤载荷当前或待执行的有限重试次数。
    pub trajectory_raw_attempts:
        HashMap<desktop_session::message::chat::trajectory::SessionStepRawKey, u8>,
    /// 轨迹原始步骤载荷的可恢复错误。
    pub trajectory_raw_errors:
        HashMap<desktop_session::message::chat::trajectory::SessionStepRawKey, String>,
    /// 流式场景下上次自动贴底触发时间（毫秒）
    pub chat_stream_autoscroll_last_ms: u64,
    /// 程序化贴底后的保护窗口截止时间（毫秒）
    pub chat_autoscroll_hold_until_ms: u64,
    pub chat_autoscroll_generation: u64,
    /// 聊天面板是否全屏占据右侧区域
    pub chat_panel_fullscreen: bool,
    /// 聊天面板是否半全屏占据主内容区域但保留文件树和终端
    pub chat_panel_half_fullscreen: bool,
    /// 是否显示聊天右上角全屏浮动控件
    pub show_chat_fullscreen_overlay: bool,
}
impl State {
    pub fn new() -> Self {
        Self {
            chat_markdown_selection: None,
            chat_think_hovered_idx: None,
            chat_tool_file_hovered: None,
            chat_tool_hovered_idx: None,
            chat_explore_summary_animations: HashMap::new(),
            tool_detail_dialog: None,
            chat_think_scroll_ids: HashMap::new(),
            chat_stream_think_msg_idx: None,
            chat_stream_think_count: 0,
            chat_stream_think_open_idx: None,
            trajectory_search_text: String::new(),
            trajectory_details: crate::trajectory::model::TrajectoryDetailsState::default(),
            trajectory_controls: crate::trajectory::model::TrajectoryControlsState::default(),
            trajectory_raw_loading: std::collections::HashSet::new(),
            trajectory_raw_attempts: std::collections::HashMap::new(),
            trajectory_raw_errors: std::collections::HashMap::new(),
            chat_stream_autoscroll_last_ms: 0,
            chat_autoscroll_hold_until_ms: 0,
            chat_autoscroll_generation: 0,
            chat_panel_fullscreen: false,
            chat_panel_half_fullscreen: false,
            show_chat_fullscreen_overlay: false,
        }
    }
}
