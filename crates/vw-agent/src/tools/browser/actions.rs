//! 浏览器动作模块
//!
//! 本模块定义了浏览器自动化工具支持的所有动作类型及其解析逻辑。
//! 这些动作可以用于执行各种浏览器操作，如导航、点击、输入、截图等。
//!
//! # 主要功能
//!
//! - 定义浏览器动作枚举 `BrowserAction`，包含所有支持的浏览器操作类型
//! - 提供从 JSON 参数解析到类型化浏览器动作的功能
//! - 提供动作类型验证的辅助函数
//!
//! # 使用示例
//!
//! ```ignore
//! use serde_json::json;
//! use crate::app::agent::tools::browser::actions::{parse_browser_action, BrowserAction};
//!
//! let args = json!({"url": "https://example.com"});
//! let action = parse_browser_action("open", &args)?;
//! assert!(matches!(action, BrowserAction::Open { .. }));
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 浏览器动作枚举
///
/// 定义所有支持的浏览器操作类型。每个变体代表一种特定的浏览器操作，
/// 并携带执行该操作所需的参数。
///
/// # 序列化说明
///
/// 使用 `snake_case` 命名约定进行序列化和反序列化，
/// 以确保与 JSON API 的命名风格一致。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserAction {
    /// 导航到指定 URL
    ///
    /// 在当前浏览器上下文中打开指定的 URL 地址。
    ///
    /// # 参数
    ///
    /// - `url`: 要导航的目标 URL 地址（必须是有效的 URL 字符串）
    Open { url: String },

    /// 获取页面的可访问性快照
    ///
    /// 提取当前页面的可访问性树，包含元素的引用标识符，
    /// 可用于后续的交互操作（如点击、填写等）。
    ///
    /// # 参数
    ///
    /// - `interactive_only`: 是否仅返回可交互元素（默认为 true）
    /// - `compact`: 是否使用紧凑格式输出（默认为 true）
    /// - `depth`: 限制快照的深度层级（可选，不设置则使用默认值）
    Snapshot {
        #[serde(default)]
        interactive_only: bool,
        #[serde(default)]
        compact: bool,
        #[serde(default)]
        depth: Option<u32>,
    },

    /// 点击指定的元素
    ///
    /// 通过 CSS 选择器或元素引用标识符定位并点击页面元素。
    ///
    /// # 参数
    ///
    /// - `selector`: CSS 选择器或元素引用（如 "#submit-button" 或 "ref=123"）
    Click { selector: String },

    /// 填写表单字段
    ///
    /// 在指定的表单字段中填写值，会清空原有内容后填入新值。
    ///
    /// # 参数
    ///
    /// - `selector`: 表单字段的 CSS 选择器
    /// - `value`: 要填写的值
    Fill { selector: String, value: String },

    /// 在当前聚焦的元素中输入文本
    ///
    /// 模拟键盘输入，向当前聚焦的元素输入文本内容。
    ///
    /// # 参数
    ///
    /// - `selector`: 目标元素的选择器
    /// - `text`: 要输入的文本内容
    Type { selector: String, text: String },

    /// 获取元素的文本内容
    ///
    /// 提取指定元素的内部文本内容。
    ///
    /// # 参数
    ///
    /// - `selector`: 目标元素的 CSS 选择器
    GetText { selector: String },

    /// 获取页面标题
    ///
    /// 返回当前页面的 `<title>` 元素内容。
    GetTitle,

    /// 获取当前 URL
    ///
    /// 返回浏览器地址栏中当前显示的完整 URL。
    GetUrl,

    /// 截取屏幕快照
    ///
    /// 捕获当前页面或整个页面的屏幕截图。
    ///
    /// # 参数
    ///
    /// - `path`: 保存截图的文件路径（可选，不设置则返回 base64 编码）
    /// - `full_page`: 是否截取整个页面（默认为 false，仅截取可视区域）
    Screenshot {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        full_page: bool,
    },

    /// 等待条件满足
    ///
    /// 等待指定的元素出现、文本出现或固定时间后继续执行。
    /// 至少需要提供其中一个等待条件。
    ///
    /// # 参数
    ///
    /// - `selector`: 等待指定选择器的元素出现（可选）
    /// - `ms`: 等待指定的毫秒数（可选）
    /// - `text`: 等待指定文本在页面中出现（可选）
    Wait {
        #[serde(default)]
        selector: Option<String>,
        #[serde(default)]
        ms: Option<u64>,
        #[serde(default)]
        text: Option<String>,
    },

    /// 按下键盘按键
    ///
    /// 模拟按下指定的键盘按键，支持组合键和特殊键。
    ///
    /// # 参数
    ///
    /// - `key`: 按键名称（如 "Enter"、"Escape"、"Control+c"）
    Press { key: String },

    /// 鼠标悬停在元素上
    ///
    /// 将鼠标指针移动到指定元素上方，触发 hover 状态。
    ///
    /// # 参数
    ///
    /// - `selector`: 目标元素的 CSS 选择器
    Hover { selector: String },

    /// 滚动页面
    ///
    /// 按指定方向和距离滚动页面内容。
    ///
    /// # 参数
    ///
    /// - `direction`: 滚动方向（"up"、"down"、"left"、"right"）
    /// - `pixels`: 滚动的像素距离（可选，不设置则使用默认值）
    Scroll {
        direction: String,
        #[serde(default)]
        pixels: Option<u32>,
    },

    /// 检查元素是否可见
    ///
    /// 判断指定元素在当前视口中是否可见（未被隐藏且在视口内）。
    ///
    /// # 参数
    ///
    /// - `selector`: 目标元素的 CSS 选择器
    IsVisible { selector: String },

    /// 关闭浏览器
    ///
    /// 关闭当前浏览器实例并释放相关资源。
    Close,

    /// 通过语义定位器查找元素并执行动作
    ///
    /// 使用语义化的方式（如角色、文本、标签等）定位元素，
    /// 并在找到元素后执行指定的交互动作。
    ///
    /// # 参数
    ///
    /// - `by`: 定位方式（"role"、"text"、"label"、"placeholder"、"testid"）
    /// - `value`: 定位依据的值（如按钮文本、Aria 标签等）
    /// - `action`: 找到元素后要执行的动作（"click"、"fill"、"text"、"hover"）
    /// - `fill_value`: 当 action 为 "fill" 时要填写的值（可选）
    Find {
        by: String,
        value: String,
        action: String,
        #[serde(default)]
        fill_value: Option<String>,
    },
}

/// 将 JSON 参数对象解析为类型化的浏览器动作
///
/// 根据动作名称字符串和参数对象，构造对应的 `BrowserAction` 枚举变体。
/// 此函数负责参数验证和类型转换，确保返回的动作是有效且完整的。
///
/// # 参数
///
/// - `action_str`: 动作名称字符串（如 "open"、"click"、"fill" 等）
/// - `args`: 包含动作参数的 JSON 对象
///
/// # 返回值
///
/// 成功时返回解析后的 `BrowserAction` 枚举变体，
/// 失败时返回错误信息（如缺少必需参数、参数类型错误等）。
///
/// # 错误
///
/// 以下情况会导致错误：
/// - 缺少必需的参数（如 "open" 动作缺少 "url"）
/// - 参数类型不正确
/// - 不支持的 action_str 值
///
/// # 示例
///
/// ```ignore
/// use serde_json::json;
///
/// // 解析 open 动作
/// let args = json!({"url": "https://example.com"});
/// let action = parse_browser_action("open", &args)?;
///
/// // 解析 click 动作
/// let args = json!({"selector": "#submit-btn"});
/// let action = parse_browser_action("click", &args)?;
///
/// // 解析 wait 动作（可选参数）
/// let args = json!({"ms": 1000});
/// let action = parse_browser_action("wait", &args)?;
/// ```
pub fn parse_browser_action(action_str: &str, args: &Value) -> anyhow::Result<BrowserAction> {
    match action_str {
        // 解析 open 动作：从参数中提取必需的 url 字段
        "open" => {
            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing 'url' for open action"))?;
            Ok(BrowserAction::Open { url: url.into() })
        }

        // 解析 snapshot 动作：所有参数都是可选的，使用默认值
        "snapshot" => Ok(BrowserAction::Snapshot {
            interactive_only: args
                .get("interactive_only")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true), // 默认仅返回可交互元素
            compact: args.get("compact").and_then(serde_json::Value::as_bool).unwrap_or(true), // 默认使用紧凑格式
            depth: args
                .get("depth")
                .and_then(serde_json::Value::as_u64)
                .map(|d| u32::try_from(d).unwrap_or(u32::MAX)), // 转换深度值，超出范围则使用最大值
        }),

        // 解析 click 动作：从参数中提取必需的 selector 字段
        "click" => {
            let selector = args.get("selector").and_then(|v| v.as_str()).ok_or_else(|| {
                anyhow::anyhow!(
                    "Missing 'selector' for click (required: @ref from snapshot, CSS, or text=...)"
                )
            })?;
            Ok(BrowserAction::Click { selector: selector.into() })
        }

        // 解析 fill 动作：需要 selector 和 value 两个必需参数
        "fill" => {
            let selector = args.get("selector").and_then(|v| v.as_str()).ok_or_else(|| {
                anyhow::anyhow!(
                    "Missing 'selector' for fill (required: @ref from snapshot, CSS, or text=...)"
                )
            })?;
            let value = args
                .get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing 'value' for fill"))?;
            Ok(BrowserAction::Fill { selector: selector.into(), value: value.into() })
        }

        // 解析 type 动作：需要 selector 和 text 两个必需参数
        "type" => {
            let selector = args.get("selector").and_then(|v| v.as_str()).ok_or_else(|| {
                anyhow::anyhow!(
                    "Missing 'selector' for type (required: @ref from snapshot, CSS, or text=...)"
                )
            })?;
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing 'text' for type"))?;
            Ok(BrowserAction::Type { selector: selector.into(), text: text.into() })
        }

        // 解析 get_text 动作：需要 selector 参数（不会读取整页）
        "get_text" => {
            let selector = args
                .get("selector")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Missing 'selector' for get_text (required: @ref from snapshot, CSS, or text=...; does not read whole page)"
                    )
                })?;
            Ok(BrowserAction::GetText { selector: selector.into() })
        }

        // get_title 和 get_url 不需要参数
        "get_title" => Ok(BrowserAction::GetTitle),
        "get_url" => Ok(BrowserAction::GetUrl),

        // 解析 screenshot 动作：path 和 full_page 都是可选参数
        "screenshot" => Ok(BrowserAction::Screenshot {
            path: args.get("path").and_then(|v| v.as_str()).map(String::from),
            full_page: args.get("full_page").and_then(serde_json::Value::as_bool).unwrap_or(false), // 默认仅截取可视区域
        }),

        // 解析 wait 动作：selector、ms 和 text 都是可选参数，至少需要一个
        "wait" => Ok(BrowserAction::Wait {
            selector: args.get("selector").and_then(|v| v.as_str()).map(String::from),
            ms: args.get("ms").and_then(serde_json::Value::as_u64),
            text: args.get("text").and_then(|v| v.as_str()).map(String::from),
        }),

        // 解析 press 动作：需要 key 参数
        "press" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing 'key' for press"))?;
            Ok(BrowserAction::Press { key: key.into() })
        }

        // 解析 hover 动作：需要 selector 参数
        "hover" => {
            let selector = args.get("selector").and_then(|v| v.as_str()).ok_or_else(|| {
                anyhow::anyhow!(
                    "Missing 'selector' for hover (required: @ref from snapshot, CSS, or text=...)"
                )
            })?;
            Ok(BrowserAction::Hover { selector: selector.into() })
        }

        // 解析 scroll 动作：direction 必需，pixels 可选
        "scroll" => {
            let direction = args
                .get("direction")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing 'direction' for scroll"))?;
            Ok(BrowserAction::Scroll {
                direction: direction.into(),
                pixels: args
                    .get("pixels")
                    .and_then(serde_json::Value::as_u64)
                    .map(|p| u32::try_from(p).unwrap_or(u32::MAX)), // 转换像素值，超出范围则使用最大值
            })
        }

        // 解析 is_visible 动作：需要 selector 参数
        "is_visible" => {
            let selector = args
                .get("selector")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Missing 'selector' for is_visible (required: @ref from snapshot, CSS, or text=...)"
                    )
                })?;
            Ok(BrowserAction::IsVisible { selector: selector.into() })
        }

        // close 动作不需要参数
        "close" => Ok(BrowserAction::Close),

        // 解析 find 动作：需要 by、value 和 find_action 三个必需参数，fill_value 可选
        "find" => {
            let by = args
                .get("by")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing 'by' for find"))?;
            let value = args
                .get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing 'value' for find"))?;
            // 注意：参数名为 find_action 而不是 action，以避免与函数名冲突
            let action = args
                .get("find_action")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing 'find_action' for find"))?;
            Ok(BrowserAction::Find {
                by: by.into(),
                value: value.into(),
                action: action.into(),
                fill_value: args.get("fill_value").and_then(|v| v.as_str()).map(String::from),
            })
        }

        // 不支持的 action，返回错误
        other => anyhow::bail!("Unsupported browser action: {other}"),
    }
}

/// 检查指定的动作字符串是否为支持的浏览器动作
///
/// 验证给定的动作名称是否在浏览器工具支持的范围内。
/// 包括标准浏览器动作和计算机使用（Computer Use）动作。
///
/// # 参数
///
/// - `action`: 要检查的动作名称字符串
///
/// # 返回值
///
/// 如果动作在支持列表中返回 `true`，否则返回 `false`
///
/// # 支持的动作列表
///
/// 标准浏览器动作：
/// - open, snapshot, click, fill, type
/// - get_text, get_title, get_url
/// - screenshot, wait, press, hover
/// - scroll, is_visible, close, find
///
/// 计算机使用（Computer Use）动作：
/// - mouse_move, mouse_click, mouse_drag
/// - key_type, key_press, screen_capture
///
/// # 示例
///
/// ```ignore
/// assert!(is_supported_browser_action("open"));
/// assert!(is_supported_browser_action("click"));
/// assert!(is_supported_browser_action("mouse_move"));
/// assert!(!is_supported_browser_action("unknown"));
/// ```
pub fn is_supported_browser_action(action: &str) -> bool {
    matches!(
        action,
        // 标准浏览器动作
        "open"
            | "snapshot"
            | "click"
            | "fill"
            | "type"
            | "get_text"
            | "get_title"
            | "get_url"
            | "screenshot"
            | "wait"
            | "press"
            | "hover"
            | "scroll"
            | "is_visible"
            | "close"
            | "find"
            // 计算机使用（Computer Use）动作
            | "mouse_move"
            | "mouse_click"
            | "mouse_drag"
            | "key_type"
            | "key_press"
            | "screen_capture"
    )
}

/// 检查指定的动作是否为计算机使用（Computer Use）专用动作
///
/// Computer Use 动作是一组低级别的输入控制动作，
/// 提供更细粒度的鼠标和键盘控制能力，与高级浏览器动作不同。
///
/// # 参数
///
/// - `action`: 要检查的动作名称字符串
///
/// # 返回值
///
/// 如果是 Computer Use 专用动作返回 `true`，否则返回 `false`
///
/// # Computer Use 动作列表
///
/// - `mouse_move`: 移动鼠标到指定位置
/// - `mouse_click`: 在当前位置执行鼠标点击
/// - `mouse_drag`: 执行鼠标拖拽操作
/// - `key_type`: 输入文本（逐字符）
/// - `key_press`: 按下并释放按键
/// - `screen_capture`: 捕获屏幕内容
///
/// # 示例
///
/// ```ignore
/// assert!(is_computer_use_only_action("mouse_move"));
/// assert!(is_computer_use_only_action("screen_capture"));
/// assert!(!is_computer_use_only_action("click"));
/// assert!(!is_computer_use_only_action("open"));
/// ```
pub fn is_computer_use_only_action(action: &str) -> bool {
    matches!(
        action,
        "mouse_move" | "mouse_click" | "mouse_drag" | "key_type" | "key_press" | "screen_capture"
    )
}
#[cfg(test)]
#[path = "actions_tests.rs"]
mod actions_tests;
