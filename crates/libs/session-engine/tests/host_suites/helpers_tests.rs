//! 验证工具筛选、ACP 判定以及敏感预览的清理与截断。

use super::*;
use serde_json::json;
use std::collections::HashSet;

#[test]
fn allowed_tool_ids_include_aliases_and_request_filter_intersects_base_set() {
    let base = allowed_tool_ids(None);
    assert!(!base.is_empty());

    let selected = base
        .iter()
        .find(|tool| !tool.eq_ignore_ascii_case("skill"))
        .expect("at least one registered non-skill tool")
        .clone();
    let filtered = allowed_tool_ids_for_request(
        None,
        &json!({"allowed_tools": [selected.clone(), "missing-tool", "  ", 42]}),
    );
    assert!(filtered.contains(&selected));
    let expected = base
        .iter()
        .filter(|tool| {
            tool.as_str() == selected || config_tools::is_builtin_required_tool_id(tool)
        })
        .cloned()
        .collect::<HashSet<_>>();
    assert_eq!(filtered, expected);

    assert_eq!(allowed_tool_ids_for_request(None, &json!({})), base);
    let empty_selection = allowed_tool_ids_for_request(None, &json!({"allowed_tools": ["  ", 42]}));
    assert!(
        empty_selection
            .iter()
            .all(|tool| { config_tools::is_builtin_required_tool_id(tool) })
    );
}
