// 输入元素只保存字节范围与可选占位文本，实际内容仍由调用方字符串持有。
pub const MAX_USER_INPUT_TEXT_CHARS: usize = 1 << 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextElement {
    pub byte_range: ByteRange,
    placeholder: Option<String>,
}

impl TextElement {
    pub fn new(byte_range: ByteRange, placeholder: Option<String>) -> Self {
        Self { byte_range, placeholder }
    }

    pub fn placeholder<'a>(&'a self, text: &'a str) -> Option<&'a str> {
        self.placeholder
            .as_deref()
            .or_else(|| text.get(self.byte_range.start..self.byte_range.end))
    }
}
