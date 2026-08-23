//! 验证补丁预览合并跨平台路径时能够规范化并去除重复变更。
//! 同一路径同时使用正反斜杠，避免平台表示差异生成重复文件卡片。

use super::{ChangeFile, merge_apply_patch_changes};

#[test]
fn merge_apply_patch_changes_deduplicates_normalized_cross_platform_paths() {
    let primary = vec![ChangeFile {
        path: r"a\src\main.rs".to_string(),
        additions: 0,
        deletions: 1,
        before: "old".to_string(),
        after: String::new(),
    }];
    let fallback = vec![ChangeFile {
        path: "b/src/main.rs".to_string(),
        additions: 1,
        deletions: 0,
        before: String::new(),
        after: "new".to_string(),
    }];

    let merged = merge_apply_patch_changes(primary, fallback);

    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].before, "old");
    assert_eq!(merged[0].after, "new");
    assert_eq!(merged[0].additions, 1);
    assert_eq!(merged[0].deletions, 1);
}
