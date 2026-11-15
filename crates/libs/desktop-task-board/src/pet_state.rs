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
