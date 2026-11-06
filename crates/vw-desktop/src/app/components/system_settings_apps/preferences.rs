//! Saved values are replaced only by the gateway response.
use vw_gateway_client::vw_api_types::agent_apps::*;

#[derive(Clone, Debug, Default)]
pub(crate) struct PreferencesState {
    pub saved: Option<AppPreferencesResult>,
    pub draft: Option<AppPreferencesResult>,
    pub open: bool,
    pub busy: bool,
    pub generation: u64,
    pub error: Option<String>,
}
impl PreferencesState {
    pub fn finish(
        &mut self,
        generation: u64,
        result: Result<AppPreferencesResult, String>,
    ) -> bool {
        if !self.busy || self.generation != generation {
            return false;
        }
        self.busy = false;
        match result {
            Ok(value) => {
                self.saved = Some(value.clone());
                self.draft = Some(value);
                self.error = None;
            }
            Err(error) => {
                self.draft = self.saved.clone();
                self.error = Some(error);
            }
        }
        true
    }
}

#[derive(Clone, Debug)]
pub enum PreferencesMessage {
    Open,
    Load,
    Close,
    Visible(AppId, bool),
    MoveUp(String),
    Terminal(String),
    Shell(AppId, String),
    Flag(AppId, String),
    SaveUser,
    SaveHost,
    Reply {
        context: crate::app::state::AgentAppsContext,
        host: String,
        catalog_generation: u64,
        generation: u64,
        result: Result<AppPreferencesResult, String>,
    },
}

impl PreferencesState {
    pub fn invalidate(&mut self) {
        *self = Self { generation: self.generation.wrapping_add(1), ..Default::default() };
    }
    pub fn visible(&mut self, id: AppId, visible: bool) {
        if self.busy {
            return;
        }
        let Some(draft) = &mut self.draft else {
            return;
        };
        if visible && !draft.user.visible_apps.contains(&id) {
            draft.user.visible_apps.push(id);
        } else if !visible && draft.user.visible_apps.contains(&id) {
            if draft.user.visible_apps.len() == 1 {
                self.error = Some("至少保留一项可见应用".into());
                return;
            }
            draft.user.visible_apps.retain(|item| *item != id);
        }
        self.error = None;
    }
    pub fn wsl_edit(&mut self, id: AppId, value: String, flag: bool) {
        if !self.host_editable() || id == AppId::ClaudeDesktop {
            return;
        }
        let draft = self.draft.as_mut().unwrap();
        if !draft.capabilities.as_ref().is_some_and(|c| c.wsl_preferences) {
            return;
        }
        let at = draft.host.wsl_shells.iter().position(|p| p.app_id == id).unwrap_or_else(|| {
            draft.host.wsl_shells.push(AppWslPreference { app_id: id, shell: None, flag: None });
            draft.host.wsl_shells.len() - 1
        });
        let entry = &mut draft.host.wsl_shells[at];
        let value = (!value.is_empty()).then_some(value);
        if flag {
            entry.flag = value;
        } else {
            entry.shell = value;
        }
    }
    pub fn host_editable(&self) -> bool {
        !self.busy && self.draft.as_ref().and_then(|v| v.capabilities.as_ref()).is_some()
    }
}

fn event(value: PreferencesMessage) -> crate::app::Message {
    super::message(crate::app::message::settings::AppsCatalogMessage::Preferences(value))
}
fn current_context() -> Option<crate::app::state::AgentAppsContext> {
    crate::app::config::gateway_client().ok().map(|client| crate::app::state::AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: None,
    })
}

pub(crate) fn update(
    app: &mut crate::app::App,
    message: PreferencesMessage,
) -> iced::Task<crate::app::Message> {
    use iced::Task;
    let mut current = current_context();
    if let Some(context) = &mut current {
        context.project = app.project_path.clone();
    }
    if let PreferencesMessage::Reply { context, host, catalog_generation, generation, result } =
        message
    {
        if current.as_ref() != Some(&context)
            || app.apps_catalog.context.as_ref() != Some(&context)
            || app.apps_catalog.host != host
            || app.apps_catalog.generation != catalog_generation
        {
            return Task::none();
        }
        let error = result.as_ref().err().cloned();
        if app.apps_catalog.preferences.finish(generation, result) {
            if let Some(error) = error {
                return app.show_error_toast(error);
            }
            return app.show_success_toast("应用设置已由网关确认");
        }
        return Task::none();
    }
    if app.apps_catalog.context.as_ref() != current.as_ref() || current.is_none() {
        app.apps_catalog.preferences.invalidate();
        return app.show_error_toast("连接上下文已更改，请重新加载应用");
    }
    let state = &mut app.apps_catalog.preferences;
    if state.busy {
        return Task::none();
    }
    let patch = match message {
        PreferencesMessage::Load => None,
        PreferencesMessage::Open => {
            state.open = true;
            None
        }
        PreferencesMessage::Close => {
            state.open = false;
            state.draft = state.saved.clone();
            return Task::none();
        }
        PreferencesMessage::Visible(id, visible) => {
            state.visible(id, visible);
            return Task::none();
        }
        PreferencesMessage::MoveUp(id) => {
            if let Some(draft) = &mut state.draft
                && let Some(at) = draft.user.app_order.iter().position(|item| *item == id)
                && at > 0
            {
                draft.user.app_order.swap(at - 1, at);
            }
            return Task::none();
        }
        PreferencesMessage::Terminal(terminal) => {
            if state.host_editable()
                && let Some(draft) = &mut state.draft
                && (terminal.is_empty()
                    || draft.capabilities.as_ref().is_some_and(|c| c.terminals.contains(&terminal)))
            {
                draft.host.preferred_terminal = (!terminal.is_empty()).then_some(terminal);
            }
            return Task::none();
        }
        PreferencesMessage::Shell(id, value) => {
            state.wsl_edit(id, value, false);
            return Task::none();
        }
        PreferencesMessage::Flag(id, value) => {
            state.wsl_edit(id, value, true);
            return Task::none();
        }
        PreferencesMessage::SaveUser => {
            let Some(draft) = &state.draft else {
                return Task::none();
            };
            if draft.user.visible_apps.is_empty() {
                return app.show_error_toast("至少保留一项可见应用");
            }
            Some(AppPreferencesPatch::User(draft.user.clone()))
        }
        PreferencesMessage::SaveHost => {
            if !state.host_editable() {
                return Task::none();
            }
            Some(AppPreferencesPatch::Host(state.draft.as_ref().unwrap().host.clone()))
        }
        PreferencesMessage::Reply { .. } => unreachable!(),
    };
    let Ok(client) = crate::app::config::gateway_client() else {
        return app.show_error_toast("网关不可用");
    };
    let context = current.unwrap();
    let host = app.apps_catalog.host.clone();
    let token = app.apps_catalog.token.clone();
    if host.is_empty() || token.is_empty() {
        return app.show_error_toast("请先选择已授权执行主机");
    }
    state.busy = true;
    state.error = None;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    let catalog_generation = app.apps_catalog.generation;
    Task::perform(
        async move {
            let result = match patch {
                Some(patch) => {
                    client.agent_apps_preferences_update_authorized(&host, &patch, &token).await
                }
                None => client.agent_apps_preferences_authorized(&host, &token).await,
            }
            .map_err(|error| {
                match error {
                    vw_gateway_client::AgentAppsSettingsError::Http {
                        status: 401 | 403, ..
                    } => "未授权，设置未保存",
                    vw_gateway_client::AgentAppsSettingsError::Http { status: 422, .. } => {
                        "设置被执行主机拒绝，已恢复原值"
                    }
                    _ => "未确认保存结果，已恢复原值；请重新打开设置读取后再保存",
                }
                .to_string()
            });
            (host, result)
        },
        move |(host, result)| {
            event(PreferencesMessage::Reply {
                context: context.clone(),
                host,
                catalog_generation,
                generation,
                result,
            })
        },
    )
}

#[path = "preferences_view.rs"]
mod view;
pub(crate) use view::overlay;

/// Project the server's saved order and visibility; unsaved draft never changes the list.
pub(crate) fn visible_rows(
    state: &crate::app::state::AppsCatalogState,
) -> Vec<&super::catalog::CatalogRow> {
    let mut rows: Vec<_> = state.rows.iter().collect();
    if let Some(saved) = &state.preferences.saved {
        rows.retain(|app| saved.user.visible_apps.contains(&app.entry.id));
        rows.sort_by_key(|app| {
            let id =
                serde_json::to_value(app.entry.id).ok().and_then(|v| v.as_str().map(str::to_owned));
            saved
                .user
                .app_order
                .iter()
                .position(|entry| Some(entry) == id.as_ref())
                .unwrap_or(usize::MAX)
        });
    }
    rows
}
