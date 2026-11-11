//! 处理视图面板的外部打开、弹出层与使用量展示交互。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::ViewMessage;
use crate::app::message::PreviewMessage;
use crate::app::{App, Message, state::ExternalOpenApp};
use iced::Task;
use vw_gateway_client::vw_api_types::file::FileNodeKind;

/// 执行 update 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::OpenChatPath(path) => Task::perform(
            async move {
                let result = match crate::app::gateway_client() {
                    Ok(client) => client.desktop_external_path_kind(&path).await,
                    Err(error) => Err(error),
                };
                ViewMessage::ChatPathKindResolved { path, result }
            },
            Message::View,
        ),
        ViewMessage::ChatPathKindResolved { path, result } => match result {
            Ok(FileNodeKind::File) => Task::done(Message::Preview(PreviewMessage::Open(path))),
            Ok(FileNodeKind::Directory) => {
                // Preserve literal percent signs when the reveal endpoint decodes the URL.
                #[cfg(target_arch = "wasm32")]
                {
                    // The gateway decodes once; escape literal percent signs as well.
                    Task::done(Message::View(ViewMessage::OpenPathInFinder(
                        urlencoding::encode(&path).into_owned(),
                    )))
                }
                #[cfg(not(target_arch = "wasm32"))]
                match reqwest::Url::from_file_path(&path) {
                    Ok(url) => Task::done(Message::View(ViewMessage::OpenPathInFinder(url.into()))),
                    Err(()) => app.show_localized_toast(
                        crate::app::state::ToastKind::Error,
                        "无法确定本地路径的绝对位置",
                        vec![],
                    ),
                }
            }
            Err(error) => app.show_localized_toast(
                crate::app::state::ToastKind::Error,
                "无法打开路径 %s：%s",
                vec![path.into(), error.into()],
            ),
        },
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
            let save = app.save_desktop_preference("open_external_app", target.as_str().into());
            let Some(path) = app.project_path.as_deref().map(str::to_owned) else {
                return save;
            };
            if !can_open_external(app, target) {
                return save;
            }
            let open = Task::perform(
                async move {
                    let _ = crate::app::session_gateway::gateway_external_open_async(
                        &path,
                        target.as_str(),
                    )
                    .await;
                },
                |_| Message::None,
            );
            Task::batch([save, open])
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
