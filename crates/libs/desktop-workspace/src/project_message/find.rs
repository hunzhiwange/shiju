use desktop_file_tree::state::{FindInFolderTab, FindInFolderMatch, find_results_page_bounds};
use iced::widget::text_editor;
use std::path::{Component, Path, PathBuf};
#[derive(Debug, Clone)]
pub enum FindMessage {
    FileTreeFindScopeChanged(String, String),
    FileTreeFindQueryEditorAction(String, text_editor::Action),
    FileTreeFindReplaceEditorAction(String, text_editor::Action),
    FileTreeFindCaseSensitiveToggled(String, bool),
    FileTreeFindWholeWordToggled(String, bool),
    FileTreeFindRegexToggled(String, bool),
    FileTreeFindCaseSensitiveShortcut,
    FileTreeFindWholeWordShortcut,
    FileTreeFindRegexShortcut,
    FileTreeFindResultsPageChanged(String, usize),
    FileTreeFindRun(String),
    FileTreeFindRefreshActive,
    FileTreeFindInProject,
    FileTreeReplaceInProject,
    FileTreeFindCompleted {
        tab_id: String,
        title: String,
        scope_path: String,
        query: String,
        replace_text: String,
        case_sensitive: bool,
        whole_word: bool,
        use_regex: bool,
        matches: Vec<FindInFolderMatch>,
        error: Option<String>,
        localized_error: Option<desktop_i18n::input::Feedback>,
        limit_reached: bool,
    },
    FileTreeFindMatchSelected {
        path: String,
        line: usize,
        column: usize,
    },
    FileTreeFindTabSelected(String),
    FileTreeFindTabClosed(String),
    FileTreeFindTabRightClicked(String, f32, f32),
    FileTreeFindTabMenuClose,
    FileTreeFindTabMenuCloseLeft(String),
    FileTreeFindTabMenuCloseRight(String),
    FileTreeFindTabMenuCloseAll,
}
#[derive(Debug, Default)]
pub struct FindState {
 pub find_results_tabs: Vec<FindInFolderTab>,
 pub active_find_results_tab_id: Option<String>,
 pub find_results_tab_menu_id: Option<String>,
 pub find_results_tab_menu_pos: Option<iced::Point>,
 effects: Vec<FindEffect>,
}
#[derive(Debug)]
pub enum FindEffect {
 None,
 Open { id: String, count: usize },
 Close { id: String, closing_active_find: bool, active_find: Option<String> },
 Preview { path: String, line: usize, column: usize },
 Search { tab_id: String, title: String, scope_path: String, query: String, replace_text: String, case_sensitive: bool, whole_word: bool, use_regex: bool, files: Vec<String> },
 ShowFileManager,
}
fn run_find_task(tab_id: String, title: String, scope_path: String, query: String, replace_text: String, case_sensitive: bool, whole_word: bool, use_regex: bool, files: Vec<String>) -> FindEffect {
 FindEffect::Search {tab_id,title,scope_path,query,replace_text,case_sensitive,whole_word,use_regex,files}
}
use runtime_time::now_ms;
const FIND_SCOPE_ERROR: &str = "作用域必须是项目内的相对路径";

fn dismiss_find_tab_menu(app: &mut FindState) {
    app.find_results_tab_menu_id = None;
    app.find_results_tab_menu_pos = None;
}

fn close_find_results_tab(app: &mut FindState, id: &str) {
 let closing_active_find = app.active_find_results_tab_id.as_deref() == Some(id);
 app.find_results_tabs.retain(|tab| tab.id != id);
 if closing_active_find { app.active_find_results_tab_id = app.find_results_tabs.last().map(|tab| tab.id.clone()); }
 app.effects.push(FindEffect::Close { id: id.to_owned(), closing_active_find, active_find: app.active_find_results_tab_id.clone() });
 if app.find_results_tab_menu_id.as_deref() == Some(id) { dismiss_find_tab_menu(app); }
}

pub fn resolve_find_scope_path(project_root: &str, input: &str) -> Result<String, &'static str> {
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

/// 更新查找状态，返回由桌面壳执行的跨域效果。
fn update(app: &mut FindState, project_path: Option<&str>, files: Vec<String>, message: FindMessage) -> Option<FindEffect> {
    match message {
        FindMessage::FileTreeFindScopeChanged(tab_id, value) => {
            let Some(project_root) = project_path.map(str::to_owned) else {
                return Some(FindEffect::None);
            };
            let resolved = resolve_find_scope_path(&project_root, &value);
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|tab| tab.id == tab_id) {
                match resolved {
                    Ok(scope_path) => {
                        tab.scope_path = scope_path;
                        tab.error = None;
                        tab.localized_error = None;
                    }
                    Err(error) => {
                        tab.error = Some(error.to_string());
                        tab.localized_error =
                            Some(desktop_i18n::input::Feedback::new(error, vec![]));
                    }
                }
            }
            Some(FindEffect::None)
        }

        // 处理查找查询编辑器的动作（如输入、删除、粘贴等）
        FindMessage::FileTreeFindQueryEditorAction(tab_id, action) => {
            // 在查找结果标签页列表中查找指定 ID 的标签页
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                // 执行编辑器动作（如光标移动、文本编辑等）
                tab.query_editor.perform(action);
                // 同步编辑器内容到查询输入字符串
                tab.query_input = tab.query_editor.text().to_string();
                // 清除之前的错误信息
                tab.error = None;
                tab.localized_error = None;
            }
            Some(FindEffect::None)
        }

        // 处理替换文本编辑器的动作
        FindMessage::FileTreeFindReplaceEditorAction(tab_id, action) => {
            // 在查找结果标签页列表中查找指定 ID 的标签页
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                // 执行编辑器动作
                tab.replace_editor.perform(action);
                // 同步编辑器内容到替换输入字符串
                tab.replace_input = tab.replace_editor.text().to_string();
            }
            Some(FindEffect::None)
        }

        // 切换大小写敏感选项
        FindMessage::FileTreeFindCaseSensitiveToggled(tab_id, v) => {
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                // 设置大小写敏感标志
                tab.case_sensitive = v;
            }
            Some(FindEffect::None)
        }

        // 切换全词匹配选项
        FindMessage::FileTreeFindWholeWordToggled(tab_id, v) => {
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                // 设置全词匹配标志
                tab.whole_word = v;
            }
            Some(FindEffect::None)
        }

        // 切换正则表达式模式选项
        FindMessage::FileTreeFindRegexToggled(tab_id, v) => {
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                // 设置正则表达式模式标志
                tab.use_regex = v;
            }
            Some(FindEffect::None)
        }

        FindMessage::FileTreeFindCaseSensitiveShortcut => {
            if let Some(tab_id) = app.active_find_results_tab_id.as_deref()
                && let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id)
            {
                tab.case_sensitive = !tab.case_sensitive;
            }
            Some(FindEffect::None)
        }

        FindMessage::FileTreeFindWholeWordShortcut => {
            if let Some(tab_id) = app.active_find_results_tab_id.as_deref()
                && let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id)
            {
                tab.whole_word = !tab.whole_word;
            }
            Some(FindEffect::None)
        }

        FindMessage::FileTreeFindRegexShortcut => {
            if let Some(tab_id) = app.active_find_results_tab_id.as_deref()
                && let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id)
            {
                tab.use_regex = !tab.use_regex;
            }
            Some(FindEffect::None)
        }

        FindMessage::FileTreeFindResultsPageChanged(tab_id, requested_page) => {
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                let (_, page, _) = find_results_page_bounds(tab.matches.len(), requested_page);
                tab.results_page = page;
            }
            Some(FindEffect::None)
        }

        // 执行查找操作
        FindMessage::FileTreeFindRun(tab_id) => {
            // 克隆标签页信息（因为后续需要在 app 中再次查找并修改）
            let Some(tab) = app.find_results_tabs.iter().find(|t| t.id == tab_id).cloned() else {
                return Some(FindEffect::None);
            };

            // 获取搜索范围路径
            let scope_path = tab.scope_path;
            // 获取并清理查询字符串
            let query = tab.query_input.trim().to_string();

            // 验证查询字符串是否为空
            if query.is_empty() {
                if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                    tab.error = Some("请输入查找关键字".to_string());
                    tab.localized_error =
                        Some(desktop_i18n::input::Feedback::new("请输入查找关键字", vec![]));
                }
                return Some(FindEffect::None);
            }

            // 收集查找所需的所有参数
            let replace_text = tab.replace_input; // 替换文本
            let case_sensitive = tab.case_sensitive; // 大小写敏感
            let whole_word = tab.whole_word; // 全词匹配
            let use_regex = tab.use_regex; // 正则表达式模式
            let files = files; // 当前文件索引
            let title = format!("查找: {}", query); // 标签页标题

            // 标记标签页为运行中状态，并清除之前的错误和限制标志
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == tab_id) {
                tab.running = true;
                tab.error = None;
                tab.localized_error = None;
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
        FindMessage::FileTreeFindRefreshActive => {
            // 获取当前活动的查找标签页 ID
            let Some(active_id) = app.active_find_results_tab_id.clone() else {
                return Some(FindEffect::None);
            };

            // 查找并克隆活动标签页
            let Some(tab) = app.find_results_tabs.iter().find(|t| t.id == active_id).cloned()
            else {
                return Some(FindEffect::None);
            };

            // 获取并清理查询字符串
            let query = tab.query_input.trim().to_string();

            // 验证查询字符串是否为空
            if query.is_empty() {
                if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == active_id) {
                    tab.error = Some("请输入查找关键字".to_string());
                    tab.localized_error =
                        Some(desktop_i18n::input::Feedback::new("请输入查找关键字", vec![]));
                }
                return Some(FindEffect::None);
            }

            // 标记标签页为运行中状态
            if let Some(tab) = app.find_results_tabs.iter_mut().find(|t| t.id == active_id) {
                tab.running = true;
                tab.error = None;
                tab.localized_error = None;
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
                files,
            ))
        }

        // 在项目中查找或替换（创建新的查找标签页）
        FindMessage::FileTreeFindInProject | FindMessage::FileTreeReplaceInProject => {
            // 获取项目路径作为搜索范围
            let Some(scope_path) = project_path.map(str::to_owned) else {
                return Some(FindEffect::None);
            };

            // 使用当前时间戳生成唯一的标签页 ID
            let tab_id = format!("{}", now_ms());

            // 创建新的查找标签页并添加到列表
            app.open_scope(scope_path, tab_id.clone());

            // 确保文件管理器可见
            app.effects.push(FindEffect::ShowFileManager);

            // 生成打开标签页的 ID（格式：find:{tab_id}）
            Some(FindEffect::Open { id: tab_id, count: app.find_results_tabs.len() })
        }

        // 处理查找任务完成的结果
        FindMessage::FileTreeFindCompleted {
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
            localized_error,
            limit_reached, // 是否达到结果限制
        } => {

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
                tab.localized_error = localized_error;
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
                    localized_error,
                    results_page: 0,
                });
            }

            // 设置为活动标签页
            app.active_find_results_tab_id = Some(tab_id.clone());

            // 确保文件管理器可见
            app.effects.push(FindEffect::ShowFileManager);

            Some(FindEffect::Open { id: tab_id, count: app.find_results_tabs.len() })
        }

        // 打开匹配文件并定位到对应位置
        FindMessage::FileTreeFindMatchSelected { path, line, column } => {
            Some(FindEffect::Preview { path, line, column })
        }

        // 选择查找标签页
        FindMessage::FileTreeFindTabSelected(id) => {
            if app.find_results_tabs.iter().any(|tab| tab.running) {
                return Some(FindEffect::None);
            }

            if id.is_empty() {
                // 空字符串表示没有选中的标签页
                app.active_find_results_tab_id = None;
            } else {
                // 设置为活动的查找标签页
                app.active_find_results_tab_id = Some(id);

                // 点击搜索标签时文件管理器已可见；避免在 UI 主线程同步读写配置文件。
                app.effects.push(FindEffect::ShowFileManager);
            }
            Some(FindEffect::None)
        }

        FindMessage::FileTreeFindTabRightClicked(id, x, y) => {
            app.find_results_tab_menu_id = Some(id);
            app.find_results_tab_menu_pos = Some(iced::Point::new(x, y));
            Some(FindEffect::None)
        }

        FindMessage::FileTreeFindTabMenuClose => {
            dismiss_find_tab_menu(app);
            Some(FindEffect::None)
        }

        FindMessage::FileTreeFindTabMenuCloseLeft(id) => {
            let Some(position) = app.find_results_tabs.iter().position(|tab| tab.id == id) else {
                dismiss_find_tab_menu(app);
                return Some(FindEffect::None);
            };
            let ids = app.find_results_tabs[..position]
                .iter()
                .map(|tab| tab.id.clone())
                .collect::<Vec<_>>();
            for tab_id in ids {
                close_find_results_tab(app, &tab_id);
            }
            dismiss_find_tab_menu(app);
            Some(FindEffect::None)
        }

        FindMessage::FileTreeFindTabMenuCloseRight(id) => {
            let Some(position) = app.find_results_tabs.iter().position(|tab| tab.id == id) else {
                dismiss_find_tab_menu(app);
                return Some(FindEffect::None);
            };
            let ids = app.find_results_tabs[position.saturating_add(1)..]
                .iter()
                .map(|tab| tab.id.clone())
                .collect::<Vec<_>>();
            for tab_id in ids {
                close_find_results_tab(app, &tab_id);
            }
            dismiss_find_tab_menu(app);
            Some(FindEffect::None)
        }

        FindMessage::FileTreeFindTabMenuCloseAll => {
            let ids = app.find_results_tabs.iter().map(|tab| tab.id.clone()).collect::<Vec<_>>();
            for tab_id in ids {
                close_find_results_tab(app, &tab_id);
            }
            dismiss_find_tab_menu(app);
            Some(FindEffect::None)
        }

        // 关闭查找标签页
        FindMessage::FileTreeFindTabClosed(id) => {
            close_find_results_tab(app, &id);
            Some(FindEffect::None)
        }

        // 其他消息类型不由此处理器处理，返回 None 交给其他处理器

    }
}


pub fn handle(state: &mut FindState, project_path: Option<&str>, files: Vec<String>, message: FindMessage) -> Vec<FindEffect> {
 if let Some(effect) = update(state, project_path, files, message) { state.effects.push(effect); }
 std::mem::take(&mut state.effects)
}

#[cfg(test)]
#[path = "find_tests.rs"]
mod find_tests;

impl FindState {
    pub fn open_scope(&mut self, scope_path: String, tab_id: String) {
            self.find_results_tabs.push(FindInFolderTab {
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
                localized_error: None,
                results_page: 0, // 从首屏开始展示结果
            });

            // 设置为活动标签页
            self.active_find_results_tab_id = Some(tab_id.clone());

    }
}

impl<P, H, U> TryFrom<super::message::ProjectMessage<P, H, U>> for FindMessage {
    type Error = ();
    fn try_from(message: super::message::ProjectMessage<P, H, U>) -> Result<Self, Self::Error> {
        Ok(match message {
        super::message::ProjectMessage::FileTreeFindScopeChanged(v0, v1) => {
            FindMessage::FileTreeFindScopeChanged(v0, v1)
        }
        super::message::ProjectMessage::FileTreeFindQueryEditorAction(v0, v1) => {
            FindMessage::FileTreeFindQueryEditorAction(v0, v1)
        }
        super::message::ProjectMessage::FileTreeFindReplaceEditorAction(v0, v1) => {
            FindMessage::FileTreeFindReplaceEditorAction(v0, v1)
        }
        super::message::ProjectMessage::FileTreeFindCaseSensitiveToggled(v0, v1) => {
            FindMessage::FileTreeFindCaseSensitiveToggled(v0, v1)
        }
        super::message::ProjectMessage::FileTreeFindWholeWordToggled(v0, v1) => {
            FindMessage::FileTreeFindWholeWordToggled(v0, v1)
        }
        super::message::ProjectMessage::FileTreeFindRegexToggled(v0, v1) => {
            FindMessage::FileTreeFindRegexToggled(v0, v1)
        }
        super::message::ProjectMessage::FileTreeFindCaseSensitiveShortcut => {
            FindMessage::FileTreeFindCaseSensitiveShortcut
        }
        super::message::ProjectMessage::FileTreeFindWholeWordShortcut => FindMessage::FileTreeFindWholeWordShortcut,
        super::message::ProjectMessage::FileTreeFindRegexShortcut => FindMessage::FileTreeFindRegexShortcut,
        super::message::ProjectMessage::FileTreeFindResultsPageChanged(v0, v1) => {
            FindMessage::FileTreeFindResultsPageChanged(v0, v1)
        }
        super::message::ProjectMessage::FileTreeFindRun(v0) => FindMessage::FileTreeFindRun(v0),
        super::message::ProjectMessage::FileTreeFindRefreshActive => FindMessage::FileTreeFindRefreshActive,
        super::message::ProjectMessage::FileTreeFindInProject => FindMessage::FileTreeFindInProject,
        super::message::ProjectMessage::FileTreeReplaceInProject => FindMessage::FileTreeReplaceInProject,
        super::message::ProjectMessage::FileTreeFindCompleted {
            tab_id,
            title,
            scope_path,
            query,
            replace_text,
            case_sensitive,
            whole_word,
            use_regex,
            matches,
            error,
            localized_error,
            limit_reached,
        } => FindMessage::FileTreeFindCompleted {
            tab_id,
            title,
            scope_path,
            query,
            replace_text,
            case_sensitive,
            whole_word,
            use_regex,
            matches,
            error,
            localized_error,
            limit_reached,
        },
        super::message::ProjectMessage::FileTreeFindMatchSelected { path, line, column } => {
            FindMessage::FileTreeFindMatchSelected { path, line, column }
        }
        super::message::ProjectMessage::FileTreeFindTabSelected(v0) => FindMessage::FileTreeFindTabSelected(v0),
        super::message::ProjectMessage::FileTreeFindTabClosed(v0) => FindMessage::FileTreeFindTabClosed(v0),
        super::message::ProjectMessage::FileTreeFindTabRightClicked(v0, v1, v2) => {
            FindMessage::FileTreeFindTabRightClicked(v0, v1, v2)
        }
        super::message::ProjectMessage::FileTreeFindTabMenuClose => FindMessage::FileTreeFindTabMenuClose,
        super::message::ProjectMessage::FileTreeFindTabMenuCloseLeft(v0) => {
            FindMessage::FileTreeFindTabMenuCloseLeft(v0)
        }
        super::message::ProjectMessage::FileTreeFindTabMenuCloseRight(v0) => {
            FindMessage::FileTreeFindTabMenuCloseRight(v0)
        }
        super::message::ProjectMessage::FileTreeFindTabMenuCloseAll => FindMessage::FileTreeFindTabMenuCloseAll,
        _ => return Err(()),
        })
    }
}
