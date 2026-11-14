//! Host-scoped WebDAV draft; late replies cannot restore cleared credentials.
use super::data_management::export_ui::{self, Identity};
use super::device_sync_preview::{PreviewMessage, PreviewState};
use crate::app::components::system_settings_device_sync::task_panel::{
    TaskPanelMessage, TaskPanelState,
};
use crate::app::{App, Message};
use iced::Task;
use vw_gateway_client::{
    DeviceSyncSelection,
    vw_api_types::{ProjectScope, agent_apps::DataManagementTarget, device_sync_config::*},
};

#[derive(Clone)]
pub(crate) enum Event {
    Open,
    Close,
    Load,
    BaseUrl(String),
    Username(String),
    Password(String),
    Root(String),
    Profile(String),
    Principal(String),
    Selection(DeviceSyncSelection),
    Enabled(bool),
    Auto(bool),
    ConfirmAuto(bool),
    Clear,
    ConfirmClear(bool),
    Save,
    Test,
    Loaded(u64, Result<WebDavSyncConfigResponse, String>),
    Saved(u64, Result<WebDavSyncConfigResponse, String>),
    Tested(u64, Result<(), String>),
    Cleared(u64, Result<(), String>),
    Preview(PreviewMessage),
    TaskPanel(TaskPanelMessage),
}
#[derive(Clone)]
pub(crate) struct State {
    pub open: bool,
    pub busy: bool,
    pub dirty: bool,
    pub secret_touched: bool,
    pub secret_present: bool,
    pub draft: UpdateWebDavSyncConfigRequest,
    pub principal: String,
    pub selection: DeviceSyncSelection,
    pub auto_confirmation: bool,
    pub clear_confirmation: bool,
    pub preview: PreviewState,
    pub task_panel: TaskPanelState,
    identity: Option<Identity>,
    generation: u64,
}
impl Default for State {
    fn default() -> Self {
        Self {
            open: false,
            busy: false,
            dirty: false,
            secret_touched: false,
            secret_present: false,
            draft: UpdateWebDavSyncConfigRequest {
                enabled: true,
                auto_sync: false,
                base_url: String::new(),
                username: String::new(),
                remote_root: "shiju".into(),
                profile: "default".into(),
                secret: SyncSecretUpdate::Keep {},
            },
            principal: String::new(),
            selection: DeviceSyncSelection::default(),
            auto_confirmation: false,
            clear_confirmation: false,
            preview: Default::default(),
            task_panel: Default::default(),
            identity: None,
            generation: 0,
        }
    }
}
pub(crate) fn message(e: Event) -> Message {
    Message::Settings(super::SettingsMessage::DeviceSyncWebdav(e))
}
pub(crate) fn visible(app: &App) -> bool {
    let s = &app.apps_catalog.device_sync_webdav;
    s.open && s.identity.as_ref() == export_ui::identity(app).as_ref() && s.identity.is_some()
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Some(identity) = export_ui::identity(app) else {
        app.apps_catalog.device_sync_webdav = State::default();
        return Task::none();
    };
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none() };
    let token = app.apps_catalog.token.clone();
    let s = &mut app.apps_catalog.device_sync_webdav;
    if s.identity.as_ref() != Some(&identity) {
        let generation = s.generation.wrapping_add(1);
        *s = State { identity: Some(identity.clone()), generation, ..Default::default() };
        if !matches!(event, Event::Open) {
            return Task::none();
        }
    }
    if matches!(event, Event::Open) {
        s.open = true;
        return Task::done(message(Event::Load));
    }
    if !s.open {
        return Task::none();
    }
    let project: vw_gateway_client::vw_api_types::ProjectId = identity.project.clone().into();
    let host = identity.host.clone();
    let mut toast = None;
    match event {
        Event::Close if !s.busy => {
            s.open = false;
            s.preview.invalidate();
        }
        Event::Load if !s.busy => {
            s.busy = true;
            s.generation = s.generation.wrapping_add(1);
            let g = s.generation;
            return Task::perform(
                async move {
                    client
                        .device_sync_webdav_config(&project, &host, &token)
                        .await
                        .map_err(|_| "读取WebDAV配置失败".into())
                },
                move |r| message(Event::Loaded(g, r)),
            );
        }
        Event::Loaded(g, r) | Event::Saved(g, r) if g == s.generation && s.busy => {
            s.busy = false;
            match r {
                Ok(r) => {
                    if let Some(c) = r.config {
                        s.draft = UpdateWebDavSyncConfigRequest {
                            enabled: c.enabled,
                            auto_sync: c.auto_sync,
                            base_url: c.base_url,
                            username: c.username,
                            remote_root: c.remote_root,
                            profile: c.profile,
                            secret: SyncSecretUpdate::Keep {},
                        };
                        s.secret_present = c.secret_present;
                    } else {
                        s.draft = State::default().draft;
                        s.secret_present = false;
                    }
                    s.secret_touched = false;
                    s.dirty = false;
                    s.preview.invalidate();
                    let Ok(scope) = ProjectScope::new(identity.context.tenant.clone(), project)
                    else {
                        return Task::none();
                    };
                    let target = DataManagementTarget {
                        scope,
                        target_host_id: host,
                        principal_id: s.principal.clone(),
                    };
                    let (task, error) = s.task_panel.update(
                        TaskPanelMessage::Discover {
                            last_status: r.last_status,
                            active_task_id: r.active_task_id,
                        },
                        client,
                        target,
                        token,
                    );
                    if let Some(error) = error {
                        toast = Some(error)
                    } else {
                        return task.map(|m| message(Event::TaskPanel(m)));
                    }
                }
                Err(e) => toast = Some(e),
            }
        }
        Event::Save if !s.busy && !s.auto_confirmation => {
            s.busy = true;
            s.generation = s.generation.wrapping_add(1);
            let g = s.generation;
            let mut draft = s.draft.clone();
            if !s.secret_touched {
                draft.secret = SyncSecretUpdate::Keep {}
            };
            return Task::perform(
                async move {
                    let r = client
                        .device_sync_save_webdav_config(&project, &host, &draft, &token)
                        .await
                        .map_err(|_| "保存WebDAV配置失败".to_owned())?;
                    let mut tested = draft;
                    tested.secret = SyncSecretUpdate::Keep {};
                    client
                        .device_sync_test_webdav(&project, &host, &tested, &token)
                        .await
                        .map_err(|_| "配置已保存，连接测试失败，请重新读取后重试".to_owned())?;
                    Ok(r)
                },
                move |r| message(Event::Saved(g, r)),
            );
        }
        Event::Test if !s.busy => {
            s.busy = true;
            let g = s.generation;
            let draft = s.draft.clone();
            return Task::perform(
                async move {
                    client
                        .device_sync_test_webdav(&project, &host, &draft, &token)
                        .await
                        .map_err(|_| "WebDAV连接测试失败".into())
                },
                move |r| message(Event::Tested(g, r)),
            );
        }
        Event::Tested(g, r) if g == s.generation && s.busy => {
            s.busy = false;
            toast = Some(if r.is_ok() {
                "连接测试成功".into()
            } else {
                "连接测试失败".into()
            });
        }
        Event::Clear if !s.busy => s.clear_confirmation = true,
        Event::ConfirmClear(yes) if !s.busy => {
            s.clear_confirmation = false;
            if yes {
                s.generation = s.generation.wrapping_add(1);
                s.preview.invalidate();
                s.task_panel.reset();
                s.busy = true;
                let g = s.generation;
                return Task::perform(
                    async move {
                        client
                            .device_sync_clear_webdav_config(&project, &host, &token)
                            .await
                            .map_err(|_| "清除WebDAV配置失败".into())
                    },
                    move |r| message(Event::Cleared(g, r)),
                );
            }
        }
        Event::Cleared(g, r) if g == s.generation && s.busy => {
            s.busy = false;
            match r {
                Ok(()) => {
                    s.draft = State::default().draft;
                    s.secret_touched = false;
                    s.secret_present = false;
                    s.dirty = false;
                }
                Err(e) => toast = Some(e),
            }
        }
        Event::Preview(e) if !s.busy && !s.task_panel.is_busy() => {
            let Ok(scope) = ProjectScope::new(identity.context.tenant.clone(), project) else {
                return Task::none();
            };
            let target = DataManagementTarget {
                scope,
                target_host_id: host,
                principal_id: s.principal.clone(),
            };
            let selection = s.selection.clone();
            let (task, error, status) = s.preview.update(
                e,
                client.clone(),
                target.clone(),
                token.clone(),
                DeviceSyncTransport::Webdav,
                selection,
                s.dirty,
            );
            let mut tasks = vec![task.map(|m| message(Event::Preview(m)))];
            if let Some(status) = status {
                let (t, _) = s.task_panel.update(
                    TaskPanelMessage::Discover {
                        active_task_id: Some(status.task_id.clone()),
                        last_status: Some(status),
                    },
                    client,
                    target,
                    token,
                );
                tasks.push(t.map(|m| message(Event::TaskPanel(m))))
            }
            if let Some(e) = error {
                tasks.push(app.show_error_toast(e))
            }
            return Task::batch(tasks);
        }
        Event::TaskPanel(TaskPanelMessage::Refresh) if !s.busy => {
            return Task::done(message(Event::Load));
        }
        Event::TaskPanel(e) => {
            let Ok(scope) = ProjectScope::new(identity.context.tenant.clone(), project) else {
                return Task::none();
            };
            let target = DataManagementTarget {
                scope,
                target_host_id: host,
                principal_id: s.principal.clone(),
            };
            let (t, error) = s.task_panel.update(e, client, target, token);
            let mut tasks = vec![t.map(|m| message(Event::TaskPanel(m)))];
            if let Some(e) = error {
                tasks.push(app.show_error_toast(e))
            }
            return Task::batch(tasks);
        }
        e if !s.busy => {
            let changed = match e {
                Event::Selection(v) => {
                    s.selection = v;
                    s.preview.invalidate();
                    return Task::none();
                }
                Event::BaseUrl(v) => {
                    s.draft.base_url = v;
                    true
                }
                Event::Username(v) => {
                    s.draft.username = v;
                    true
                }
                Event::Password(v) => {
                    s.draft.secret = SyncSecretUpdate::Replace { value: v };
                    s.secret_touched = true;
                    true
                }
                Event::Root(v) => {
                    s.draft.remote_root = v;
                    true
                }
                Event::Profile(v) => {
                    s.draft.profile = v;
                    true
                }
                Event::Principal(v) => {
                    s.principal = v;
                    s.task_panel.reset();
                    true
                }
                Event::Enabled(v) => {
                    s.draft.enabled = v;
                    true
                }
                Event::Auto(true) if !s.draft.auto_sync => {
                    s.auto_confirmation = true;
                    false
                }
                Event::Auto(false) => {
                    s.draft.auto_sync = false;
                    true
                }
                Event::ConfirmAuto(v) => {
                    s.auto_confirmation = false;
                    if v {
                        s.draft.auto_sync = true;
                    }
                    v
                }
                _ => false,
            };
            if changed {
                s.dirty = true;
                s.preview.invalidate();
                s.generation = s.generation.wrapping_add(1);
            }
        }
        _ => {}
    }
    if let Some(e) = toast { app.show_error_toast(e) } else { Task::none() }
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WebDavEvent { [redacted] }")
    }
}
