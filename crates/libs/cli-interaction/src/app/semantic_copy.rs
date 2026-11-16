//! 从当前会话领域消息提取复制内容，不读取渲染缓冲。
use cli_interaction_model::{UiMessage, UiTurnTerminal};
use cli_interaction_state::TuiState;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};

pub(crate) fn answer_text(state: &TuiState, code_block: Option<usize>) -> Result<String, String> {
    let answer = state.messages.iter().rev().take_while(|message| !matches!(message, UiMessage::User(_))).find_map(|message| match message {
        UiMessage::Assistant(answer) => Some(answer),
        _ => None,
    }).ok_or_else(|| "当前会话没有可复制的助手回答（工具输出不能作为回答）".to_string())?;
    if answer.base.session_id.is_some()
        && answer.base.session_id.as_deref() != state.session.session_id.as_deref()
    {
        return Err("回答不属于当前会话，无法复制".to_string());
    }
    if !matches!(answer.terminal, UiTurnTerminal::Done { .. }) {
        return Err("最近的助手回答尚未成功完成，无法复制".to_string());
    }
    if answer.text.trim().is_empty() {
        return Err("最近的助手回答没有文本内容".to_string());
    }
    match code_block {
        None => Ok(answer.text.clone()),
        Some(index) => code_blocks(&answer.text).into_iter().nth(index.saturating_sub(1))
            .filter(|_| index > 0)
            .ok_or_else(|| format!("回答中没有第 {index} 个代码块")),
    }
}

/// Markdown 解析器处理围栏、缩进、列表内代码块；代码内容保持 Unicode 和相对缩进。
fn code_blocks(markdown: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current = None;
    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => current = Some(String::new()),
            Event::Text(text) => {
                if let Some(code) = current.as_mut() {
                    code.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(code) = current.take() {
                    blocks.push(code);
                }
            }
            _ => {}
        }
    }
    blocks
}
