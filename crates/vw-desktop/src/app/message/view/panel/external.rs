//! 处理视图面板的外部打开、弹出层与使用量展示交互。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::ViewMessage;
use crate::app::{App, Message, set_config_field, state::ExternalOpenApp};
use iced::Task;

/// 执行 update 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::OpenPathInFinder(path) => Task::perform(
            async move {
                let result =
                    crate::app::session_gateway::gateway_external_reveal_async(&path).await;
                ViewMessage::PathRevealFinished { path, result }
            },
            Message::View,
        ),
        ViewMessage::PathRevealFinished { path, result } => match result {
            Ok(()) => Task::none(),
            Err(error) => app.show_localized_toast(
                crate::app::state::ToastKind::Error,
                "无法在文件管理器中显示 %s：%s",
                vec![path.into(), error.into()],
            ),
        },
        ViewMessage::OpenProjectInExternalPreferred => {
            app.active_menu = None;
            let Some(path) = app.project_path.as_deref().map(str::to_owned) else {
                return Task::none();
            };
            let target = app.open_external_app;
            if !can_open_external(app, target) {
                return Task::none();
            }
            Task::perform(
                async move {
                    let _ = crate::app::session_gateway::gateway_external_open_async(
                        &path,
                        target.as_str(),
                    )
                    .await;
                },
                |_| Message::None,
            )
        }
        ViewMessage::OpenProjectInExternalWith(target) => {
            app.active_menu = None;
            app.open_external_app = target;
            set_config_field(
                "open_external_app",
                serde_json::Value::String(target.as_str().to_string()),
            );
            let Some(path) = app.project_path.as_deref().map(str::to_owned) else {
                return Task::none();
            };
            if !can_open_external(app, target) {
                return Task::none();
            }
            Task::perform(
                async move {
                    let _ = crate::app::session_gateway::gateway_external_open_async(
                        &path,
                        target.as_str(),
                    )
                    .await;
                },
                |_| Message::None,
            )
        }
        ViewMessage::CopyProjectPath => {
            app.active_menu = None;
            let Some(path) = app.project_path.as_deref().map(str::to_owned) else {
                return Task::none();
            };
            let path = std::path::Path::new(&path)
                .canonicalize()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or(path);
            iced::clipboard::write(path)
        }
        _ => Task::none(),
    }
}

fn can_open_external(app: &App, target: ExternalOpenApp) -> bool {
    app.open_external_exists.get(&target).copied().unwrap_or(false)
}
#[cfg(test)]
#[path = "external_tests.rs"]
mod external_tests;
