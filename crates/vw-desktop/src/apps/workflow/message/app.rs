//! 工作流应用级消息处理，负责应用元数据编辑和工作流文件生命周期操作。

use super::*;
use vw_api_workflow::is_builtin_workflow_uuid;

/// 构建或更新 handle 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub(super) fn handle(app: &mut App, message: WorkflowMessage) -> Option<Task<Message>> {
    if desktop_workflow_ui::message::app::handle(
        &mut app.workflow_state,
        app.window_size,
        message.clone(),
    ) {
        return Some(Task::none());
    }
    Some(match message {
        WorkflowMessage::LoadSavedApps => {
            app.workflow_state.begin_saved_apps_load();
            load_saved_apps_task()
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
        WorkflowMessage::RequestDeleteSavedApp(uuid) => {
            if is_builtin_workflow_uuid(&uuid) {
                return Some(app.show_error_toast("内置工作流不可删除"));
            }
            app.workflow_state.open_saved_app_delete_confirm(uuid);
            Task::none()
        }
        WorkflowMessage::DeleteSavedApp(uuid) => {
            if is_builtin_workflow_uuid(&uuid) {
                return Some(app.show_error_toast("内置工作流不可删除"));
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
        WorkflowMessage::SubmitAppEditor => {
            match desktop_workflow_ui::message::app::submit_editor(
                &mut app.workflow_state,
                app.window_size,
            ) {
                Ok(true) => crate::apps::workflow::sync_top_tab(app),
                Ok(false) => {}
                Err(error) => app.workflow_state.set_error(error),
            }
            Task::none()
        }
        _ => return None,
    })
}
