#[cfg(not(target_arch = "wasm32"))]
use iced::{Color, Theme};
use std::path::Path;
/// BadgeTone 描述 mod 模块支持的离散状态。
///
/// 新增变体时需要同步检查显式分支，避免未知状态被静默吞掉。
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BadgeTone {
    Ready,
    Working,
    Error,
    Neutral,
}

/// 根据主题与语义状态计算 active preview lsp badge dot color。
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
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn active_preview_lsp_badge_dot_color(theme: &Theme, tone: BadgeTone) -> Color {
    let palette = theme.extended_palette();
    match tone {
        // BadgeTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BadgeTone::Ready => palette.success.base.color,
        // BadgeTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BadgeTone::Working => palette.warning.base.color,
        // BadgeTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BadgeTone::Error => palette.danger.base.color,
        // BadgeTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BadgeTone::Neutral => palette.background.strong.color,
    }
}

/// 根据主题与语义状态计算 active preview lsp badge colors。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn active_preview_lsp_badge_colors(
    theme: &Theme,
    // tone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tone: BadgeTone,
) -> (Color, Color, Color) {
    let palette = theme.extended_palette();
    match tone {
        // BadgeTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BadgeTone::Ready => (
            palette.success.base.color.scale_alpha(0.12),
            palette.success.base.color,
            palette.success.base.color.scale_alpha(0.32),
        ),
        // BadgeTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BadgeTone::Working => (
            palette.warning.base.color.scale_alpha(0.12),
            palette.warning.base.color,
            palette.warning.base.color.scale_alpha(0.32),
        ),
        // BadgeTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BadgeTone::Error => (
            palette.danger.base.color.scale_alpha(0.10),
            palette.danger.base.color,
            palette.danger.base.color.scale_alpha(0.26),
        ),
        // BadgeTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BadgeTone::Neutral => (
            palette.background.weak.color,
            palette.background.weak.text,
            palette.background.strong.color.scale_alpha(0.18),
        ),
    }
}

/// 处理 preview breadcrumb segments 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn preview_breadcrumb_segments(
    active_path: &Path,
    // project_path 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    project_path: Option<&str>,
) -> Vec<String> {
    if let Some(relative) = project_relative_segments(active_path, project_path) {
        return relative;
    }

    vec![active_path.to_string_lossy().to_string()]
}

/// 处理 project relative segments 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
fn project_relative_segments(
    active_path: &Path,
    // project_path 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    project_path: Option<&str>,
) -> Option<Vec<String>> {
    let project_path = Path::new(project_path?);

    relative_segments_from_root(active_path, project_path).or_else(|| {
        let active_canonical = active_path.canonicalize().ok()?;
        let project_canonical = project_path.canonicalize().ok()?;
        relative_segments_from_root(&active_canonical, &project_canonical)
    })
}

/// 生成 segments from root，用于界面中显示更短的相对信息。
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
fn relative_segments_from_root(active_path: &Path, project_root: &Path) -> Option<Vec<String>> {
    let relative = active_path.strip_prefix(project_root).ok()?;
    let segments = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    if segments.is_empty() { None } else { Some(segments) }
}

/// 处理 active preview lsp badge state 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn active_preview_lsp_badge_state(
    lsp_server_key: Option<&'static str>,
    // has_progress 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    has_progress: bool,
    // has_language 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    has_language: bool,
) -> (&'static str, BadgeTone) {
    if lsp_server_key.is_some() {
        if has_progress { ("LSP", BadgeTone::Working) } else { ("LSP", BadgeTone::Ready) }
    } else if has_language {
        ("LSP", BadgeTone::Error)
    } else {
        ("Plain Text", BadgeTone::Neutral)
    }
}


#[cfg(test)] mod tests;
#[cfg(test)] mod view_tests;
#[cfg(test)] mod mod_tests;
