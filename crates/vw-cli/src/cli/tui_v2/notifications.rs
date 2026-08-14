use super::model::notification::{NotificationPriority, NotificationStore, TuiNotification};
use super::state::{TuiState, select_status_summary};

impl NotificationStore {
    pub(crate) fn push(&mut self, mut notification: TuiNotification) {
        notification.message = sanitize_notification(&notification.message);
        self.items.retain(|item| item.id != notification.id);
        self.items.push(notification);
        self.items.sort_by(|left, right| {
            right.priority.cmp(&left.priority).then_with(|| right.created_ms.cmp(&left.created_ms))
        });
    }

    pub(crate) fn expire(&mut self, now_ms: u64) {
        self.items.retain(|item| {
            item.persistent || item.expires_ms.is_none_or(|deadline| deadline > now_ms)
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExitDecision {
    Interrupt,
    ArmExit,
    Exit,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ExitFlow {
    last_press_ms: Option<u64>,
}

impl ExitFlow {
    pub(crate) fn press(&mut self, now_ms: u64, busy: bool) -> ExitDecision {
        let double =
            self.last_press_ms.is_some_and(|previous| now_ms.saturating_sub(previous) <= 750);
        self.last_press_ms = Some(now_ms);
        if double {
            self.last_press_ms = None;
            ExitDecision::Exit
        } else if busy {
            ExitDecision::Interrupt
        } else {
            ExitDecision::ArmExit
        }
    }
}

pub(crate) fn builtin_status_line(state: &TuiState) -> String {
    let status = select_status_summary(state);
    format!(
        "{} · {} · in {} out {} · steps {} · todos {}",
        state.status.model_name.as_deref().unwrap_or("-"),
        state
            .project
            .workspace_root
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("-"),
        status.token_usage.input_tokens,
        status.token_usage.output_tokens,
        status.step_count,
        status.todo_count,
    )
}

pub(crate) fn osc_title(state: &TuiState) -> String {
    let title = sanitize_notification(&state.status.session_title);
    let model = sanitize_notification(state.status.model_name.as_deref().unwrap_or("-"));
    format!("\u{1b}]0;{title} · {model}\u{7}")
}

pub(crate) fn progress_bar(done: usize, total: usize, width: usize) -> String {
    if total == 0 || width == 0 {
        return String::new();
    }
    let filled = done.min(total).saturating_mul(width) / total;
    format!("{}{}", "━".repeat(filled), "─".repeat(width - filled))
}

pub(crate) fn startup_notifications(
    gateway_warning: Option<&str>,
    update: Result<Option<&str>, String>,
    now_ms: u64,
) -> Vec<TuiNotification> {
    let mut notices = Vec::new();
    if let Some(warning) = gateway_warning {
        notices.push(TuiNotification {
            id: "startup-gateway".to_string(),
            message: warning.to_string(),
            priority: NotificationPriority::Warning,
            created_ms: now_ms,
            expires_ms: None,
            persistent: true,
        });
    }
    if let Ok(Some(version)) = update {
        notices.push(TuiNotification {
            id: "update".to_string(),
            message: format!("可用更新: {version}"),
            priority: NotificationPriority::Info,
            created_ms: now_ms,
            expires_ms: Some(now_ms.saturating_add(10_000)),
            persistent: false,
        });
    }
    notices
}

fn sanitize_notification(message: &str) -> String {
    message
        .chars()
        .map(|ch| if ch.is_control() { '�' } else { ch })
        .collect::<String>()
        .split_whitespace()
        .map(|word| {
            let normalized = word.to_ascii_lowercase();
            if normalized.contains("token=")
                || normalized.contains("secret=")
                || normalized.contains("authorization=")
            {
                "[redacted]"
            } else {
                word
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
