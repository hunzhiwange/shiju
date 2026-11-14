#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct JsonlSpan {
    /// 行首的字节偏移
    pub offset: u64,
    /// 整行字节数，含行尾 `\n`（文件最后一行可能没有）
    pub len: u32,
}

impl JsonlSpan {
    /// 指向本行内 `pointer`（RFC 6901）处内容的引用
    pub fn content_ref(&self, pointer: impl Into<String>) -> ContentRef {
        ContentRef::Jsonl {
            offset: self.offset,
            len: self.len,
            pointer: pointer.into(),
        }
    }
}

/// JSONL 的一行及其字节区间。
pub struct LineSpan<'a> {
    pub span: JsonlSpan,
    /// 行内容，已去掉行尾 `\r\n` / `\n`
    pub bytes: &'a [u8],
}

/// 用 `read_until(b'\n')` 逐行读取并累计字节偏移，供需要回溯原文的解析器使用
/// （所有 JSONL 解析器共用这一份，`ContentRef::Jsonl` 的区间口径因此一致）。
///
/// 复用同一块缓冲区，不做 UTF-8 校验（交给 `serde_json::from_slice`），
/// 所以比 `BufRead::lines()` 少一次分配和一次校验。
pub struct LineSpans<R> {
    reader: R,
    buf: Vec<u8>,
    offset: u64,
}

impl<R: BufRead> LineSpans<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buf: Vec::with_capacity(64 * 1024),
            offset: 0,
        }
    }

    /// 读取下一行；到文件末尾返回 `Ok(None)`。
    pub fn next_line(&mut self) -> io::Result<Option<LineSpan<'_>>> {
        self.buf.clear();
        let read = self.reader.read_until(b'\n', &mut self.buf)?;
        if read == 0 {
            return Ok(None);
        }
        let offset = self.offset;
        self.offset += read as u64;

        let mut end = self.buf.len();
        if end > 0 && self.buf[end - 1] == b'\n' {
            end -= 1;
            if end > 0 && self.buf[end - 1] == b'\r' {
                end -= 1;
            }
        }
        Ok(Some(LineSpan {
            span: JsonlSpan {
                offset,
                len: u32::try_from(read).unwrap_or(u32::MAX),
            },
            bytes: &self.buf[..end],
        }))
    }
}

/// user 消息不开新轮，归入当前轮。旧格式消息（无 blocks）看 `content` 是否非空。
pub fn starts_turn(message: &SessionMessage) -> bool {
    if message.role != "user" || message.injected {
        return false;
    }
    if message.blocks.is_empty() {
        return !message.content.trim().is_empty();
    }
    message.blocks.iter().any(|block| match block {
        SessionBlock::Text { text, .. } => !text.trim().is_empty(),
        SessionBlock::Image { .. } => true,
        SessionBlock::Event { kind, .. } => *kind == EventKind::SlashCommand,
        _ => false,
    })
}

/// 没有原生轮次概念的解析器共用（§4）：按 [`starts_turn`] 递增生成 `t{n}`，
/// 首条提问之前的内容归 `t0`。已有 `turn_id` 的消息保持不变。
pub fn assign_turn_ids(messages: &mut [SessionMessage]) {
    let mut turn = 0u32;
    for message in messages {
        if starts_turn(message) {
            turn += 1;
        }
        if message.turn_id.is_none() {
            message.turn_id = Some(format!("t{turn}"));
        }
    }
}

// ─── diff 统计 ───────────────────────────────────────────────────────────
