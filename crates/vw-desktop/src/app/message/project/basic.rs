//! 项目消息的基础处理器模块
//!
//! 本模块负责处理与项目管理相关的各种消息，包括：
//! - 打开文件、文件夹、项目
//! - 文件树导航和展开/折叠
//! - 文件索引管理
//! - Git 变更文件显示
//! - 文件树项添加到聊天
//!
//! 该模块是项目消息处理的核心，协调文件系统操作和 UI 状态更新。

use crate::app::message::project::ProjectMessage;
use crate::app::message::project::helpers::{append_local_attachments, load_project_info_task};

use crate::app::{App, Message, set_config_field};
use std::time::Duration;

fn schedule_file_manager_refresh_tick() -> iced::Task<Message> {
    crate::app::message::after(
        Duration::from_millis(90),
        Message::Project(ProjectMessage::FileManagerRefreshAnimationTick),
    )
}

#[cfg(test)]
#[path = "basic_tests.rs"]
mod basic_tests;

/// 处理项目相关的消息
///
/// 该函数是项目消息的主要分发器，根据不同的消息类型执行相应的操作，
/// 如打开文件/文件夹、管理设计文档、处理文件树交互等。
///
/// # 参数
///
/// * `app` - 应用程序状态的可变引用，用于更新 UI 状态和执行操作
/// * `message` - 要处理的项目消息枚举
///
/// # 返回值
///
/// 返回 `Option<iced::Task<Message>>`：
/// - `Some(task)` - 返回需要执行的异步任务
/// - `None` - 该消息不需要处理或未实现
///
/// # 示例
///
/// ```ignore
/// if let Some(task) = handle(&mut app, ProjectMessage::OpenFolderPressed) {
///     // 执行返回的任务
/// }
/// ```
pub(crate) fn handle(app: &mut App, message: ProjectMessage) -> Option<iced::Task<Message>> {
    match message {
        // 处理"打开文件"按钮点击事件
        // 在非 wasm32 平台上弹出文件选择对话框，选择后打开预览
        ProjectMessage::OpenFilePressed => Some(iced::Task::perform(
            async {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let handle = rfd::AsyncFileDialog::new().pick_file().await;
                    if let Some(file) = handle {
                        return Some(file.path().to_string_lossy().to_string());
                    }
                    None
                }
                #[cfg(target_arch = "wasm32")]
                {
                    None
                }
            },
            |opt| {
                if let Some(path) = opt {
                    Message::Preview(crate::app::message::PreviewMessage::Open(path))
                } else {
                    Message::None
                }
            },
        )),
        // 处理"打开文件夹"按钮点击事件
        // 在非 wasm32 平台上弹出文件夹选择对话框，选择后打开项目并建立索引
        ProjectMessage::OpenFolderPressed => {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(dir) = rfd::FileDialog::new().pick_folder()
                && let Some(path) = dir.to_str()
            {
                return Some(app.open_project_and_index(path.to_string()));
            }
            Some(iced::Task::none())
        }
        // 处理"打开项目"按钮点击事件
        // 如果输入框为空则使用当前目录，否则使用输入的路径
        ProjectMessage::OpenProjectPressed => {
            if app.project_path_input.is_empty() {
                #[cfg(not(target_arch = "wasm32"))]
                if let Ok(cd) = std::env::current_dir()
                    && let Some(path) = cd.to_str()
                {
                    return Some(app.open_project_and_index(path.to_string()));
                }
            } else {
                return Some(app.open_project_and_index(app.project_path_input.clone()));
            }
            Some(iced::Task::none())
        }
        ProjectMessage::RecentProjectsLoaded(result) => {
            match result {
                Ok(meta) => {
                    app.recent_projects = meta.iter().map(|item| item.path.clone()).collect();
                    app.recent_projects_edits = meta.iter().map(|item| item.name.clone()).collect();
                    app.recent_projects_meta = meta;
                }
                Err(error) => {
                    tracing::warn!(
                        target: "vw_desktop",
                        error = %error,
                        "failed to load recent projects from gateway"
                    );
                }
            }
            Some(iced::Task::none())
        }
        // 处理项目路径输入框内容变化事件
        // 更新应用程序中的项目路径输入字段
        ProjectMessage::ProjectPathChanged(v) => {
            app.project_path_input = v;
            Some(iced::Task::none())
        }
        ProjectMessage::AttachmentFilesPick => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                Some(iced::Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new().pick_files().await.map(|handles| {
                            handles
                                .into_iter()
                                .map(|file| file.path().to_string_lossy().to_string())
                                .collect::<Vec<_>>()
                        })
                    },
                    |picked| Message::Project(ProjectMessage::AttachmentFilesPicked(picked)),
                ))
            }
            #[cfg(target_arch = "wasm32")]
            {
                app.push_notification("当前平台暂不支持本地附件选择".to_string());
                Some(iced::Task::none())
            }
        }
        ProjectMessage::AttachmentFilesPicked(picked) => {
            let Some(paths) = picked else {
                return Some(iced::Task::none());
            };

            append_local_attachments(app, paths);

            Some(iced::Task::none())
        }
        ProjectMessage::RemoveAttachedFile(path) => {
            app.files.retain(|entry| entry != &path);
            Some(iced::Task::none())
        }
        // 处理文件 URL 输入框内容变化事件
        // 更新应用程序中的文件 URL 输入字段
        ProjectMessage::FileUrlChanged(v) => {
            app.file_url_input = v;
            Some(iced::Task::none())
        }
        // 处理"添加文件"按钮点击事件
        // 将当前输入的文件 URL 添加到文件列表中并清空输入框
        ProjectMessage::AddFilePressed => {
            if !app.file_url_input.is_empty() {
                app.files.push(app.file_url_input.clone());
                app.file_url_input.clear();
            }
            Some(iced::Task::none())
        }
        ProjectMessage::FileIndexLoaded(result) => {
            if let Some(path) = app.project_path.clone() {
                app.set_file_index(&path, result.files);
            }
            if result.needs_refresh && app.project_path.is_some() {
                return Some(crate::app::message::project::helpers::refresh_file_index(app));
            }
            Some(iced::Task::none())
        }
        // 处理文件索引就绪事件
        // 当文件索引构建完成后，将其设置到应用程序状态中
        ProjectMessage::FileIndexReady(files) => {
            if let Some(path) = app.project_path.clone() {
                app.set_file_index(&path, files);
            }
            if app.file_manager_file_tree_refreshing {
                app.file_manager_file_tree_refreshing = false;
                return Some(app.show_success_toast("文件树已刷新"));
            }
            Some(iced::Task::none())
        }
        // 处理文件树目录展开/折叠切换事件
        // 切换指定目录的展开状态，并保存到配置中
        ProjectMessage::ToggleTreeDir(dir) => {
            app.toggle_file_tree_dir_expanded(dir);
            // 将展开的目录列表序列化并保存到配置
            let arr = serde_json::Value::Array(
                app.file_tree_expanded.iter().cloned().map(serde_json::Value::String).collect(),
            );
            set_config_field("file_tree_expanded", arr);
            Some(iced::Task::none())
        }
        // 处理文件管理器显示模式切换事件
        // 切换文件管理器在"文件浏览"和"Git 变更"模式之间
        ProjectMessage::FileManagerShowChanges(b) => {
            app.file_manager_show_changes = b;
            app.active_find_results_tab_id = None;
            if b {
                // 当切换到"变更"模式时，清除活动预览选择，
                // 以便右侧面板可以渲染 Git diff 面板而不是停留在已打开的预览标签页
                app.active_preview_path = None;
            }
            // 保存显示模式到配置
            set_config_field("file_manager_show_changes", serde_json::Value::Bool(b));
            app.show_file_manager = true;
            set_config_field("show_file_manager", serde_json::Value::Bool(true));
            if b {
                // 自动展开包含变更文件的目录
                // 收集所有变更文件的父目录路径
                let mut keys = std::collections::BTreeSet::<String>::new();
                for f in &app.git_changed_files {
                    let parts = f.split('/').filter(|s| !s.is_empty()).collect::<Vec<_>>();
                    if parts.len() <= 1 {
                        continue;
                    }
                    // 构建每一层目录路径
                    let mut current = String::new();
                    for i in 0..parts.len() - 1 {
                        if !current.is_empty() {
                            current.push('/');
                        }
                        current.push_str(parts[i]);
                    }
                    keys.insert(current.clone());
                }
                // 将这些目录添加到展开列表中
                for k in keys {
                    app.ensure_file_tree_dir_expanded(k);
                }
            }
            Some(iced::Task::none())
        }
        ProjectMessage::FileManagerRefreshChanges => {
            if app.git_changed_files_loading || app.file_manager_changes_refreshing {
                return Some(iced::Task::none());
            }
            app.file_manager_changes_refreshing = true;
            let mut tasks = vec![iced::Task::done(Message::Git(
                crate::app::message::GitMessage::RefreshGitPanelData,
            ))];
            if !app.file_manager_file_tree_refreshing {
                tasks.push(schedule_file_manager_refresh_tick());
            }
            Some(iced::Task::batch(tasks))
        }
        ProjectMessage::FileManagerRefreshFileTree => {
            if app.file_manager_file_tree_refreshing {
                return Some(iced::Task::none());
            }
            app.file_manager_file_tree_refreshing = true;
            let mut tasks = vec![crate::app::message::project::helpers::refresh_file_index(app)];
            if !app.file_manager_changes_refreshing {
                tasks.push(schedule_file_manager_refresh_tick());
            }
            Some(iced::Task::batch(tasks))
        }
        ProjectMessage::FileManagerRefreshAnimationTick => {
            if !app.file_manager_changes_refreshing
                && !app.file_manager_file_tree_refreshing
                && !app.git_commit_in_progress
            {
                return Some(iced::Task::none());
            }
            app.file_manager_refresh_frame = app.file_manager_refresh_frame.wrapping_add(1);
            Some(schedule_file_manager_refresh_tick())
        }
        // 处理打开 Git 变更文件事件
        // 构建文件的完整路径并打开预览
        ProjectMessage::OpenChangedFile(path) => {
            if let Some(root) = &app.project_path {
                let full = std::path::Path::new(root).join(&path);
                let full_str = full.to_string_lossy().to_string();
                return Some(iced::Task::done(Message::Preview(
                    crate::app::message::PreviewMessage::Open(full_str),
                )));
            }
            Some(iced::Task::none())
        }
        // 处理将文件树项添加到聊天的事件
        // 在聊天输入框的光标位置插入文件引用（格式：@相对路径:行号:列号）
        ProjectMessage::FileTreeAddToChat { path, line, column } => {
            if let Some(root) = &app.project_path
                && let Ok(rel_path) = std::path::Path::new(&path).strip_prefix(root)
            {
                // 将路径转换为相对路径并统一使用正斜杠
                let rel = rel_path.to_string_lossy().replace('\\', "/");
                // 构建文件引用格式的字符串
                let mention = format!("@{}:{}:{} ", rel, line, column);
                // 关闭文件搜索弹窗
                app.show_file_search = false;
                app.file_search_query.clear();
                app.refresh_file_search_cache();
                app.file_search_selected_index = 0;
                // 获取当前会话的运行时并插入文件引用
                let runtime = app.current_session_runtime_mut();
                // 将光标移动到文档末尾
                runtime.input_editor.perform(iced::widget::text_editor::Action::Move(
                    iced::widget::text_editor::Motion::DocumentEnd,
                ));
                // 在光标位置插入文件引用
                crate::app::ui::chat::insert_at_cursor(&mut runtime.input_editor, &mention);
                // 如果没有活动会话，同步输入编辑器状态
                if app.active_session_id.is_none() {
                    let runtime = app.current_session_runtime();
                    app.input_editor = runtime.input_editor;
                }
                app.focus_area = crate::app::FocusArea::None;
                // 返回聚焦输入框的任务
                return Some(iced::widget::operation::focus(app.input_editor_id.clone()));
            }
            Some(iced::Task::none())
        }
        ProjectMessage::StartDeferredTasks { project_path } => {
            if app.project_path.as_ref() != Some(&project_path) {
                return Some(iced::Task::none());
            }
            let info_task = load_project_info_task(project_path.clone());
            #[cfg(target_arch = "wasm32")]
            {
                return Some(info_task);
            }

            #[cfg(not(target_arch = "wasm32"))]
            let branch_path = project_path.clone();
            #[cfg(not(target_arch = "wasm32"))]
            let branch_task = iced::Task::perform(
                async move {
                    crate::app::message::spawn_blocking_opt(move || {
                        let selected_branch =
                            crate::app::components::git_panel::current_branch(&branch_path);
                        let branches =
                            crate::app::components::git_panel::list_branches(&branch_path)
                                .unwrap_or_default();
                        Some(crate::app::projects::ProjectBranchSnapshot {
                            project_path: branch_path,
                            selected_branch,
                            branches,
                        })
                    })
                    .await
                    .unwrap_or_else(|| {
                        crate::app::projects::ProjectBranchSnapshot {
                            project_path: String::new(),
                            selected_branch: None,
                            branches: Vec::new(),
                        }
                    })
                },
                |snapshot| {
                    Message::Project(ProjectMessage::ProjectBranchesLoaded {
                        project_path: snapshot.project_path,
                        selected_branch: snapshot.selected_branch,
                        branches: snapshot.branches,
                    })
                },
            );
            #[cfg(not(target_arch = "wasm32"))]
            Some(iced::Task::batch(vec![info_task, branch_task]))
        }
        _ => None,
    }
}
