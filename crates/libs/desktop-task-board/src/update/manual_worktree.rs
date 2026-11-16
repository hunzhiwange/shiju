//! 手动工作树动作确认、反馈和有界日志更新。
use crate::helpers::execution::*;
use crate::helpers::settings::now_ms;
use crate::helpers::worktree_logs::*;
use crate::message::TaskBoardMessage;
use desktop_i18n::format::Argument;
use desktop_ui_controls::toast::ToastKind;
pub enum WorktreeEffect {
    Notify(&'static str, Vec<Argument>),
    Toast(ToastKind, &'static str, Vec<Argument>),
    ExpandNotifications,
    Cleanup(String, std::sync::mpsc::Sender<desktop_task_executor::TaskLogStream>),
    Delete,
    Merge,
    Recycle(String, String),
    Refresh(bool),
    LogTick,
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
    pub task_board_worktree_action_log_rx:
        &'a mut Option<std::sync::mpsc::Receiver<desktop_task_executor::TaskLogStream>>,
    pub task_board_worktree_action_logs: &'a mut Vec<String>,
    pub task_board_worktree_action_logs_visible_until_ms: &'a mut Option<u64>,
    pub task_board_worktree_manual_action_kind: &'a mut Option<&'static str>,
    pub task_board_worktree_manual_confirm_kind: &'a mut Option<&'static str>,
    pub task_board_worktree_snapshot:
        &'a mut Option<desktop_task_executor::executor::state::WorktreePoolSnapshot>,
}
impl ManualWorktreeState<'_> {
    fn logs(&mut self) -> WorktreeActionLogs<'_> {
        WorktreeActionLogs {
            task_board_worktree_action_log_rx: self.task_board_worktree_action_log_rx,
            task_board_worktree_action_logs: self.task_board_worktree_action_logs,
            task_board_worktree_action_logs_visible_until_ms: self
                .task_board_worktree_action_logs_visible_until_ms,
            task_board_worktree_manual_action_kind: self.task_board_worktree_manual_action_kind,
            task_board_worktree_manual_confirm_kind: self.task_board_worktree_manual_confirm_kind,
        }
    }
}
pub fn update_manual_worktree(
    mut state: ManualWorktreeState<'_>,
    project_path: Option<String>,
    message: TaskBoardMessage,
) -> Result<Vec<WorktreeEffect>, TaskBoardMessage> {
    let mut ui = Feedback::default();
    match message {
        TaskBoardMessage::CleanAllWorktreesPressed => {
            let Some(project_path) = project_path.clone() else {
                return Ok(ui.effects);
            };
            if (*state.task_board_worktree_manual_action_kind).is_some() {
                ui.toast(
                    desktop_ui_controls::toast::ToastKind::Warning,
                    "已有 worktree 操作进行中",
                    vec![],
                );
                return Ok(ui.effects);
            }
            if (*state.task_board_worktree_manual_confirm_kind) != Some("cleanup") {
                (*state.task_board_worktree_manual_confirm_kind) = Some("cleanup");
                ui.notify(
                    "一键清理风险较高，请再次点击“确认一键清理”继续，或点击“取消”返回",
                    vec![],
                );
                ui.effects.push(WorktreeEffect::ExpandNotifications);
                return Ok(ui.effects);
            }
            let (log_tx, log_rx) = std::sync::mpsc::channel();
            (*state.task_board_worktree_manual_confirm_kind) = None;
            (*state.task_board_worktree_manual_action_kind) = Some("cleanup");
            (*state.task_board_worktree_action_logs).clear();
            (*state.task_board_worktree_action_logs_visible_until_ms) = None;
            (*state.task_board_worktree_action_log_rx) = Some(log_rx);
            push_worktree_action_log(&mut state.logs(), "准备开始一键清理...".to_string());
            ui.effects.push(WorktreeEffect::Cleanup(project_path, log_tx));
            ui.effects.push(WorktreeEffect::LogTick);
        }
        TaskBoardMessage::CleanAllWorktreesCompleted(result) => {
            poll_worktree_action_logs(&mut state.logs());
            match result {
                Ok(count) => ui.notify(
                    "已完成 worktree 一键清理，共处理 %s 个槽位",
                    vec![count.to_string().into()],
                ),
                Err(error) => {
                    push_worktree_action_log(&mut state.logs(), error.clone());
                    ui.notify(
                        "worktree 一键清理有失败，请查看右下角通知：%s",
                        vec![truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS).into()],
                    );
                    ui.effects.push(WorktreeEffect::ExpandNotifications);
                }
            }
            finish_worktree_action_logs(&mut state.logs());
            ui.effects.push(WorktreeEffect::Refresh(true))
        }
        TaskBoardMessage::DeleteAllWorktreesPressed => {
            let Some(_) = project_path.as_ref() else {
                return Ok(ui.effects);
            };
            if (*state.task_board_worktree_manual_action_kind).is_some() {
                ui.toast(
                    desktop_ui_controls::toast::ToastKind::Warning,
                    "已有 worktree 操作进行中",
                    vec![],
                );
                return Ok(ui.effects);
            }
            if (*state.task_board_worktree_manual_confirm_kind) != Some("delete") {
                (*state.task_board_worktree_manual_confirm_kind) = Some("delete");
                ui.notify("删除所有 worktree 风险极高，请再次点击“确认删除所有 worktree”继续，或点击“取消”返回", vec![]);
                ui.effects.push(WorktreeEffect::ExpandNotifications);
                ui.toast(
                    desktop_ui_controls::toast::ToastKind::Warning,
                    "再次点击确认按钮才会删除 worktree",
                    vec![],
                );
                return Ok(ui.effects);
            }
            let (_log_tx, log_rx) = std::sync::mpsc::channel();
            (*state.task_board_worktree_manual_confirm_kind) = None;
            (*state.task_board_worktree_manual_action_kind) = Some("delete");
            (*state.task_board_worktree_action_logs).clear();
            (*state.task_board_worktree_action_logs_visible_until_ms) = None;
            (*state.task_board_worktree_action_log_rx) = Some(log_rx);
            push_worktree_action_log(&mut state.logs(), "准备开始删除所有 worktree...".to_string());
            ui.effects.push(WorktreeEffect::Delete);
            ui.effects.push(WorktreeEffect::LogTick);
        }
        TaskBoardMessage::DeleteAllWorktreesCompleted(result) => {
            poll_worktree_action_logs(&mut state.logs());
            match result {
                Ok(0) => {
                    ui.notify("没有找到可删除的受管 worktree", vec![]);
                    ui.toast(
                        desktop_ui_controls::toast::ToastKind::Warning,
                        "没有找到可删除的受管 worktree",
                        vec![],
                    )
                }
                Ok(count) => {
                    ui.notify(
                        "已删除所有 worktree，共处理 %s 个槽位",
                        vec![(count).to_string().into()],
                    );
                    ui.toast(
                        desktop_ui_controls::toast::ToastKind::Success,
                        "已删除 %u 个 worktree",
                        vec![count.into()],
                    )
                }
                Err(error) => {
                    push_worktree_action_log(&mut state.logs(), error.clone());
                    ui.notify(
                        "删除所有 worktree 有失败，请查看右下角通知：%s",
                        vec![
                            (truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS))
                                .to_string()
                                .into(),
                        ],
                    );
                    ui.effects.push(WorktreeEffect::ExpandNotifications);
                    ui.toast(
                        desktop_ui_controls::toast::ToastKind::Error,
                        "删除所有 worktree 失败",
                        vec![],
                    )
                }
            };
            finish_worktree_action_logs(&mut state.logs());
            ui.effects.push(WorktreeEffect::Refresh(true));
        }
        TaskBoardMessage::RecycleWorktreeSlotPressed { slot_id } => {
            let Some(project_path) = project_path.clone() else {
                return Ok(ui.effects);
            };
            if (*state.task_board_worktree_manual_action_kind).is_some() {
                ui.toast(
                    desktop_ui_controls::toast::ToastKind::Warning,
                    "已有 worktree 操作进行中",
                    vec![],
                );
                return Ok(ui.effects);
            }
            if let Some(snapshot) = &(*state.task_board_worktree_snapshot)
                && let Some(slot) = snapshot.slots.iter().find(|slot| slot.id == slot_id)
                && matches!(
                    slot.state,
                    desktop_task_executor::executor::state::WorktreeState::Busy
                        | desktop_task_executor::executor::state::WorktreeState::Recycling
                )
            {
                ui.toast(
                    desktop_ui_controls::toast::ToastKind::Warning,
                    "该 worktree 正在使用中，暂不能回收",
                    vec![],
                );
                return Ok(ui.effects);
            }

            let (_log_tx, log_rx) = std::sync::mpsc::channel();
            (*state.task_board_worktree_manual_confirm_kind) = None;
            (*state.task_board_worktree_manual_action_kind) = Some("slot_recycle");
            (*state.task_board_worktree_action_logs).clear();
            (*state.task_board_worktree_action_logs_visible_until_ms) = None;
            (*state.task_board_worktree_action_log_rx) = Some(log_rx);
            push_worktree_action_log(
                &mut state.logs(),
                format!("准备回收 worktree 槽位 {}...", slot_id),
            );
            ui.effects.push(WorktreeEffect::Recycle(project_path, slot_id));
            ui.effects.push(WorktreeEffect::LogTick);
        }
        TaskBoardMessage::WorktreeSlotRecycled { slot_id, result } => {
            poll_worktree_action_logs(&mut state.logs());
            match result {
                Ok(()) => {
                    ui.notify("worktree 槽位已回收: %s", vec![(slot_id).to_string().into()]);
                    ui.toast(
                        desktop_ui_controls::toast::ToastKind::Success,
                        "已回收 worktree %s",
                        vec![slot_id.into()],
                    )
                }
                Err(error) => {
                    ui.notify(
                        "worktree 槽位回收失败 slot=%s：%s",
                        vec![
                            (slot_id).to_string().into(),
                            (truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS))
                                .to_string()
                                .into(),
                        ],
                    );
                    ui.effects.push(WorktreeEffect::ExpandNotifications);
                    ui.toast(
                        desktop_ui_controls::toast::ToastKind::Error,
                        "worktree 槽位回收失败",
                        vec![],
                    )
                }
            };
            finish_worktree_action_logs(&mut state.logs());
            ui.effects.push(WorktreeEffect::Refresh(true));
        }
        TaskBoardMessage::CommitMergeAllWorktreesPressed => {
            let Some(_) = project_path.as_ref() else {
                return Ok(ui.effects);
            };
            if (*state.task_board_worktree_manual_action_kind).is_some() {
                ui.toast(
                    desktop_ui_controls::toast::ToastKind::Warning,
                    "已有 worktree 操作进行中",
                    vec![],
                );
                return Ok(ui.effects);
            }
            if (*state.task_board_worktree_manual_confirm_kind) != Some("merge") {
                (*state.task_board_worktree_manual_confirm_kind) = Some("merge");
                ui.notify(
                    "一键合并风险较高，请再次点击“确认一键合并”继续，或点击“取消”返回",
                    vec![],
                );
                ui.effects.push(WorktreeEffect::ExpandNotifications);
                return Ok(ui.effects);
            }
            let (_log_tx, log_rx) = std::sync::mpsc::channel();
            (*state.task_board_worktree_manual_confirm_kind) = None;
            (*state.task_board_worktree_manual_action_kind) = Some("merge");
            (*state.task_board_worktree_action_logs).clear();
            (*state.task_board_worktree_action_logs_visible_until_ms) = None;
            (*state.task_board_worktree_action_log_rx) = Some(log_rx);
            push_worktree_action_log(&mut state.logs(), "准备开始一键合并...".to_string());
            ui.effects.push(WorktreeEffect::Merge);
            ui.effects.push(WorktreeEffect::LogTick);
        }
        TaskBoardMessage::CommitMergeAllWorktreesCompleted(result) => {
            poll_worktree_action_logs(&mut state.logs());
            match result {
                Ok(count) => ui.notify(
                    "已完成 worktree 一键合并，共合并 %s 个分支",
                    vec![count.to_string().into()],
                ),
                Err(error) => {
                    push_worktree_action_log(&mut state.logs(), error.clone());
                    ui.notify(
                        "worktree 一键合并有失败，请查看右下角通知：%s",
                        vec![truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS).into()],
                    );
                    ui.effects.push(WorktreeEffect::ExpandNotifications);
                }
            }
            finish_worktree_action_logs(&mut state.logs());
            ui.effects.push(WorktreeEffect::Refresh(true))
        }
        TaskBoardMessage::CancelWorktreeManualConfirm => {
            let Some(kind) = (*state.task_board_worktree_manual_confirm_kind).take() else {
                return Ok(ui.effects);
            };
            let action_label = match kind {
                "cleanup" => "一键清理",
                "delete" => "一键删除",
                "merge" => "一键合并",
                _ => "worktree 操作",
            };
            ui.notify("已取消 %s 确认，可重新发起操作", vec![(action_label).to_string().into()]);
            ()
        }
        TaskBoardMessage::WorktreeActionLogTick => {
            poll_worktree_action_logs(&mut state.logs());
            if (*state.task_board_worktree_action_log_rx).is_some() {
                ui.effects.push(WorktreeEffect::LogTick);
                return Ok(ui.effects);
            }
            if let Some(deadline_ms) = (*state.task_board_worktree_action_logs_visible_until_ms)
                && now_ms() >= deadline_ms
            {
                clear_worktree_action_logs(&mut state.logs());
            }
            ()
        }
        TaskBoardMessage::UiTick => {
            poll_worktree_action_logs(&mut state.logs());
            if (*state.task_board_worktree_action_log_rx).is_none()
                && let Some(deadline_ms) = (*state.task_board_worktree_action_logs_visible_until_ms)
                && now_ms() >= deadline_ms
            {
                clear_worktree_action_logs(&mut state.logs());
            }
            ui.effects.push(WorktreeEffect::Refresh(false));
        }
        other => return Err(other),
    }
    Ok(ui.effects)
}
