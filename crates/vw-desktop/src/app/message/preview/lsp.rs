//! LSP（语言服务器协议）预览模块
//!
//! 本模块为预览标签页中的代码编辑器提供完整的 LSP 支持功能。
//! 包括语法高亮、悬停提示、自动补全、跳转到定义等核心特性。
//!
//! # 主要功能
//!
//! - **LSP 服务器管理**：自动检测文件语言类型并启动对应的 LSP 服务器
//! - **悬停提示**：鼠标悬停时显示代码的详细文档和类型信息
//! - **自动补全**：提供智能代码补全建议，支持过滤和选择
//! - **跳转定义**：支持跳转到符号定义位置
//! - **进度追踪**：显示 LSP 服务器的初始化和工作进度
//!
//! # 架构说明
//!
//! 该模块采用事件驱动架构，通过 `LspEvent` 接收来自 LSP 服务器的异步消息，
//! 并通过定时器机制处理悬停提示的延迟显示和自动隐藏。

#![cfg(not(target_arch = "wasm32"))]

// 导入父模块的 PreviewMessage 消息类型
use super::PreviewMessage;
// 导入 LSP 配置和语言检测功能
use crate::app::lsp::config::lsp_language_for_path;
// 导入 LSP 事件类型
use crate::app::lsp::LspEvent;
// 导入应用相关的核心类型和工具函数
use crate::app::{App, Message, PreviewTab, lsp_root_uri_for_path, path_to_file_uri};
// 导入 Iced 框架的任务和 widget 相关类型
use iced::Task;
use iced::widget::operation::{focus, scroll_to};
use iced::widget::{Id, scrollable};
// 导入代码编辑器的 LSP 相关类型
use iced_code_editor::LspDocument;
// 导入标准库的路径和时间类型
use std::path::Path;
use std::time::{Duration, Instant};

/// 断开预览标签页与 LSP 服务器的连接
///
/// 清除标签页中所有与 LSP 相关的状态，包括：
/// - 从编辑器中分离 LSP 客户端
/// - 清除 LSP 服务器标识
/// - 清除 LSP URI 标识
/// - 清除语言 ID
///
/// # 参数
///
/// * `tab` - 需要断开 LSP 连接的预览标签页的可变引用
pub(crate) fn detach_lsp_for_tab(tab: &mut PreviewTab) {
    // 从编辑器中分离 LSP 客户端
    tab.editor.detach_lsp();

    // 清除 LSP 服务器标识
    tab.lsp_server_key = None;

    // 清除 LSP URI 标识
    tab.lsp_uri = None;

    // 清除语言 ID
    tab.lsp_language_id = None;
}

/// 为指定路径的文件同步 LSP 服务器
///
/// 根据文件路径自动检测语言类型，并启动或复用对应的 LSP 服务器。
/// 如果 LSP 服务器已启动且语言匹配，则复用现有连接；否则启动新的 LSP 实例。
///
/// # 参数
///
/// * `app` - 应用程序实例的可变引用
/// * `path` - 需要同步 LSP 的文件路径
///
/// # 返回值
///
/// 返回 `bool` 表示是否成功同步：
/// - `true`：LSP 服务器已就绪
/// - `false`：无法启动或连接 LSP 服务器
///
/// # 工作流程
///
/// 1. 查找文件对应的预览标签页
/// 2. 检测文件语言类型
/// 3. 获取 LSP 事件发送器
/// 4. 确定项目根目录 URI
/// 5. 如果已有匹配的 LSP 服务器，则复用
/// 6. 否则启动新的 LSP 服务器实例
pub(crate) fn sync_lsp_for_path(app: &mut App, path: &str) -> bool {
    if app.lsp_disabled {
        if let Some(tab) = app.preview_tabs.iter_mut().find(|tab| tab.path == path) {
            detach_lsp_for_tab(tab);
        }
        return false;
    }

    // 将路径字符串转换为 Path 对象
    let path_obj = Path::new(path);

    // 查找文件对应的预览标签页索引，不存在则返回失败
    let Some(tab_index) = app.preview_tabs.iter().position(|t| t.path == path) else {
        return false;
    };

    // 检测文件语言类型，无法识别则断开 LSP 并返回失败
    let Some(language) = lsp_language_for_path(path_obj) else {
        let tab = &mut app.preview_tabs[tab_index];
        detach_lsp_for_tab(tab);
        return false;
    };

    // 确定项目根目录 URI，无法确定则断开 LSP 并返回失败
    let Some(root_uri) = lsp_root_uri_for_path(Some(path_obj)) else {
        let tab = &mut app.preview_tabs[tab_index];
        detach_lsp_for_tab(tab);
        return false;
    };

    // 将文件路径转换为 file:// URI 格式
    let uri = path_to_file_uri(path_obj);

    // 检查是否可以复用现有的 LSP 服务器连接
    let res = {
        let tab = &mut app.preview_tabs[tab_index];
        // 如果 LSP 服务器标识匹配且编辑器已附加 LSP，则复用连接
        if tab.lsp_server_key == Some(language.server_key) && tab.editor.has_lsp_attached() {
            // 打开文档并更新相关状态
            tab.editor.lsp_open_document(LspDocument::new(uri.clone(), language.language_id));
            tab.lsp_uri = Some(uri.clone());
            tab.lsp_language_id = Some(language.language_id.to_string());
            true
        } else {
            false
        }
    };
    if res {
        return true;
    }

    // 无法复用现有连接，需要启动新的 LSP 服务器
    let (notify, status, success) = {
        let tab = &mut app.preview_tabs[tab_index];
        // 先断开旧的 LSP 连接
        detach_lsp_for_tab(tab);

        let Some(manager) = app.lsp_manager.as_mut() else {
            return false;
        };
        if let Some(project_path) = app.project_path.as_deref() {
            manager.prestart_for_project(project_path);
        }

        // 尝试附加或复用共享 LSP 服务
        match manager.get_or_create(language.server_key, &root_uri) {
            Ok(client) => {
                // 成功启动，附加 LSP 客户端到编辑器
                tab.editor.attach_lsp(client, LspDocument::new(uri.clone(), language.language_id));
                // 启用自动刷新，确保编辑内容及时同步到 LSP
                tab.editor.set_lsp_auto_flush(true);
                // 更新标签页的 LSP 相关状态
                tab.lsp_server_key = Some(language.server_key);
                tab.lsp_uri = Some(uri.clone());
                tab.lsp_language_id = Some(language.language_id.to_string());
                // 返回：无通知消息、服务器就绪状态、成功标志
                (None, Some(format!("{} 就绪", language.server_key)), true)
            }
            Err(err) => {
                // 启动失败，清除服务器标识
                tab.lsp_server_key = None;
                // 返回：错误通知、失败状态、失败标志
                (Some(format!("LSP 启动失败: {}", err)), Some(format!("LSP 失败: {}", err)), false)
            }
        }
    };

    // 如果有通知消息，推送到应用通知队列
    if let Some(message) = notify {
        app.push_notification(message);
    }
    // 更新全局 LSP 状态显示
    app.lsp_status = status;
    success
}

pub(crate) fn disable_lsp(app: &mut App) {
    app.lsp_disabled = true;
    clear_lsp_hover(app);
    clear_lsp_completion(app, false);
    for tab in &mut app.preview_tabs {
        detach_lsp_for_tab(tab);
    }
    app.lsp_progress.clear();
    app.lsp_status = Some("LSP 已禁用".to_string());
    if let Some(manager) = app.lsp_manager.as_mut() {
        manager.shutdown_all();
    }
}

pub(crate) fn enable_lsp(app: &mut App) {
    app.lsp_disabled = false;
    app.lsp_status = Some("LSP 已启用".to_string());
    if let (Some(manager), Some(project_path)) =
        (app.lsp_manager.as_mut(), app.project_path.as_deref())
    {
        manager.prestart_for_project(project_path);
    }
    if let Some(path) = app.active_preview_path.clone() {
        let _ = sync_lsp_for_path(app, &path);
    }
}

/// 清除 LSP 悬停提示
///
/// 完全清除当前显示的悬停提示及其相关状态：
/// - 清除悬停覆盖层
/// - 清除悬停锚点位置
/// - 清除待处理的悬停请求
/// - 清除悬停隐藏计时器
/// - 如果没有显示自动补全，同时清除 LSP 路径
///
/// # 参数
///
/// * `app` - 应用程序实例的可变引用
pub(crate) fn clear_lsp_hover(app: &mut App) {
    // 清除悬停覆盖层的内容
    app.lsp_overlay.clear_hover();

    // 清除悬停锚点（用于防抖）
    app.lsp_hover_anchor = None;

    // 清除待处理的悬停请求
    app.lsp_hover_pending = None;

    // 清除悬停隐藏计时器
    app.lsp_hover_hide_deadline = None;

    // 如果没有显示自动补全窗口，同时清除 LSP 路径
    // 这样可以避免在没有 LSP 功能时保持不必要的状态
    if !app.lsp_overlay.completion_visible {
        app.lsp_overlay_path = None;
    }
}

/// 清除 LSP 自动补全列表
///
/// 完全清除当前显示的自动补全列表及其相关状态：
/// - 清空所有补全项
/// - 清空已过滤的补全项
/// - 清空补全过滤器
/// - 隐藏补全窗口
/// - 重置选中索引
/// - 如果没有显示悬停提示，同时清除 LSP 路径
///
/// # 参数
///
/// * `app` - 应用程序实例的可变引用
/// * `suppress` - 是否抑制后续补全触发（防止补全窗口反复弹出）
pub(crate) fn clear_lsp_completion(app: &mut App, suppress: bool) {
    // 清空所有补全项（包括原始列表）
    app.lsp_overlay.all_completions.clear();

    // 清空已过滤的补全项
    app.lsp_overlay.completion_items.clear();

    // 清空补全过滤器
    app.lsp_overlay.completion_filter.clear();

    // 隐藏补全窗口
    app.lsp_overlay.completion_visible = false;

    // 设置抑制标志，防止补全窗口在短期内反复弹出
    app.lsp_overlay.completion_suppressed = suppress;

    // 重置选中索引为第一项
    app.lsp_overlay.completion_selected = 0;

    // 如果没有显示悬停提示，同时清除 LSP 路径
    // 这样可以避免在没有 LSP 功能时保持不必要的状态
    if !app.lsp_overlay.hover_visible {
        app.lsp_overlay_path = None;
    }
}

/// 处理 LSP 悬停提示的定时器逻辑
///
/// 本函数在每次定时器触发时调用，负责处理悬停提示的延迟显示和自动隐藏逻辑：
///
/// 1. **一致性检查**：如果悬停窗口可见但没有关联路径，立即清除悬停
/// 2. **延迟显示**：处理待处理的悬停请求，在达到延迟时间后发送实际的悬停请求
/// 3. **自动隐藏**：检查悬停隐藏截止时间，如果超时且用户未交互则自动隐藏
///
/// # 参数
///
/// * `app` - 应用程序实例的可变引用
///
/// # 悬停延迟机制
///
/// 为了避免鼠标快速划过时频繁触发悬停请求，系统会等待 400 毫秒后才发送请求。
/// 同时，当鼠标离开后也会等待 400 毫秒才隐藏悬停窗口。
pub(crate) fn process_lsp_hover_timers(app: &mut App) {
    // 获取当前时间，用于所有定时器检查
    let now = Instant::now();

    // 一致性检查：如果悬停窗口可见但没有关联的 LSP 路径，说明状态不一致，立即清除
    if app.lsp_overlay.hover_visible && app.lsp_overlay_path.is_none() {
        tracing::debug!("[LSP HOVER] clear_lsp_hover: hover_visible=true but path is None");
        clear_lsp_hover(app);
    }

    // 处理待处理的悬停请求
    if let Some(pending) = app.lsp_hover_pending.take() {
        tracing::debug!("[LSP HOVER] pending ready_at={:?}, now={:?}", pending.ready_at, now);
        // 检查是否达到延迟时间
        if now >= pending.ready_at {
            // 延迟时间已到，发送实际的悬停请求
            let request_sent =
                if let Some(tab) = app.preview_tabs.iter_mut().find(|t| t.path == pending.path) {
                    // 先刷新编辑器中的待处理更改，确保 LSP 看到最新内容
                    tab.editor.lsp_flush_pending_changes();
                    // 发送悬停请求到 LSP 服务器
                    let result = tab.editor.lsp_request_hover_at_position(pending.position);
                    tracing::debug!("[LSP HOVER] request_hover_at_position returned {:?}", result);
                    result
                } else {
                    // 找不到对应的标签页，无法发送请求
                    tracing::debug!("[LSP HOVER] tab not found for path {:?}", pending.path);
                    false
                };

            if request_sent {
                // 请求成功发送，设置悬停窗口的显示位置
                app.lsp_overlay.set_hover_position(pending.point);
                // 记录当前 LSP 路径
                app.lsp_overlay_path = Some(pending.path);
                tracing::debug!(
                    "[LSP HOVER] request sent, set_hover_position at {:?}",
                    pending.point
                );
            } else {
                // 请求发送失败，清除悬停锚点
                app.lsp_hover_anchor = None;
                tracing::debug!("[LSP HOVER] request NOT sent, cleared anchor");
            }
        } else {
            // 延迟时间未到，放回待处理队列等待下次检查
            app.lsp_hover_pending = Some(pending);
        }
    }

    // 检查悬停窗口是否需要自动隐藏
    if let Some(deadline) = app.lsp_hover_hide_deadline
        && now >= deadline // 已超过隐藏截止时间
        && !app.lsp_overlay.hover_interactive
    // 用户未在与悬停窗口交互
    {
        clear_lsp_hover(app);
    }
}

/// 过滤自动补全列表
///
/// 根据用户输入的过滤文本筛选补全项。过滤采用大小写不敏感的包含匹配。
/// 如果过滤文本为空，则显示所有补全项。
///
/// # 参数
///
/// * `app` - 应用程序实例的可变引用
///
/// # 过滤逻辑
///
/// - 将过滤文本转换为小写
/// - 如果过滤文本为空，显示所有补全项
/// - 否则只显示包含过滤文本的补全项（不区分大小写）
/// - 如果没有匹配项，隐藏补全窗口
/// - 自动调整选中索引，确保不越界
pub(crate) fn filter_completions(app: &mut App) {
    // 获取过滤文本并转换为小写（用于不区分大小写的匹配）
    let filter = app.lsp_overlay.completion_filter.to_lowercase();
    // 更新过滤器为小写版本
    app.lsp_overlay.completion_filter = filter.to_lowercase();

    // 根据过滤文本筛选补全项
    if filter.is_empty() {
        // 如果没有过滤文本，显示所有补全项
        app.lsp_overlay.completion_items = app.lsp_overlay.all_completions.clone();
    } else {
        // 否则只显示包含过滤文本的补全项（不区分大小写）
        app.lsp_overlay.completion_items = app
            .lsp_overlay
            .all_completions
            .iter()
            .filter(|item| item.to_lowercase().contains(&filter))
            .cloned()
            .collect();
    }

    // 根据是否有匹配项更新补全窗口的可见性
    app.lsp_overlay.completion_visible = !app.lsp_overlay.completion_items.is_empty();

    // 确保选中索引不越界（如果过滤后列表变短）
    if app.lsp_overlay.completion_selected >= app.lsp_overlay.completion_items.len() {
        app.lsp_overlay.completion_selected =
            app.lsp_overlay.completion_items.len().saturating_sub(1);
    }
}

/// 查找光标所在单词的起始位置
///
/// 从光标位置向前扫描，找到当前单词的起始列位置。
/// 单词定义为连续的字母数字字符或下划线。
///
/// # 参数
///
/// * `line` - 当前行内容
/// * `cursor_col` - 光标所在的列位置（0-based）
///
/// # 返回值
///
/// 返回单词起始的列位置（0-based）。如果光标在单词中间，
/// 返回该单词的第一个字符位置；如果光标不在单词中，返回光标位置本身。
///
/// # 示例
///
/// ```ignore
/// let line = "hello_world";
/// let start = find_word_start(line, 7); // 光标在 'o' 上
/// assert_eq!(start, 0); // 返回 'h' 的位置
/// ```
pub(super) fn find_word_start(line: &str, cursor_col: usize) -> usize {
    // 将行内容转换为字符数组，便于索引访问
    let chars: Vec<char> = line.chars().collect();

    // 从光标位置开始
    let mut word_start = cursor_col;

    // 向前扫描，查找单词的起始位置
    while word_start > 0 {
        // 获取光标前一个字符
        let ch = chars.get(word_start - 1).copied().unwrap_or(' ');

        // 如果字符不是字母数字或下划线，说明到达单词边界
        if !ch.is_alphanumeric() && ch != '_' {
            break;
        }

        // 继续向前扫描
        word_start -= 1;
    }

    word_start
}

/// 应用选中的自动补全项
///
/// 将用户选中的补全文本插入到当前光标位置，同时删除光标前的部分单词。
/// 这样可以实现智能的补全替换，而不是简单的插入。
///
/// # 参数
///
/// * `app` - 应用程序实例的可变引用
/// * `completion_text` - 要插入的补全文本
///
/// # 工作流程
///
/// 1. 获取当前活动预览标签页
/// 2. 找到光标所在位置
/// 3. 查找当前单词的起始位置
/// 4. 删除光标到单词起始位置之间的字符
/// 5. 逐字符插入补全文本
///
/// # 示例
///
/// 假设代码为 `let hel|`（| 表示光标），补全项为 `hello`：
/// - 删除 `hel`
/// - 插入 `hello`
/// - 最终结果为 `let hello|`
pub(crate) fn apply_completion(app: &mut App, completion_text: &str) {
    // 获取当前活动的预览路径，没有则直接返回
    let Some(path) = app.active_preview_path.clone() else {
        return;
    };

    // 查找对应的预览标签页
    if let Some(tab) = app.preview_tabs.iter_mut().find(|t| t.path.as_str() == path.as_str()) {
        // 获取编辑器内容和光标位置
        let content = tab.editor.content();
        let (line, col) = tab.editor.cursor_position();

        // 获取当前行的内容
        let line_content = content.lines().nth(line).unwrap_or("");

        // 查找光标所在单词的起始位置
        let word_start_col = find_word_start(line_content, col);

        // 计算需要删除的字符数（从单词起始位置到光标位置）
        let chars_to_delete = col.saturating_sub(word_start_col);

        // 删除光标前的单词部分
        for _ in 0..chars_to_delete {
            let _ = tab.editor.inner.update(&iced_code_editor::Message::Backspace);
        }

        // 逐字符插入补全文本
        for ch in completion_text.chars() {
            let _ = tab.editor.inner.update(&iced_code_editor::Message::CharacterInput(ch));
        }
    }
}

/// 处理鼠标触发的 LSP 悬停请求
///
/// 当鼠标在编辑器中移动时调用此函数，根据鼠标位置触发悬停提示。
/// 实现了智能的悬停交互逻辑，包括防抖、交互锁定和自动隐藏。
///
/// # 参数
///
/// * `app` - 应用程序实例的可变引用
/// * `point` - 鼠标的屏幕坐标位置
///
/// # 交互逻辑
///
/// 1. **上下文检查**：如果显示了上下文菜单或导航弹窗，跳过悬停
/// 2. **交互锁定**：如果用户正在与悬停窗口交互（hover_interactive），保持显示
/// 3. **锚点查找**：根据鼠标位置查找可悬停的代码元素
/// 4. **延迟请求**：设置 400 毫秒的延迟，避免频繁触发
/// 5. **自动隐藏**：如果鼠标移动到无效位置，设置隐藏计时器
///
/// # 防抖机制
///
/// 系统会记录当前的悬停锚点（文件路径 + LSP 位置），只有当鼠标移动到
/// 不同的代码元素时才会触发新的悬停请求，避免重复查询。
pub(crate) fn handle_lsp_hover_from_mouse(app: &mut App, point: iced::Point) {
    // 如果显示了上下文菜单或导航弹窗，不触发悬停提示
    if app.show_preview_context_menu || app.preview_nav_popup.is_some() {
        return;
    }

    // 获取当前活动的预览路径，没有则直接返回
    let Some(path) = app.active_preview_path.clone() else {
        tracing::debug!("[LSP HOVER] handle_lsp_hover_from_mouse: no active_preview_path");
        return;
    };

    // 处理用户正在与悬停窗口交互的情况
    if app.lsp_overlay.hover_interactive {
        // 如果悬停窗口不可见或路径不匹配，解除交互锁定
        if !app.lsp_overlay.hover_visible || app.lsp_overlay_path.as_ref() != Some(&path) {
            app.lsp_overlay.hover_interactive = false;
            app.lsp_hover_hide_deadline = None;
            app.lsp_hover_pending = None;
        } else {
            // 用户正在交互，保持显示，不处理新的悬停请求
            tracing::debug!("[LSP HOVER] hover_interactive=true, skipping");
            return;
        }
    }

    // 根据鼠标位置查找可悬停的代码元素锚点
    let anchor =
        if let Some(tab) = app.preview_tabs.iter().find(|t| t.path.as_str() == path.as_str()) {
            tab.editor.lsp_hover_anchor_at_point(point)
        } else {
            None
        };

    // 如果没有找到锚点，处理隐藏逻辑
    let Some((position, anchor_point)) = anchor else {
        tracing::debug!("[LSP HOVER] no anchor at point {:?}", point);
        // 如果当前路径的悬停窗口仍然可见，保持显示（可能鼠标还在同一个元素上）
        if app.lsp_overlay.hover_visible && app.lsp_overlay_path.as_ref() == Some(&path) {
            return;
        }
        // 如果悬停窗口可见但鼠标已移到无效位置，设置延迟隐藏
        if app.lsp_overlay.hover_visible {
            app.lsp_hover_hide_deadline = Some(Instant::now() + Duration::from_millis(400));
        }
        return;
    };

    tracing::debug!(
        "[LSP HOVER] got anchor: position={:?}, anchor_point={:?}",
        position,
        anchor_point
    );

    // 更新悬停锚点（用于防抖，避免重复查询）
    app.lsp_hover_anchor = Some((path.clone(), position));
    app.lsp_overlay.hover_interactive = false;

    // 创建待处理的悬停请求，设置 400 毫秒的延迟
    app.lsp_hover_pending = Some(crate::app::LspHoverPending {
        path,
        position,
        point: anchor_point,
        ready_at: Instant::now() + Duration::from_millis(400),
    });

    // 清除之前的隐藏计时器（因为有了新的悬停请求）
    app.lsp_hover_hide_deadline = None;
}

/// 将 file:// URI 转换为本地文件路径
///
/// 解析 LSP 返回的 file:// 格式 URI，提取本地文件系统路径。
/// 同时处理 URI 中的转义字符（如 %20 -> 空格）。
///
/// # 参数
///
/// * `uri` - file:// 格式的 URI 字符串
///
/// # 返回值
///
/// - `Some(String)`：成功解析的本地路径
/// - `None`：如果 URI 不是 file:// 格式
///
/// # 示例
///
/// ```ignore
/// let uri = "file:///path/to/my%20file.rs";
/// let path = file_uri_to_path(uri);
/// assert_eq!(path, Some("/path/to/my file.rs".to_string()));
/// ```
pub(super) fn file_uri_to_path(uri: &str) -> Option<String> {
    // 移除 "file://" 前缀，并将 URI 转义字符替换为实际字符
    uri.strip_prefix("file://").map(|path| path.replace("%20", " "))
}

/// 处理所有待处理的 LSP 事件
///
/// 从 LSP 事件接收器中读取并处理所有待处理的事件，将其转换为相应的 UI 更新和任务。
/// 这是 LSP 事件驱动系统的核心处理函数。
///
/// # 参数
///
/// * `app` - 应用程序实例的可变引用
///
/// # 返回值
///
/// 返回一个 `Task<Message>`，包含需要执行的所有异步任务（如跳转到定义）。
///
/// # 支持的事件类型
///
/// - **Hover**：显示悬停提示信息
/// - **Completion**：显示自动补全列表
/// - **Definition**：跳转到符号定义位置
/// - **Progress**：更新 LSP 服务器进度状态
/// - **Log**：显示 LSP 日志消息
///
/// # 事件处理流程
///
/// 循环读取事件队列直到为空或通道断开：
/// 1. 对于 Hover 事件，显示或清除悬停提示
/// 2. 对于 Completion 事件，更新补全列表和位置
/// 3. 对于 Definition 事件，创建跳转任务
/// 4. 对于 Progress 事件，更新进度状态栏
/// 5. 对于 Log 事件，显示通知
pub(crate) fn drain_lsp_events(app: &mut App) -> Task<Message> {
    // 尝试获取 LSP 事件接收器，没有则返回空任务
    let Some(receiver) = app.lsp_events.take() else {
        return Task::none();
    };

    // 收集所有需要执行的跳转任务（用于跳转到定义功能）
    let mut jump_tasks = Vec::new();

    // 循环处理所有待处理的事件
    loop {
        match receiver.try_recv() {
            Ok(event) => match event {
                // 处理悬停提示事件
                LspEvent::Hover { text } => {
                    tracing::debug!(
                        "[LSP HOVER] received Hover event, text.len={}, hover_visible={}",
                        text.len(),
                        app.lsp_overlay.hover_visible
                    );
                    // 如果文本为空，清除悬停提示
                    if text.trim().is_empty() {
                        clear_lsp_hover(app);
                    } else {
                        app.lsp_overlay.show_hover(text);
                        app.lsp_hover_hide_deadline = None;
                        if app.lsp_overlay_path.is_none() {
                            app.lsp_overlay_path = app.active_preview_path.clone();
                        }
                        tracing::debug!(
                            "[LSP HOVER] show_hover called, hover_visible={}, hover_position={:?}",
                            app.lsp_overlay.hover_visible,
                            app.lsp_overlay.hover_position
                        );
                    }
                }
                // 处理自动补全事件
                LspEvent::Completion { items } => {
                    // 计算补全窗口的显示位置（基于光标的屏幕位置）
                    let position = app
                        .active_preview_path
                        .as_ref()
                        .and_then(|path| {
                            app.preview_tabs
                                .iter()
                                .find(|t| &t.path == path)
                                .and_then(|tab| tab.editor.cursor_screen_position())
                        })
                        .unwrap_or(iced::Point::new(4.0, 4.0));

                    // 设置补全项列表和显示位置
                    app.lsp_overlay.set_completions(items, position);

                    // 更新补全窗口的精确位置（用于滚动和布局）
                    if let Some(path) = app.active_preview_path.clone()
                        && let Some(tab) =
                            app.preview_tabs.iter().find(|t| t.path.as_str() == path.as_str())
                    {
                        app.lsp_overlay.completion_position = tab.editor.cursor_screen_position();
                    }

                    // 如果补全窗口可见且没有关联路径，设置当前路径
                    if app.lsp_overlay_path.is_none() && app.lsp_overlay.completion_visible {
                        app.lsp_overlay_path = app.active_preview_path.clone();
                    }
                }
                // 处理跳转到定义事件
                LspEvent::Definition { uri, range } => {
                    // 将 URI 转换为本地路径
                    if let Some(path) = file_uri_to_path(&uri) {
                        // 记录待跳转的位置信息
                        app.pending_preview_goto = Some((
                            path.clone(),
                            range.start.line as usize,
                            range.start.character as usize,
                        ));
                        // 创建打开文件的任务
                        jump_tasks
                            .push(Task::done(Message::Preview(PreviewMessage::Open(path.clone()))));
                    }
                }
                // 处理 LSP 服务器进度事件
                LspEvent::Progress { token, server_key, title, message, percentage, done } => {
                    if done {
                        // 进度完成，从进度映射中移除
                        if let Some(map) = app.lsp_progress.get_mut(&server_key) {
                            map.remove(&token);
                            // 如果映射为空，移除整个服务器的进度记录
                            if map.is_empty() {
                                app.lsp_progress.remove(&server_key);
                            }
                        }
                        // 更新状态为就绪
                        app.lsp_status = Some(format!("{} Ready", server_key));
                    } else {
                        // 进度进行中，更新状态和进度映射
                        app.lsp_status = Some(format!("{} {}", server_key, title));
                        app.lsp_progress
                            .entry(server_key)
                            .or_default()
                            .insert(token, crate::app::LspProgress { title, message, percentage });
                    }
                }
                // 处理 LSP 日志事件
                LspEvent::Log { server_key: _, message } => {
                    // 将日志显示为应用通知
                    app.push_notification(format!("LSP 日志: {}", message));
                }
            },
            // 事件队列为空，将接收器放回并退出循环
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                app.lsp_events = Some(receiver);
                break;
            }
            // 通道已断开，清除接收器并退出循环
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                app.lsp_events = None;
                break;
            }
        }
    }

    // 返回所有跳转任务的批处理结果
    if jump_tasks.is_empty() { Task::none() } else { Task::batch(jump_tasks) }
}

/// 应用程序定时器触发函数
///
/// 每隔固定时间间隔调用一次，处理需要定期更新的任务：
/// - 更新加载动画帧计数器
/// - 处理悬停提示定时器（延迟显示/自动隐藏）
/// - 处理待处理的 LSP 事件
///
/// # 参数
///
/// * `app` - 应用程序实例的可变引用
///
/// # 返回值
///
/// 返回由事件处理产生的所有任务。
pub(crate) fn tick(app: &mut App) -> Task<Message> {
    // 更新加载动画的帧计数器（用于显示旋转动画）
    app.spinner_frame = app.spinner_frame.wrapping_add(1);

    if app.lsp_disabled {
        return Task::none();
    }

    // 处理悬停提示的定时器逻辑（延迟显示和自动隐藏）
    process_lsp_hover_timers(app);

    // 处理所有待处理的 LSP 事件并返回产生的任务
    drain_lsp_events(app)
}

/// 将焦点设置到悬停覆盖层
///
/// 用于键盘交互场景，允许用户使用键盘在悬停窗口中导航和复制内容。
///
/// # 返回值
///
/// 返回一个焦点设置任务，目标 ID 为 "preview_hover_overlay"。
pub(crate) fn focus_hover_overlay() -> Task<Message> {
    focus(Id::new("preview_hover_overlay"))
}

/// 创建自动补全列表的滚动任务
///
/// 根据当前选中的补全项索引，计算并创建滚动任务，确保选中项在视口中可见。
/// 每个补全项高度为 20 像素。
///
/// # 参数
///
/// * `selected` - 当前选中的补全项索引（0-based）
///
/// # 返回值
///
/// 返回一个滚动任务，将补全列表滚动到选中项位置。
///
/// # 滚动计算
///
/// 滚动位置 = 选中索引 × 每项高度（20 像素）
pub(crate) fn completion_scroll_task(selected: usize) -> Task<Message> {
    // 计算滚动位置：选中索引 × 每项高度（20 像素）
    let scroll_y = selected as f32 * 20.0;

    // 创建滚动到指定位置的任务
    scroll_to(
        Id::new("preview_completion_scrollable"),
        scrollable::AbsoluteOffset { x: 0.0, y: scroll_y },
    )
}
