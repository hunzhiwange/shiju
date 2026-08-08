//! Overlay 优先键位层。
//!
//! 本模块只归一化 overlay reducer 命令，不执行 gateway 请求或 terminal I/O。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::completion::slash_palette_overlay;
use super::controller::{TuiControllerCommand, TuiOverlayCommand};
use super::keymap::{KeymapLayerResult, scroll_to_tail};
use super::model::settings_panel::SettingKey;
use super::model::integration::IntegrationKind;
use super::model::{
    PermissionDecision, UiOverlay, UiOverlayKind, UiQuestionOverlay, UiTaskOverlay, UiTodoOverlay,
};
use super::state::{TuiAction, TuiState, reduce_tui_state, select_transcript_message_anchors};

const QUESTION_CUSTOM_ANSWER_PREFIX: &str = "__custom__:";

pub(super) fn handle_overlay_layer_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    let Some(kind) = state.overlays.active().map(UiOverlay::kind) else {
        return KeymapLayerResult::Pass;
    };

    match kind {
        UiOverlayKind::Confirm => handle_confirm_overlay_key(state, key),
        UiOverlayKind::Search => handle_search_overlay_key(state, key),
        UiOverlayKind::Question => handle_question_overlay_key(state, key),
        UiOverlayKind::Todo => handle_todo_overlay_key(state, key),
        UiOverlayKind::Task => handle_task_overlay_key(state, key),
        UiOverlayKind::Error => handle_error_overlay_key(state, key),
        UiOverlayKind::CommandPalette => handle_command_palette_key(state, key),
        UiOverlayKind::Report => handle_report_overlay_key(state, key),
        UiOverlayKind::Mcp => handle_mcp_overlay_key(state, key),
        UiOverlayKind::Memory => handle_memory_overlay_key(state, key),
        UiOverlayKind::Settings => handle_settings_overlay_key(state, key),
        UiOverlayKind::Permissions => handle_permissions_overlay_key(state, key),
        UiOverlayKind::Integration => handle_integration_overlay_key(state, key),
        UiOverlayKind::BackgroundTasks => handle_background_tasks_overlay_key(state, key),
        UiOverlayKind::Agents => handle_agents_overlay_key(state, key),
        UiOverlayKind::Workflows => handle_workflows_overlay_key(state, key),
        UiOverlayKind::Schedules => handle_schedules_overlay_key(state, key),
    }
}

fn handle_schedules_overlay_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    if key.code == KeyCode::Esc {
        reduce_tui_state(state, TuiAction::OverlayPopped);
        return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
    }
    let Some(UiOverlay::Schedules(panel)) = state.overlays.stack.last_mut() else {
        return KeymapLayerResult::Pass;
    };
    match key.code {
        KeyCode::Up => panel.selected = panel.selected.saturating_sub(1),
        KeyCode::Down => {
            panel.selected =
                panel.selected.saturating_add(1).min(panel.items.len().saturating_sub(1));
        }
        _ => {}
    }
    KeymapLayerResult::Handled(TuiControllerCommand::Continue)
}

fn handle_workflows_overlay_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    if key.code == KeyCode::Esc {
        reduce_tui_state(state, TuiAction::OverlayPopped);
        return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
    }
    if key.code == KeyCode::Enter {
        let replacement = state.overlays.active().and_then(|overlay| {
            let UiOverlay::Workflows(panel) = overlay else {
                return None;
            };
            panel.items.get(panel.selected).map(|item| {
                format!(
                    "/workflow:{}--{} ",
                    workflow_label(item.name.as_str()),
                    workflow_label(item.uuid.as_str())
                )
            })
        });
        if let Some(replacement) = replacement {
            reduce_tui_state(state, TuiAction::PromptValueSet(replacement));
            reduce_tui_state(state, TuiAction::OverlayPopped);
        }
        return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
    }
    let Some(UiOverlay::Workflows(panel)) = state.overlays.stack.last_mut() else {
        return KeymapLayerResult::Pass;
    };
    match key.code {
        KeyCode::Up => panel.selected = panel.selected.saturating_sub(1),
        KeyCode::Down => {
            panel.selected =
                panel.selected.saturating_add(1).min(panel.items.len().saturating_sub(1));
        }
        _ => {}
    }
    KeymapLayerResult::Handled(TuiControllerCommand::Continue)
}

fn workflow_label(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn handle_background_tasks_overlay_key(
    state: &mut TuiState,
    key: KeyEvent,
) -> KeymapLayerResult {
    if key.code == KeyCode::Esc {
        reduce_tui_state(state, TuiAction::OverlayPopped);
        return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
    }
    let Some(UiOverlay::BackgroundTasks(panel)) = state.overlays.stack.last_mut() else {
        return KeymapLayerResult::Pass;
    };
    match key.code {
        KeyCode::Up => panel.selected = panel.selected.saturating_sub(1),
        KeyCode::Down => {
            panel.selected =
                panel.selected.saturating_add(1).min(panel.tasks.len().saturating_sub(1));
        }
        _ => {}
    }
    KeymapLayerResult::Handled(TuiControllerCommand::Continue)
}

fn handle_agents_overlay_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    if key.code == KeyCode::Esc {
        reduce_tui_state(state, TuiAction::OverlayPopped);
        return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
    }
    let Some(UiOverlay::Agents(view)) = state.overlays.stack.last_mut() else {
        return KeymapLayerResult::Pass;
    };
    let current = view
        .viewed_agent_id
        .as_deref()
        .and_then(|id| view.members.iter().position(|member| member.id == id))
        .unwrap_or_default();
    let next = match key.code {
        KeyCode::Up => current.saturating_sub(1),
        KeyCode::Down => current.saturating_add(1).min(view.members.len().saturating_sub(1)),
        _ => return KeymapLayerResult::Handled(TuiControllerCommand::Continue),
    };
    view.viewed_agent_id = view.members.get(next).map(|member| member.id.clone());
    KeymapLayerResult::Handled(TuiControllerCommand::Continue)
}

fn handle_integration_overlay_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    if key.code == KeyCode::Esc {
        if let Some(UiOverlay::Integration(panel)) = state.overlays.stack.last_mut()
            && panel.cancel_skill_provider_picker()
        {
            return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
        }
        reduce_tui_state(state, TuiAction::OverlayPopped);
        return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
    }
    if let Some(UiOverlay::Integration(panel)) = state.overlays.stack.last_mut()
        && panel.skill_provider_picker.is_some()
    {
        match key.code {
            KeyCode::Up => panel.move_skill_provider_picker_selection(-1),
            KeyCode::Down => panel.move_skill_provider_picker_selection(1),
            KeyCode::Enter => {
                let Some(provider) = panel.selected_skill_provider() else {
                    return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
                };
                let mut next = panel.clone();
                next.set_skill_provider(provider);
                return KeymapLayerResult::Handled(TuiControllerCommand::Overlay(
                    TuiOverlayCommand::IntegrationSkillProviderSelected(next, provider),
                ));
            }
            _ => {}
        }
        return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
    }
    if matches!(key.code, KeyCode::Char('p' | 'P'))
        || (key.code == KeyCode::Enter
            && matches!(
                state.overlays.active(),
                Some(UiOverlay::Integration(panel))
                    if panel.kind == IntegrationKind::Skill
                        && panel.skill_platform_focused
            ))
    {
        if let Some(UiOverlay::Integration(panel)) = state.overlays.stack.last_mut() {
            panel.open_skill_provider_picker();
        }
        return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
    }
    if key.code == KeyCode::Char('r') {
        let Some(UiOverlay::Integration(panel)) = state.overlays.active().cloned() else {
            return KeymapLayerResult::Pass;
        };
        if panel.kind == IntegrationKind::Skill && panel.skill_platform_focused {
            return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
        }
        return KeymapLayerResult::Handled(TuiControllerCommand::Overlay(
            TuiOverlayCommand::IntegrationRefresh(panel),
        ));
    }
    if matches!(
        key.code,
        KeyCode::Enter | KeyCode::Char(' ' | 't' | 'T' | 'd' | 'D')
    ) {
        let Some(UiOverlay::Integration(panel)) = state.overlays.active().cloned() else {
            return KeymapLayerResult::Pass;
        };
        let command = if matches!(
            key.code,
            KeyCode::Enter | KeyCode::Char(' ' | 't' | 'T')
        ) {
            if !panel
                .items
                .get(panel.selected)
                .is_some_and(|item| item.actions.iter().any(|action| action.enabled))
            {
                return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
            }
            TuiOverlayCommand::IntegrationToggle(panel)
        } else {
            TuiOverlayCommand::IntegrationDeleteRequested(panel)
        };
        return KeymapLayerResult::Handled(TuiControllerCommand::Overlay(command));
    }
    let Some(UiOverlay::Integration(panel)) = state.overlays.stack.last_mut() else {
        return KeymapLayerResult::Pass;
    };
    match key.code {
        KeyCode::Up => {
            if panel.kind == IntegrationKind::Skill && panel.selected == 0 {
                panel.skill_platform_focused = true;
            } else {
                panel.selected = panel.selected.saturating_sub(1);
            }
        }
        KeyCode::Down => {
            if panel.kind == IntegrationKind::Skill
                && panel.skill_platform_focused
                && !panel.items.is_empty()
            {
                panel.skill_platform_focused = false;
                panel.selected = 0;
            } else {
                panel.selected =
                    panel.selected.saturating_add(1).min(panel.items.len().saturating_sub(1));
            }
        }
        _ => {}
    }
    KeymapLayerResult::Handled(TuiControllerCommand::Continue)
}

fn handle_settings_overlay_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    if key.code == KeyCode::Esc {
        if let Some(UiOverlay::Settings(panel)) = state.overlays.stack.last_mut()
            && (panel.cancel_model_picker()
                || panel.cancel_delegate_agent_picker()
                || panel.cancel_acp_backend_picker())
        {
            return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
        }
        reduce_tui_state(state, TuiAction::OverlayPopped);
        return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
    }
    if key.code == KeyCode::Enter {
        let picker_result =
            if let Some(UiOverlay::Settings(panel)) = state.overlays.stack.last_mut() {
                if panel.model_picker.is_some() {
                    Some(panel.confirm_model_picker())
                } else if panel.delegate_agent_picker.is_some() {
                    panel.confirm_delegate_agent_picker();
                    Some(false)
                } else if panel.acp_backend_picker.is_some() {
                    Some(panel.confirm_acp_backend_picker())
                } else {
                    None
                }
            } else {
                None
            };
        if let Some(save_immediately) = picker_result {
            if save_immediately
                && let Some(UiOverlay::Settings(panel)) = state.overlays.active().cloned()
            {
                return KeymapLayerResult::Handled(TuiControllerCommand::Overlay(
                    TuiOverlayCommand::SettingsSave(Box::new(panel)),
                ));
            }
            return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
        }
        let picker_key = state.overlays.active().and_then(|overlay| {
            let UiOverlay::Settings(panel) = overlay else {
                return None;
            };
            let row = panel.selected_row().filter(|row| row.unavailable_reason.is_none())?;
            match row.key {
                SettingKey::Model if panel.pending.model == panel.original.model => {
                    Some(SettingKey::Model)
                }
                SettingKey::DelegateAgent
                    if panel.pending.delegate_agent == panel.original.delegate_agent =>
                {
                    Some(SettingKey::DelegateAgent)
                }
                SettingKey::AcpBackend
                    if panel.pending.acp_backend == panel.original.acp_backend =>
                {
                    Some(SettingKey::AcpBackend)
                }
                _ => None,
            }
        });
        if let Some(picker_key) = picker_key {
            if let Some(UiOverlay::Settings(panel)) = state.overlays.stack.last_mut() {
                match picker_key {
                    SettingKey::Model => {
                        panel.open_model_picker();
                    }
                    SettingKey::DelegateAgent => {
                        panel.open_delegate_agent_picker();
                    }
                    SettingKey::AcpBackend => {
                        panel.open_acp_backend_picker();
                    }
                    _ => {}
                }
            }
            return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
        }
        let Some(UiOverlay::Settings(panel)) = state.overlays.active().cloned() else {
            return KeymapLayerResult::Pass;
        };
        return KeymapLayerResult::Handled(TuiControllerCommand::Overlay(
            TuiOverlayCommand::SettingsSave(Box::new(panel)),
        ));
    }
    match key.code {
        KeyCode::Up => {
            if let Some(UiOverlay::Settings(panel)) = state.overlays.stack.last_mut() {
                if panel.model_picker.is_some() {
                    panel.move_model_picker_selection(-1);
                } else if panel.delegate_agent_picker.is_some() {
                    panel.move_delegate_agent_picker_selection(-1);
                } else if panel.acp_backend_picker.is_some() {
                    panel.move_acp_backend_picker_selection(-1);
                } else {
                    panel.move_selection(-1);
                }
            }
        }
        KeyCode::Down => {
            if let Some(UiOverlay::Settings(panel)) = state.overlays.stack.last_mut() {
                if panel.model_picker.is_some() {
                    panel.move_model_picker_selection(1);
                } else if panel.delegate_agent_picker.is_some() {
                    panel.move_delegate_agent_picker_selection(1);
                } else if panel.acp_backend_picker.is_some() {
                    panel.move_acp_backend_picker_selection(1);
                } else {
                    panel.move_selection(1);
                }
            }
        }
        KeyCode::Left
            if state
                .overlays
                .active()
                .is_some_and(|overlay| matches!(overlay, UiOverlay::Settings(panel) if panel.model_picker.is_none() && panel.delegate_agent_picker.is_none() && panel.acp_backend_picker.is_none())) =>
        {
            cycle_setting(state, -1);
        }
        KeyCode::Right | KeyCode::Tab
            if state
                .overlays
                .active()
                .is_some_and(|overlay| matches!(overlay, UiOverlay::Settings(panel) if panel.model_picker.is_none() && panel.delegate_agent_picker.is_none() && panel.acp_backend_picker.is_none())) =>
        {
            cycle_setting(state, 1);
        }
        _ => {}
    }
    KeymapLayerResult::Handled(TuiControllerCommand::Continue)
}

fn cycle_setting(state: &mut TuiState, delta: isize) {
    let key = state.overlays.active().and_then(|overlay| {
        let UiOverlay::Settings(panel) = overlay else {
            return None;
        };
        panel.selected_row().filter(|row| row.unavailable_reason.is_none()).map(|row| row.key)
    });
    let Some(key) = key else {
        return;
    };
    let models = state.session_capabilities.models.clone();
    let efforts = state.session_capabilities.efforts.clone();
    let modes = state.session_capabilities.permission_modes.clone();
    let Some(UiOverlay::Settings(panel)) = state.overlays.stack.last_mut() else {
        return;
    };
    match key {
        SettingKey::Model => {
            panel.pending.model = cycle_value(panel.pending.model.as_ref(), &models, delta);
        }
        SettingKey::DelegateAgent => {
            let values = &panel.delegate_agent_options;
            if !values.is_empty() {
                let current = values
                    .iter()
                    .position(|candidate| candidate.key == panel.pending.delegate_agent)
                    .unwrap_or_default();
                let next = if delta < 0 {
                    current.checked_sub(1).unwrap_or(values.len().saturating_sub(1))
                } else {
                    (current + 1) % values.len()
                };
                panel.pending.delegate_agent.clone_from(&values[next].key);
            }
        }
        SettingKey::AcpBackend => {
            let values = &panel.acp_backend_options;
            if !values.is_empty() {
                let current = values
                    .iter()
                    .position(|candidate| candidate.key == panel.pending.acp_backend)
                    .unwrap_or_default();
                let next = if delta < 0 {
                    current.checked_sub(1).unwrap_or(values.len().saturating_sub(1))
                } else {
                    (current + 1) % values.len()
                };
                panel.pending.acp_backend.clone_from(&values[next].key);
            }
        }
        SettingKey::FullAccess => {
            panel.pending.full_access_enabled = !panel.pending.full_access_enabled;
        }
        SettingKey::SubagentMode => {
            panel.pending.subagent_mode_enabled = !panel.pending.subagent_mode_enabled;
        }
        SettingKey::WorkflowMode => {
            panel.pending.workflow_mode = if panel.pending.workflow_mode
                == super::model::session_mode::WorkflowMode::Enabled
            {
                super::model::session_mode::WorkflowMode::Disabled
            } else {
                super::model::session_mode::WorkflowMode::Enabled
            };
        }
        SettingKey::GoalMode => {
            // 目标模式底层固定开启，设置项已从面板移除；保留分支防止旧状态误关。
            panel.pending.goal_mode = super::model::session_mode::GoalMode::Enabled;
        }
        SettingKey::Effort => {
            panel.pending.effort = cycle_copy_value(panel.pending.effort, &efforts, delta);
        }
        SettingKey::Fast => panel.pending.fast = !panel.pending.fast,
        SettingKey::Theme => {
            let themes = [
                "Dark".to_string(),
                "Light".to_string(),
                "DarkDaltonized".to_string(),
                "LightDaltonized".to_string(),
                "DarkAnsi".to_string(),
                "LightAnsi".to_string(),
            ];
            if let Some(theme) = cycle_value(Some(&panel.pending.theme), &themes, delta) {
                panel.pending.theme = theme;
            }
        }
        SettingKey::Permissions => {
            panel.pending.permission_mode =
                cycle_copy_value(panel.pending.permission_mode, &modes, delta);
        }
        SettingKey::Vim => {
            panel.pending.vim_mode = match panel.pending.vim_mode {
                crate::cli::tui_v2::vim::VimMode::Disabled => {
                    crate::cli::tui_v2::vim::VimMode::Normal
                }
                _ => crate::cli::tui_v2::vim::VimMode::Disabled,
            };
        }
        SettingKey::OutputStyle => {}
    }
}

fn cycle_value(current: Option<&String>, values: &[String], delta: isize) -> Option<String> {
    if values.is_empty() {
        return current.cloned();
    }
    let index = current
        .and_then(|current| values.iter().position(|value| value == current))
        .unwrap_or_default();
    let next = if delta < 0 {
        index.checked_sub(1).unwrap_or(values.len().saturating_sub(1))
    } else {
        (index + 1) % values.len()
    };
    values.get(next).cloned()
}

fn cycle_copy_value<T: Copy + PartialEq>(current: T, values: &[T], delta: isize) -> T {
    if values.is_empty() {
        return current;
    }
    let index = values.iter().position(|value| *value == current).unwrap_or_default();
    let next = if delta < 0 {
        index.checked_sub(1).unwrap_or(values.len().saturating_sub(1))
    } else {
        (index + 1) % values.len()
    };
    values[next]
}

fn handle_permissions_overlay_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    if key.code == KeyCode::Esc {
        reduce_tui_state(state, TuiAction::OverlayPopped);
        return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
    }
    let decision = match key.code {
        KeyCode::Char('1') => Some(PermissionDecision::Once),
        KeyCode::Char('2') => Some(PermissionDecision::Always),
        KeyCode::Char('r' | 'R') => Some(PermissionDecision::Reject { reason: None }),
        _ => None,
    };
    match decision {
        Some(decision) => KeymapLayerResult::Handled(TuiControllerCommand::Overlay(
            TuiOverlayCommand::PermissionDecision(decision),
        )),
        None => KeymapLayerResult::Handled(TuiControllerCommand::Continue),
    }
}

fn handle_report_overlay_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    if key.code == KeyCode::Esc {
        reduce_tui_state(state, TuiAction::OverlayPopped);
        return KeymapLayerResult::Handled(TuiControllerCommand::Continue);
    }
    let Some(UiOverlay::Report(overlay)) = state.overlays.stack.last_mut() else {
        return KeymapLayerResult::Pass;
    };
    let max_offset = overlay.lines.len().saturating_sub(1);
    match key.code {
        KeyCode::Up => {
            overlay.scroll_offset = overlay.scroll_offset.saturating_sub(1);
        }
        KeyCode::Down => {
            overlay.scroll_offset = overlay.scroll_offset.saturating_add(1).min(max_offset);
        }
        KeyCode::PageUp => {
            overlay.scroll_offset = overlay.scroll_offset.saturating_sub(8);
        }
        KeyCode::PageDown => {
            overlay.scroll_offset = overlay.scroll_offset.saturating_add(8).min(max_offset);
        }
        _ => {}
    }
    KeymapLayerResult::Handled(TuiControllerCommand::Continue)
}

fn handle_command_palette_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            reduce_tui_state(state, TuiAction::OverlayPopped);
        }
        (_, KeyCode::Backspace) => {
            let mut query = command_palette_query(state).unwrap_or_default();
            query.pop();
            replace_command_palette_query(state, query);
        }
        (_, KeyCode::Delete) => {
            replace_command_palette_query(state, String::new());
        }
        (_, KeyCode::Up | KeyCode::BackTab) | (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
            move_command_palette_selection(state, PaletteMotion::Previous);
        }
        (_, KeyCode::Down | KeyCode::Tab) | (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
            move_command_palette_selection(state, PaletteMotion::Next);
        }
        (_, KeyCode::PageUp) => {
            move_command_palette_selection(state, PaletteMotion::PreviousPage);
        }
        (_, KeyCode::PageDown) => {
            move_command_palette_selection(state, PaletteMotion::NextPage);
        }
        (_, KeyCode::Enter) => {
            accept_command_palette_selection(state);
        }
        (_, KeyCode::Char(ch))
            if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
        {
            let mut query = command_palette_query(state).unwrap_or_default();
            query.push(ch);
            replace_command_palette_query(state, query);
        }
        _ => {}
    }
    KeymapLayerResult::Handled(TuiControllerCommand::Continue)
}

fn command_palette_query(state: &TuiState) -> Option<String> {
    let Some(UiOverlay::CommandPalette(overlay)) = state.overlays.active() else {
        return None;
    };
    Some(overlay.query.clone())
}

fn replace_command_palette_query(state: &mut TuiState, query: String) {
    let next = slash_palette_overlay(&state.command_registry, query.as_str());
    if let Some(UiOverlay::CommandPalette(overlay)) = state.overlays.stack.last_mut() {
        *overlay = next;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaletteMotion {
    Previous,
    Next,
    PreviousPage,
    NextPage,
}

fn move_command_palette_selection(state: &mut TuiState, motion: PaletteMotion) {
    let Some(UiOverlay::CommandPalette(overlay)) = state.overlays.stack.last_mut() else {
        return;
    };
    if overlay.items.is_empty() {
        overlay.selected_index = None;
        overlay.first_visible_index = 0;
        return;
    }

    let len = overlay.items.len();
    let current = overlay.selected_index.unwrap_or_default().min(len.saturating_sub(1));
    let page = overlay.visible_rows.max(1);
    let next = match motion {
        PaletteMotion::Previous => current.checked_sub(1).unwrap_or(len.saturating_sub(1)),
        PaletteMotion::Next => (current + 1) % len,
        PaletteMotion::PreviousPage => current.saturating_sub(page),
        PaletteMotion::NextPage => current.saturating_add(page).min(len.saturating_sub(1)),
    };
    overlay.selected_index = Some(next);
    if next < overlay.first_visible_index {
        overlay.first_visible_index = next;
    } else if next >= overlay.first_visible_index.saturating_add(page) {
        overlay.first_visible_index = next.saturating_add(1).saturating_sub(page);
    }
}

fn accept_command_palette_selection(state: &mut TuiState) {
    let selected = state.overlays.active().and_then(|overlay| {
        let UiOverlay::CommandPalette(palette) = overlay else {
            return None;
        };
        palette.selected_index.and_then(|index| palette.items.get(index)).cloned()
    });
    let Some(selected) = selected else {
        return;
    };

    if !selected.available {
        if let Some(UiOverlay::CommandPalette(overlay)) = state.overlays.stack.last_mut() {
            overlay.notice =
                selected.unavailable_reason.or_else(|| Some("该命令当前不可用".to_string()));
        }
        return;
    }

    reduce_tui_state(state, TuiAction::PromptValueSet(selected.replacement));
    reduce_tui_state(state, TuiAction::OverlayPopped);
}

fn handle_confirm_overlay_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    match key.code {
        KeyCode::Esc => {
            reduce_tui_state(state, TuiAction::OverlayPopped);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        KeyCode::Enter => {
            let Some(UiOverlay::Confirm(overlay)) = state.overlays.active().cloned() else {
                return KeymapLayerResult::Pass;
            };
            KeymapLayerResult::Handled(TuiControllerCommand::Overlay(
                TuiOverlayCommand::ConfirmAccepted(overlay),
            ))
        }
        _ => KeymapLayerResult::Handled(TuiControllerCommand::Continue),
    }
}

fn handle_error_overlay_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    match key.code {
        KeyCode::Esc => {
            reduce_tui_state(state, TuiAction::OverlayPopped);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        KeyCode::Enter => {
            if let Some(UiOverlay::Error(overlay)) = state.overlays.active()
                && overlay.recoverable
            {
                reduce_tui_state(state, TuiAction::OverlayPopped);
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        _ => KeymapLayerResult::Handled(TuiControllerCommand::Continue),
    }
}

fn handle_search_overlay_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            reduce_tui_state(state, TuiAction::OverlayPopped);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Backspace) => {
            backspace_search_query(state);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Delete) => {
            clear_search_query(state);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Up | KeyCode::BackTab) | (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
            move_search_selection(state, -1);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Down | KeyCode::Tab) | (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
            move_search_selection(state, 1);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Enter) => {
            if jump_to_selected_search_match(state) {
                reduce_tui_state(state, TuiAction::OverlayPopped);
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::End) => {
            scroll_to_tail(state);
            reduce_tui_state(state, TuiAction::OverlayPopped);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Char('u')) => {
            if jump_to_unread_message(state) {
                reduce_tui_state(state, TuiAction::OverlayPopped);
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
            toggle_search_case_sensitivity(state);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Char(ch))
            if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
        {
            append_search_query(state, ch);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        _ => KeymapLayerResult::Handled(TuiControllerCommand::Continue),
    }
}

fn search_overlay_query(state: &TuiState) -> Option<String> {
    let Some(UiOverlay::Search(overlay)) = state.overlays.active() else {
        return None;
    };
    Some(overlay.query.clone())
}

fn append_search_query(state: &mut TuiState, ch: char) {
    let mut query = search_overlay_query(state).unwrap_or_default();
    query.push(ch);
    reduce_tui_state(state, TuiAction::SearchQuerySet(query));
}

fn backspace_search_query(state: &mut TuiState) {
    let Some(mut query) = search_overlay_query(state) else {
        return;
    };
    query.pop();
    reduce_tui_state(state, TuiAction::SearchQuerySet(query));
}

fn clear_search_query(state: &mut TuiState) {
    if search_overlay_query(state).is_none() {
        return;
    }
    reduce_tui_state(state, TuiAction::SearchQuerySet(String::new()));
}

fn toggle_search_case_sensitivity(state: &mut TuiState) {
    let Some(query) = search_overlay_query(state) else {
        return;
    };

    if let Some(UiOverlay::Search(overlay)) = state.overlays.stack.last_mut() {
        overlay.case_sensitive = !overlay.case_sensitive;
    }

    reduce_tui_state(state, TuiAction::SearchQuerySet(query));
}

fn move_search_selection(state: &mut TuiState, delta: isize) {
    let Some(UiOverlay::Search(overlay)) = state.overlays.stack.last_mut() else {
        return;
    };

    if overlay.matches.is_empty() {
        overlay.selected_index = None;
        return;
    }

    let len = overlay.matches.len();
    let current = overlay.selected_index.unwrap_or_default();
    let next = if delta.is_negative() {
        if current == 0 {
            len.saturating_sub(1)
        } else {
            current.saturating_sub(delta.unsigned_abs())
        }
    } else {
        current.saturating_add(delta.cast_unsigned()) % len
    };
    overlay.selected_index = Some(next);
}

fn jump_to_selected_search_match(state: &mut TuiState) -> bool {
    let Some(UiOverlay::Search(overlay)) = state.overlays.active() else {
        return false;
    };
    let Some(selected_index) = overlay.selected_index else {
        return false;
    };
    let Some(message_id) =
        overlay.matches.get(selected_index).and_then(|item| item.message_id.as_ref())
    else {
        return false;
    };
    let Some(message_index) = state.messages.iter().position(|message| message.id() == message_id)
    else {
        return false;
    };

    jump_to_message(state, message_index)
}

fn jump_to_unread_message(state: &mut TuiState) -> bool {
    let Some(first_unread_message) = state
        .scroll
        .last_seen_message
        .map(|last_seen| last_seen.saturating_add(1))
        .filter(|index| *index < state.messages.len())
        .or_else(|| (!state.messages.is_empty()).then_some(0))
    else {
        return false;
    };

    jump_to_message(state, first_unread_message)
}

fn jump_to_message(state: &mut TuiState, message_index: usize) -> bool {
    let anchors = select_transcript_message_anchors(state);
    let Some(target_anchor) = anchors
        .iter()
        .copied()
        .rfind(|anchor| *anchor <= message_index)
        .or_else(|| anchors.first().copied())
    else {
        return false;
    };

    let mut scroll = state.scroll.clone();
    scroll.follow_tail = false;
    scroll.top_message = target_anchor;
    reduce_tui_state(state, TuiAction::ScrollSet(scroll));
    true
}

fn handle_question_overlay_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            reduce_tui_state(state, TuiAction::OverlayPopped);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (KeyModifiers::CONTROL, KeyCode::Char('r')) => {
            let Some(UiOverlay::Question(overlay)) = state.overlays.active().cloned() else {
                return KeymapLayerResult::Pass;
            };
            KeymapLayerResult::Handled(TuiControllerCommand::Overlay(
                TuiOverlayCommand::QuestionRejected(overlay),
            ))
        }
        (_, KeyCode::Enter) => {
            let Some(UiOverlay::Question(overlay)) = state.overlays.active().cloned() else {
                return KeymapLayerResult::Pass;
            };
            KeymapLayerResult::Handled(TuiControllerCommand::Overlay(
                TuiOverlayCommand::QuestionSubmitted(overlay),
            ))
        }
        (_, KeyCode::Tab | KeyCode::Down) | (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
            if let Some(UiOverlay::Question(overlay)) = state.overlays.stack.last_mut() {
                move_question_prompt_selection(overlay, 1);
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::BackTab | KeyCode::Up) | (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
            if let Some(UiOverlay::Question(overlay)) = state.overlays.stack.last_mut() {
                move_question_prompt_selection(overlay, -1);
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Backspace) => {
            if let Some(UiOverlay::Question(overlay)) = state.overlays.stack.last_mut() {
                backspace_question_custom_answer(overlay);
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Delete) => {
            if let Some(UiOverlay::Question(overlay)) = state.overlays.stack.last_mut() {
                clear_question_custom_answer(overlay);
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Char(ch)) if ch.is_ascii_digit() && ch != '0' => {
            if let Some(UiOverlay::Question(overlay)) = state.overlays.stack.last_mut() {
                toggle_question_option_by_digit(overlay, ch);
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Char(ch))
            if question_overlay_allows_custom_input(state)
                && matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
        {
            if let Some(UiOverlay::Question(overlay)) = state.overlays.stack.last_mut() {
                append_question_custom_answer(overlay, ch);
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        _ => KeymapLayerResult::Handled(TuiControllerCommand::Continue),
    }
}

fn question_overlay_allows_custom_input(state: &TuiState) -> bool {
    let Some(UiOverlay::Question(overlay)) = state.overlays.active() else {
        return false;
    };
    overlay.prompts.get(overlay.selected_index).is_some_and(|prompt| prompt.allow_custom_input)
}

fn move_question_prompt_selection(overlay: &mut UiQuestionOverlay, delta: isize) {
    if overlay.prompts.is_empty() {
        overlay.selected_index = 0;
        return;
    }

    let max_index = overlay.prompts.len().saturating_sub(1);
    overlay.selected_index = if delta.is_negative() {
        overlay.selected_index.saturating_sub(delta.unsigned_abs())
    } else {
        overlay.selected_index.saturating_add(delta.cast_unsigned()).min(max_index)
    };
}

fn toggle_question_option_by_digit(overlay: &mut UiQuestionOverlay, digit: char) {
    let Some(prompt) = overlay.prompts.get(overlay.selected_index) else {
        return;
    };
    let Some(option_index) = digit
        .to_digit(10)
        .map(|value| usize::try_from(value.saturating_sub(1)).unwrap_or_default())
    else {
        return;
    };
    let Some(option) = prompt.options.get(option_index) else {
        return;
    };

    let answers = &mut overlay.answers[overlay.selected_index];
    let selected_label = option.label.clone();

    if prompt.multiple {
        if let Some(index) = answers.iter().position(|answer| answer == &selected_label) {
            answers.remove(index);
        } else {
            answers.push(selected_label);
        }
        return;
    }

    answers.retain(|answer| !prompt.options.iter().any(|candidate| candidate.label == *answer));
    answers.push(selected_label);
}

fn append_question_custom_answer(overlay: &mut UiQuestionOverlay, ch: char) {
    let answers = &mut overlay.answers[overlay.selected_index];
    let mut custom = question_custom_answer(answers);
    custom.push(ch);
    set_question_custom_answer(answers, custom);
}

fn backspace_question_custom_answer(overlay: &mut UiQuestionOverlay) {
    let answers = &mut overlay.answers[overlay.selected_index];
    let mut custom = question_custom_answer(answers);
    custom.pop();
    set_question_custom_answer(answers, custom);
}

fn clear_question_custom_answer(overlay: &mut UiQuestionOverlay) {
    let answers = &mut overlay.answers[overlay.selected_index];
    set_question_custom_answer(answers, String::new());
}

fn question_custom_answer(answers: &[String]) -> String {
    answers
        .iter()
        .find_map(|answer| answer.strip_prefix(QUESTION_CUSTOM_ANSWER_PREFIX))
        .map(ToOwned::to_owned)
        .unwrap_or_default()
}

fn set_question_custom_answer(answers: &mut Vec<String>, value: String) {
    answers.retain(|answer| !answer.starts_with(QUESTION_CUSTOM_ANSWER_PREFIX));
    if !value.is_empty() {
        answers.push(format!("{QUESTION_CUSTOM_ANSWER_PREFIX}{value}"));
    }
}

fn handle_todo_overlay_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            reduce_tui_state(state, TuiAction::OverlayPopped);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Up) | (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
            if let Some(UiOverlay::Todo(overlay)) = state.overlays.stack.last_mut() {
                move_todo_selection(overlay, -1);
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Down) | (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
            if let Some(UiOverlay::Todo(overlay)) = state.overlays.stack.last_mut() {
                move_todo_selection(overlay, 1);
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Char(' ') | KeyCode::Enter) => {
            if let Some(UiOverlay::Todo(overlay)) = state.overlays.stack.last_mut() {
                toggle_selected_todo_item(overlay);
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Char('r')) => {
            let Some(UiOverlay::Todo(overlay)) = state.overlays.active().cloned() else {
                return KeymapLayerResult::Pass;
            };
            KeymapLayerResult::Handled(TuiControllerCommand::Overlay(
                TuiOverlayCommand::TodoRefresh(overlay),
            ))
        }
        (_, KeyCode::Char('s')) => {
            let Some(UiOverlay::Todo(overlay)) = state.overlays.active().cloned() else {
                return KeymapLayerResult::Pass;
            };
            KeymapLayerResult::Handled(TuiControllerCommand::Overlay(TuiOverlayCommand::TodoSave(
                overlay,
            )))
        }
        _ => KeymapLayerResult::Handled(TuiControllerCommand::Continue),
    }
}

fn move_todo_selection(overlay: &mut UiTodoOverlay, delta: isize) {
    if overlay.items.is_empty() {
        overlay.selected_index = 0;
        return;
    }

    let max_index = overlay.items.len().saturating_sub(1);
    overlay.selected_index = if delta.is_negative() {
        overlay.selected_index.saturating_sub(delta.unsigned_abs())
    } else {
        overlay.selected_index.saturating_add(delta.cast_unsigned()).min(max_index)
    };
}

fn toggle_selected_todo_item(overlay: &mut UiTodoOverlay) {
    let Some(item) = overlay.items.get_mut(overlay.selected_index) else {
        return;
    };

    item.status = if item.status.eq_ignore_ascii_case("completed") {
        "pending".to_string()
    } else {
        "completed".to_string()
    };
    overlay.dirty = true;
}

fn handle_task_overlay_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            reduce_tui_state(state, TuiAction::OverlayPopped);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Up | KeyCode::BackTab) | (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
            if let Some(UiOverlay::Task(overlay)) = state.overlays.stack.last_mut() {
                move_task_selection(overlay, -1);
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Down | KeyCode::Tab) | (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
            if let Some(UiOverlay::Task(overlay)) = state.overlays.stack.last_mut() {
                move_task_selection(overlay, 1);
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Enter) => {
            if jump_to_selected_task_step(state) {
                reduce_tui_state(state, TuiAction::OverlayPopped);
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        _ => KeymapLayerResult::Handled(TuiControllerCommand::Continue),
    }
}

fn move_task_selection(overlay: &mut UiTaskOverlay, delta: isize) {
    if overlay.steps.is_empty() {
        overlay.selected_index = 0;
        return;
    }

    let len = overlay.steps.len();
    let current = overlay.selected_index.min(len.saturating_sub(1));
    overlay.selected_index = if delta.is_negative() {
        if current == 0 {
            len.saturating_sub(1)
        } else {
            current.saturating_sub(delta.unsigned_abs())
        }
    } else {
        current.saturating_add(delta.cast_unsigned()) % len
    };
}

fn jump_to_selected_task_step(state: &mut TuiState) -> bool {
    let Some(UiOverlay::Task(overlay)) = state.overlays.active() else {
        return false;
    };
    let Some(message_id) = overlay.steps.get(overlay.selected_index).map(|step| &step.message_id)
    else {
        return false;
    };
    let Some(message_index) = state.messages.iter().position(|message| message.id() == message_id)
    else {
        return false;
    };

    jump_to_message(state, message_index)
}

fn handle_mcp_overlay_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    match key.code {
        KeyCode::Esc => {
            reduce_tui_state(state, TuiAction::OverlayPopped);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        KeyCode::Up | KeyCode::BackTab => {
            if let Some(UiOverlay::Mcp(overlay)) = state.overlays.stack.last_mut() {
                let len = overlay.servers.len();
                if len > 0 {
                    overlay.selected_index = (overlay.selected_index + len.saturating_sub(1)) % len;
                }
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        KeyCode::Down | KeyCode::Tab => {
            if let Some(UiOverlay::Mcp(overlay)) = state.overlays.stack.last_mut() {
                let len = overlay.servers.len();
                if len > 0 {
                    overlay.selected_index = (overlay.selected_index + 1) % len;
                }
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        _ => KeymapLayerResult::Handled(TuiControllerCommand::Continue),
    }
}

fn handle_memory_overlay_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    match key.code {
        KeyCode::Esc => {
            reduce_tui_state(state, TuiAction::OverlayPopped);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        KeyCode::Up | KeyCode::BackTab => {
            if let Some(UiOverlay::Memory(overlay)) = state.overlays.stack.last_mut() {
                let len = overlay.entries.len();
                if len > 0 {
                    overlay.selected_index = (overlay.selected_index + len.saturating_sub(1)) % len;
                }
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        KeyCode::Down | KeyCode::Tab => {
            if let Some(UiOverlay::Memory(overlay)) = state.overlays.stack.last_mut() {
                let len = overlay.entries.len();
                if len > 0 {
                    overlay.selected_index = (overlay.selected_index + 1) % len;
                }
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        _ => KeymapLayerResult::Handled(TuiControllerCommand::Continue),
    }
}
