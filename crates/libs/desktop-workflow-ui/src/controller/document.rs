//! 工作流文档消息处理，负责文档导入、预览和内容同步。

use super::*;

/// 构建或更新 handle 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub(crate) fn handle(app: &mut Context<'_>, message: WorkflowMessage) -> Option<Task<Output>> {
    if crate::message::document::handle(&mut app.workflow_state, app.window_size, message.clone()) {
        return Some(Task::none());
    }
    Some(match message {
        WorkflowMessage::SaveActiveApp => {
            app.workflow_state.close_floating_panels();
            save_active_app(app, false)
        }
        WorkflowMessage::SaveActiveAppAs => {
            app.workflow_state.close_floating_panels();
            save_active_app(app, true)
        }
        WorkflowMessage::SaveActiveAppFinished(result) => match result {
            Ok(Some(WorkflowSaveTarget::LocalUuid(uuid))) => {
                app.workflow_state.update_active_local_uuid(uuid.clone());
                app.workflow_state.set_status(desktop_i18n::format::Message::new(
                    "已保存到本地数据库: %s",
                    vec![uuid.to_string().into()],
                ));
                app.workflow_state.begin_saved_apps_load();
                load_saved_apps_task(app)
            }
            Ok(Some(WorkflowSaveTarget::FilePath(path))) => {
                app.workflow_state.update_active_source_path(path.clone());
                app.workflow_state.set_status(desktop_i18n::format::Message::new(
                    "已另存为 %s",
                    vec![path.to_string().into()],
                ));
                Task::none()
            }
            Ok(None) => Task::none(),
            Err(error) => {
                app.workflow_state.set_error(error.clone());
                Task::done(Output::Effect(Effect::SaveError(error)))
            }
        },
        WorkflowMessage::ExportPng => export_png(app),
        WorkflowMessage::ExportJpeg => export_jpeg(app),
        WorkflowMessage::ExportSvg => export_svg(app),
        WorkflowMessage::ExportFinished(result) => {
            export_finished(app, result);
            Task::none()
        }
        WorkflowMessage::Reload => {
            app.workflow_state.close_floating_panels();
            let result =
                if let Some(path) = app.workflow_state.source_path.as_deref().map(str::to_owned) {
                    load_document_from_path(&path)
                } else if let Some(entry) = app.workflow_state.active_entry_snapshot() {
                    load_document_from_value(None, entry.raw_root)
                } else {
                    Err(desktop_i18n::workflow::Feedback::new(
                        "当前没有可重新载入的 Workflow 应用",
                        vec![],
                    ))
                };

            match result {
                Ok(loaded) => {
                    app.workflow_state.replace_active_loaded(loaded, app.window_size);
                    sync_top_tab(app);
                }
                Err(error) => app.workflow_state.set_error(error),
            }

            Task::none()
        }
        _ => return None,
    })
}
