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
//!   - 剪切、复制、粘贴
//!   - 复制路径（绝对路径和相对路径）
//!   - 重命名和删除
//!
//! # 平台兼容性
//!
//! 部分功能（如重命名、在终端打开、在文件管理器显示）仅在非 WASM 目标平台可用。

#[cfg(not(target_arch = "wasm32"))]
use crate::app::FocusArea;
use crate::app::message::project::helpers::{
    gateway_copy_path, gateway_delete_path, gateway_move_path, now_ms, refresh_file_index,
    relative_to_project,
};
use crate::app::message::project::{FileTreeAction, ProjectMessage};
use crate::app::{
    App, AppTab, Message, Screen, set_config_field,
    state::{FileTreeClipboard, FileTreeClipboardMode, FindInFolderTab},
};
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
        // 处理右键点击事件：记录点击位置、源和坐标，用于显示上下文菜单
        ProjectMessage::FileTreeRightClicked(path, source, x, y) => {
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
        // 开始拖拽：记录被拖拽的文件路径和位置，清空放置相关状态
        ProjectMessage::FileTreeDragStart(path, position) => {
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
            // 将拖拽的文件转移到待放置状态
            app.pending_drop_file_paths = app.dragging_file_paths.clone();
            app.pending_drop_file_position = app.dragging_file_position;
            // 清空拖拽状态
            app.dragging_file_paths.clear();
            app.dragging_file_position = None;
            app.input_drop_hovered = false;
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
                app.error_message = Some("文件名不合法".to_string());
                return Some(iced::Task::none());
            }
            let old = std::path::PathBuf::from(&old_path);
            let Some(parent) = old.parent() else {
                return Some(iced::Task::none());
            };
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
                        app.show_success_toast(format!(
                            "已重命名为 {}",
                            path_display_name(&new_path)
                        )),
                    ]));
                }
                Err(err) => {
                    app.error_message = Some(format!("重命名失败: {err}"));
                }
            }
            Some(refresh_file_index(app))
        }
        // 处理文件树上下文菜单操作
        // 首先获取菜单操作的目标路径，如果没有则直接返回
        ProjectMessage::FileTreeAction(action) => {
            let path = if let Some(p) = app.file_tree_menu_path.clone() {
                p
            } else {
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
                // 在文件管理器中显示：使用系统默认程序打开文件所在目录
                // 该操作在 WASM 目标平台不可用
                FileTreeAction::RevealInFinder => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let target = std::path::Path::new(&path);
                        // 如果是目录则直接打开，否则打开其父目录
                        let open_path = if target.is_dir() {
                            target
                        } else {
                            target.parent().unwrap_or(target)
                        };
                        let _ = open::that(open_path);
                    }
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
                                // 保存终端显示状态到配置
                                set_config_field("show_terminal", serde_json::Value::Bool(true));
                                app.focus_area = FocusArea::Terminal;
                            }
                        }
                    }
                }
                // 添加到聊天：将文件路径以 @引用 的形式插入到聊天输入框
                FileTreeAction::AddToChat => {
                    // 获取项目根目录并计算相对路径
                    if let Some(root) = &app.project_path
                        && let Some(rel_path) = relative_to_project(root, &path)
                    {
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
                            &format!("@{} ", rel_path),
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
                        results_page: 0,
                    });

                    // 设置为活动查找标签页
                    app.active_find_results_tab_id = Some(tab_id.clone());

                    // 显示文件管理器面板
                    app.show_file_manager = true;
                    set_config_field("show_file_manager", serde_json::Value::Bool(true));

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
                }
                // 剪切：将文件/文件夹路径保存到剪贴板，模式为剪切
                FileTreeAction::Cut => {
                    app.file_tree_clipboard = Some(FileTreeClipboard {
                        mode: FileTreeClipboardMode::Cut,
                        src_path: path,
                    });
                    return Some(app.show_success_toast("已剪切到文件树剪贴板"));
                }
                // 复制：将文件/文件夹路径保存到剪贴板，模式为复制
                FileTreeAction::Copy => {
                    app.file_tree_clipboard = Some(FileTreeClipboard {
                        mode: FileTreeClipboardMode::Copy,
                        src_path: path,
                    });
                    return Some(app.show_success_toast("已复制到文件树剪贴板"));
                }
                // 粘贴：根据剪贴板模式执行复制或移动操作
                FileTreeAction::Paste => {
                    if let Some(clipboard) = &app.file_tree_clipboard {
                        let src_path = clipboard.src_path.clone();
                        let src = std::path::Path::new(&src_path);
                        let dst_dir = if is_directory_path(&path, &app.files) {
                            std::path::Path::new(&path)
                        } else {
                            std::path::Path::new(&path)
                                .parent()
                                .unwrap_or(std::path::Path::new(&path))
                        };
                        let existing = app.files.iter().cloned().collect::<HashSet<_>>();
                        let dst = unique_name_for_target(
                            dst_dir,
                            src.file_name().and_then(|s| s.to_str()).unwrap_or(""),
                            &existing,
                        );
                        let Some(project_path) = app.project_path.clone() else {
                            return Some(iced::Task::none());
                        };
                        let clear_clipboard = matches!(clipboard.mode, FileTreeClipboardMode::Cut);
                        let mode = clipboard.mode.clone();
                        return Some(iced::Task::perform(
                            async move {
                                match mode {
                                    FileTreeClipboardMode::Copy => {
                                        gateway_copy_path(&project_path, &src_path, &dst).await
                                    }
                                    FileTreeClipboardMode::Cut => {
                                        gateway_move_path(&project_path, &src_path, &dst).await
                                    }
                                }
                            },
                            move |result| {
                                Message::Project(ProjectMessage::FileTreePasteCompleted {
                                    clear_clipboard,
                                    result,
                                })
                            },
                        ));
                    }
                }
                FileTreeAction::CopyPath => {
                    return Some(iced::clipboard::write(path).map(|_: ()| Message::None));
                }
                FileTreeAction::CopyRelativePath => {
                    if let Some(root) = &app.project_path {
                        let rel = relative_to_project(root, &path)
                            .unwrap_or_else(|| path.replace('\\', "/"));
                        return Some(iced::clipboard::write(rel).map(|_: ()| Message::None));
                    }
                }
                FileTreeAction::Rename => {
                    app.file_tree_rename_path = Some(path.clone());
                    app.file_tree_rename_value = std::path::Path::new(&path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                }
                FileTreeAction::Delete => {
                    let recursive = is_directory_path(&path, &app.files);
                    let Some(project_path) = app.project_path.clone() else {
                        return Some(iced::Task::none());
                    };
                    remove_local_file_tree_path(&mut app.files, &path, recursive);
                    return Some(iced::Task::perform(
                        async move { gateway_delete_path(&project_path, &path, recursive).await },
                        |result| Message::Project(ProjectMessage::FileTreeDeleteCompleted(result)),
                    ));
                }
            }
            Some(iced::Task::none())
        }
        ProjectMessage::FileTreePasteCompleted { clear_clipboard, result } => {
            match result {
                Ok(()) => {
                    if clear_clipboard {
                        app.file_tree_clipboard = None;
                    }
                    return Some(iced::Task::batch(vec![
                        refresh_file_index(app),
                        app.show_success_toast("文件树操作已完成"),
                    ]));
                }
                Err(err) => {
                    app.error_message = Some(format!("粘贴失败: {err}"));
                }
            }
            Some(refresh_file_index(app))
        }
        ProjectMessage::FileTreeDeleteCompleted(result) => {
            match result {
                Ok(()) => {
                    return Some(iced::Task::batch(vec![
                        refresh_file_index(app),
                        app.show_success_toast("已删除"),
                    ]));
                }
                Err(err) => {
                    app.error_message = Some(format!("删除失败: {err}"));
                }
            }
            Some(refresh_file_index(app))
        }
        // 其他消息类型不在此处理器中处理
        _ => None,
    }
}
