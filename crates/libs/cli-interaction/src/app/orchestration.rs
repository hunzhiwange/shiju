//! 编排面板与控制。

use super::{
    CronJobSnapshot,
    CronMutation,
    OrchestrationPanelKind,
    Result,
    SCHEDULE_SELECTION_SURFACE,
    TASK_SELECTION_SURFACE,
    TaskControlAction,
    UiConfirmIntent,
    UiConfirmOverlay,
    V3GatewayApp,
    V3SelectionItem,
    WORKFLOW_SELECTION_SURFACE,
    control_owned_task_blocking,
    load_orchestration_blocking,
    mutate_owned_cron_and_refresh_blocking,
    read_only_item,
    selectable_item,
};

impl V3GatewayApp {
    pub(super) fn open_orchestration(&mut self, kind: OrchestrationPanelKind) {
        let (surface, title, mut items, warning): (
            &str,
            &str,
            Vec<V3SelectionItem>,
            Option<String>,
        ) = match kind {
            OrchestrationPanelKind::Workflows => (
                WORKFLOW_SELECTION_SURFACE,
                "工作流",
                self.state
                    .orchestration
                    .workflows
                    .items
                    .iter()
                    .map(|item| selectable_item(&item.uuid, &item.name, &item.description))
                    .collect::<Vec<_>>(),
                self.state.orchestration.workflows.error.clone(),
            ),
            OrchestrationPanelKind::Schedules => (
                SCHEDULE_SELECTION_SURFACE,
                "定时任务",
                self.state
                    .orchestration
                    .schedules
                    .items
                    .iter()
                    .map(|item| {
                        selectable_item(
                            &item.id,
                            &item.name,
                            &format!(
                                "{} · {} · next {}",
                                if item.enabled { "enabled" } else { "disabled" },
                                item.schedule,
                                item.next_run,
                            ),
                        )
                    })
                    .collect::<Vec<_>>(),
                self.state.orchestration.schedules.error.clone(),
            ),
            OrchestrationPanelKind::Agents => (
                "orchestration",
                "Agents",
                self.state
                    .orchestration
                    .agents
                    .members
                    .iter()
                    .map(|item| {
                        read_only_item(
                            &item.id,
                            &item.name,
                            &format!(
                                "{:?} · team {}",
                                item.status,
                                item.team_id.as_deref().unwrap_or("-")
                            ),
                        )
                    })
                    .collect::<Vec<_>>(),
                self.state.orchestration.agents.error.clone(),
            ),
            OrchestrationPanelKind::Tasks => (
                TASK_SELECTION_SURFACE,
                "后台任务",
                self.state
                    .orchestration
                    .tasks
                    .tasks
                    .iter()
                    .map(|item| {
                        selectable_item(
                            &item.id,
                            &item.title,
                            &format!("{:?} · {}", item.status, item.detail),
                        )
                    })
                    .collect::<Vec<_>>(),
                self.state.orchestration.tasks.error.clone(),
            ),
        };
        if items.is_empty() {
            items.push(V3SelectionItem {
                id: "empty".to_string(),
                label: "无可用条目".to_string(),
                description: warning,
                disabled: true,
                ..V3SelectionItem::default()
            });
        }
        self.widget.show_selection_overlay(surface, title, items);
    }

    pub(super) fn open_task_control_confirmation(&mut self, id: &str, action: TaskControlAction) {
        let result = self
            .state
            .orchestration
            .tasks
            .tasks
            .iter()
            .find(|task| task.id == id.trim())
            .ok_or_else(|| {
                "task control requires an ID owned by the current project snapshot".to_string()
            })
            .and_then(|task| {
                let scope =
                    self.state.orchestration.scope.as_ref().ok_or_else(|| {
                        "task control requires a resolved project scope".to_string()
                    })?;
                if task.session_id != scope.project_scope().project_id().as_ref() {
                    return Err("task control snapshot does not match the resolved project scope"
                        .to_string());
                }
                if !task.canonical_status.allows(action) {
                    return Err(format!(
                        "task canonical state does not allow {}",
                        action.command_name()
                    ));
                }
                let label = action.display_label();
                Ok(UiConfirmOverlay {
                    request_id: format!("task-{}-{}", action.command_name(), task.id),
                    title: format!("确认{label}任务"),
                    body: format!("{label}当前项目任务 {}", task.id),
                    confirm_label: format!("{label}任务"),
                    cancel_label: if action.destructive() {
                        "保留任务".to_string()
                    } else {
                        "取消".to_string()
                    },
                    destructive: action.destructive(),
                    intent: UiConfirmIntent::TaskControl { id: task.id.clone(), action },
                })
            });
        self.show_integration_confirmation(result);
    }

    pub(super) fn open_cron_toggle_confirmation(&mut self, id: &str, enabled: bool) {
        let result = self.require_cron_job(id).map(|_| UiConfirmOverlay {
            request_id: format!("cron-{}-{id}", if enabled { "enable" } else { "disable" }),
            title: "确认更改定时任务状态".to_string(),
            body: format!("{}当前定时任务 {id}", if enabled { "启用" } else { "禁用" }),
            confirm_label: (if enabled { "启用" } else { "禁用" }).to_string(),
            cancel_label: "取消".to_string(),
            destructive: false,
            intent: UiConfirmIntent::ScheduleToggle { id: id.to_string(), enabled },
        });
        self.show_integration_confirmation(result);
    }

    pub(super) fn open_cron_delete_confirmation(&mut self, id: &str) {
        let result = self.require_cron_job(id).map(|_| UiConfirmOverlay {
            request_id: format!("cron-delete-{id}"),
            title: "确认删除定时任务".to_string(),
            body: format!("删除当前定时任务 {id}"),
            confirm_label: "删除".to_string(),
            cancel_label: "保留任务".to_string(),
            destructive: true,
            intent: UiConfirmIntent::ScheduleDelete { id: id.to_string() },
        });
        self.show_integration_confirmation(result);
    }

    fn require_cron_job(&self, id: &str) -> Result<&CronJobSnapshot, String> {
        let id = id.trim();
        self.state.orchestration.cron_jobs.iter().find(|job| job.owns_id(id)).ok_or_else(|| {
            "Cron mutation requires an ID from the current Cron snapshot".to_string()
        })
    }

    pub(super) fn mutate_owned_cron(&mut self, id: &str, mutation: CronMutation) -> Result<(), String> {
        let refreshed = mutate_owned_cron_and_refresh_blocking(
            &self.runtime,
            &self.state.orchestration.cron_jobs,
            id,
            mutation,
        )?;
        self.state.orchestration.schedules = refreshed.schedules;
        self.state.orchestration.cron_jobs = refreshed.cron_jobs;
        Ok(())
    }

    pub(super) fn control_owned_task(&mut self, id: &str, action: TaskControlAction) -> Result<(), String> {
        control_owned_task_blocking(&self.runtime, &self.state.orchestration, id, action)?;
        self.state.orchestration = load_orchestration_blocking(&self.runtime);
        Ok(())
    }
}
