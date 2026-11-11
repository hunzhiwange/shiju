//! Versioned template editing and handoff to the existing native chat channel.
use super::*;
pub(super) use crate::apps::workflow::state::video_edit::TEMPLATE_ID;
use vw_gateway_client::{WorkflowRecord, vw_api_types::TenantScope};

pub(super) fn load_template(graph: bool) -> Task<Message> {
    let tenant = crate::app::session_gateway::active_tenant_scope();
    let client = crate::app::config::gateway_client();
    Task::perform(
        async move {
            let result = async { client?.workflow_application_get(TEMPLATE_ID).await }.await;
            (tenant, result)
        },
        move |(tenant, result)| {
            Message::WorkflowTool(WorkflowMessage::VideoTemplateLoaded { tenant, graph, result })
        },
    )
}

fn persist(
    tenant: TenantScope,
    record: WorkflowRecord,
    yaml: String,
    graph: bool,
    restore: bool,
) -> Task<Message> {
    let client = crate::app::config::gateway_client();
    Task::perform(
        async move {
            let result = async {
                let client = client?;
                if restore {
                    client.workflow_template_restore(TEMPLATE_ID, record.updated_at_ms, true).await
                } else {
                    client
                        .workflow_template_update(
                            TEMPLATE_ID,
                            &vw_gateway_client::WorkflowRecordUpsertBody {
                                uuid: Some(TEMPLATE_ID.into()),
                                name: record.name,
                                description: record.description,
                                workflow_yaml: yaml,
                            },
                            record.updated_at_ms,
                        )
                        .await
                }
            }
            .await;
            (tenant, result)
        },
        move |(tenant, result)| {
            Message::WorkflowTool(WorkflowMessage::VideoTemplatePersisted {
                tenant,
                graph,
                restore,
                result,
            })
        },
    )
}

pub(super) fn save_graph(
    app: &mut App,
    entry: super::super::state::WorkflowAppEntry,
) -> Task<Message> {
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
    persist(crate::app::session_gateway::active_tenant_scope(), record, yaml, true, false)
}

pub(super) fn handle(app: &mut App, message: WorkflowMessage) -> Option<Task<Message>> {
    Some(match message {
        WorkflowMessage::ScopedSavedAppsLoaded { tenant, result } => {
            if tenant == crate::app::session_gateway::active_tenant_scope() {
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
            load_template(false)
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
                load_template(false)
            }
        }
        WorkflowMessage::VideoTemplateLoaded { tenant, graph, result } => {
            if tenant != crate::app::session_gateway::active_tenant_scope() {
                return Some(Task::none());
            }
            app.workflow_state.video_edit.busy = false;
            match result {
                Ok(record) => {
                    if graph && app.workflow_state.select_app_by_local_uuid(TEMPLATE_ID) {
                        crate::apps::workflow::sync_top_tab(app);
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
        WorkflowMessage::CloseVideoTemplate => {
            app.workflow_state.video_edit.open = false;
            app.workflow_state.video_edit.restore_confirm = false;
            Task::none()
        }
        WorkflowMessage::VideoTemplateYamlAction(action) => {
            if !app.workflow_state.video_edit.busy
                && let Some(editor) = &mut app.workflow_state.video_edit.editor
            {
                editor.perform(action);
            }
            Task::none()
        }
        WorkflowMessage::VideoTemplatePathChanged(path) => {
            app.workflow_state.video_edit.path = path;
            Task::none()
        }
        #[cfg(target_arch = "wasm32")]
        WorkflowMessage::PickVideoTemplatePath => {
            app.show_warning_toast("浏览器无法读取本机路径，请填写网关服务器的视频路径")
        }
        #[cfg(not(target_arch = "wasm32"))]
        WorkflowMessage::PickVideoTemplatePath => {
            let tenant = crate::app::session_gateway::active_tenant_scope();
            Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .add_filter("视频", &["mp4", "mov", "mkv", "webm"])
                        .pick_file()
                        .await
                        .map(|f| f.path().to_string_lossy().into_owned())
                },
                move |path| {
                    Message::WorkflowTool(WorkflowMessage::VideoTemplatePathPicked {
                        tenant: tenant.clone(),
                        path,
                    })
                },
            )
        }
        WorkflowMessage::VideoTemplatePathPicked { tenant, path } => {
            if tenant == crate::app::session_gateway::active_tenant_scope()
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
            persist(
                crate::app::session_gateway::active_tenant_scope(),
                record,
                yaml,
                false,
                restore,
            )
        }
        WorkflowMessage::RequestVideoTemplateRestore => {
            app.workflow_state.video_edit.restore_confirm = true;
            Task::none()
        }
        WorkflowMessage::CancelVideoTemplateRestore => {
            app.workflow_state.video_edit.restore_confirm = false;
            Task::none()
        }
        WorkflowMessage::VideoTemplatePersisted { tenant, graph, restore, result } => {
            if tenant != crate::app::session_gateway::active_tenant_scope() {
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
            let command = match app.workflow_state.video_edit.native_command() {
                Ok(command) => command,
                Err(e) => return Some(app.show_error_toast(e)),
            };
            if app.project_path.is_none() {
                return Some(app.show_error_toast("请先打开原生项目，再从模板发起剪辑"));
            }
            if app.acp_agent.is_some() || app.current_session_runtime().acp_agent.is_some() {
                return Some(app.show_error_toast("请先切换原生会话，剪辑不支持 ACP"));
            }
            let runtime = app.current_session_runtime();
            if runtime.task_mode_enabled || runtime.workflow_mode_enabled || !app.files.is_empty() {
                return Some(
                    app.show_error_toast("请使用普通原生聊天模式，并先处理已有附件后发起剪辑"),
                );
            }
            if !app.current_session_runtime().input_editor.text().trim().is_empty() {
                return Some(app.show_error_toast("会话中已有未发送草稿，请先处理后再启动剪辑"));
            }
            app.workflow_state.video_edit.open = false;
            app.screen = crate::app::Screen::Project;
            app.active_tab_id = app.active_session_id.clone();
            app.current_session_runtime_mut().input_editor =
                text_editor::Content::with_text(&command);
            app.input_editor = text_editor::Content::with_text(&command);
            crate::app::message::chat::update(app, crate::app::message::ChatMessage::SendPressed)
        }
        _ => return None,
    })
}
