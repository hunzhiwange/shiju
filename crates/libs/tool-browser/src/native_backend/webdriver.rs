//! WebDriver 辅助工具模块
//!
//! 本模块提供 WebDriver 相关的辅助函数和类型定义，用于浏览器自动化操作。
//!
//! ## 主要功能
//!
//! - **连接检测**：验证 WebDriver 端点是否可达
//! - **选择器构建**：将高层定位策略转换为 CSS 选择器或 XPath
//! - **XPath 工具**：XPath 字符串字面量构建与转义
//! - **按键映射**：将友好按键名称映射为 WebDriver 键码
//!
//! ## 选择器语法
//!
//! | 前缀       | 说明           | 示例              |
//! |-----------|---------------|-------------------|
//! | `text=`   | 包含文本       | `text=登录`       |
//! | `label=`  | 按标签定位     | `label=用户名`    |
//! | `@`       | 自定义属性     | `@my-ref`         |
//! | 其他      | 原始 CSS       | `#id`, `.class`   |

use fantoccini::key::Key;
use std::net::ToSocketAddrs;

/// 检测 WebDriver 端点是否在指定超时时间内可达
///
/// 此函数执行轻量级 TCP 连接测试，不发送 HTTP 请求。
/// 适用于启动前健康检查或连接排障。
///
/// # 参数
///
/// - `webdriver_url`: WebDriver 服务的完整 URL（如 `http://localhost:4444`）
/// - `timeout`: TCP 连接超时时间
///
/// # 返回值
///
/// - `true`: 端点可达（TCP 连接成功）
/// - `false`: URL 无效、协议不支持、DNS 解析失败或连接超时
///
/// # 示例
///
/// ```ignore
/// use std::time::Duration;
///
/// let reachable = webdriver_endpoint_reachable(
///     "http://localhost:4444",
///     Duration::from_secs(5)
/// );
/// ```
pub fn webdriver_endpoint_reachable(webdriver_url: &str, timeout: std::time::Duration) -> bool {
    // 解析 URL，失败则不可达
    let parsed = match reqwest::Url::parse(webdriver_url) {
        Ok(url) => url,
        Err(_) => return false,
    };

    // 仅支持 HTTP/HTTPS 协议
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return false;
    }

    // 提取主机名，空主机名视为无效
    let host = match parsed.host_str() {
        Some(h) if !h.is_empty() => h,
        _ => return false,
    };

    // 获取端口，默认使用 4444（WebDriver 常用端口）
    let port = parsed.port_or_known_default().unwrap_or(4444);

    // 将主机名解析为 socket 地址
    let mut addrs = match (host, port).to_socket_addrs() {
        Ok(iter) => iter,
        Err(_) => return false,
    };

    // 取第一个解析结果进行连接测试
    let addr = match addrs.next() {
        Some(a) => a,
        None => return false,
    };

    // 执行带超时的 TCP 连接测试
    std::net::TcpStream::connect_timeout(&addr, timeout).is_ok()
}

/// 为查找元素操作构建 CSS 选择器字符串
///
/// 根据 `by` 策略类型，将值转换为对应的 CSS 选择器或 Playwright 风格选择器。
///
/// # 参数
///
/// - `by`: 定位策略类型（`role`, `label`, `placeholder`, `testid` 或其他）
/// - `value`: 选择器值，将被自动转义
///
/// # 返回值
///
/// 返回对应策略的 CSS 选择器字符串
///
/// # 选择器映射
///
/// | by          | 输出格式                      |
/// |-------------|------------------------------|
/// | `role`      | `[role="value"]`             |
/// | `label`     | `label=value`                |
/// | `placeholder` | `[placeholder="value"]`     |
/// | `testid`    | `[data-testid="value"]`      |
/// | 其他        | `text=value`                 |
pub fn selector_for_find(by: &str, value: &str) -> String {
    // 转义 CSS 属性值中的特殊字符
    let escaped = css_attr_escape(value);
    match by {
        "role" => format!(r#"[role=\"{escaped}\"]"#),
        "label" => format!("label={value}"),
        "placeholder" => format!(r#"[placeholder=\"{escaped}\"]"#),
        "testid" => format!(r#"[data-testid=\"{escaped}\"]"#),
        _ => format!("text={value}"),
    }
}

/// 选择器类型枚举
///
/// 表示解析后的选择器是 CSS 选择器还是 XPath 表达式。
/// WebDriver/fantoccini 需要区分这两种定位方式。
pub enum SelectorKind {
    /// CSS 选择器（如 `#id`, `.class`, `[attr="value"]`）
    Css(String),
    /// XPath 表达式（如 `//div[@class="example"]`）
    XPath(String),
}

/// 解析选择器字符串并确定其类型
///
/// 根据前缀识别选择器类型，自动构建对应的 CSS 或 XPath 表达式。
///
/// # 参数
///
/// - `selector`: 原始选择器字符串
///
/// # 返回值
///
/// 返回 [`SelectorKind`] 枚举，包含构建好的选择器表达式
///
/// # 前缀规则
///
/// | 前缀      | 类型   | 说明                           |
/// |----------|--------|-------------------------------|
/// | `text=`  | XPath  | 查找包含指定文本的元素          |
/// | `label=` | XPath  | 通过标签文本定位关联的表单元素   |
/// | `@`      | CSS    | 转换为 `data-zc-ref` 属性选择器 |
/// | 无前缀   | CSS    | 作为原始 CSS 选择器使用         |
///
/// # 示例
///
/// ```ignore
/// // 文本选择器 -> XPath
/// let kind = parse_selector("text=登录按钮");
/// // 返回: XPath("//*[contains(normalize-space(.), \"登录按钮\")]")
///
/// // 标签选择器 -> XPath
/// let kind = parse_selector("label=用户名");
/// // 返回: 复合 XPath 定位关联输入框
///
/// // 自定义属性 -> CSS
/// let kind = parse_selector("@submit-btn");
/// // 返回: Css("[data-zc-ref=\"@submit-btn\"]")
///
/// // 原始 CSS -> CSS
/// let kind = parse_selector("#login-form");
/// // 返回: Css("#login-form")
/// ```
pub fn parse_selector(selector: &str) -> SelectorKind {
    let trimmed = selector.trim();

    // 处理 "text=" 前缀：查找包含指定文本的任意元素
    if let Some(text_query) = trimmed.strip_prefix("text=") {
        return SelectorKind::XPath(xpath_contains_text(text_query));
    }

    // 处理 "label=" 前缀：通过标签文本定位关联的表单控件
    // 支持三种情况：
    // 1. <label>标签后的第一个 input/textarea/select
    // 2. 带有 aria-label 属性的元素
    // 3. <label> 元素本身（作为回退）
    if let Some(label_query) = trimmed.strip_prefix("label=") {
        let literal = xpath_literal(label_query);
        return SelectorKind::XPath(format!(
            "(//label[contains(normalize-space(.), {literal})]/following::*[self::input or self::textarea or self::select][1] | //*[@aria-label and contains(normalize-space(@aria-label), {literal})] | //label[contains(normalize-space(.), {literal})])"
        ));
    }

    // 处理 "@" 前缀：转换为自定义数据属性选择器
    if trimmed.starts_with('@') {
        let escaped = css_attr_escape(trimmed);
        return SelectorKind::Css(format!(r#"[data-zc-ref=\"{escaped}\"]"#));
    }

    // 无特殊前缀：作为原始 CSS 选择器
    SelectorKind::Css(trimmed.to_string())
}

/// 转义 CSS 属性值中的特殊字符
///
/// 对 CSS 属性选择器中的值进行安全转义，防止注入和语法错误。
///
/// # 参数
///
/// - `input`: 原始字符串
///
/// # 返回值
///
/// 返回转义后的安全字符串
///
/// # 转义规则
///
/// | 字符   | 转义为    |
/// |-------|----------|
/// | `\`   | `\\`     |
/// | `"`   | `\"`     |
/// | 换行   | 空格      |
pub fn css_attr_escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ")
}

/// 构建包含指定文本的 XPath 表达式
///
/// 生成一个 XPath 表达式，用于查找任意包含指定文本内容的元素。
/// 使用 `normalize-space()` 函数忽略首尾空白和多余空格。
///
/// # 参数
///
/// - `text`: 要查找的文本内容
///
/// # 返回值
///
/// 返回完整的 XPath 表达式字符串
///
/// # 示例
///
/// ```ignore
/// let xpath = xpath_contains_text("登录");
/// // 返回: "//*[contains(normalize-space(.), \"登录\")]"
/// ```
pub fn xpath_contains_text(text: &str) -> String {
    format!("//*[contains(normalize-space(.), {})]", xpath_literal(text))
}

/// 构建 XPath 字符串字面量
///
/// 将任意字符串安全地转换为 XPath 字符串字面量，正确处理引号嵌套。
///
/// # 参数
///
/// - `input`: 原始字符串
///
/// # 返回值
///
/// 返回可安全嵌入 XPath 表达式的字符串字面量
///
/// # 转义策略
///
/// 1. 如果不含双引号，用双引号包裹：`value` -> `"value"`
/// 2. 如果不含单引号，用单引号包裹：`val"ue` -> `'val"ue'`
/// 3. 如果同时包含两种引号，使用 `concat()` 函数拼接：
///    `a"b'c` -> `concat("a",'"',"b'c")`
///
/// # 示例
///
/// ```ignore
/// // 简单情况
/// xpath_literal("hello");  // 返回: "\"hello\""
///
/// // 包含双引号
/// xpath_literal("say \"hi\"");  // 返回: "'say \"hi\"'"
///
/// // 包含两种引号
/// xpath_literal("a\"b'c");
/// // 返回: "concat(\"a\",'\"',\"b'c\")"
/// ```
pub fn xpath_literal(input: &str) -> String {
    // 策略1：无双引号，直接用双引号包裹
    if !input.contains('"') {
        return format!("\"{input}\"");
    }

    // 策略2：无双引号时用单引号包裹
    if !input.contains('\'') {
        return format!("'{input}'");
    }

    // 策略3：同时包含两种引号，使用 concat() 拼接
    // 将字符串按双引号分割，然后用 concat() 重新组合
    let segments: Vec<&str> = input.split('"').collect();
    let mut parts: Vec<String> = Vec::new();

    for (index, part) in segments.iter().enumerate() {
        // 非空片段用双引号包裹
        if !part.is_empty() {
            parts.push(format!("\"{part}\""));
        }
        // 在双引号位置插入字面量 '"'
        if index + 1 < segments.len() {
            parts.push("'\"'".to_string());
        }
    }

    // 拼接所有部分
    if parts.is_empty() { "\"\"".to_string() } else { format!("concat({})", parts.join(",")) }
}

/// 将友好按键名称转换为 WebDriver 键码字符串
///
/// 将人类可读的按键名称映射为 fantoccini/WebDriver 所需的键码。
///
/// # 参数
///
/// - `key`: 按键名称（不区分大小写，支持多种别名）
///
/// # 返回值
///
/// 返回 WebDriver 兼容的键码字符串
///
/// # 支持的按键名称
///
/// | 名称              | 说明           |
/// |------------------|----------------|
/// | `enter`          | 回车键         |
/// | `return`         | 回车键（别名）  |
/// | `tab`            | 制表键         |
/// | `escape`, `esc`  | 退出键         |
/// | `backspace`      | 退格键         |
/// | `delete`         | 删除键         |
/// | `space`          | 空格键         |
/// | `arrowup`, `up`  | 上箭头         |
/// | `arrowdown`, `down` | 下箭头      |
/// | `arrowleft`, `left` | 左箭头      |
/// | `arrowright`, `right` | 右箭头    |
/// | `home`           | Home 键        |
/// | `end`            | End 键         |
/// | `pageup`         | Page Up 键     |
/// | `pagedown`       | Page Down 键   |
/// | 其他             | 原样返回       |
///
/// # 示例
///
/// ```ignore
/// webdriver_key("Enter");      // 返回 WebDriver Enter 键码
/// webdriver_key("ARROW_UP");   // 返回 WebDriver 上箭头键码
/// webdriver_key("a");          // 返回 "a"
/// ```
pub fn webdriver_key(key: &str) -> String {
    match key.trim().to_ascii_lowercase().as_str() {
        "enter" => Key::Enter.to_string(),
        "return" => Key::Return.to_string(),
        "tab" => Key::Tab.to_string(),
        "escape" | "esc" => Key::Escape.to_string(),
        "backspace" => Key::Backspace.to_string(),
        "delete" => Key::Delete.to_string(),
        "space" => Key::Space.to_string(),
        "arrowup" | "up" => Key::Up.to_string(),
        "arrowdown" | "down" => Key::Down.to_string(),
        "arrowleft" | "left" => Key::Left.to_string(),
        "arrowright" | "right" => Key::Right.to_string(),
        "home" => Key::Home.to_string(),
        "end" => Key::End.to_string(),
        "pageup" => Key::PageUp.to_string(),
        "pagedown" => Key::PageDown.to_string(),
        other => other.to_string(),
    }
}
#[cfg(test)]
#[path = "webdriver_tests.rs"]
mod webdriver_tests;
