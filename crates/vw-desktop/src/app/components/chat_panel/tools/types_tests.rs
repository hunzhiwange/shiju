//! 验证工具视图领域类型完整保留调用方显式提供的字段。
//! 该构造测试防止后续默认值或字段重排悄然覆盖上游解析结果。

use super::types::{
    ChangeFile, ChangeFileSummary, CommandItem, EXPLORE_GROUP_TOOL_IDX, ExploreItem,
};

#[test]
fn tool_types_preserve_explicit_fields() {
    let change = ChangeFile {
        path: "src/main.rs".to_string(),
        additions: 2,
        deletions: 1,
        before: "old".to_string(),
        after: "new".to_string(),
    };
    let summary =
        ChangeFileSummary { kind: 'M', path: change.path.clone(), additions: 2, deletions: 1 };
    let item = ExploreItem { tool_idx: EXPLORE_GROUP_TOOL_IDX, raw: "raw".to_string() };
    let command = CommandItem { tool_idx: 7, raw: "tool bash\n{}".to_string() };

    assert_eq!(summary.path, "src/main.rs");
    assert_eq!(summary.additions, change.additions);
    assert_eq!(item.tool_idx, EXPLORE_GROUP_TOOL_IDX);
    assert_eq!(command.tool_idx, 7);
    assert_eq!(command.raw, "tool bash\n{}");
}
