//! Recoverable task display. Retrying observation never repeats a transfer.
use crate::app::components::system_settings_common::rounded_action_btn_style;
use iced::widget::{button, column, row, text};
use iced::{Element, Task};
use vw_api_types::{TaskId, agent_apps::DataManagementTarget, device_sync_task::*};
use vw_gateway_client::GatewayClient;

const POLL_LIMIT: u16 = 120;

#[derive(Debug, Clone)]
pub enum TaskPanelMessage {
    Discover { last_status: Option<SyncTaskStatus>, active_task_id: Option<TaskId> },
    Refresh,
    Cancel,
    Poll(u64),
    Observed(u64, Result<SyncTaskStatus, String>),
    Cancelled(u64, Result<SyncTaskCancelResponse, String>),
}

#[derive(Debug, Clone, Default)]
pub struct TaskPanelState {
    pub status: Option<SyncTaskStatus>,
    pub active_task_id: Option<TaskId>,
    target: Option<DataManagementTarget>,
    epoch: u64,
    polls: u16,
    busy: bool,
    cancel_busy: bool,
    error: Option<String>,
}

impl TaskPanelState {
    /// Changing scope invalidates all in-flight replies, including old cancellations.
    pub fn reset(&mut self) {
        let epoch = self.epoch.wrapping_add(1);
        *self = Self { epoch, ..Self::default() };
    }

    pub fn is_busy(&self) -> bool {
        self.active_task_id.is_some() || self.busy || self.cancel_busy
    }

    pub fn update(
        &mut self,
        message: TaskPanelMessage,
        client: GatewayClient,
        target: DataManagementTarget,
        token: String,
    ) -> (Task<TaskPanelMessage>, Option<String>) {
        if self.target.as_ref() != Some(&target) {
            self.reset();
            self.target = Some(target.clone());
        }
        let mut toast = None;
        match message {
            TaskPanelMessage::Discover { last_status, active_task_id } => {
                self.epoch = self.epoch.wrapping_add(1);
                self.busy = false;
                self.cancel_busy = false;
                self.polls = 0;
                self.error = None;
                self.status = last_status.filter(|s| s.target == target);
                self.active_task_id = active_task_id;
            }
            TaskPanelMessage::Refresh => {
                self.polls = 0;
                self.error = None;
            }
            TaskPanelMessage::Poll(epoch) => {
                if epoch != self.epoch || self.busy || self.error.is_some() {
                    return (Task::none(), None);
                }
            }
            TaskPanelMessage::Cancel => {
                if self.cancel_busy {
                    return (Task::none(), None);
                }
                let Some(id) = self.active_task_id.clone() else {
                    return (Task::none(), None);
                };
                self.cancel_busy = true;
                let epoch = self.epoch;
                return (
                    Task::perform(
                        async move {
                            client
                                .device_sync_task_cancel(&target, &id, &token)
                                .await
                                .map_err(|_| "取消请求失败；请刷新状态后重试".to_owned())
                        },
                        move |r| TaskPanelMessage::Cancelled(epoch, r),
                    ),
                    None,
                );
            }
            TaskPanelMessage::Observed(epoch, result) => {
                if epoch != self.epoch {
                    return (Task::none(), None);
                }
                self.busy = false;
                match result {
                    Ok(status)
                        if status.target == target
                            && self.active_task_id.as_ref() == Some(&status.task_id) =>
                    {
                        toast = self.accept(status);
                    }
                    Ok(_) => return (Task::none(), None),
                    Err(error) => {
                        self.error = Some(error.clone());
                        return (Task::none(), Some(error));
                    }
                }
                if self.active_task_id.is_some() && self.polls < POLL_LIMIT {
                    let epoch = self.epoch;
                    return (
                        Task::perform(
                            async move {
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                epoch
                            },
                            TaskPanelMessage::Poll,
                        ),
                        toast,
                    );
                }
                if self.active_task_id.is_some() {
                    self.error = Some("已暂停自动查询，请刷新继续；同步任务仍在网关运行".into());
                }
                return (Task::none(), toast);
            }
            TaskPanelMessage::Cancelled(epoch, result) => {
                if epoch != self.epoch {
                    return (Task::none(), None);
                }
                self.cancel_busy = false;
                match result {
                    Ok(response) => {
                        let (status, late) = match response {
                            SyncTaskCancelResponse::Accepted { status } => (status, false),
                            SyncTaskCancelResponse::TooLate { status } => (status, true),
                        };
                        if status.target != target
                            || self.active_task_id.as_ref() != Some(&status.task_id)
                        {
                            return (Task::none(), None);
                        }
                        toast = self.accept(status);
                        if late {
                            toast = Some("任务已进入提交阶段，无法取消；请等待最终结果".into());
                        }
                    }
                    Err(error) => {
                        self.error = Some(error.clone());
                        toast = Some(error);
                    }
                }
            }
        }
        if self.busy || self.polls >= POLL_LIMIT {
            return (Task::none(), toast);
        }
        let Some(id) = self.active_task_id.clone() else {
            return (Task::none(), toast);
        };
        self.busy = true;
        self.polls += 1;
        let epoch = self.epoch;
        (
            Task::perform(
                async move {
                    client
                        .device_sync_task_status(&target, &id, &token)
                        .await
                        .map_err(|_| "连接中断，请重新读取连接配置以恢复任务状态".to_owned())
                },
                move |r| TaskPanelMessage::Observed(epoch, r),
            ),
            toast,
        )
    }

    fn accept(&mut self, status: SyncTaskStatus) -> Option<String> {
        let terminal = status.state.is_terminal();
        let notice = if terminal { Some(result_label(&status)) } else { None };
        if terminal {
            self.active_task_id = None;
        }
        self.error = None;
        self.status = Some(status);
        notice
    }

    pub fn view(&self) -> Element<'_, TaskPanelMessage> {
        let mut content = column![text("同步任务")].spacing(8);
        if let Some(status) = &self.status {
            let origin = match status.origin {
                SyncTaskOrigin::Manual => "手动同步",
                SyncTaskOrigin::Auto => "自动同步",
            };
            content = content.push(text(format!("{origin} · {}", phase_label(status.phase))));
            let bytes = match status.total_bytes {
                Some(total) => format!("{} / {total} 字节", status.bytes_transferred),
                None => format!("{} 字节（总量未知）", status.bytes_transferred),
            };
            content = content.push(text(bytes)).push(text(result_label(status)));
            for warning in &status.warnings {
                content = content.push(text(match warning {
                    SyncTaskWarning::ProjectionFailed => {
                        "数据已提交，但运行时投影失败；尚未恢复旧数据"
                    }
                    SyncTaskWarning::StatePersistenceFailed => {
                        "数据已提交，但状态持久化失败，请重新读取状态"
                    }
                }));
            }
        } else {
            content = content.push(text("暂无同步任务"));
        }
        if let Some(error) = &self.error {
            content = content.push(text(error));
        }
        let refresh = button(text("刷新状态"))
            .padding([7, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe((!self.busy).then_some(TaskPanelMessage::Refresh));
        let cancel =
            button(text(if self.cancel_busy { "正在取消…" } else { "取消同步" }))
                .padding([7, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe(
                    (self.active_task_id.is_some() && !self.cancel_busy)
                        .then_some(TaskPanelMessage::Cancel),
                );
        content.push(row![refresh, cancel].spacing(8)).into()
    }
}

fn phase_label(phase: SyncTaskPhase) -> &'static str {
    match phase {
        SyncTaskPhase::WaitingForLock => "等待锁",
        SyncTaskPhase::InspectingRemote => "检查远端",
        SyncTaskPhase::Exporting => "导出",
        SyncTaskPhase::Transferring => "传输",
        SyncTaskPhase::Verifying => "校验",
        SyncTaskPhase::Previewing => "预览",
        SyncTaskPhase::Committing => "提交",
        SyncTaskPhase::Projecting => "更新运行时投影",
        SyncTaskPhase::RollingBack => "恢复旧数据",
        SyncTaskPhase::PersistingStatus => "保存状态",
    }
}

fn result_label(status: &SyncTaskStatus) -> String {
    match &status.state {
        SyncTaskState::Queued => "排队中".into(),
        SyncTaskState::Running => "进行中".into(),
        SyncTaskState::Succeeded { .. } => "同步完成".into(),
        SyncTaskState::Cancelled => "已取消".into(),
        SyncTaskState::Failed { error, committed } => {
            let outcome = if *committed {
                "数据已提交，未恢复旧数据"
            } else if error.code == SyncErrorCode::ApplyFailed {
                "应用失败，旧数据已恢复"
            } else {
                "未提交新数据"
            };
            format!("同步失败（{:?}）：{outcome}", error.code)
        }
        SyncTaskState::Interrupted { committed } => {
            if *committed {
                "任务中断：数据已提交，请核对状态".into()
            } else {
                "任务中断：未提交新数据".into()
            }
        }
    }
}
