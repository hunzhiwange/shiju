//! 聊天面板通用辅助函数。
//!
//! 本模块提供状态、路径、文本、主题、时间或菜单相关的小型工具，供聊天面板视图复用。

use web_time::{Duration as WebDuration, SystemTime as WebSystemTime};



/// 处理 get session title 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回字符串已经按界面展示或比较需求做过必要整理。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn get_session_title<'a>(active_session_id: Option<&str>, sessions: impl Iterator<Item = &'a session_info::Info>, i18n: desktop_i18n::I18n, render_title: impl Fn(&str, &str) -> String) -> String {
    if let Some(session_id) = active_session_id {
        if let Some(session) = sessions.into_iter().find(|session| session.id == session_id) {
            return render_title(&session.id, &session.title);
        }
    }
    i18n.__("暂无").to_string()
}

/// 处理 current project path label 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回字符串已经按界面展示或比较需求做过必要整理。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn current_project_path_label(project_path: Option<&str>, i18n: desktop_i18n::I18n) -> String {
    project_path.map(str::to_owned).unwrap_or_else(|| {
        i18n.__("未打开项目").to_string()
    })
}

/// 处理 current branch label 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回字符串已经按界面展示或比较需求做过必要整理。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn current_branch_label(selected_branch: Option<&str>) -> String {
    selected_branch.map(str::to_owned).unwrap_or_else(|| "-".to_string())
}

/// 处理 is recent copy 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `true` 表示当前输入满足该辅助函数描述的条件。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn is_recent_copy(last_copied_code_hash: Option<u64>, last_copy_time: Option<WebSystemTime>, content_hash: u64) -> bool {
    let Some(last_hash) = last_copied_code_hash else {
        return false;
    };
    if content_hash != last_hash {
        return false;
    }
    let Some(last_time) = last_copy_time else {
        return false;
    };
    WebSystemTime::now()
        .duration_since(last_time)
        .ok()
        .is_some_and(|d| d <= WebDuration::from_millis(1500))
}
