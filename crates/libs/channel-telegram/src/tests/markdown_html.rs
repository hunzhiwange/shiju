//! Markdown 转 Telegram HTML 格式的测试模块
//!
//! 本模块测试 `TelegramChannel::markdown_to_telegram_html` 方法的安全性和正确性，
//! 主要验证以下关键安全特性：
//!
//! - 链接 href 中的引号转义（防止 XSS 攻击）
//! - 纯文本中的引号和特殊字符转义
//! - 代码块语言属性的过滤（防止恶意属性注入）
//!
//! 这些测试确保在将 Markdown 内容转换为 Telegram 支持的 HTML 子集时，
//! 所有用户输入都经过适当的转义处理，避免注入攻击风险。

use super::*;

/// 测试链接 href 属性中引号的转义处理
///
/// 验证当 Markdown 链接的目标 URL 中包含双引号和单引号时，
/// 转换函数能够正确地对这些字符进行 HTML 实体编码。
///
/// # 安全意义
///
/// 防止攻击者通过在 URL 中注入引号来：
/// - 提前关闭 href 属性
/// - 注入新的事件处理器（如 onclick）
/// - 突破 Telegram HTML 的安全边界
///
/// # 转义规则
///
/// - 双引号 `"` → `&quot;`
/// - 单引号 `'` → `&#39;`
/// - & 符号 `&` → `&amp;`
#[test]
fn telegram_markdown_to_html_escapes_quotes_in_link_href() {
    // 构造包含特殊字符的测试 URL：双引号和单引号混合
    let rendered =
        TelegramChannel::markdown_to_telegram_html("[click](https://example.com?q=\"x\"&a='b')");

    // 验证所有引号都被正确转义为 HTML 实体
    assert_eq!(
        rendered,
        "<a href=\"https://example.com?q=&quot;x&quot;&amp;a=&#39;b&#39;\">click</a>"
    );
}

/// 测试纯文本内容中引号和特殊字符的转义处理
///
/// 验证当 Markdown 纯文本中包含多种特殊字符时，
/// 转换函数能够对所有需要转义的字符进行正确的 HTML 编码。
///
/// # 转义的字符类型
///
/// - 双引号 `"` → `&quot;`
/// - 单引号 `'` → `&#39;`
/// - & 符号 `&` → `&amp;`
/// - 小于号 `<` → `&lt;`
/// - 大于号 `>` → `&gt;`
///
/// # 为什么需要转义这些字符
///
/// 在 HTML 上下文中，这些字符具有特殊含义，如果未转义可能导致：
/// - HTML 标签解析错误
/// - 潜在的 HTML 注入
/// - 布局破坏或内容显示异常
#[test]
fn telegram_markdown_to_html_escapes_quotes_in_plain_text() {
    // 构造包含多种特殊字符的文本：引号、& 符号、HTML 标签
    let rendered = TelegramChannel::markdown_to_telegram_html("say \"hi\" & <tag> 'ok'");

    // 验证所有特殊字符都被正确转义
    assert_eq!(rendered, "say &quot;hi&quot; &amp; &lt;tag&gt; &#39;ok&#39;");
}

/// 测试代码块语言属性的安全过滤
///
/// 验证当代码块的语言标识符中包含恶意属性时，
/// 转换函数能够完全丢弃语言属性，防止属性注入攻击。
///
/// # 攻击场景
///
/// 攻击者可能尝试在语言标识符中注入：
/// - 事件处理器（如 `onclick="alert(1)"`)
/// - 危险的 CSS 类名
/// - 其他 HTML 属性
///
/// # 防御策略
///
/// Telegram 的 HTML 子集不支持代码块的语言属性，
/// 因此转换函数会完全移除语言标识符，只保留代码内容本身。
/// 这是一种安全的设计：丢弃而非过滤可疑内容。
///
/// # 测试验证点
///
/// 1. 语言标识符 `rust` 不应出现在输出中
/// 2. 恶意属性 `onclick` 不应出现在输出中
/// 3. 代码内容应完整保留
#[test]
fn telegram_markdown_to_html_code_block_drops_language_attribute() {
    // 构造恶意代码块：语言标识符中混入 onclick 事件处理器
    let rendered =
        TelegramChannel::markdown_to_telegram_html("```rust\" onclick=\"alert(1)\nlet x = 1;\n```");

    // 验证输出只包含基本代码块结构，无语言类名和恶意属性
    assert_eq!(rendered, "<pre><code>let x = 1;</code></pre>");

    // 二次确认：输出中不应包含语言类名前缀
    assert!(!rendered.contains("language-"));

    // 二次确认：输出中不应包含注入的 onclick 属性
    assert!(!rendered.contains("onclick"));
}
