pub const TASK_PET_CARD_HEIGHT: f32 = 72.0;
pub const TASK_PET_CARD_REPLY_HEIGHT: f32 = 120.0;
pub const TASK_PET_CARD_SPACING: f32 = 8.0;
pub const TASK_PET_LIST_HEIGHT: f32 =
    TASK_PET_CARD_HEIGHT * 3.0 + TASK_PET_CARD_SPACING * 2.0;
pub const TASK_PET_STACK_DEPTH: usize = 3;
pub const TASK_PET_STACK_OFFSET: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
/// 任务宠物可选择的头像类别；无头像仍保留窗口控件。
pub enum TaskPetAvatarKind {
    None,
    Robot,
    Beauty,
    Handsome,
}

impl TaskPetAvatarKind {
    pub fn next(self) -> Self {
        match self {
            Self::None => Self::Robot,
            Self::Robot => Self::Beauty,
            Self::Beauty => Self::Handsome,
            Self::Handsome => Self::Robot,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 任务宠物条目对应请求的执行状态。
pub enum TaskPetStatus {
    Running,
    Completed,
}

#[derive(Debug, Clone)]
/// 宠物窗口中单个代理请求的稳定展示快照。
pub struct TaskPetItem {
    pub request_id: u64,
    pub session_id: String,
    pub title: String,
    pub detail: String,
    pub detail_feedback: Option<desktop_i18n::input::Feedback>,
    pub project: Option<String>,
    pub status: TaskPetStatus,
    pub last_click_at: Option<web_time::Instant>,
}

/// Validate and apply the persisted companion scale without changing invalid input.
pub fn set_task_pet_scale(current: &mut f32, scale: f32) -> Result<(), &'static str> {
    if !scale.is_finite() {
        return Err("伙伴大小必须是有限数值");
    }
    *current = scale.clamp(0.5, 2.0);
    Ok(())
}

/// Apply only recognized companion preferences, preserving absent fields.
pub fn apply_task_pet_preferences(
    cfg: &serde_json::Value,
    avatar: &mut TaskPetAvatarKind,
    scale: &mut f32,
    visible: &mut bool,
) {
    let Some(preferences) = cfg.get("mini_companion").and_then(serde_json::Value::as_object) else {
        return;
    };
    if let Some(value) = preferences.get("avatar")
        && let Ok(value) = serde_json::from_value(value.clone())
    {
        *avatar = value;
    }
    if let Some(value) = preferences.get("scale").and_then(serde_json::Value::as_f64) {
        let _ = set_task_pet_scale(scale, value.clamp(0.5, 2.0) as f32);
    }
    if let Some(value) = preferences.get("visible").and_then(serde_json::Value::as_bool) {
        *visible = value;
    }
}

/// Owned domain state held by the desktop host.
pub struct State {
    /// 小宠物任务列表滚动容器 ID
    pub task_pet_scroll_id: iced::widget::Id,
    /// 小宠物面板位置
    pub task_pet_position: iced::Point,
    /// 小宠物面板是否收起
    pub task_pet_collapsed: bool,
    /// 小宠物展开动画进度（0.0 收起，1.0 展开）
    pub task_pet_expand_progress: f32,
    /// 小宠物展开动画目标
    pub task_pet_expand_target: Option<f32>,
    /// 小宠物面板是否正在拖拽
    pub task_pet_dragging: bool,
    /// 小宠物拖拽锚点
    pub task_pet_drag_anchor: Option<iced::Point>,
    /// 小宠物拖拽起始位置
    pub task_pet_drag_start: iced::Point,
    /// 小宠物拖动朝向（-1 左，1 右）
    pub task_pet_drag_direction: i8,
    /// 小宠物脚步动画保持到的时间戳
    pub task_pet_walk_until_ms: u64,
    /// 小宠物当前形态
    pub task_pet_avatar_kind: TaskPetAvatarKind,
    /// 伙伴比例，有限值限制在 0.5–2.0。
    pub task_pet_scale: f32,
    /// 弹窗预览独立于持久化值；保存期间禁止重复提交和取消。
    pub task_pet_size_draft: Option<f32>,
    /// 小宠物展示的任务条目
    pub task_pet_items: Vec<TaskPetItem>,
    /// 当前鼠标悬停的小宠物任务请求 ID
    pub task_pet_hovered_request_id: Option<u64>,
    /// 当前正在回复的小宠物任务请求 ID
    pub task_pet_reply_request_id: Option<u64>,
    /// 小宠物回复输入内容
    pub task_pet_reply_input: String,
    /// 已主动隐藏的请求 ID
    pub task_pet_dismissed_request_ids: std::collections::HashSet<u64>,
}
impl State {
    pub fn new() -> Self {
        Self {
            task_pet_scroll_id: iced::widget::Id::new("task_pet"),
            task_pet_position: iced::Point::new(620.0, 96.0),
            task_pet_collapsed: true,
            task_pet_expand_progress: 0.0,
            task_pet_expand_target: None,
            task_pet_dragging: false,
            task_pet_drag_anchor: None,
            task_pet_drag_start: iced::Point::ORIGIN,
            task_pet_drag_direction: 1,
            task_pet_walk_until_ms: 0,
            task_pet_avatar_kind: TaskPetAvatarKind::Robot,
            task_pet_scale: 1.0,
            task_pet_size_draft: None,
            task_pet_items: Vec::new(),
            task_pet_hovered_request_id: None,
            task_pet_reply_request_id: None,
            task_pet_reply_input: String::new(),
            task_pet_dismissed_request_ids: std::collections::HashSet::new(),
        }
    }
}

impl State {
    pub fn interaction<'a>(
        &'a mut self,
        robot_hovered: &'a mut bool,
        window_size: (f32, f32),
        now_ms: u64,
    ) -> crate::pet_interaction::TaskPetState<'a> {
        crate::pet_interaction::TaskPetState {
            task_pet_avatar_kind: &mut self.task_pet_avatar_kind,
            task_pet_collapsed: &mut self.task_pet_collapsed,
            task_pet_dismissed_request_ids: &mut self.task_pet_dismissed_request_ids,
            task_pet_drag_anchor: &mut self.task_pet_drag_anchor,
            task_pet_drag_direction: &mut self.task_pet_drag_direction,
            task_pet_drag_start: &mut self.task_pet_drag_start,
            task_pet_dragging: &mut self.task_pet_dragging,
            task_pet_expand_progress: &mut self.task_pet_expand_progress,
            task_pet_expand_target: &mut self.task_pet_expand_target,
            task_pet_hovered_request_id: &mut self.task_pet_hovered_request_id,
            task_pet_items: &mut self.task_pet_items,
            task_pet_position: &mut self.task_pet_position,
            task_pet_reply_input: &mut self.task_pet_reply_input,
            task_pet_reply_request_id: &mut self.task_pet_reply_request_id,
            task_pet_robot_hovered: robot_hovered,
            task_pet_scale: &mut self.task_pet_scale,
            task_pet_size_draft: &mut self.task_pet_size_draft,
            task_pet_walk_until_ms: &mut self.task_pet_walk_until_ms,
            window_size,
            now_ms,
        }
    }
}
