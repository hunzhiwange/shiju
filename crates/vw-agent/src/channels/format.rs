//! 通道消息格式化模块
//!
//! 本模块提供用于处理输出到各通道的消息格式化功能，主要职责包括：
//!
//! - **工具调用标签清理**：从 LLM 响应中移除内部协议标签（如 `<function_calls>`、`<toolcall>` 等），
//!   防止将内部协议细节泄露给终端用户
//! - **通道投递指令**：为不同通道（如 Telegram、WhatsApp）提供格式化指导，
//!   确保 LLM 输出符合各通道的渲染特性
//! - **内部详情暴露判断**：根据用户消息内容智能判断是否应展示内部工具调用细节
//! - **工具上下文摘要提取**：从历史消息中提取已使用工具的摘要信息
//! - **通道响应净化**：清理响应中的工具相关 JSON 片段，确保输出干净友好
//!
//! # 设计原则
//!
//! - **安全优先**：默认隐藏内部工具调用细节，仅在用户明确请求时才暴露
//! - **语言感知**：支持中文和英文的意图识别，正确理解否定表达
//! - **通道适配**：针对不同通道的渲染能力提供定制化的格式化指导

use super::*;

/// 从外发消息中移除工具调用 XML 标签
///
/// LLM 响应可能包含 `<function_calls>`、`<function_call>`、`atok`、`<toolcall>`、
/// `<tool-call>`、`<tool>` 或 `<invoke>` 等内部协议块，这些内容不应转发给任何通道的终端用户。
///
/// # 参数
///
/// - `message`: 原始消息字符串，可能包含工具调用标签
///
/// # 返回值
///
/// 返回清理后的消息字符串，所有工具调用标签及其内容已被移除，多余的空行已被压缩
///
/// # 处理逻辑
///
/// 1. 扫描消息中的所有工具调用开标签
/// 2. 对于每个匹配的开标签，寻找对应的闭标签
/// 3. 如果找到闭标签，移除整个标签块
/// 4. 如果未找到闭标签但有有效 JSON，尝试提取 JSON 后的闭标签
/// 5. 压缩多余的空行（三个以上换行压缩为两个）
///
/// # 示例
///
/// ```ignore
/// let msg = "这是回复内容<function_calls>{\"name\":\"test\"}</function_calls>继续";
/// let cleaned = strip_tool_call_tags(msg);
/// assert_eq!(cleaned, "这是回复内容继续");
/// ```
pub(crate) fn strip_tool_call_tags(message: &str) -> String {
    /// 工具调用标签列表
    ///
    /// 包含所有需要被识别和移除的工具调用标签格式及对应闭标签
    const TOOL_CALL_SENTINEL: &str = "ฦ";
    const TOOL_CALL_TAGS: [(&str, &str); 8] = [
        ("<function_calls>", "</function_calls>"),
        ("<function_call>", "</function_call>"),
        ("<tool_call>", "</tool_call>"),
        ("<toolcall>", "</toolcall>"),
        ("<tool-call>", "</tool-call>"),
        ("<tool>", "</tool>"),
        ("<invoke>", "</invoke>"),
        (TOOL_CALL_SENTINEL, TOOL_CALL_SENTINEL),
    ];

    /// 在字符串中查找第一个出现的标签
    ///
    /// 遍历所有候选标签，返回最先出现的位置和标签内容
    ///
    /// # 参数
    ///
    /// - `haystack`: 待搜索的字符串
    /// - `tags`: 候选标签列表
    ///
    /// # 返回值
    ///
    /// 返回 `Some((位置, 标签))` 如果找到任一标签，否则返回 `None`
    fn find_first_tag<'a>(
        haystack: &str,
        tags: &'a [(&'a str, &'a str)],
    ) -> Option<(usize, &'a str, &'a str)> {
        tags.iter()
            .filter_map(|(open_tag, close_tag)| {
                haystack.find(open_tag).map(|idx| (idx, *open_tag, *close_tag))
            })
            .min_by_key(|(idx, _, _)| *idx)
    }

    fn trim_block_open_boundary(input: &str) -> String {
        input
            .rfind('\n')
            .and_then(|idx| input[idx + 1..].chars().all(char::is_whitespace).then_some(idx + 1))
            .map_or_else(|| input.to_string(), |idx| input[..idx].to_string())
    }

    fn trim_block_close_boundary(input: &str) -> &str {
        let Some(after_newline) = input.strip_prefix('\n') else {
            return input;
        };
        let trimmed = after_newline.trim_start_matches([' ', '\t']);
        if trimmed.is_empty() { input } else { trimmed }
    }

    /// 从字符串开头提取第一个完整 JSON 的结束位置
    ///
    /// 扫描字符串，找到第一个有效的 JSON 对象或数组，返回其结束位置
    ///
    /// # 参数
    ///
    /// - `input`: 待解析的字符串
    ///
    /// # 返回值
    ///
    /// 返回 `Some(结束位置)` 如果找到有效 JSON，否则返回 `None`
    fn extract_first_json_end(input: &str) -> Option<usize> {
        let trimmed = input.trim_start();
        let trim_offset = input.len().saturating_sub(trimmed.len());

        // 遍历每个字符，寻找 JSON 开头的 '{' 或 '['
        for (byte_idx, ch) in trimmed.char_indices() {
            if ch != '{' && ch != '[' {
                continue;
            }

            let slice = &trimmed[byte_idx..];
            let mut stream =
                serde_json::Deserializer::from_str(slice).into_iter::<serde_json::Value>();
            if let Some(Ok(_value)) = stream.next() {
                let consumed = stream.byte_offset();
                return Some(trim_offset + byte_idx + consumed);
            }
        }

        None
    }

    /// 移除字符串开头的所有闭标签
    ///
    /// 循环移除开头以 `</` 开始的标签，直到遇到非闭标签内容
    ///
    /// # 参数
    ///
    /// - `input`: 待处理的字符串
    ///
    /// # 返回值
    ///
    /// 返回移除开头闭标签后的字符串切片
    fn strip_leading_close_tags(mut input: &str) -> &str {
        loop {
            let trimmed = input.trim_start();
            if !trimmed.starts_with("</") {
                return trimmed;
            }

            let Some(close_end) = trimmed.find('>') else {
                return "";
            };
            input = &trimmed[close_end + 1..];
        }
    }

    // 保留的消息片段列表
    let mut kept_segments = Vec::new();
    // 剩余待处理的消息
    let mut remaining = message;

    // 循环处理消息中的所有工具调用标签
    while let Some((start, open_tag, close_tag)) = find_first_tag(remaining, &TOOL_CALL_TAGS) {
        // 提取标签前的内容
        let before = &remaining[..start];
        if !before.is_empty() {
            kept_segments.push(trim_block_open_boundary(before));
        }

        let after_open = &remaining[start + open_tag.len()..];

        // 尝试找到闭标签
        if let Some(close_idx) = after_open.find(close_tag) {
            remaining = trim_block_close_boundary(&after_open[close_idx + close_tag.len()..]);
            continue;
        }

        // 如果未找到闭标签，尝试提取 JSON 后的位置
        if let Some(consumed_end) = extract_first_json_end(after_open) {
            remaining =
                trim_block_close_boundary(strip_leading_close_tags(&after_open[consumed_end..]));
            continue;
        }

        // 无法处理的情况，保留剩余内容并终止
        kept_segments.push(remaining[start..].to_string());
        remaining = "";
        break;
    }

    // 添加最后剩余的内容
    if !remaining.is_empty() {
        kept_segments.push(remaining.to_string());
    }

    let mut result = kept_segments.concat();

    // 清理多余空行（但保留段落分隔）
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    result.trim().to_string()
}

/// 获取指定通道的投递格式指令
///
/// 返回针对特定通道的格式化指导文本，帮助 LLM 生成符合该通道渲染特性的输出
///
/// # 参数
///
/// - `channel_name`: 通道名称（如 "telegram"、"whatsapp"）
///
/// # 返回值
///
/// 返回 `Some(指令文本)` 如果该通道有特定的格式化指导，否则返回 `None`
///
/// # 支持的通道
///
/// - **telegram**: 支持 Markdown 格式、媒体附件标记、表情符号等
/// - **whatsapp**: 支持基础格式（粗体）、媒体附件标记，但不支持标题和表格
pub(crate) fn channel_delivery_instructions(channel_name: &str) -> Option<&'static str> {
    match channel_name {
        "telegram" => Some(
            "When responding on Telegram:\n\
             - Include media markers for files or URLs that should be sent as attachments\n\
             - Use **bold** for key terms, section titles, and important info (renders as <b>)\n\
             - Use *italic* for emphasis (renders as <i>)\n\
             - Use `backticks` for inline code, commands, or technical terms\n\
             - Use triple backticks for code blocks\n\
             - Use emoji naturally to add personality — but don't overdo it\n\
             - Be concise and direct. Skip filler phrases like 'Great question!' or 'Certainly!'\n\
             - Structure longer answers with bold headers, not raw markdown ## headers\n\
             - For media attachments use markers: [IMAGE:<path-or-url>], [DOCUMENT:<path-or-url>], [VIDEO:<path-or-url>], [AUDIO:<path-or-url>], or [VOICE:<path-or-url>]\n\
             - Keep normal text outside markers and never wrap markers in code fences.\n\
             - Use tool results silently: answer the latest user message directly, and do not narrate delayed/internal tool execution bookkeeping.",
        ),
        "whatsapp" => Some(
            "When responding on WhatsApp:\n\
             - Use *bold* for emphasis (WhatsApp uses single asterisks).\n\
             - Be concise. No markdown headers (## etc.) — they don't render.\n\
             - No markdown tables — use bullet lists instead.\n\
             - For sending images, documents, videos, or audio files use markers: [IMAGE:<absolute-path>], [DOCUMENT:<absolute-path>], [VIDEO:<absolute-path>], [AUDIO:<absolute-path>]\n\
             - The path MUST be an absolute filesystem path to a local file (e.g. [IMAGE:/home/nicolas/.vibewindow/workspace/images/chart.png]).\n\
             - Keep normal text outside markers and never wrap markers in code fences.\n\
             - You can combine text and media in one response — text is sent first, then each attachment.\n\
             - Use tool results silently: answer the latest user message directly, and do not narrate delayed/internal tool execution bookkeeping.",
        ),
        _ => None,
    }
}

/// 判断是否应向用户暴露内部工具调用细节
///
/// 分析用户消息内容，判断用户是否明确请求查看命令、工具调用、函数调用等内部细节。
/// 支持中英文意图识别，并正确处理否定表达（如"不要显示命令"）。
///
/// # 参数
///
/// - `user_message`: 用户发送的消息文本
///
/// # 返回值
///
/// - `true`: 用户明确请求查看内部工具调用细节
/// - `false`: 用户未请求，或明确表示不想看到内部细节
///
/// # 识别逻辑
///
/// 1. 首先检测消息是否提及内部细节（中英文关键词）
/// 2. 如果提及，检查是否存在否定表达（优先返回 `false`，采用安全关闭策略）
/// 3. 如果没有否定表达，检查是否存在肯定表达
/// 4. 结合动词（如"显示"、"输出"、"show"、"output"）进行综合判断
///
/// # 示例
///
/// ```ignore
/// assert_eq!(should_expose_internal_tool_details("show command"), true);
/// assert_eq!(should_expose_internal_tool_details("don't show command"), false);
/// assert_eq!(should_expose_internal_tool_details("显示命令"), true);
/// assert_eq!(should_expose_internal_tool_details("不要显示命令"), false);
/// ```
pub(crate) fn should_expose_internal_tool_details(user_message: &str) -> bool {
    let trimmed = user_message.trim();
    if trimmed.is_empty() {
        return false;
    }

    let lower = trimmed.to_ascii_lowercase();
    // 检测英文的内部细节提及
    let mentions_internal_details_en = lower.contains("command")
        || lower.contains("tool call")
        || lower.contains("function call")
        || lower.contains("execution trace")
        || lower.contains("internal step");
    // 检测中文/日文/韩文的内部细节提及
    let mentions_internal_details_cjk = trimmed.contains("命令")
        || trimmed.contains("工具调用")
        || trimmed.contains("函数调用")
        || trimmed.contains("执行过程");

    /// 英文否定提示词列表
    ///
    /// 当检测到这些短语时，表明用户不想看到内部细节
    /// 采用"安全关闭"策略：即使有其他肯定信号，否定表达优先
    const ENGLISH_NEGATIVE_HINTS: [&str; 18] = [
        "don't show command",
        "don't show commands",
        "do not show command",
        "do not show commands",
        "don't output command",
        "do not output command",
        "without command",
        "without commands",
        "no command output",
        "hide command",
        "hide commands",
        "omit command",
        "omit commands",
        "skip command",
        "skip commands",
        "don't show tool call",
        "do not show tool call",
        "do not show function call",
    ];
    // 英文否定检测：如果提及内部细节且包含否定表达，返回 false
    if mentions_internal_details_en
        && ENGLISH_NEGATIVE_HINTS.iter().any(|hint| lower.contains(hint))
    {
        return false;
    }

    /// 中文否定提示词列表
    ///
    /// 涵盖各种表达"不想看到命令/工具调用"的中文说法
    const CJK_NEGATIVE_HINTS: [&str; 22] = [
        "不要输出命令",
        "不要显示命令",
        "不要展示命令",
        "不要带上命令",
        "不要附上命令",
        "别输出命令",
        "别显示命令",
        "别展示命令",
        "不要输出工具调用",
        "不要显示工具调用",
        "不要展示工具调用",
        "别输出工具调用",
        "别显示工具调用",
        "不要输出函数调用",
        "不要显示函数调用",
        "不要展示函数调用",
        "别输出函数调用",
        "别显示函数调用",
        "不要执行过程",
        "不要过程",
        "不要内部步骤",
        "别把命令",
    ];
    // 中文否定检测：如果提及内部细节且包含否定表达，返回 false
    if mentions_internal_details_cjk && CJK_NEGATIVE_HINTS.iter().any(|hint| trimmed.contains(hint))
    {
        return false;
    }

    /// 英文肯定提示词列表
    ///
    /// 这些短语明确表示用户想要看到内部细节
    const ENGLISH_HINTS: [&str; 20] = [
        "show command",
        "show commands",
        "output command",
        "output commands",
        "print command",
        "print commands",
        "include command",
        "include commands",
        "with command",
        "with commands",
        "show tool call",
        "show tool calls",
        "show function call",
        "show function calls",
        "reveal tool call",
        "reveal function call",
        "tool call json",
        "function call json",
        "execution trace",
        "internal steps",
    ];
    // 英文肯定检测：直接匹配肯定表达
    if ENGLISH_HINTS.iter().any(|hint| lower.contains(hint)) {
        return true;
    }

    /// 英文动词列表
    ///
    /// 当消息提及内部细节且包含这些动词时，倾向于返回 true
    const ENGLISH_VERBS: [&str; 7] =
        ["show", "output", "print", "include", "reveal", "display", "share"];
    // 结合内部细节提及和动词进行判断
    if mentions_internal_details_en && ENGLISH_VERBS.iter().any(|verb| lower.contains(verb)) {
        return true;
    }

    /// 中文肯定提示词列表
    ///
    /// 这些短语明确表示用户想要看到内部细节
    const CJK_HINTS: [&str; 14] = [
        "输出命令",
        "显示命令",
        "展示命令",
        "命令发给我",
        "带上命令",
        "输出工具调用",
        "显示工具调用",
        "展示工具调用",
        "输出函数调用",
        "显示函数调用",
        "展示函数调用",
        "函数指令",
        "工具指令",
        "执行过程",
    ];
    // 中文肯定检测：直接匹配肯定表达
    if CJK_HINTS.iter().any(|hint| trimmed.contains(hint)) {
        return true;
    }

    /// 中文动词列表
    ///
    /// 当消息提及内部细节且包含这些动词时，倾向于返回 true
    const CJK_VERBS: [&str; 9] =
        ["输出", "显示", "展示", "发我", "给我", "带上", "附上", "贴出", "列出"];
    // 结合内部细节提及和动词进行最终判断
    mentions_internal_details_cjk && CJK_VERBS.iter().any(|verb| trimmed.contains(verb))
}

/// 分离内部进度标记与实际内容
///
/// 检测消息增量（delta）是否以草稿进度标记开头，如果是则分离标记和剩余内容
///
/// # 参数
///
/// - `delta`: 消息增量字符串
///
/// # 返回值
///
/// 返回元组 `(is_draft_progress, content)`:
/// - `is_draft_progress`: 是否为草稿进度消息
/// - `content`: 移除标记后的内容（如果不是进度消息，返回原始 delta）
pub(crate) fn split_internal_progress_delta(delta: &str) -> (bool, &str) {
    if let Some(rest) = delta.strip_prefix(crate::app::agent::agent::loop_::DRAFT_PROGRESS_SENTINEL)
    {
        if rest.starts_with(crate::app::agent::agent::loop_::DRAFT_WS_EVENT_SENTINEL) {
            (true, "")
        } else {
            (true, rest)
        }
    } else {
        (false, delta)
    }
}

/// 从历史消息中提取工具交互的紧凑摘要
///
/// 扫描在 `run_tool_call_loop` 期间添加的历史消息，从助手消息中提取 `atok` 标签
/// 或原生工具调用 JSON，收集使用的工具名称。
///
/// # 参数
///
/// - `history`: 聊天历史消息列表
/// - `start_index`: 开始扫描的历史消息索引
///
/// # 返回值
///
/// 返回工具使用摘要字符串，格式为 `[Used tools: tool1, tool2, ...]`；
/// 如果没有工具被调用，返回空字符串
///
/// # 扫描逻辑
///
/// 1. 遍历从 `start_index` 开始的所有历史消息
/// 2. 对于助手消息：解析工具调用标签和原生 JSON 格式
/// 3. 对于用户消息：解析工具结果标签（Prompt 模式下的工具调用结果）
/// 4. 去重收集所有工具名称
pub(crate) fn extract_tool_context_summary(history: &[ChatMessage], start_index: usize) -> String {
    /// 向工具名称列表添加唯一名称
    ///
    /// 只有当名称非空且列表中不存在时才添加
    fn push_unique_tool_name(tool_names: &mut Vec<String>, name: &str) {
        let candidate = name.trim();
        if candidate.is_empty() {
            return;
        }
        if !tool_names.iter().any(|existing| existing == candidate) {
            tool_names.push(candidate.to_string());
        }
    }

    /// 从工具调用标签中收集工具名称
    ///
    /// 解析消息内容中的 `atok`、`<toolcall>` 等标签，提取其中的工具名称
    fn collect_tool_names_from_tool_call_tags(content: &str, tool_names: &mut Vec<String>) {
        /// 标签对列表：开标签和对应的闭标签
        const TAG_PAIRS: [(&str, &str); 4] = [
            ("<tool_call>", "</tool_call>"),
            ("<toolcall>", "</toolcall>"),
            ("<tool-call>", "</tool-call>"),
            ("<invoke>", "</invoke>"),
        ];

        for (open_tag, close_tag) in TAG_PAIRS {
            for segment in content.split(open_tag) {
                if let Some(json_end) = segment.find(close_tag) {
                    let json_str = segment[..json_end].trim();
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let Some(name) = val.get("name").and_then(|n| n.as_str()) {
                            push_unique_tool_name(tool_names, name);
                        }
                    }
                }
            }
        }
    }

    /// 从原生 JSON 工具调用格式中收集工具名称
    ///
    /// 解析包含 `tool_calls` 字段的 JSON 对象，提取工具名称
    fn collect_tool_names_from_native_json(content: &str, tool_names: &mut Vec<String>) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(calls) = val.get("tool_calls").and_then(|c| c.as_array()) {
                for call in calls {
                    let name = call
                        .get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(|n| n.as_str())
                        .or_else(|| call.get("name").and_then(|n| n.as_str()));
                    if let Some(name) = name {
                        push_unique_tool_name(tool_names, name);
                    }
                }
            }
        }
    }

    /// 从工具结果标签中收集工具名称
    ///
    /// 解析 `<tool_result name="...">` 标签，提取工具名称
    fn collect_tool_names_from_tool_results(content: &str, tool_names: &mut Vec<String>) {
        let marker = "<tool_result name=\"";
        let mut remaining = content;
        while let Some(start) = remaining.find(marker) {
            let name_start = start + marker.len();
            let after_name_start = &remaining[name_start..];
            if let Some(name_end) = after_name_start.find('"') {
                let name = &after_name_start[..name_end];
                push_unique_tool_name(tool_names, name);
                remaining = &after_name_start[name_end + 1..];
            } else {
                break;
            }
        }
    }

    let mut tool_names: Vec<String> = Vec::new();

    // 从指定索引开始遍历历史消息
    for msg in history.iter().skip(start_index) {
        match msg.role.as_str() {
            "assistant" => {
                // 从助手消息中收集工具调用
                collect_tool_names_from_tool_call_tags(&msg.content, &mut tool_names);
                collect_tool_names_from_native_json(&msg.content, &mut tool_names);
            }
            "user" => {
                // Prompt 模式下的工具调用后会有 [Tool results] 条目
                // 其中包含 `<tool_result name="...">` 标签和规范的工具名称
                collect_tool_names_from_tool_results(&msg.content, &mut tool_names);
            }
            _ => {}
        }
    }

    if tool_names.is_empty() {
        return String::new();
    }

    format!("[Used tools: {}]", tool_names.join(", "))
}

/// 净化通道响应，移除工具调用相关内容
///
/// 对响应字符串进行两阶段清理：首先移除工具调用标签，然后清理孤立的工具 JSON 片段
///
/// # 参数
///
/// - `response`: 原始响应字符串
/// - `tools`: 已注册的工具列表，用于识别工具名称
///
/// # 返回值
///
/// 返回净化后的响应字符串
pub(crate) fn sanitize_channel_response(response: &str, tools: &[Box<dyn Tool>]) -> String {
    let without_tool_tags = strip_tool_call_tags(response);
    let known_tool_names: HashSet<String> =
        tools.iter().map(|tool| tool.spec().id.to_ascii_lowercase()).collect();
    strip_isolated_tool_json_artifacts(&without_tool_tags, &known_tool_names)
}

/// 判断 JSON 值是否为工具调用载荷
///
/// 检查 JSON 值是否符合工具调用的结构特征：
/// - 包含 `function.name` 或 `name` 字段
/// - 包含 `arguments` 或 `parameters` 字段
/// - 名称匹配已知工具名
///
/// # 参数
///
/// - `value`: 待检查的 JSON 值
/// - `known_tool_names`: 已知工具名称集合（小写）
///
/// # 返回值
///
/// - `true`: JSON 值是有效的工具调用载荷
/// - `false`: 不是工具调用载荷
pub(crate) fn is_tool_call_payload(
    value: &serde_json::Value,
    known_tool_names: &HashSet<String>,
) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };

    // 提取名称和判断是否有参数字段
    // 支持两种格式：OpenAI 风格（function.name + function.arguments）
    // 和简化格式（name + arguments/parameters）
    let (name, has_args) =
        if let Some(function) = object.get("function").and_then(|f| f.as_object()) {
            (
                function
                    .get("name")
                    .and_then(|v| v.as_str())
                    .or_else(|| object.get("name").and_then(|v| v.as_str())),
                function.contains_key("arguments")
                    || function.contains_key("parameters")
                    || object.contains_key("arguments")
                    || object.contains_key("parameters"),
            )
        } else {
            (
                object.get("name").and_then(|v| v.as_str()),
                object.contains_key("arguments") || object.contains_key("parameters"),
            )
        };

    let Some(name) = name.map(str::trim).filter(|name| !name.is_empty()) else {
        return false;
    };

    has_args && known_tool_names.contains(&name.to_ascii_lowercase())
}

/// 判断 JSON 对象是否为工具结果载荷
///
/// 检查 JSON 对象是否符合工具调用结果的结构特征
///
/// # 参数
///
/// - `object`: JSON 对象
/// - `saw_tool_call_payload`: 之前是否已看到工具调用载荷
///
/// # 返回值
///
/// - `true`: JSON 对象是有效的工具结果载荷
/// - `false`: 不是工具结果载荷
///
/// # 判断条件
///
/// 1. 必须之前已看到工具调用载荷
/// 2. 必须包含 `result` 字段
/// 3. 只能包含已知的结果相关字段（result、id、tool_call_id、name、tool）
pub(crate) fn is_tool_result_payload(
    object: &serde_json::Map<String, serde_json::Value>,
    saw_tool_call_payload: bool,
) -> bool {
    if !saw_tool_call_payload || !object.contains_key("result") {
        return false;
    }

    object
        .keys()
        .all(|key| matches!(key.as_str(), "result" | "id" | "tool_call_id" | "name" | "tool"))
}

/// 净化工具相关的 JSON 值
///
/// 分析 JSON 值，如果是工具调用或工具结果相关内容，返回替换字符串和标记
///
/// # 参数
///
/// - `value`: 待净化的 JSON 值
/// - `known_tool_names`: 已知工具名称集合
/// - `saw_tool_call_payload`: 之前是否已看到工具调用载荷
///
/// # 返回值
///
/// 返回 `Some((替换内容, 是否标记工具调用))` 如果需要替换，否则返回 `None`
///
/// # 处理逻辑
///
/// 1. 单个工具调用对象：返回空字符串
/// 2. 工具调用数组：返回空字符串
/// 3. 包含 tool_calls 的对象：提取 content 字段返回
/// 4. 工具结果对象：返回空字符串
pub(crate) fn sanitize_tool_json_value(
    value: &serde_json::Value,
    known_tool_names: &HashSet<String>,
    saw_tool_call_payload: bool,
) -> Option<(String, bool)> {
    // 检查是否为单个工具调用载荷
    if is_tool_call_payload(value, known_tool_names) {
        return Some((String::new(), true));
    }

    // 检查是否为工具调用数组
    if let Some(array) = value.as_array() {
        if !array.is_empty()
            && array.iter().all(|item| is_tool_call_payload(item, known_tool_names))
        {
            return Some((String::new(), true));
        }
        return None;
    }

    let object = value.as_object()?;

    // 检查是否为包含 tool_calls 的消息对象
    if let Some(tool_calls) = object.get("tool_calls").and_then(|value| value.as_array()) {
        if !tool_calls.is_empty()
            && tool_calls.iter().all(|call| is_tool_call_payload(call, known_tool_names))
        {
            // 提取 content 字段作为替换内容
            let content = object
                .get("content")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            return Some((content, true));
        }
    }

    // 检查是否为工具结果载荷
    if is_tool_result_payload(object, saw_tool_call_payload) {
        return Some((String::new(), false));
    }

    None
}

/// 判断 JSON 片段是否在行内孤立存在
///
/// 检查 JSON 片段前后是否只有空白字符，用于确定是否应该清理该片段
///
/// # 参数
///
/// - `message`: 完整消息字符串
/// - `start`: JSON 片段起始位置
/// - `end`: JSON 片段结束位置
///
/// # 返回值
///
/// - `true`: JSON 片段在行内孤立存在（前后只有空白）
/// - `false`: JSON 片段与周围文本混合
pub(crate) fn is_line_isolated_json_segment(message: &str, start: usize, end: usize) -> bool {
    // 找到 JSON 片段所在行的起始位置
    let line_start = message[..start].rfind('\n').map_or(0, |idx| idx + 1);
    // 找到 JSON 片段所在行的结束位置
    let line_end = message[end..].find('\n').map_or(message.len(), |idx| end + idx);

    // 检查行内 JSON 片段前后是否只有空白
    message[line_start..start].trim().is_empty() && message[end..line_end].trim().is_empty()
}

/// 移除消息中孤立存在的工具 JSON 片段
///
/// 扫描消息内容，清理单独成行的工具调用/结果 JSON 片段，保留其他内容
///
/// # 参数
///
/// - `message`: 原始消息字符串
/// - `known_tool_names`: 已知工具名称集合（小写）
///
/// # 返回值
///
/// 返回清理后的消息字符串
///
/// # 处理流程
///
/// 1. 逐个扫描消息中的 JSON 对象/数组
/// 2. 判断是否为孤立的 JSON 片段（单独成行）
/// 3. 如果是工具相关内容，用空字符串替换
/// 4. 如果是包含 tool_calls 的消息对象，提取 content 字段
/// 5. 压缩多余空行并修剪首尾空白
pub(crate) fn strip_isolated_tool_json_artifacts(
    message: &str,
    known_tool_names: &HashSet<String>,
) -> String {
    // 预分配结果字符串容量
    let mut cleaned = String::with_capacity(message.len());
    // 当前处理位置
    let mut cursor = 0usize;
    // 标记是否已看到工具调用载荷
    let mut saw_tool_call_payload = false;

    // 主循环：扫描并处理消息中的所有 JSON 片段
    while cursor < message.len() {
        // 查找下一个 JSON 开头（'{' 或 '['）
        let Some(rel_start) = message[cursor..].find(['{', '[']) else {
            // 没有更多 JSON 片段，添加剩余内容并退出
            cleaned.push_str(&message[cursor..]);
            break;
        };

        let start = cursor + rel_start;
        // 添加 JSON 片段之前的内容
        cleaned.push_str(&message[cursor..start]);

        // 尝试解析 JSON
        let candidate = &message[start..];
        let mut stream =
            serde_json::Deserializer::from_str(candidate).into_iter::<serde_json::Value>();

        if let Some(Ok(value)) = stream.next() {
            let consumed = stream.byte_offset();
            let end = start + consumed;
            // 检查是否为孤立的 JSON 片段
            if is_line_isolated_json_segment(message, start, end) {
                // 尝试净化工具相关 JSON
                if let Some((replacement, marks_tool_call)) =
                    sanitize_tool_json_value(&value, known_tool_names, saw_tool_call_payload)
                {
                    if marks_tool_call {
                        saw_tool_call_payload = true;
                    }
                    // 添加替换内容（非空时）
                    if !replacement.trim().is_empty() {
                        cleaned.push_str(replacement.trim());
                    }
                    cursor = end;
                    continue;
                }
            }
        }

        // JSON 不是工具相关内容，保留开头的字符
        let ch = message[start..].chars().next().expect("json marker start must be valid");
        cleaned.push(ch);
        cursor = start + ch.len_utf8();
    }

    // 后处理：统一换行符并压缩多余空行
    let mut result = cleaned.replace("\r\n", "\n");
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }
    result.trim().to_string()
}

#[cfg(test)]
#[path = "format_tests.rs"]
mod format_tests;
