//! # 项目文件查找消息处理模块
//!
//! 本模块负责处理项目范围内文件查找和替换功能的所有消息。
//!
//! ## 主要功能
//!
//! - **查找编辑器操作**：处理查找查询输入框的编辑器动作
//! - **替换编辑器操作**：处理替换文本输入框的编辑器动作
//! - **搜索选项切换**：大小写敏感、全词匹配、正则表达式等选项
//! - **查找任务执行**：在项目文件中执行查找操作
//! - **查找结果管理**：查找标签页的创建、选择、关闭等操作
//!
//! ## 消息类型
//!
//! 处理的 `ProjectMessage` 变体包括：
//! - `FileTreeFindQueryEditorAction`：查找查询编辑器动作
//! - `FileTreeFindReplaceEditorAction`：替换文本编辑器动作
//! - `FileTreeFindCaseSensitiveToggled`：大小写敏感切换
//! - `FileTreeFindWholeWordToggled`：全词匹配切换
//! - `FileTreeFindRegexToggled`：正则表达式模式切换
//! - `FileTreeFindRun`：执行查找
//! - `FileTreeFindRefreshActive`：刷新当前活动查找
//! - `FileTreeFindInProject`/`FileTreeReplaceInProject`：在项目中查找/替换
//! - `FileTreeFindCompleted`：查找完成回调
//! - `FileTreeFindMatchSelected`：打开文件并定位到匹配位置
//! - `FileTreeFindTabSelected`：选择查找标签页
//! - `FileTreeFindTabClosed`：关闭查找标签页

use crate::app::message::project::ProjectMessage;
use crate::app::message::project::helpers::{now_ms, run_find_task};
use crate::app::{
    App, AppTab, Message, Screen, set_config_field,
    state::{FindInFolderTab, find_results_page_bounds},
};
use iced::widget::text_editor;
use std::path::{Component, Path, PathBuf};

const FIND_SCOPE_ERROR: &str = "作用域必须是项目内的相对路径";

fn dismiss_find_tab_menu(app: &mut App) {
    app.find_results_tab_menu_id = None;
    app.find_results_tab_menu_pos = None;
}

fn close_find_results_tab(app: &mut App, id: &str) {
    let open_tab_id = format!("find:{id}");
    let closing_active_find = app.active_find_results_tab_id.as_deref() == Some(id);
    let closing_active_open = app.active_tab_id.as_deref() == Some(open_tab_id.as_str());

    app.find_results_tabs.retain(|tab| tab.id != id);
    app.open_tabs.retain(|tab| tab.id != open_tab_id);

    if closing_active_find {
        app.active_find_results_tab_id = app.find_results_tabs.last().map(|tab| tab.id.clone());
    }

    if closing_active_find
        && let Some(active_find_id) = app.active_find_results_tab_id.as_deref()
        && let Some(active_tab) =
            app.open_tabs.iter().find(|tab| tab.id == format!("find:{active_find_id}"))
    {
        app.active_tab_id = Some(active_tab.id.clone());
        app.screen = active_tab.screen;
    } else if closing_active_open {
        app.active_tab_id = app.open_tabs.last().map(|tab| tab.id.clone());
        if let Some(active_tab) = app
            .active_tab_id
            .as_deref()
            .and_then(|active_id| app.open_tabs.iter().find(|tab| tab.id == active_id))
        {
            app.screen = active_tab.screen;
        }
    }

    if app.find_results_tab_menu_id.as_deref() == Some(id) {
        dismiss_find_tab_menu(app);
    }
}

fn resolve_find_scope_path(project_root: &str, input: &str) -> Result<String, &'static str> {
    let normalized = input.trim().replace('\\', "/");
    if normalized.is_empty() {
        return Ok(project_root.to_string());
    }

    let path = Path::new(&normalized);
    let looks_like_windows_absolute =
        normalized.as_bytes().get(1) == Some(&b':') && normalized.as_bytes().get(2) == Some(&b'/');
    if path.is_absolute() || looks_like_windows_absolute {
        return Err(FIND_SCOPE_ERROR);
    }

    let mut relative = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => relative.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(FIND_SCOPE_ERROR);
            }
        }
    }

    Ok(Path::new(project_root).join(relative).to_string_lossy().into_owned())
}

/// 处理项目文件查找相关的消息
///
/// 此函数是项目查找功能的核心消息处理器，负责响应各种查找相关操作，
/// 包括编辑器输入、选项切换、查找执行、结果管理等。
///
/// # 参数
///
/// - `app`：应用程序状态的可变引用，包含查找标签页、项目路径等状态
/// - `message`：要处理的项目消息，包含具体的操作指令和参数
///
/// # 返回值
///
/// 返回 `Option<iced::Task<Message>>`：
/// - `Some(Task)`：返回一个 Iced 任务，可能用于异步操作或命令执行
/// - `None`：表示当前消息不被此处理器处理，应交给其他处理器
///
/// # 消息处理说明
///
/// ## 编辑器操作消息
///
/// - `FileTreeFindQueryEditorAction`：处理查找输入框的编辑动作
/// - `FileTreeFindReplaceEditorAction`：处理替换输入框的编辑动作
///
/// ## 选项切换消息
///
/// - `FileTreeFindCaseSensitiveToggled`：启用/禁用大小写敏感搜索
/// - `FileTreeFindWholeWordToggled`：启用/禁用全词匹配
/// - `FileTreeFindRegexToggled`：启用/禁用正则表达式模式
/// - `FileTreeFind*Shortcut`：通过快捷键切换活动搜索标签页的对应选项
///
/// ## 查找执行消息
///
/// - `FileTreeFindRun`：启动指定标签页的查找任务
/// - `FileTreeFindRefreshActive`：刷新当前活动标签页的查找结果
///
/// ## 标签页管理消息
///
/// - `FileTreeFindInProject`/`FileTreeReplaceInProject`：创建新的查找标签页
/// - `FileTreeFindCompleted`：处理查找任务完成后的结果
/// - `FileTreeFindMatchSelected`：打开文件并定位到匹配位置
/// - `FileTreeFindTabSelected`：切换活动的查找标签页
/// - `FileTreeFindTabClosed`：关闭指定的查找标签页
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
        ProjectMessage::FileTreeFindScopeChanged(tab_id, value) => {
            let Some(project_root) = app.project_path.clone() else {
                return Some(iced::Task::none());
            };
            let resolved = resolve_find_scope_path(&project_root, &value);
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|tab| tab.id == tab_id) {
                match resolved {
                    Ok(scope_path) => {
                        tab.scope_path = scope_path;
                        tab.error = None;
                    }
                    Err(error) => tab.error = Some(error.to_string()),
                }
            }
            Some(iced::Task::none())
        }

        // 处理查找查询编辑器的动作（如输入、删除、粘贴等）
        ProjectMessage::FileTreeFindQueryEditorAction(tab_id, action) => {
            // 在查找结果标签页列表中查找指定 ID 的标签页
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                // 执行编辑器动作（如光标移动、文本编辑等）
                tab.query_editor.perform(action);
                // 同步编辑器内容到查询输入字符串
                tab.query_input = tab.query_editor.text().to_string();
                // 清除之前的错误信息
                tab.error = None;
            }
            Some(iced::Task::none())
        }

        // 处理替换文本编辑器的动作
        ProjectMessage::FileTreeFindReplaceEditorAction(tab_id, action) => {
            // 在查找结果标签页列表中查找指定 ID 的标签页
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                // 执行编辑器动作
                tab.replace_editor.perform(action);
                // 同步编辑器内容到替换输入字符串
                tab.replace_input = tab.replace_editor.text().to_string();
            }
            Some(iced::Task::none())
        }

        // 切换大小写敏感选项
        ProjectMessage::FileTreeFindCaseSensitiveToggled(tab_id, v) => {
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                // 设置大小写敏感标志
                tab.case_sensitive = v;
            }
            Some(iced::Task::none())
        }

        // 切换全词匹配选项
        ProjectMessage::FileTreeFindWholeWordToggled(tab_id, v) => {
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                // 设置全词匹配标志
                tab.whole_word = v;
            }
            Some(iced::Task::none())
        }

        // 切换正则表达式模式选项
        ProjectMessage::FileTreeFindRegexToggled(tab_id, v) => {
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                // 设置正则表达式模式标志
                tab.use_regex = v;
            }
            Some(iced::Task::none())
        }

        ProjectMessage::FileTreeFindCaseSensitiveShortcut => {
            if let Some(tab_id) = app.active_find_results_tab_id.as_deref()
                && let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id)
            {
                tab.case_sensitive = !tab.case_sensitive;
            }
            Some(iced::Task::none())
        }

        ProjectMessage::FileTreeFindWholeWordShortcut => {
            if let Some(tab_id) = app.active_find_results_tab_id.as_deref()
                && let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id)
            {
                tab.whole_word = !tab.whole_word;
            }
            Some(iced::Task::none())
        }

        ProjectMessage::FileTreeFindRegexShortcut => {
            if let Some(tab_id) = app.active_find_results_tab_id.as_deref()
                && let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id)
            {
                tab.use_regex = !tab.use_regex;
            }
            Some(iced::Task::none())
        }

        ProjectMessage::FileTreeFindResultsPageChanged(tab_id, requested_page) => {
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                let (_, page, _) = find_results_page_bounds(tab.matches.len(), requested_page);
                tab.results_page = page;
            }
            Some(iced::Task::none())
        }

        // 执行查找操作
        ProjectMessage::FileTreeFindRun(tab_id) => {
            // 克隆标签页信息（因为后续需要在 app 中再次查找并修改）
            let Some(tab) = app.find_results_tabs.iter().find(|t| t.id == tab_id).cloned() else {
                return Some(iced::Task::none());
            };

            // 获取搜索范围路径
            let scope_path = tab.scope_path;
            // 获取并清理查询字符串
            let query = tab.query_input.trim().to_string();

            // 验证查询字符串是否为空
            if query.is_empty() {
                if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                    tab.error = Some("请输入查找关键字".to_string());
                }
                return Some(iced::Task::none());
            }

            // 收集查找所需的所有参数
            let replace_text = tab.replace_input; // 替换文本
            let case_sensitive = tab.case_sensitive; // 大小写敏感
            let whole_word = tab.whole_word; // 全词匹配
            let use_regex = tab.use_regex; // 正则表达式模式
            let files = app.current_file_index().to_vec(); // 当前文件索引
            let title = format!("查找: {}", query); // 标签页标题

            // 标记标签页为运行中状态，并清除之前的错误和限制标志
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                tab.running = true;
                tab.error = None;
                tab.limit_reached = false;
            }

            // 运行异步查找任务并返回任务句柄
            Some(run_find_task(
                tab_id,
                title,
                scope_path,
                query,
                replace_text,
                case_sensitive,
                whole_word,
                use_regex,
                files,
            ))
        }

        // 刷新当前活动的查找标签页
        ProjectMessage::FileTreeFindRefreshActive => {
            // 获取当前活动的查找标签页 ID
            let Some(active_id) = app.active_find_results_tab_id.clone() else {
                return Some(iced::Task::none());
            };

            // 查找并克隆活动标签页
            let Some(tab) = app.find_results_tabs.iter().find(|t| t.id == active_id).cloned()
            else {
                return Some(iced::Task::none());
            };

            // 获取并清理查询字符串
            let query = tab.query_input.trim().to_string();

            // 验证查询字符串是否为空
            if query.is_empty() {
                if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == active_id) {
                    tab.error = Some("请输入查找关键字".to_string());
                }
                return Some(iced::Task::none());
            }

            // 标记标签页为运行中状态
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == active_id) {
                tab.running = true;
                tab.error = None;
                tab.limit_reached = false;
            }

            // 运行异步查找任务
            Some(run_find_task(
                tab.id,
                format!("查找: {}", query),
                tab.scope_path,
                query,
                tab.replace_input,
                tab.case_sensitive,
                tab.whole_word,
                tab.use_regex,
                app.current_file_index().to_vec(),
            ))
        }

        // 在项目中查找或替换（创建新的查找标签页）
        ProjectMessage::FileTreeFindInProject | ProjectMessage::FileTreeReplaceInProject => {
            // 获取项目路径作为搜索范围
            let Some(scope_path) = app.project_path.clone() else {
                return Some(iced::Task::none());
            };

            // 使用当前时间戳生成唯一的标签页 ID
            let tab_id = format!("{}", now_ms());

            // 创建新的查找标签页并添加到列表
            app.find_results_tabs.push(FindInFolderTab {
                id: tab_id.clone(),                          // 唯一标识符
                title: "查找".to_string(),                   // 初始标题
                scope_path,                                  // 搜索范围路径
                query_input: String::new(),                  // 查询输入（初始为空）
                replace_input: String::new(),                // 替换输入（初始为空）
                query_editor: text_editor::Content::new(),   // 查询编辑器内容
                replace_editor: text_editor::Content::new(), // 替换编辑器内容
                query: String::new(),                        // 当前查询字符串
                replace_text: String::new(),                 // 当前替换文本
                case_sensitive: false,                       // 默认不区分大小写
                whole_word: false,                           // 默认不匹配全词
                use_regex: false,                            // 默认不使用正则
                running: false,                              // 初始不处于运行状态
                error: None,                                 // 无错误
                limit_reached: false,                        // 未达到限制
                matches: Vec::new(),                         // 匹配结果列表
                results_page: 0,                             // 从首屏开始展示结果
            });

            // 设置为活动标签页
            app.active_find_results_tab_id = Some(tab_id.clone());

            // 确保文件管理器可见
            app.show_file_manager = true;
            set_config_field("show_file_manager", serde_json::Value::Bool(true));

            // 生成打开标签页的 ID（格式：find:{tab_id}）
            let open_tab_id = format!("find:{}", tab_id);

            // 如果打开标签页列表中不存在此标签页，则添加
            if !app.open_tabs.iter().any(|t| t.id == open_tab_id) {
                app.open_tabs.push(AppTab {
                    id: open_tab_id.clone(),
                    title: format!("查找结果 {}", app.find_results_tabs.len()),
                    screen: Screen::Project,
                    project_path: app.project_path.clone(),
                });
            }

            // 设置为活动的打开标签页
            app.active_tab_id = Some(open_tab_id);
            app.screen = Screen::Project;

            Some(iced::Task::none())
        }

        // 处理查找任务完成的结果
        ProjectMessage::FileTreeFindCompleted {
            tab_id,         // 标签页 ID
            title,          // 标题
            scope_path,     // 搜索范围路径
            query,          // 查询字符串
            replace_text,   // 替换文本
            case_sensitive, // 大小写敏感
            whole_word,     // 全词匹配
            use_regex,      // 正则表达式模式
            matches,        // 匹配结果列表
            error,          // 错误信息（如果有）
            limit_reached,  // 是否达到结果限制
        } => {
            // 生成打开标签页的 ID
            let open_tab_id = format!("find:{}", tab_id);

            // 尝试更新现有标签页
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                // 更新标签页的所有属性
                tab.title = title;
                tab.scope_path = scope_path;
                tab.query = query;
                tab.replace_text = replace_text;
                tab.case_sensitive = case_sensitive;
                tab.whole_word = whole_word;
                tab.use_regex = use_regex;
                tab.running = false; // 标记为已完成
                tab.error = error;
                tab.limit_reached = limit_reached;
                tab.results_page = 0;

                // 只有在没有错误时才更新匹配结果
                if tab.error.is_none() {
                    tab.matches = matches;
                }
            } else {
                // 如果标签页不存在（可能是从其他来源完成的查找），创建新标签页
                app.find_results_tabs.push(FindInFolderTab {
                    id: tab_id.clone(),
                    title,
                    scope_path,
                    query_input: query.clone(),
                    replace_input: replace_text.clone(),
                    // 使用查询文本初始化编辑器
                    query_editor: text_editor::Content::with_text(&query),
                    replace_editor: text_editor::Content::with_text(&replace_text),
                    query,
                    replace_text,
                    case_sensitive,
                    whole_word,
                    use_regex,
                    running: false,
                    error,
                    limit_reached,
                    matches,
                    results_page: 0,
                });
            }

            // 设置为活动标签页
            app.active_find_results_tab_id = Some(tab_id.clone());

            // 确保文件管理器可见
            app.show_file_manager = true;
            set_config_field("show_file_manager", serde_json::Value::Bool(true));

            // 如果打开标签页列表中不存在此标签页，则添加
            if !app.open_tabs.iter().any(|t| t.id == open_tab_id) {
                app.open_tabs.push(AppTab {
                    id: open_tab_id.clone(),
                    title: format!("查找结果 {}", app.find_results_tabs.len()),
                    screen: Screen::Project,
                    project_path: app.project_path.clone(),
                });
            }

            // 设置为活动的打开标签页
            app.active_tab_id = Some(open_tab_id);
            app.screen = Screen::Project;

            Some(iced::Task::none())
        }

        // 打开匹配文件并定位到对应位置
        ProjectMessage::FileTreeFindMatchSelected { path, line, column } => {
            app.pending_preview_goto =
                Some((path.clone(), line.saturating_sub(1), column.saturating_sub(1)));
            Some(iced::Task::done(Message::Preview(
                crate::app::message::preview::PreviewMessage::Open(path),
            )))
        }

        // 选择查找标签页
        ProjectMessage::FileTreeFindTabSelected(id) => {
            if app.find_results_tabs.iter().any(|tab| tab.running) {
                return Some(iced::Task::none());
            }

            if id.is_empty() {
                // 空字符串表示没有选中的标签页
                app.active_find_results_tab_id = None;
            } else {
                // 设置为活动的查找标签页
                app.active_find_results_tab_id = Some(id);

                // 点击搜索标签时文件管理器已可见；避免在 UI 主线程同步读写配置文件。
                app.show_file_manager = true;
            }
            Some(iced::Task::none())
        }

        ProjectMessage::FileTreeFindTabRightClicked(id, x, y) => {
            app.find_results_tab_menu_id = Some(id);
            app.find_results_tab_menu_pos = Some(iced::Point::new(x, y));
            Some(iced::Task::none())
        }

        ProjectMessage::FileTreeFindTabMenuClose => {
            dismiss_find_tab_menu(app);
            Some(iced::Task::none())
        }

        ProjectMessage::FileTreeFindTabMenuCloseLeft(id) => {
            let Some(position) = app.find_results_tabs.iter().position(|tab| tab.id == id) else {
                dismiss_find_tab_menu(app);
                return Some(iced::Task::none());
            };
            let ids = app.find_results_tabs[..position]
                .iter()
                .map(|tab| tab.id.clone())
                .collect::<Vec<_>>();
            for tab_id in ids {
                close_find_results_tab(app, &tab_id);
            }
            dismiss_find_tab_menu(app);
            Some(iced::Task::none())
        }

        ProjectMessage::FileTreeFindTabMenuCloseRight(id) => {
            let Some(position) = app.find_results_tabs.iter().position(|tab| tab.id == id) else {
                dismiss_find_tab_menu(app);
                return Some(iced::Task::none());
            };
            let ids = app.find_results_tabs[position.saturating_add(1)..]
                .iter()
                .map(|tab| tab.id.clone())
                .collect::<Vec<_>>();
            for tab_id in ids {
                close_find_results_tab(app, &tab_id);
            }
            dismiss_find_tab_menu(app);
            Some(iced::Task::none())
        }

        ProjectMessage::FileTreeFindTabMenuCloseAll => {
            let ids = app.find_results_tabs.iter().map(|tab| tab.id.clone()).collect::<Vec<_>>();
            for tab_id in ids {
                close_find_results_tab(app, &tab_id);
            }
            dismiss_find_tab_menu(app);
            Some(iced::Task::none())
        }

        // 关闭查找标签页
        ProjectMessage::FileTreeFindTabClosed(id) => {
            close_find_results_tab(app, &id);
            Some(iced::Task::none())
        }

        // 其他消息类型不由此处理器处理，返回 None 交给其他处理器
        _ => None,
    }
}

#[cfg(test)]
#[path = "find_tests.rs"]
mod find_tests;
