//! Versioned template editing and handoff to the existing native chat channel.
use super::*;
pub(crate) use desktop_workflow_state::state::video_edit::TEMPLATE_ID;
use {vw_api_transport::TenantScope, vw_api_workflow::WorkflowRecord};

pub(crate) fn load_template(app: &Context<'_>, graph: bool) -> Task<Output> {
    crate::message::video_edit::load_template(app.tenant.clone(), (app.gateway)(), graph)
        .map(Output::Workflow)
}
fn persist(
    app: &Context<'_>,
    tenant: TenantScope,
    record: WorkflowRecord,
    yaml: String,
    graph: bool,
    restore: bool,
) -> Task<Output> {
    crate::message::video_edit::persist((app.gateway)(), tenant, record, yaml, graph, restore)
        .map(Output::Workflow)
}

pub(crate) fn save_graph(
    app: &mut Context<'_>,
    entry: desktop_workflow_state::state::WorkflowAppEntry,
) -> Task<Output> {
    let Some(mut record) = app.workflow_state.video_edit.record.clone() else {
        return app.show_error_toast("请重新打开模板后保存，缺少读取版本");
    };
    let Some(version) = app.workflow_state.video_edit.graph_version else {
        return app.show_error_toast("缺少图编辑器版本，请重新打开模板");
    };
    record.updated_at_ms = version;
    let yaml = match serialize_workflow_yaml(
        &entry.meta,
        &entry.document,
        &entry.environment_variables,
        &entry.conversation_variables,
        &entry.raw_root,
        WorkflowViewport { x: entry.pan.x, y: entry.pan.y, zoom: entry.zoom },
    ) {
        Ok(yaml) => yaml,
        Err(e) => return app.show_error_toast(e),
    };
    persist(app, app.tenant.clone(), record, yaml, true, false)
}

pub(crate) fn handle(app: &mut Context<'_>, message: WorkflowMessage) -> Option<Task<Output>> {
    if crate::message::video_edit::handle(&mut app.workflow_state, app.window_size, message.clone())
    {
        return Some(Task::none());
    }
    Some(match message {
        WorkflowMessage::ScopedSavedAppsLoaded { tenant, result } => {
            if tenant == app.tenant.clone() {
                app.workflow_state.finish_saved_apps_load(result);
            }
            Task::none()
        }
        WorkflowMessage::ReloadVideoTemplate => {
            let state = &mut app.workflow_state.video_edit;
            if state.busy {
                return Some(Task::none());
            }
            state.editor = None;
            state.record = None;
            state.error = None;
            state.busy = true;
            load_template(app, false)
        }

        WorkflowMessage::OpenVideoTemplate => {
            if app.workflow_state.local_uuid.as_deref() == Some(TEMPLATE_ID)
                && app.workflow_state.active_is_dirty
            {
                return Some(app.show_error_toast("请先保存画布草稿，再打开完整模板 YAML"));
            }
            app.workflow_state.video_edit.open = true;
            if app.workflow_state.video_edit.record.is_some() {
                Task::none()
            } else {
                app.workflow_state.video_edit.busy = true;
                load_template(app, false)
            }
        }
        WorkflowMessage::VideoTemplateLoaded { tenant, graph, result } => {
            if tenant != app.tenant.clone() {
                return Some(Task::none());
            }
            app.workflow_state.video_edit.busy = false;
            match result {
                Ok(record) => {
                    if graph && app.workflow_state.select_app_by_local_uuid(TEMPLATE_ID) {
                        sync_top_tab(app);
                    } else if graph {
                        match load_document_from_text(None, record.workflow_yaml.clone()) {
                            Ok(mut loaded) => {
                                loaded.local_uuid = Some(TEMPLATE_ID.into());
                                app.workflow_state.video_edit.graph_version =
                                    Some(record.updated_at_ms);
                                apply_loaded(app, loaded);
                            }
                            Err(e) => return Some(app.show_error_toast(e)),
                        }
                    }
                    app.workflow_state.video_edit.accept_record(record);
                }
                Err(e) => app.workflow_state.video_edit.error = Some(e),
            }
            Task::none()
        }
        #[cfg(target_arch = "wasm32")]
        WorkflowMessage::PickVideoTemplatePath => {
            app.show_warning_toast("浏览器无法读取本机路径，请填写网关服务器的视频路径")
        }
        #[cfg(not(target_arch = "wasm32"))]
        WorkflowMessage::PickVideoTemplatePath => {
            let tenant = app.tenant.clone();
            Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .add_filter("视频", &["mp4", "mov", "mkv", "webm"])
                        .pick_file()
                        .await
                        .map(|f| f.path().to_string_lossy().into_owned())
                },
                move |path| {
                    Output::Workflow(WorkflowMessage::VideoTemplatePathPicked {
                        tenant: tenant.clone(),
                        path,
                    })
                },
            )
        }
        WorkflowMessage::VideoTemplatePathPicked { tenant, path } => {
            if tenant == app.tenant.clone()
                && let Some(path) = path
            {
                app.workflow_state.video_edit.path = path;
            }
            Task::none()
        }
        WorkflowMessage::SaveVideoTemplate | WorkflowMessage::ConfirmVideoTemplateRestore => {
            let restore = matches!(message, WorkflowMessage::ConfirmVideoTemplateRestore);
            let state = &mut app.workflow_state.video_edit;
            if state.busy || (restore && !state.restore_confirm) {
                return Some(Task::none());
            }
            let Some(record) = state.record.clone() else {
                return Some(Task::none());
            };
            let yaml = match state.validated_yaml() {
                Ok(yaml) => yaml,
                Err(e) if !restore => {
                    state.error = Some(e);
                    return Some(Task::none());
                }
                Err(_) => String::new(),
            };
            state.submitted = Some(yaml.clone());
            state.busy = true;
            state.error = None;
            persist(app, app.tenant.clone(), record, yaml, false, restore)
        }
        WorkflowMessage::VideoTemplatePersisted { tenant, graph, restore, result } => {
            if tenant != app.tenant.clone() {
                return Some(Task::none());
            }
            let state = &mut app.workflow_state.video_edit;
            state.busy = false;
            match result {
                Ok(record) => {
                    let version = record.updated_at_ms;
                    let yaml = record.workflow_yaml.clone();
                    if graph {
                        state.graph_version = Some(version);
                        state.accept_record(record);
                    } else {
                        if restore || state.editor.as_ref().map(|e| e.text()) == state.submitted {
                            state.editor = Some(text_editor::Content::with_text(&yaml));
                        }
                        state.record = Some(record);
                    }
                    state.restore_confirm = false;
                    state.error = None;
                    if app.workflow_state.local_uuid.as_deref() == Some(TEMPLATE_ID) {
                        if graph {
                            let current =
                                app.workflow_state.active_entry_snapshot().and_then(|entry| {
                                    serialize_workflow_yaml(
                                        &entry.meta,
                                        &entry.document,
                                        &entry.environment_variables,
                                        &entry.conversation_variables,
                                        &entry.raw_root,
                                        WorkflowViewport {
                                            x: entry.pan.x,
                                            y: entry.pan.y,
                                            zoom: entry.zoom,
                                        },
                                    )
                                    .ok()
                                });
                            if current.as_deref() == Some(yaml.as_str()) {
                                app.workflow_state.update_active_local_uuid(TEMPLATE_ID.into());
                            }
                        } else if !app.workflow_state.active_is_dirty
                            && let Ok(mut loaded) = load_document_from_text(None, yaml)
                        {
                            loaded.local_uuid = Some(TEMPLATE_ID.into());
                            app.workflow_state.replace_active_loaded(loaded, app.window_size);
                            app.workflow_state.video_edit.graph_version = Some(version);
                        }
                    } else if !graph {
                        app.workflow_state.apps.retain(|entry| {
                            entry.local_uuid.as_deref() != Some(TEMPLATE_ID) || entry.is_dirty
                        });
                    }
                    app.show_success_toast("模板已保存，下一次原生剪辑将读取此版本")
                }
                Err(e) => {
                    state.error = Some(e.clone());
                    app.show_error_toast(e)
                }
            }
        }
        WorkflowMessage::StartNativeVideoEdit => {
            Task::done(Output::Effect(Effect::StartNativeVideoEdit))
        }
        _ => return None,
    })
}
