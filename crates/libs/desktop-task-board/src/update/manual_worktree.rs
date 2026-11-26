//! 员工视图的单槽回收及操作反馈。
use crate::helpers::execution::{TASK_LOG_UI_MAX_DETAIL_CHARS, truncate_for_ui};
use crate::message::TaskBoardMessage;
use desktop_i18n::format::Argument;
use desktop_ui_controls::toast::ToastKind;

pub enum WorktreeEffect {
    Notify(&'static str, Vec<Argument>),
    Toast(ToastKind, &'static str, Vec<Argument>),
    ExpandNotifications,
    Recycle(String, String),
    Refresh(bool),
}
#[derive(Default)]
struct Feedback {
    effects: Vec<WorktreeEffect>,
}
impl Feedback {
    fn notify(&mut self, text: &'static str, args: Vec<Argument>) {
        self.effects.push(WorktreeEffect::Notify(text, args));
    }
    fn toast(&mut self, kind: ToastKind, text: &'static str, args: Vec<Argument>) {
        self.effects.push(WorktreeEffect::Toast(kind, text, args));
    }
}
pub struct ManualWorktreeState<'a> {
    pub slot_recycle_in_flight: &'a mut bool,
    pub task_board_worktree_snapshot:
        Option<&'a desktop_task_executor::executor::state::WorktreePoolSnapshot>,
}
pub fn update_manual_worktree(
    state: ManualWorktreeState<'_>,
    project_path: Option<String>,
    message: TaskBoardMessage,
) -> Result<Vec<WorktreeEffect>, TaskBoardMessage> {
    let mut ui = Feedback::default();
    match message {
        TaskBoardMessage::RecycleWorktreeSlotPressed { slot_id } => {
            let Some(project_path) = project_path else {
                return Ok(ui.effects);
            };
            if *state.slot_recycle_in_flight {
                ui.toast(
                    ToastKind::Warning,
                    "已有 worktree 操作进行中",
                    vec![],
                );
                return Ok(ui.effects);
            }
            if let Some(snapshot) = state.task_board_worktree_snapshot
                && let Some(slot) = snapshot.slots.iter().find(|slot| slot.id == slot_id)
                && matches!(
                    slot.state,
                    desktop_task_executor::executor::state::WorktreeState::Busy
                        | desktop_task_executor::executor::state::WorktreeState::Recycling
                )
            {
                ui.toast(
                    ToastKind::Warning,
                    "该 worktree 正在使用中，暂不能回收",
                    vec![],
                );
                return Ok(ui.effects);
            }

            *state.slot_recycle_in_flight = true;
            ui.effects.push(WorktreeEffect::Recycle(project_path, slot_id));
        }
        TaskBoardMessage::WorktreeSlotRecycled { slot_id, result } => {
            *state.slot_recycle_in_flight = false;
            match result {
                Ok(()) => {
                    ui.notify("worktree 槽位已回收: %s", vec![slot_id.clone().into()]);
                    ui.toast(ToastKind::Success, "已回收 worktree %s", vec![slot_id.into()]);
                }
                Err(error) => {
                    ui.notify(
                        "worktree 槽位回收失败 slot=%s：%s",
                        vec![
                            slot_id.into(),
                            truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS).into(),
                        ],
                    );
                    ui.effects.push(WorktreeEffect::ExpandNotifications);
                    ui.toast(ToastKind::Error, "worktree 槽位回收失败", vec![]);
                }
            };
            ui.effects.push(WorktreeEffect::Refresh(true));
        }
        TaskBoardMessage::UiTick => {
            ui.effects.push(WorktreeEffect::Refresh(false));
        }
        other => return Err(other),
    }
    Ok(ui.effects)
}
