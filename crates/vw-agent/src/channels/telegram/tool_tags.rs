//! Telegram 通道工具调用标签处理模块
//!
//! 本模块提供了移除 Telegram 消息中工具调用标签的功能。
//! 主要用于清理消息内容，移除与工具调用相关的标记，以便向用户展示纯净的消息文本。
//!
//! # 功能
//!
//! - 移除标准的工具调用标签（通过通用函数）
//! - 移除遗留格式的 "erte...ttri" 标签对
//! - 正确处理标签内嵌的 JSON 内容

/// 从消息文本中移除工具调用标签
///
/// 该函数执行两阶段清理：
/// 1. 首先调用通用工具标签移除函数处理标准格式的标签
/// 2. 然后处理遗留格式的 "erte...ttri" 标签对
///
/// # 参数
///
/// * `message` - 需要清理的原始消息文本
///
/// # 返回值
///
/// 返回移除所有工具调用标签后的清理文本
///
/// # 示例
///
/// ```ignore
/// let raw_message = "Hello erte{\"tool\":\"calc\"}ttri world";
/// let cleaned = strip_tool_call_tags(raw_message);
/// assert_eq!(cleaned, "Hello  world");
/// ```
pub fn strip_tool_call_tags(message: &str) -> String {
    /// 移除遗留格式的 "erte...ttri" 标签对
    ///
    /// 该函数处理特定的遗留标签格式：`erte <JSON> ttri`，
    /// 其中 JSON 可以是对象或数组。函数会正确解析 JSON 内容，
    /// 确保只移除完整且有效的标签对。
    ///
    /// # 参数
    ///
    /// * `input` - 输入文本
    ///
    /// # 返回值
    ///
    /// 返回移除所有 "erte...ttri" 标签对后的文本
    ///
    /// # 处理逻辑
    ///
    /// 1. 查找 "erte" 标签开始标记
    /// 2. 跳过空白字符找到 JSON 开始位置
    /// 3. 验证 JSON 以 `{` 或 `[` 开头
    /// 4. 使用流式 JSON 解析器解析完整的 JSON 值
    /// 5. 跳过 JSON 后的空白字符
    /// 6. 验证 "ttri" 结束标记存在
    /// 7. 移除整个标签对，继续处理剩余文本
    fn strip_legacy_erte_ttri_tags(input: &str) -> String {
        // 预分配输出缓冲区，容量与输入相同以避免频繁扩容
        let mut output = String::with_capacity(input.len());
        let mut cursor = 0usize;

        // 主循环：逐个查找并处理 "erte" 标签
        while let Some(rel_start) = input[cursor..].find("erte") {
            let start = cursor + rel_start;
            // 将 "erte" 之前的文本复制到输出
            output.push_str(&input[cursor..start]);

            // 定位到 "erte" 之后的第一个非空白字符
            let mut json_start = start + "erte".len();
            while let Some(ch) = input[json_start..].chars().next() {
                if ch.is_whitespace() {
                    json_start += ch.len_utf8();
                } else {
                    break;
                }
            }

            // 检查是否存在有效字符
            let Some(first) = input[json_start..].chars().next() else {
                cursor = start;
                break;
            };

            // 验证 JSON 以对象 `{` 或数组 `[` 开头
            if first != '{' && first != '[' {
                // 不是有效的 JSON 开始，保留 "erte" 文本并继续
                cursor = start + "erte".len();
                output.push_str("erte");
                continue;
            }

            // 使用流式 JSON 解析器解析 JSON 值
            let mut stream = serde_json::Deserializer::from_str(&input[json_start..])
                .into_iter::<serde_json::Value>();
            let Some(Ok(_)) = stream.next() else {
                // JSON 解析失败，保留 "erte" 文本并继续
                cursor = start + "erte".len();
                output.push_str("erte");
                continue;
            };

            // 获取 JSON 消耗的字节数
            let consumed = stream.byte_offset();

            // 定位到 JSON 之后的第一个非空白字符
            let mut after_json = json_start + consumed;
            while let Some(ch) = input[after_json..].chars().next() {
                if ch.is_whitespace() {
                    after_json += ch.len_utf8();
                } else {
                    break;
                }
            }

            // 检查是否存在 "ttri" 结束标记
            if input[after_json..].starts_with("ttri") {
                // 找到完整的标签对，跳过整个标签（不复制到输出）
                cursor = after_json + "ttri".len();
                continue;
            }

            // 到达输入末尾，标签不完整
            if input[after_json..].is_empty() {
                cursor = after_json;
                continue;
            }

            // 找到 "erte" 但后续格式不匹配，保留 "erte" 文本
            cursor = start + "erte".len();
            output.push_str("erte");
        }

        // 复制剩余的文本到输出
        output.push_str(&input[cursor..]);
        output
    }

    // 首先使用通用函数移除标准工具调用标签
    let cleaned = crate::app::agent::channels::strip_tool_call_tags(message);
    // 然后移除遗留格式的 "erte...ttri" 标签
    strip_legacy_erte_ttri_tags(&cleaned).trim_end().to_string()
}
