//! 工具 diff 预览解析门面。

pub use diff_text::{
    count_apply_patch_format_changes, count_unified_diff_changes, extract_diff_block, file_preview,
    looks_like_unified_diff, parse_apply_patch_line_changes, parse_apply_patch_summary,
};
