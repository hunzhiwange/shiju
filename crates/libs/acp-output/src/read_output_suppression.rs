//! 识别读类工具并抑制其冗余输出内容。

pub const SUPPRESSED_READ_OUTPUT: &str = "[read output suppressed]";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReadLikeToolDescriptor {
    pub title: Option<String>,
    pub kind: Option<String>,
}

fn infer_tool_kind_from_title(title: Option<&str>) -> Option<&'static str> {
    let normalized = title?.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }

    let head = normalized.split(':').next().map(str::trim)?;
    if head.is_empty() {
        return None;
    }

    if head.contains("read")
        || head.contains("cat")
        || head.contains("open")
        || head.contains("view")
    {
        return Some("read");
    }

    None
}

pub fn is_read_like_tool(tool: &ReadLikeToolDescriptor) -> bool {
    tool.kind
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .is_some_and(|kind| kind == "read")
        || infer_tool_kind_from_title(tool.title.as_deref()) == Some("read")
}

#[cfg(test)]
#[path = "read_output_suppression_tests.rs"]
mod read_output_suppression_tests;
