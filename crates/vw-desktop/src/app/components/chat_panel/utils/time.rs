//! 聊天面板通用辅助函数。
//!
//! 本模块提供状态、路径、文本、主题、时间或菜单相关的小型工具，供聊天面板视图复用。

use once_cell::sync::Lazy;
/// 重新导出 use std::time::UNIX_EPOCH，让上层模块通过稳定路径访问。
#[cfg(not(target_arch = "wasm32"))]
use std::time::UNIX_EPOCH;
/// 重新导出 use time::format_description::FormatItem，让上层模块通过稳定路径访问。
use time::format_description::FormatItem;

/// 重新导出 use crate::app::App，让上层模块通过稳定路径访问。
use crate::app::App;

/// 处理 project last modified ms 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `None` 表示输入缺少必要字段、当前状态不匹配，或该视图片段不需要展示。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn project_last_modified_ms(app: &App) -> Option<u64> {
    if let Some(updated_at_ms) = app.project_updated_at_ms
        && updated_at_ms > 0
    {
        return Some(updated_at_ms);
    }

    let path = app.project_path.as_deref()?;
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        return None;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(repo) = git2::Repository::open(path)
            && let Ok(head) = repo.head()
            && let Ok(commit) = head.peel_to_commit()
        {
            let secs = commit.time().seconds();
            if secs > 0 {
                return Some((secs as u64).saturating_mul(1000));
            }
        }

        let meta = std::fs::metadata(path).ok()?;
        let modified = meta.modified().ok()?;
        let elapsed = modified.duration_since(UNIX_EPOCH).ok()?;
        u64::try_from(elapsed.as_millis()).ok()
    }
}

/// 生成 modified label，用于界面中显示更短的相对信息。
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
pub fn relative_modified_label(app: &App) -> String {
    let Some(modified_ms) = project_last_modified_ms(app) else {
        return "最后修改 暂无".to_string();
    };
    let bucket = relative_time_bucket(modified_ms, crate::app::time::now_ms());
    format!("最后修改 {}", relative_time_label_for_bucket(bucket))
}

/// 生成 time bucket，用于界面中显示更短的相对信息。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn relative_time_bucket(ts_ms: u64, now_ms: u64) -> (u8, u64) {
    if ts_ms == 0 || ts_ms >= now_ms {
        return (0, 0);
    }

    let secs = (now_ms - ts_ms) / 1000;
    if secs < 60 {
        (0, 0)
    } else if secs < 3600 {
        (1, secs / 60)
    } else if secs < 86_400 {
        (2, secs / 3600)
    } else if secs < 2_592_000 {
        (3, secs / 86_400)
    } else if secs < 31_536_000 {
        (4, secs / 2_592_000)
    } else {
        (5, secs / 31_536_000)
    }
}

/// 生成 time label for bucket，用于界面中显示更短的相对信息。
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
pub fn relative_time_label_for_bucket(bucket: (u8, u64)) -> String {
    match bucket {
        (0, _) => "刚刚".to_string(),
        (1, value) => format!("{value} 分钟前"),
        (2, value) => format!("{value} 小时前"),
        (3, value) => format!("{value} 天前"),
        (4, value) => format!("{value} 个月前"),
        (5, value) => format!("{value} 年前"),
        _ => "刚刚".to_string(),
    }
}

/// 生成 time label，用于界面中显示更短的相对信息。
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
pub fn relative_time_label(ts_ms: u64) -> String {
    relative_time_label_for_bucket(relative_time_bucket(ts_ms, crate::app::time::now_ms()))
}

/// 处理 format chat time label 对应的局部职责。
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
pub fn format_chat_time_label(ts_ms: u64) -> String {
    static FMT: Lazy<Vec<FormatItem<'static>>> = Lazy::new(|| {
        // time 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        time::format_description::parse_borrowed::<2>("[year]-[month]-[day] [hour]:[minute]")
            .unwrap_or_default()
    });

    if ts_ms == 0 {
        return "刚刚".to_string();
    }

    let nanos = (ts_ms as i128).saturating_mul(1_000_000);
    time::OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .ok()
        .and_then(|dt| dt.format(&FMT).ok())
        .unwrap_or_else(|| "刚刚".to_string())
}
