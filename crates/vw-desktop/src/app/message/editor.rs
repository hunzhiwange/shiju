//! 编辑器消息处理模块
//!
//! 本模块负责处理代码编辑器的所有交互消息，包括：
//! - 编辑器设置（字体、字号、行高、语言、主题）
//! - 搜索替换功能
//! - 剪贴板操作（复制、剪切、粘贴、删除）
//! - 撤销/重做操作
//!
//! 已提交的编辑器设置会异步保存到网关。

use crate::app::{App, Message};
use iced::Task;

pub(crate) mod metrics;
pub(crate) mod persistence;

use persistence::EditorSettingsPatch;

pub use desktop_editor::components::preview_panel::view::options::{EditorOption, EditorPopover};

pub use desktop_editor::message::editor::EditorMessage;

/// 处理编辑器消息并更新应用状态
///
/// 该函数是编辑器消息的核心处理器，根据不同的消息类型执行相应的操作：
/// - 更新编辑器配置（字体、字号、行高、主题等）
/// - 执行编辑操作（撤销、重做、复制、剪切、粘贴等）
/// - 管理搜索对话框状态
///
/// # 参数
///
/// * `app` - 应用状态的可变引用，包含所有编辑器相关配置
/// * `message` - 待处理的编辑器消息
///
/// # 返回值
///
/// 返回一个 `Task<Message>`，可能包含后续需要执行的异步任务：
/// - 搜索对话框操作返回编辑器事件映射任务
/// - 粘贴操作返回剪贴板读取任务
/// - 其他操作通常返回 `Task::none()`
pub fn update(app: &mut App, message: EditorMessage) -> Task<Message> {
    let read_only = app.active_preview_path.as_ref().is_some_and(|path|
        app.preview_tabs.iter().any(|tab| tab.path == *path && tab.read_only));
    if !desktop_editor::message::editor::prepare_message(
        app.shutdown_requested, read_only, &mut app.editor_popover, &message,
    ) { return Task::none(); }
    match message {

        #[cfg(not(target_arch = "wasm32"))]
        EditorMessage::CopyLspCommand(server) => iced::clipboard::write(server.manual_command()),
        #[cfg(not(target_arch = "wasm32"))]
        EditorMessage::InstallLsp(server) => Task::batch([
            app.show_info_toast("正在安装 LSP，可能需要数分钟。"),
            Task::perform(crate::app::lsp::install::install(server), |result| {
                Message::Editor(EditorMessage::LspInstalled(result))
            }),
        ]),
        #[cfg(not(target_arch = "wasm32"))]
        EditorMessage::LspInstalled(result) => match result {
            Ok(()) => app.show_success_toast(
                "LSP 安装完成。请重启 ShiJu 后重新打开文件；若未生效，请检查 PATH。",
            ),
            Err(error) => app.show_error_toast(error),
        },
        EditorMessage::TogglePopover(kind) => {
            let has_file = app
                .active_preview_path
                .as_ref()
                .is_some_and(|path| app.preview_tabs.iter().any(|tab| tab.path == *path));
            desktop_editor::message::editor::toggle_popover(&mut app.editor_popover, has_file, kind);
            app.show_preview_context_menu = false;
            app.preview_nav_popup = None;
            app.show_preview_settings = false;
            #[cfg(not(target_arch = "wasm32"))]
            {
                app.lsp_overlay.hover_visible = false;
                app.lsp_overlay.completion_visible = false;
                app.lsp_overlay_path = None;
            }
            if let Some(path) = app.active_preview_path.as_ref()
                && let Some(tab) = app.preview_tabs.iter_mut().find(|tab| tab.path == *path)
            {
                return tab
                    .editor
                    .inner
                    .update(&iced_code_editor::Message::CloseCommandPalette)
                    .map(|event| {
                        Message::Preview(crate::app::message::PreviewMessage::EditorEvent(event))
                    });
            }
            Task::none()
        }
        EditorMessage::ToggleSettings => {
            app.show_preview_settings = !app.show_preview_settings;
            if app.show_preview_settings { Task::none() } else { metrics::commit(app, None) }
        }

        EditorMessage::FontSizePreview(size) => {
            app.editor_metrics_preview.font_size = Some(size);
            let generation = app.editor_metrics_preview.schedule_commit();
            crate::app::message::after(
                std::time::Duration::from_millis(200),
                Message::Editor(EditorMessage::CommitMetricsDue(generation)),
            )
        }
        EditorMessage::LineHeightPreview(height) => {
            app.editor_metrics_preview.line_height = Some(height);
            let generation = app.editor_metrics_preview.schedule_commit();
            crate::app::message::after(
                std::time::Duration::from_millis(200),
                Message::Editor(EditorMessage::CommitMetricsDue(generation)),
            )
        }
        EditorMessage::CommitMetrics => metrics::commit(app, None),
        EditorMessage::CommitMetricsDue(generation) => {
            if app.editor_metrics_preview.generation == generation {
                metrics::commit(app, None)
            } else {
                Task::none()
            }
        }
        EditorMessage::ToggleAutoLineHeight(auto) => metrics::commit(app, Some(auto)),

        // 编辑器界面语言变更：更新所有编辑器实例的语言设置
        EditorMessage::LanguageChanged(lang) => {
            app.current_language = lang;

            desktop_editor::message::editor::apply_language(
                app.preview_tabs.iter_mut().map(|tab| &mut tab.editor),
                &mut app.git_copy_modal_code_editor, lang,
            );
            Task::none()
        }

        // 编辑器主题变更：更新所有编辑器实例并持久化配置
        EditorMessage::ThemeChanged(theme) => {
            // 更新当前主题设置，并禁用跟随系统主题
            app.editor_theme = theme.clone();
            app.editor_follow_system_theme = false;

            desktop_editor::message::editor::apply_theme(
                app.preview_tabs.iter_mut().map(|tab| &mut tab.editor),
                &mut app.git_copy_modal_code_editor, theme.clone(),
            );

            persistence::enqueue(app, EditorSettingsPatch::theme(theme.to_string()))
        }

        // 切换是否跟随系统主题
        EditorMessage::ToggleFollowSystemTheme(follow) => {
            app.editor_follow_system_theme = follow;

            // 获取有效主题（考虑系统主题设置）
            let editor_theme = app.effective_editor_theme();

            desktop_editor::message::editor::apply_theme(
                app.preview_tabs.iter_mut().map(|tab| &mut tab.editor),
                &mut app.git_copy_modal_code_editor, editor_theme.clone(),
            );

            persistence::enqueue(app, EditorSettingsPatch::follow_system_theme(follow))
        }

        EditorMessage::ConfigSaved(result) => persistence::save_complete(app, result),

        // 撤销操作（仅限当前激活的预览标签页）
        message => {
            let editor = app.preview_tabs.iter_mut()
                .find(|tab| Some(&tab.path) == app.active_preview_path.as_ref())
                .map(|tab| &mut tab.editor);
            desktop_editor::message::editor::update(
                editor, &mut app.editor_popover, &mut app.show_preview_settings, message,
            ).map(|event| match event {
                desktop_editor::message::editor::Event::Preview(event) =>
                    Message::Preview(crate::app::message::PreviewMessage::EditorEvent(event)),
                desktop_editor::message::editor::Event::Message(message) => Message::Editor(message),
            })
        }
    }
}
