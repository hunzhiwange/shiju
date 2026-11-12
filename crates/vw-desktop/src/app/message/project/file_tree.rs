//! 文件树操作消息处理模块
//!
//! 本模块负责处理与项目文件树相关的所有用户交互操作，包括：
//! - 右键菜单的显示与关闭
//! - 文件/文件夹的重命名操作
//! - 拖拽操作的开始与结束
//! - 文件树上下文菜单的各种操作（打开、删除、复制、粘贴等）
//!
//! # 主要功能
//!
//! - **右键菜单管理**：处理文件树项的右键点击事件，控制菜单的显示位置与关闭
//! - **重命名功能**：支持文件和文件夹的重命名，包含名称合法性校验和冲突处理
//! - **拖拽支持**：处理文件/文件夹的拖拽开始和结束事件，支持移动操作
//! - **上下文菜单操作**：
//!   - 打开文件/在文件管理器中显示
//!   - 在终端中打开
//!   - 添加到聊天
//!   - 在文件夹中查找
//!   - 剪切、复制、原地复制、粘贴
//!   - 复制路径（绝对路径和相对路径）
//!   - 重命名和删除
//!
//! # 平台兼容性
//!
//! 部分功能（如重命名、在终端打开、在文件管理器显示）仅在非 WASM 目标平台可用。

use crate::app::message::project::helpers::{
    gateway_copy_path, gateway_delete_path, gateway_duplicate_path, gateway_move_path, now_ms,
    refresh_file_index, relative_to_project,
};
use crate::app::message::project::{FileTreeAction, ProjectMessage};
use crate::app::{
    App, AppTab, Message, Screen,
    state::{FileTreeClipboard, FileTreeClipboardMode, FindInFolderTab},
};
#[path = "file_tree_batch.rs"]
mod batch;
#[path = "file_tree_external_drop.rs"]
mod external_drop;
#[path = "file_tree_move_drop.rs"]
mod move_drop;

use iced::Point;
use std::collections::HashSet;

fn is_directory_path(path: &str, files: &[String]) -> bool {
    let prefix = format!("{path}/");
    if files.iter().any(|item| item.starts_with(&prefix)) {
        return true;
    }
    !files.iter().any(|item| item == path)
}

fn remove_local_file_tree_path(files: &mut Vec<String>, path: &str, recursive: bool) {
    let descendant_prefix = format!("{path}/");
    files.retain(|item| item != path && (!recursive || !item.starts_with(&descendant_prefix)));
}

#[cfg(test)]
#[path = "file_tree_tests.rs"]
mod file_tree_tests;

fn unique_name_for_target(
    dst_dir: &std::path::Path,
    name: &str,
    existing: &HashSet<String>,
) -> String {
    let original = dst_dir.join(name).to_string_lossy().to_string();
    if !existing.contains(&original) {
        return original;
    }
    let path = std::path::Path::new(name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = path.extension().and_then(|s| s.to_str());
    for i in 1..=999 {
        let suffix = if i == 1 { "copy".to_string() } else { format!("copy {}", i) };
        let candidate_name = if let Some(ext) = ext {
            format!("{stem} {suffix}.{ext}")
        } else {
            format!("{stem} {suffix}")
        };
        let candidate = dst_dir.join(candidate_name).to_string_lossy().to_string();
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
    original
}

fn replace_path_prefix(target: &str, old_path: &str, new_path: &str) -> Option<String> {
    if target == old_path {
        return Some(new_path.to_string());
    }
    let old_prefix = format!("{old_path}/");
    target.strip_prefix(&old_prefix).map(|rest| format!("{new_path}/{}", rest))
}

// 生成文件树节点的显示名称。

fn path_display_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}
///
/// 该函数是文件树消息的中央处理器，根据不同的消息类型执行相应的操作，
/// 并返回可选的 Iced Task 用于执行异步命令或 UI 更新。
///
/// # 参数
///
/// - `app`: 可变引用到应用状态，用于读取和修改文件树相关的状态
/// - `message`: 要处理的项目消息，包含具体的操作类型和参数
///
/// # 返回值
///
/// 返回 `Option<iced::Task<Message>>`：
/// - `Some(Task)`：包含需要执行的 Iced 任务（如刷新文件索引、剪贴板操作等）
/// - `None`：表示该消息不在此处理器中处理，应交由其他处理器
///
/// # 处理的消息类型
///
/// - `FileTreeRightClicked`：右键点击文件树项，显示上下文菜单
/// - `FileTreeMenuClose`：关闭文件树上下文菜单
/// - `FileTreeRenameChanged`：重命名输入框内容变化
/// - `FileTreeRenameCancel`：取消重命名操作
/// - `FileTreeDragStart`：开始拖拽文件/文件夹
/// - `FileTreeDragEnd`：结束拖拽操作，准备放置
/// - `FileTreeRenameSave`：保存重命名结果
/// - `FileTreeAction`：执行文件树上下文菜单操作
///
/// # 示例
///
/// ```ignore
/// // 在消息处理循环中调用
/// if let Some(task) = handle(&mut app, message) {
///     return Some(task);
/// }
/// ```
pub(crate) fn handle(app: &mut App, message: ProjectMessage) -> Option<iced::Task<Message>> {
    match message {
        ProjectMessage::FileTreeSelected { project_path, path, directory, modifiers } => {
            if app.project_path.as_deref() != Some(&project_path) {
                return Some(iced::Task::none());
            }
            let visible =
                crate::app::components::file_tree::selection::visible_paths(app, &project_path);
            if !visible.contains(&path) {
                return Some(iced::Task::none());
            }
            app.file_tree_rename_path = None;
            app.file_tree_rename_value.clear();
            app.file_tree_selection.select(
                &project_path,
                path.clone(),
                modifiers.command(),
                modifiers.shift(),
                &visible,
            );
            if modifiers.command() || modifiers.shift() {
                return Some(iced::Task::none());
            }
            if directory {
                let Some(relative) = relative_to_project(&project_path, &path) else {
                    return Some(iced::Task::none());
                };
                return super::basic::handle(app, ProjectMessage::ToggleTreeDir(relative));
            }
            Some(crate::app::message::preview::update(
                app,
                crate::app::message::PreviewMessage::Open(path),
            ))
        }
        // 处理右键点击事件：记录点击位置、源和坐标，用于显示上下文菜单
        ProjectMessage::FileTreeRightClicked(path, source, x, y) => {
            if matches!(source.as_str(), "tree-file" | "tree-dir")
                && let Some(root) = app.project_path.clone()
                && !app.file_tree_selection.contains(&root, &path)
            {
                let visible =
                    crate::app::components::file_tree::selection::visible_paths(app, &root);
                app.file_tree_selection.select(&root, path.clone(), false, false, &visible);
            }
            // 保存被右键点击的文件路径
            app.file_tree_menu_path = Some(path);
            // 保存点击来源（文件树还是其他位置）
            app.file_tree_menu_source = Some(source);
            // 保存菜单显示的锚点坐标
            app.file_tree_menu_anchor = Some(Point::new(x, y));
            Some(iced::Task::none())
        }
        // 关闭文件树上下文菜单：清空所有菜单相关的状态
        ProjectMessage::FileTreeMenuClose => {
            app.file_tree_menu_path = None;
            app.file_tree_menu_source = None;
            app.file_tree_menu_anchor = None;
            Some(iced::Task::none())
        }
        // 重命名输入框内容变化：更新重命名输入值
        ProjectMessage::FileTreeRenameChanged(v) => {
            app.file_tree_rename_value = v;
            Some(iced::Task::none())
        }
        // 取消重命名操作：清空重命名相关状态
        ProjectMessage::FileTreeRenameCancel => {
            app.file_tree_rename_path = None;
            app.file_tree_rename_value.clear();
            Some(iced::Task::none())
        }
        ProjectMessage::FileTreeInternalDragStart { project_path, source_path } => {
            if app.file_tree_operation_in_progress
                || app.project_path.as_ref() != Some(&project_path)
            {
                return Some(iced::Task::none());
            }
            let visible =
                crate::app::components::file_tree::selection::visible_paths(app, &project_path);
            let paths = if app.file_tree_selection.contains(&project_path, &source_path) {
                visible
                    .iter()
                    .filter(|path| app.file_tree_selection.contains(&project_path, path))
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                if visible.contains(&source_path) {
                    app.file_tree_selection.select(
                        &project_path,
                        source_path.clone(),
                        false,
                        false,
                        &visible,
                    );
                }
                vec![source_path.clone()]
            };
            // Freeze the complete gesture before later selection or release messages.
            let task = handle(app, ProjectMessage::FileTreeDragStart(source_path.clone(), None));
            app.dragging_file_paths =
                if paths.is_empty() { vec![source_path.clone()] } else { paths };
            app.file_tree_drag_source = Some((project_path, source_path));
            task
        }
        ProjectMessage::FileTreeDropTargetChanged(target) => {
            app.file_tree_drop_target = if app
                .file_tree_drag_source
                .as_ref()
                .is_some_and(|(root, _)| app.project_path.as_ref() == Some(root))
                && !app.file_tree_operation_in_progress
            {
                target
            } else {
                None
            };
            Some(iced::Task::none())
        }
        ProjectMessage::FileTreeDropTargetReleased { project_path, target_directory } => {
            let Some((root, source)) = app.file_tree_drag_source.clone() else {
                return Some(iced::Task::none());
            };
            if root != project_path {
                return Some(iced::Task::none());
            }
            Some(move_drop::submit(app, root, source, target_directory))
        }
        ProjectMessage::FileTreeDropRequested { project_path, source_path, target_directory } => {
            Some(move_drop::submit(app, project_path, source_path, target_directory))
        }
        ProjectMessage::FileTreeMoveCompleted {
            project_path,
            source_path,
            target_path,
            result,
        } => Some(move_drop::complete(app, project_path, source_path, target_path, result)),
        ProjectMessage::FileTreeBatchMoveCompleted { project_path, moved_paths, errors } => {
            Some(move_drop::complete_batch(app, project_path, moved_paths, errors))
        }
        ProjectMessage::FileTreeExternalDropRequested {
            project_path,
            source_path,
            target_directory,
        } => Some(external_drop::submit(app, project_path, source_path, target_directory)),
        ProjectMessage::FileTreeImportCompleted { project_path, source_path, result } => {
            Some(external_drop::complete(app, project_path, source_path, result))
        }
        ProjectMessage::FileTreeDragCancel => {
            move_drop::clear_drag(app);
            Some(iced::Task::none())
        }
        // 开始拖拽：记录被拖拽的文件路径和位置，清空放置相关状态
        ProjectMessage::FileTreeDragStart(path, position) => {
            app.file_tree_drag_source = None;
            app.file_tree_drop_target = None;
            // 记录正在拖拽的文件路径
            app.dragging_file_paths = vec![path];
            // 记录拖拽起始位置
            app.dragging_file_position = position;
            // 清空待放置状态
            app.pending_drop_file_paths.clear();
            app.pending_drop_file_position = None;
            // 重置输入框拖拽悬停状态
            app.input_drop_hovered = false;
            Some(iced::Task::none())
        }
        // 结束拖拽：将拖拽中的文件转移到待放置状态，清空拖拽状态
        ProjectMessage::FileTreeDragEnd => {
            if let Some(task) = move_drop::release(app) {
                return Some(task);
            }
            if app.dragging_file_paths.is_empty() {
                return Some(iced::Task::none());
            }
            // 将拖拽的文件转移到待放置状态
            app.pending_drop_file_paths = app.dragging_file_paths.clone();
            app.pending_drop_file_position = app.dragging_file_position;
            // 清空拖拽状态
            app.dragging_file_paths.clear();
            app.dragging_file_position = None;
            Some(iced::Task::none())
        }
        // 保存重命名结果：执行文件/文件夹重命名操作
        ProjectMessage::FileTreeRenameSave => {
            let Some(old_path) = app.file_tree_rename_path.clone() else {
                return Some(iced::Task::none());
            };
            let new_name = app.file_tree_rename_value.trim().to_string();
            if new_name.is_empty()
                || new_name == "."
                || new_name == ".."
                || new_name.contains('/')
                || new_name.contains('\\')
            {
                return Some(app.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    "文件名不合法",
                    vec![],
                ));
            }
            let old = std::path::PathBuf::from(&old_path);
            let Some(parent) = old.parent() else {
                return Some(iced::Task::none());
            };
            if old.file_name().and_then(|name| name.to_str()) == Some(new_name.as_str()) {
                app.file_tree_rename_path = None;
                app.file_tree_rename_value.clear();
                return Some(iced::Task::none());
            }
            let existing = app.files.iter().cloned().collect::<HashSet<_>>();
            let new_path = unique_name_for_target(parent, &new_name, &existing);
            let Some(project_path) = app.project_path.clone() else {
                return Some(iced::Task::none());
            };
            let old_path_for_task = old_path.clone();
            let old_path_for_msg = old_path.clone();
            app.file_tree_rename_path = None;
            app.file_tree_rename_value.clear();
            Some(iced::Task::perform(
                async move {
                    gateway_move_path(&project_path, &old_path_for_task, &new_path)
                        .await
                        .map(|_| new_path.clone())
                },
                move |result| {
                    Message::Project(ProjectMessage::FileTreeRenameCompleted {
                        old_path: old_path_for_msg.clone(),
                        result,
                    })
                },
            ))
        }
        ProjectMessage::FileTreeRenameCompleted { old_path, result } => {
            match result {
                Ok(new_path) => {
                    for tab in &mut app.preview_tabs {
                        if let Some(updated) = replace_path_prefix(&tab.path, &old_path, &new_path)
                        {
                            tab.path = updated;
                        }
                    }
                    if let Some(active) = app.active_preview_path.clone()
                        && let Some(updated) = replace_path_prefix(&active, &old_path, &new_path)
                    {
                        app.active_preview_path = Some(updated);
                    }
                    return Some(iced::Task::batch(vec![
                        refresh_file_index(app),
                        app.show_localized_toast(
                            crate::app::state::ToastKind::Success,
                            "已重命名为 %s",
                            vec![path_display_name(&new_path).into()],
                        ),
                    ]));
                }
                Err(err) => {
                    app.set_localized_input_error(
                        crate::app::i18n::input::ErrorField::General,
                        "重命名失败: %s",
                        vec![err.into()],
                    );
                }
            }
            Some(refresh_file_index(app))
        }
        // 处理文件树上下文菜单操作
        // 首先获取菜单操作的目标路径，如果没有则直接返回
        ProjectMessage::FileTreeAction(action) => {
            if app.file_tree_operation_in_progress
                && matches!(
                    action,
                    FileTreeAction::Delete | FileTreeAction::Paste | FileTreeAction::Duplicate
                )
            {
                return Some(iced::Task::none());
            }
            let paths = batch::menu_paths(app);
            let Some(path) = paths.first().cloned() else {
                return Some(iced::Task::none());
            };

            // 清空菜单相关状态
            app.file_tree_menu_path = None;
            app.file_tree_menu_source = None;
            app.file_tree_menu_anchor = None;

            match action {
                // 打开文件：发送预览消息打开文件
                FileTreeAction::Open => {
                    return Some(iced::Task::done(Message::Preview(
                        crate::app::message::PreviewMessage::Open(path),
                    )));
                }
                // 由数据网关所在主机定位文件，桌面与 WASM 共用同一入口。
                FileTreeAction::RevealInFinder => {
                    return Some(iced::Task::done(Message::View(
                        crate::app::message::ViewMessage::OpenPathInFinder(path),
                    )));
                }
                // 在终端中打开：在该目录下打开新的终端标签页
                // 该操作在 WASM 目标平台不可用
                FileTreeAction::OpenInTerminal => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let target = std::path::Path::new(&path);
                        // 确定要打开终端的目录（如果是文件则使用其父目录）
                        let dir = if target.is_dir() { Some(target) } else { target.parent() };
                        if let Some(dir) = dir {
                            // 添加新终端并应用当前主题
                            if app.terminal.add_terminal(Some(dir.to_path_buf())) {
                                app.terminal.apply_app_theme(&app.app_theme);
                                app.terminal.is_visible = true;
                                return Some(iced::Task::batch([
                                    crate::app::message::terminal::event::focus_active_terminal(
                                        app,
                                    ),
                                    app.save_desktop_preference(
                                        "show_terminal",
                                        serde_json::Value::Bool(true),
                                    ),
                                ]));
                            }
                        }
                    }
                }
                // 添加到聊天：将文件路径以 @引用 的形式插入到聊天输入框
                FileTreeAction::AddToChat => {
                    // 获取项目根目录并计算相对路径
                    if let Some(root) = &app.project_path {
                        let references = paths
                            .iter()
                            .filter_map(|path| relative_to_project(root, path))
                            .map(|path| format!("@{path} "))
                            .collect::<String>();
                        if references.is_empty() {
                            return Some(iced::Task::none());
                        }
                        // 关闭文件搜索面板
                        app.show_file_search = false;
                        app.file_search_query.clear();
                        app.file_search_selected_index = 0;

                        // 获取当前会话的运行时并移动光标到文档末尾
                        let runtime = app.current_session_runtime_mut();
                        runtime.input_editor.perform(iced::widget::text_editor::Action::Move(
                            iced::widget::text_editor::Motion::DocumentEnd,
                        ));

                        // 在光标位置插入 @文件路径 引用
                        crate::app::ui::chat::insert_at_cursor(
                            &mut runtime.input_editor,
                            &references,
                        );

                        // 如果没有活动会话，同步输入编辑器内容
                        if app.active_session_id.is_none() {
                            let runtime = app.current_session_runtime();
                            app.input_editor = runtime.input_editor;
                        }

                        // 设置焦点并返回聚焦任务
                        app.focus_area = crate::app::FocusArea::None;
                        return Some(iced::widget::operation::focus(app.input_editor_id.clone()));
                    }
                }
                // 在文件夹中查找：创建新的查找标签页并打开查找界面
                FileTreeAction::FindInFolder => {
                    // 生成唯一的标签页 ID
                    let tab_id = format!("{}", now_ms());

                    // 创建新的查找标签页
                    app.find_results_tabs.push(FindInFolderTab {
                        id: tab_id.clone(),
                        title: "查找".to_string(),
                        scope_path: path, // 设置查找范围为当前路径
                        query_input: String::new(),
                        replace_input: String::new(),
                        query_editor: iced::widget::text_editor::Content::new(),
                        replace_editor: iced::widget::text_editor::Content::new(),
                        query: String::new(),
                        replace_text: String::new(),
                        case_sensitive: false,
                        whole_word: false,
                        use_regex: false,
                        running: false,
                        error: None,
                        limit_reached: false,
                        matches: Vec::new(),
                        localized_error: None,
                        results_page: 0,
                    });

                    // 设置为活动查找标签页
                    app.active_find_results_tab_id = Some(tab_id.clone());

                    // 显示文件管理器面板
                    app.show_file_manager = true;

                    // 创建并打开查找结果标签页
                    let open_tab_id = format!("find:{}", tab_id);
                    if !app.open_tabs.iter().any(|t| t.id == open_tab_id) {
                        app.open_tabs.push(AppTab {
                            id: open_tab_id.clone(),
                            title: format!("查找结果 {}", app.find_results_tabs.len()),
                            screen: Screen::Project,
                            project_path: app.project_path.clone(),
                        });
                    }

                    // 切换到新创建的查找标签页
                    app.active_tab_id = Some(open_tab_id);
                    app.screen = Screen::Project;
                    return Some(app.save_desktop_preference(
                        "show_file_manager",
                        serde_json::Value::Bool(true),
                    ));
                }
                // 剪切：将文件/文件夹路径保存到剪贴板，模式为剪切
                FileTreeAction::Cut => {
                    app.file_tree_clipboard = Some(FileTreeClipboard {
                        mode: FileTreeClipboardMode::Cut,
                        src_paths: batch::top_level_paths(paths),
                    });
                    return Some(app.show_localized_toast(
                        crate::app::state::ToastKind::Success,
                        "已剪切到文件树剪贴板",
                        vec![],
                    ));
                }
                // 复制：将文件/文件夹路径保存到剪贴板，模式为复制
                FileTreeAction::Copy => {
                    app.file_tree_clipboard = Some(FileTreeClipboard {
                        mode: FileTreeClipboardMode::Copy,
                        src_paths: batch::top_level_paths(paths),
                    });
                    return Some(app.show_localized_toast(
                        crate::app::state::ToastKind::Success,
                        "已复制到文件树剪贴板",
                        vec![],
                    ));
                }
                FileTreeAction::Duplicate => {
                    let Some(project_path) = app.project_path.clone() else {
                        return Some(iced::Task::none());
                    };
                    return Some(batch::duplicate(
                        app,
                        project_path,
                        batch::top_level_paths(paths),
                    ));
                }
                // 粘贴到第一项对应目录，按快照处理整个剪贴板。
                FileTreeAction::Paste => {
                    return Some(batch::paste(app, &path));
                }
                FileTreeAction::CopyPath => {
                    return Some(
                        iced::clipboard::write(paths.join("\n")).map(|_: ()| Message::None),
                    );
                }
                FileTreeAction::CopyRelativePath => {
                    if let Some(root) = &app.project_path {
                        let rel = paths
                            .iter()
                            .map(|path| {
                                relative_to_project(root, path)
                                    .unwrap_or_else(|| path.replace('\\', "/"))
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        return Some(iced::clipboard::write(rel).map(|_: ()| Message::None));
                    }
                }
                FileTreeAction::Rename => {
                    let directory = is_directory_path(&path, app.current_file_index());
                    app.file_tree_rename_path = Some(path.clone());
                    app.file_tree_rename_value = std::path::Path::new(&path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    let end = crate::app::components::file_tree::inline_rename::selection_end(
                        &app.file_tree_rename_value,
                        directory,
                    );
                    let id = crate::app::components::file_tree::inline_rename::INPUT_ID;
                    return Some(
                        iced::widget::operation::focus(id)
                            .chain(iced::widget::operation::select_range(id, 0, end)),
                    );
                }
                FileTreeAction::Delete => {
                    if let Some(project_path) = app.project_path.clone() {
                        app.file_tree_pending_delete =
                            Some((project_path, batch::top_level_paths(paths)));
                    }
                }
            }
            Some(iced::Task::none())
        }
        ProjectMessage::FileTreePasteCompleted {
            project_path,
            clipboard,
            completed_sources,
            result,
        } => {
            app.file_tree_operation_in_progress = false;
            if matches!(clipboard.mode, FileTreeClipboardMode::Cut)
                && app.file_tree_clipboard.as_ref() == Some(&clipboard)
            {
                let mut remaining = clipboard;
                remaining.src_paths.retain(|path| !completed_sources.contains(path));
                app.file_tree_clipboard = (!remaining.src_paths.is_empty()).then_some(remaining);
            }
            let refresh = if app.project_path.as_ref() == Some(&project_path) {
                refresh_file_index(app)
            } else {
                iced::Task::none()
            };
            let feedback = match result {
                Ok(()) => app.show_localized_toast(
                    crate::app::state::ToastKind::Success,
                    "文件树操作已完成",
                    vec![],
                ),
                Err(err) => app.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    "粘贴失败: %s",
                    vec![err.into()],
                ),
            };
            Some(iced::Task::batch([refresh, feedback]))
        }
        ProjectMessage::FileTreeDuplicateCompleted { project_path, result } => {
            app.file_tree_operation_in_progress = false;
            let refresh = if app.project_path.as_deref() == Some(project_path.as_str()) {
                refresh_file_index(app)
            } else {
                iced::Task::none()
            };
            let feedback = match result {
                Ok(()) => app.show_localized_toast(
                    crate::app::state::ToastKind::Success,
                    "已创建副本",
                    vec![],
                ),
                Err(err) => app.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    "创建副本失败: %s",
                    vec![err.into()],
                ),
            };
            Some(iced::Task::batch([refresh, feedback]))
        }
        ProjectMessage::FileTreeDeleteCanceled => {
            app.file_tree_pending_delete = None;
            Some(iced::Task::none())
        }
        ProjectMessage::FileTreeDeleteConfirmed => Some(batch::delete(app)),
        ProjectMessage::FileTreeDeleteCompleted { project_path, deleted_paths, result } => {
            app.file_tree_operation_in_progress = false;
            if app.project_path.as_ref() == Some(&project_path) {
                app.file_tree_selection.paths.retain(|path| {
                    !deleted_paths
                        .iter()
                        .any(|deleted| std::path::Path::new(path).starts_with(deleted))
                });
            }
            let feedback = match result {
                Ok(()) => app.show_localized_toast(
                    crate::app::state::ToastKind::Success,
                    "已删除",
                    vec![],
                ),
                Err(err) => app.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    "删除失败: %s",
                    vec![err.into()],
                ),
            };
            let refresh = if app.project_path.as_ref() == Some(&project_path) {
                refresh_file_index(app)
            } else {
                iced::Task::none()
            };
            Some(iced::Task::batch([refresh, feedback]))
        }
        // 其他消息类型不在此处理器中处理
        _ => None,
    }
}

/// Consume the same tree release used by the source widget and global subscription.
pub(crate) fn release_tree_drop(app: &mut App) -> iced::Task<Message> {
    move_drop::release(app).unwrap_or_else(iced::Task::none)
}

#[cfg(test)]
#[path = "file_tree_external_drop_tests.rs"]
mod external_drop_tests;
