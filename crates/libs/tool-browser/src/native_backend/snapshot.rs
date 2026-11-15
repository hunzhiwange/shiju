//! 浏览器页面快照生成模块
//!
//! 本模块提供用于生成浏览器页面 DOM 树快照的 JavaScript 脚本。
//! 快照功能用于收集页面元素的结构化信息，包括元素的层级关系、
//! 可交互性、文本内容等，便于后续的页面分析和自动化操作。
//!
//! # 核心功能
//!
//! - 遍历 DOM 树并收集可见元素信息
//! - 支持过滤仅可交互元素
//! - 支持紧凑模式（过滤无文本的非交互元素）
//! - 支持深度限制以控制遍历范围
//!
//! # 使用场景
//!
//! - 页面元素定位与分析
//! - 自动化测试中的元素快照
//! - 辅助功能检测
//! - 页面结构审查

/// 生成浏览器页面 DOM 快照的 JavaScript 脚本
///
/// 该函数生成一段立即执行函数表达式（IIFE）形式的 JavaScript 代码，
/// 用于在浏览器环境中执行并收集页面的 DOM 结构信息。
///
/// # 参数
///
/// * `interactive_only` - 是否仅收集可交互元素
///   - `true`: 只收集链接、按钮、输入框等可交互元素
///   - `false`: 收集所有可见元素
///
/// * `compact` - 是否启用紧凑模式
///   - `true`: 过滤掉没有文本内容的非交互元素，减少输出体积
///   - `false`: 保留所有符合条件的元素
///
/// * `depth` - DOM 树遍历的最大深度
///   - `Some(n)`: 限制遍历深度为 n 层
///   - `None`: 不限制遍历深度
///
/// # 返回值
///
/// 返回一个 JavaScript 代码字符串，该代码在浏览器中执行后会返回一个对象，包含：
/// - `title`: 页面标题
/// - `url`: 当前页面 URL
/// - `count`: 收集到的节点数量
/// - `nodes`: 节点信息数组，每个节点包含：
///   - `ref`: 元素引用标识符（如 "@e1"）
///   - `depth`: 元素在 DOM 树中的深度
///   - `tag`: 元素标签名（小写）
///   - `id`: 元素 ID（如果存在）
///   - `role`: 元素的 ARIA role 属性（如果存在）
///   - `text`: 元素的文本内容（截断至 140 字符）
///   - `interactive`: 是否为可交互元素
///
/// # 示例
///
/// ```rust
/// // 生成仅收集可交互元素的紧凑快照脚本
/// let script = snapshot_script(true, true, Some(5));
/// // 在浏览器中执行 script 后可获得页面结构信息
///
/// // 生成完整的页面快照（无深度限制）
/// let script = snapshot_script(false, false, None);
/// ```
///
/// # 注意事项
///
/// - 脚本会为每个收集到的元素添加 `data-zc-ref` 属性用于标识
/// - 最多收集 400 个节点以避免性能问题
/// - 仅收集可见元素（通过 CSS 样式和几何尺寸判断）
pub fn snapshot_script(interactive_only: bool, compact: bool, depth: Option<i64>) -> String {
    // 将深度参数转换为 JavaScript 字面量
    // Some(n) 转换为数字字符串，None 转换为 "null"
    let depth_literal = depth.map(|level| level.to_string()).unwrap_or_else(|| "null".to_string());

    format!(
        r#"(() => {{
  // 配置参数：是否仅收集可交互元素
  const interactiveOnly = {interactive_only};
  // 配置参数：是否启用紧凑模式
  const compact = {compact};
  // 配置参数：最大遍历深度（null 表示无限制）
  const maxDepth = {depth_literal};

  // 收集到的节点数组
  const nodes = [];
  // DOM 树根元素（优先使用 body，回退到 documentElement）
  const root = document.body || document.documentElement;
  // 节点引用计数器，用于生成唯一标识符
  let counter = 0;

  // 检查元素是否可见
  // 通过 CSS 样式和几何尺寸判断元素的可见性
  const isVisible = (el) => {{
    const style = window.getComputedStyle(el);
    // 检查 display、visibility 和 opacity 属性
    if (style.display === 'none' || style.visibility === 'hidden' || Number(style.opacity || 1) === 0) {{
      return false;
    }}
    // 检查元素是否有实际的宽度和高度
    const rect = el.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0;
  }};

  // 检查元素是否为可交互元素
  // 通过标签名、属性和事件处理器判断
  const isInteractive = (el) => {{
    // 匹配常见的可交互元素选择器
    if (el.matches('a,button,input,select,textarea,summary,[role],*[tabindex]')) return true;
    // 检查是否有 onclick 事件处理器
    return typeof el.onclick === 'function';
  }};

  // 描述元素并添加到节点列表
  // 提取元素的关键信息并生成结构化数据
  const describe = (el, depth) => {{
    const interactive = isInteractive(el);
    // 提取并清理文本内容（去除多余空白，截断至 140 字符）
    const text = (el.innerText || el.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 140);

    // 根据 filter 参数决定是否跳过此元素
    if (interactiveOnly && !interactive) return;
    if (compact && !interactive && !text) return;

    // 生成唯一引用标识符并标记元素
    const ref = '@e' + (++counter);
    el.setAttribute('data-zc-ref', ref);

    // 将元素信息添加到节点数组
    nodes.push({{
      ref,
      depth,
      tag: el.tagName.toLowerCase(),
      id: el.id || null,
      role: el.getAttribute('role'),
      text,
      interactive,
    }});
  }};

  // 递归遍历 DOM 树
  // 深度优先遍历所有子元素
  const walk = (el, depth) => {{
    // 仅处理 Element 类型的节点
    if (!(el instanceof Element)) return;
    // 深度限制检查
    if (maxDepth !== null && depth > maxDepth) return;

    // 如果元素可见，则描述并记录
    if (isVisible(el)) {{
      describe(el, depth);
    }}

    // 递归处理所有子元素
    for (const child of el.children) {{
      walk(child, depth + 1);
      // 节点数量限制（防止内存溢出和性能问题）
      if (nodes.length >= 400) return;
    }}
  }};

  // 从根元素开始遍历
  if (root) walk(root, 0);

  // 返回快照结果对象
  return {{
    title: document.title,
    url: window.location.href,
    count: nodes.length,
    nodes,
  }};
}})();"#
    )
}
#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod snapshot_tests;
