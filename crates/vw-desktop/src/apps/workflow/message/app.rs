//! 工作流应用级消息处理，负责应用元数据编辑和工作流文件生命周期操作。

use super::*;

/// 构建或更新 handle 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub(super) fn handle(app: &mut App, message: WorkflowMessage) -> Option<Task<Message>> {
    Some(match message {
        WorkflowMessage::LoadSavedApps => {
            app.workflow_state.begin_saved_apps_load();
            load_saved_apps_task()
        }
        WorkflowMessage::LoadSavedAppsFinished(result) => {
            app.workflow_state.finish_saved_apps_load(result);
            Task::none()
        }
        WorkflowMessage::OpenSavedApp(uuid) => {
            if uuid == super::video_edit::TEMPLATE_ID {
                if app.workflow_state.select_app_by_local_uuid(&uuid) {
                    crate::apps::workflow::sync_top_tab(app);
                    return Some(Task::none());
                }
                return Some(super::video_edit::load_template(true));
            }
            if app.workflow_state.select_app_by_local_uuid(&uuid) {
                crate::apps::workflow::sync_top_tab(app);
                return Some(Task::none());
            }

            app.workflow_state.begin_saved_app_open(uuid.clone());
            open_saved_app_task(uuid)
        }
        WorkflowMessage::OpenSavedAppFinished(result) => {
            app.workflow_state.finish_saved_app_open();
            match result {
                Ok(loaded) => apply_loaded(app, loaded),
                Err(error) => app.workflow_state.set_error(error),
            }
            Task::none()
        }
        WorkflowMessage::ShowSavedApps => {
            app.workflow_state.show_saved_apps();
            crate::apps::workflow::sync_top_tab(app);

            if !app.workflow_state.saved_apps_loaded && !app.workflow_state.saved_apps_loading {
                app.workflow_state.begin_saved_apps_load();
                load_saved_apps_task()
            } else {
                Task::none()
            }
        }
        WorkflowMessage::SavedAppSearchChanged(query) => {
            app.workflow_state.set_saved_app_search_query(query);
            Task::none()
        }
        WorkflowMessage::ToggleSavedAppActions(uuid) => {
            app.workflow_state.toggle_saved_app_actions(uuid);
            Task::none()
        }
        WorkflowMessage::CloseSavedAppActions => {
            app.workflow_state.close_saved_app_actions();
            Task::none()
        }
        WorkflowMessage::CopySavedAppUuid(uuid) => {
            app.workflow_state.mark_saved_app_uuid_copied(uuid.clone());
            Task::batch(vec![
                iced::clipboard::write(uuid.clone()),
                crate::app::message::after(
                    std::time::Duration::from_secs(2),
                    Message::WorkflowTool(WorkflowMessage::ClearCopiedSavedAppUuid(uuid)),
                ),
            ])
        }
        WorkflowMessage::ClearCopiedSavedAppUuid(uuid) => {
            app.workflow_state.clear_saved_app_uuid_copied(&uuid);
            Task::none()
        }
        WorkflowMessage::RequestDeleteSavedApp(uuid) => {
            if uuid == super::video_edit::TEMPLATE_ID {
                return Some(app.show_error_toast("内置剪辑模板不能删除，请使用恢复默认"));
            }
            app.workflow_state.open_saved_app_delete_confirm(uuid);
            Task::none()
        }
        WorkflowMessage::CancelDeleteSavedApp => {
            app.workflow_state.close_saved_app_delete_confirm();
            Task::none()
        }
        WorkflowMessage::DeleteSavedApp(uuid) => {
            if uuid == super::video_edit::TEMPLATE_ID {
                return Some(app.show_error_toast("内置剪辑模板不能删除"));
            }
            app.workflow_state.begin_saved_app_delete(uuid.clone());
            delete_saved_app_task(uuid)
        }
        WorkflowMessage::DeleteSavedAppFinished(result) => {
            app.workflow_state.finish_saved_app_delete();
            match result {
                Ok(uuid) => {
                    let removed_active = app.workflow_state.remove_saved_app(&uuid);
                    app.workflow_state
                        .set_status(crate::app::i18n::format::Message::new("已删除应用", vec![]));
                    if removed_active {
                        crate::apps::workflow::sync_top_tab(app);
                    }
                }
                Err(error) => app.workflow_state.set_error(error),
            }
            Task::none()
        }
        WorkflowMessage::OpenInlineYaml { workflow_yaml, focus_node_id } => {
            app.workflow_state.close_floating_panels();
            #[cfg(not(target_arch = "wasm32"))]
            {
                match load_document_from_text(None, workflow_yaml) {
                    Ok(loaded) => {
                        apply_loaded(app, loaded);
                        if let Some(node_id) =
                            focus_node_id.as_deref().filter(|value| !value.trim().is_empty())
                            && let Err(error) =
                                app.workflow_state.focus_node(node_id, app.window_size)
                        {
                            app.workflow_state.set_error(error);
                        }
                        app.workflow_state.set_status(crate::app::i18n::format::Message::new(
                            "已打开临时工作流预览",
                            vec![],
                        ));
                    }
                    Err(error) => app.workflow_state.set_error(error),
                }
            }
            #[cfg(target_arch = "wasm32")]
            {
                let _ = (workflow_yaml, focus_node_id);
                app.workflow_state.set_error("Web 平台暂不支持预览内联 Workflow YAML");
            }
            Task::none()
        }
        WorkflowMessage::OpenFile => {
            app.workflow_state.close_floating_panels();
            open_file()
        }
        WorkflowMessage::OpenFileFinished(result) => {
            match result {
                Ok(Some(loaded)) => apply_loaded(app, loaded),
                Ok(None) => {}
                Err(error) => return Some(app.show_error_toast(error)),
            }
            Task::none()
        }
        WorkflowMessage::SelectApp(id) => {
            if app.workflow_state.select_app(&id) {
                crate::apps::workflow::sync_top_tab(app);
            }
            Task::none()
        }
        WorkflowMessage::OpenCreateAppEditor => {
            app.workflow_state.open_create_editor();
            Task::none()
        }
        WorkflowMessage::OpenEditAppEditor(id) => {
            app.workflow_state.open_edit_editor(id.as_deref());
            Task::none()
        }
        WorkflowMessage::CloseAppEditor => {
            app.workflow_state.close_editor();
            Task::none()
        }
        WorkflowMessage::AppEditorNameChanged(value) => {
            app.workflow_state.set_editor_name(value);
            Task::none()
        }
        WorkflowMessage::AppEditorDescriptionChanged(value) => {
            app.workflow_state.set_editor_description(value);
            Task::none()
        }
        WorkflowMessage::AppEditorIconChanged(value) => {
            app.workflow_state.set_editor_icon(value);
            Task::none()
        }
        WorkflowMessage::AppEditorUseIconAsAnswerIconChanged(value) => {
            app.workflow_state.set_editor_use_icon_as_answer_icon(value);
            Task::none()
        }
        WorkflowMessage::AppEditorMaxActiveRequestsChanged(value) => {
            app.workflow_state.set_editor_max_active_requests_input(value);
            Task::none()
        }
        WorkflowMessage::OrganizeActiveApp => {
            if let Err(error) = app.workflow_state.organize_active_app(app.window_size) {
                app.workflow_state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::SubmitAppEditor => {
            let mode = app.workflow_state.app_editor.as_ref().map(|draft| draft.mode.clone());
            let Some(mode) = mode else {
                return Some(Task::none());
            };

            let template = match mode {
                WorkflowAppEditorMode::Create | WorkflowAppEditorMode::Edit(_) => {
                    match create_blank_workflow(WorkflowAppMeta::default()) {
                        Ok(loaded) => loaded,
                        Err(error) => {
                            app.workflow_state.set_error(error);
                            return Some(Task::none());
                        }
                    }
                }
            };

            if let Err(error) = app.workflow_state.submit_editor(app.window_size, template) {
                app.workflow_state.set_error(error);
            } else {
                crate::apps::workflow::sync_top_tab(app);
            }
            Task::none()
        }
        _ => return None,
    })
}
