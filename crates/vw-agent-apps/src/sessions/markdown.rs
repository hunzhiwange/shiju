//! Host-side Markdown generation, adapted from reader/exportMarkdown.ts.
//! Only server-parsed transcripts may be supplied, with the fingerprint captured
//! when they were loaded. This is not a wire entry point for arbitrary blocks.
//! Full text/input/diff references are resolved through the existing readers;
//! errors and missing truncated bodies fail the export rather than silently
//! exporting a preview. Each body and the complete export are bounded to 32MiB.
//! Images follow upstream's description-only export: no paths, URLs, bytes or
//! image references are embedded or loaded. Saving/clipboard belongs to clients.
use vw_api_types::external_session::{ContentRef, ImageRef, SessionBlock, SessionMessage};
use super::{cache::{self, Fingerprint, Transcript}, content_file, content_sqlite,
    source::ValidatedSource, turns::{self, ReaderStep, ReaderText, SessionTurn}};

#[derive(Debug, Default)]
pub struct MarkdownOptions {
    /// Thinking is opt-in. Redacted thinking never exports text, summary or refs.
    pub include_thinking: bool,
}

/// `turn_key` is the stable key issued by the reading projection, not a message
/// index. None exports the transcript; an unknown key is an explicit error.
/// Authorization and SourceRegistry resolution must precede every invocation.
pub fn export_session_markdown(
    source: &ValidatedSource,
    transcript: &Transcript,
    fingerprint: &Fingerprint,
    title: &str,
    turn_key: Option<&str>,
    options: &MarkdownOptions,
) -> Result<String, String> {
    check_source(source, fingerprint)?;
    let turns = turns::project(transcript.messages(), source.identity().provider);
    let selected: Vec<_> = match turn_key {
        Some(key) => vec![turns.iter().find(|t| t.key == key).ok_or("轮次不存在")?],
        None => turns.iter().collect(),
    };
    let mut writer = Writer { source, options, messages: transcript.messages(), output: String::new() };
    writer.push(&format!("# {}\n\n", one_line(title)))?;
    for (index, turn) in selected.iter().enumerate() {
        if index > 0 { writer.push("---\n\n")?; }
        writer.turn(turn)?;
    }
    check_source(source, fingerprint)?;
    Ok(writer.output)
}
fn check_source(source: &ValidatedSource, expected: &Fingerprint) -> Result<(), String> {
    let current = cache::source_fingerprint(source).map_err(|_| "会话已更新，请重新读取")?;
    if &current != expected { return Err("会话已更新，请重新读取".into()); }
    Ok(())
}
fn one_line(value: &str) -> String { value.split_whitespace().collect::<Vec<_>>().join(" ") }
struct Writer<'a> { source: &'a ValidatedSource, options: &'a MarkdownOptions, messages: &'a [SessionMessage], output: String }
impl Writer<'_> {
    fn push(&mut self, text: &str) -> Result<(), String> {
        if self.output.len().saturating_add(text.len()) as u64 > content_file::MAX_TEXT_BYTES {
            return Err("Markdown超过32MiB".into());
        }
        self.output.push_str(text);
        Ok(())
    }
    fn body(&self, preview: &str, full: Option<&ContentRef>) -> Result<String, String> {
        match full {
            Some(reference @ ContentRef::Sqlite { .. }) => content_sqlite::resolve_content_ref(self.source, reference),
            Some(reference) => content_file::resolve_content_ref(self.source, reference),
            None => Ok(preview.to_owned()),
        }
    }
    fn fenced(&mut self, body: &str, language: &str) -> Result<(), String> {
        // Choose a fence longer than every run in the original content.
        let longest = body.split(|c| c != '`').map(str::len).max().unwrap_or(0);
        let fence = "`".repeat(longest.max(2) + 1);
        self.push(&format!("{fence}{language}\n{body}\n{fence}\n\n"))
    }
    fn image(&mut self, image: &ImageRef) -> Result<(), String> {
        self.push(&format!("[图片：{}]\n\n", one_line(image.alt.as_deref().unwrap_or("图片"))))
    }
    fn text(&mut self, label: &str, text: &ReaderText) -> Result<(), String> {
        self.push(&format!("**{label}**"))?;
        if let Some(ts) = text.ts { self.push(&format!(" · {ts} ms"))?; }
        if let Some(model) = &text.model { self.push(&format!(" · model: {}", one_line(model)))?; }
        self.push("\n\n")?;
        for block in &text.blocks { self.block(block)?; }
        Ok(())
    }
    fn turn(&mut self, turn: &SessionTurn) -> Result<(), String> {
        // Preserve metadata even when the message is an intermediate tool step.
        let messages = self.messages;
        for message in &messages[turn.first_message_index..=turn.last_message_index] {
            if message.injected { continue; }
            if let Some(meta) = &message.meta {
                let json = serde_json::to_string(meta).map_err(|_| "无法导出消息元数据")?;
                if json != "{}" { self.push("消息元数据：\n\n")?; self.fenced(&json, "json")?; }
            }
        }
        for step in &turn.leading_events { self.step(step)?; }
        if let Some(question) = &turn.question { self.text("你", question)?; }
        for step in &turn.steps { self.step(step)?; }
        if let Some(reply) = &turn.final_reply { self.text("助手", reply)?; }
        for step in &turn.trailing_events { self.step(step)?; }
        // Costs can exist without a visible step/final reply; export each once.
        for cost in &turn.step_costs { self.block(cost)?; }
        Ok(())
    }
    fn step(&mut self, step: &ReaderStep) -> Result<(), String> {
        if !step.children.is_empty() {
            for child in &step.children { self.step(child)?; }
            return Ok(());
        }
        self.block(&step.block)?;
        if let Some((_, result)) = &step.result { self.block(result)?; }
        if step.body.is_some() {
            // The reader attaches system text to its event. Recover its issued
            // reference instead of accidentally exporting the attached preview.
            let blocks = turns::effective_blocks(&self.messages[step.message_index], step.message_index);
            for block in &blocks {
                if matches!(block, SessionBlock::Text { .. }) { self.block(block)?; }
            }
        }
        Ok(())
    }
    fn block(&mut self, block: &SessionBlock) -> Result<(), String> {
        match block {
            SessionBlock::Text { text, full } => {
                let body = self.body(text, full.as_ref())?;
                self.push(&body)?; self.push("\n\n")
            }
            SessionBlock::Thinking { redacted: true, .. } => {
                if self.options.include_thinking { self.push("- 思考：[已隐藏]\n\n") } else { Ok(()) }
            }
            SessionBlock::Thinking { text, summary, full, duration_ms, .. } => {
                if !self.options.include_thinking { return Ok(()); }
                self.push(&format!("- 思考{}\n\n", duration_ms.map(|ms|format!(" · {ms} ms")).unwrap_or_default()))?;
                let body = self.body(if text.is_empty() { summary.as_deref().unwrap_or("") } else { text }, full.as_ref())?;
                self.fenced(&body, "text")
            }
            SessionBlock::ToolCall { raw_name, title, input_preview, input_total_len, input_full, diff, .. } => {
                self.push(&format!("- ⏺ {}({})\n\n", one_line(raw_name), one_line(title)))?;
                if input_full.is_none() && *input_total_len as usize > input_preview.chars().count() {
                    return Err("工具输入不完整，无法导出".into());
                }
                let input = self.body(input_preview, input_full.as_ref())?;
                if !input.is_empty() { self.fenced(&input, "text")?; }
                if let Some(diff) = diff {
                    self.push(&format!("- diff: +{} −{}\n", diff.added, diff.removed))?;
                    for file in &diff.files {
                        self.push(&format!("  - {:?} {} +{} −{}\n", file.op, one_line(&file.path), file.added, file.removed))?;
                    }
                    self.push("\n")?;
                    if let Some(reference) = &diff.full {
                        let body = self.body("", Some(reference))?;
                        self.fenced(&body, "diff")?;
                    }
                }
                Ok(())
            }
            SessionBlock::ToolResult { status, preview, truncated, full, exit_code, duration_ms, images, .. } => {
                if *truncated && full.is_none() { return Err("工具输出不完整，无法导出".into()); }
                self.push(&format!("- 结果：{status:?}"))?;
                if let Some(code) = exit_code { self.push(&format!(" · exit {code}"))?; }
                if let Some(ms) = duration_ms { self.push(&format!(" · {ms} ms"))?; }
                self.push("\n\n")?;
                let body = self.body(preview, full.as_ref())?;
                if !body.is_empty() { self.fenced(&body, "text")?; }
                for image in images { self.image(image)?; }
                Ok(())
            }
            SessionBlock::Image { image } => self.image(image),
            SessionBlock::Event { kind, text, url, full } => {
                self.push(&format!("- {kind:?}\n\n"))?;
                let body = self.body(text.as_deref().unwrap_or(""), full.as_ref())?;
                if !body.is_empty() { self.push(&body)?; self.push("\n\n")?; }
                if let Some(url) = url { self.push(&format!("{}\n\n", one_line(url)))?; }
                Ok(())
            }
            SessionBlock::Step { phase, tokens, cost_usd, reason } => {
                self.push(&format!("- {phase:?}"))?;
                if let Some(tokens) = tokens { self.push(&format!(" · tokens: {tokens}"))?; }
                if let Some(usd) = cost_usd { self.push(&format!(" · USD: {usd}"))?; }
                if let Some(reason) = reason { self.push(&format!(" · {}", one_line(reason)))?; }
                self.push("\n\n")
            }
        }
    }
}
