use crate::code_review::build_code_review_view_from_diff;
use std::collections::HashMap;
#[test]
fn build_code_review_view_from_diff_renders_collapsed_expanded_and_missing_patch_files() {
    let mut expanded = std::collections::HashSet::new();
    expanded.insert("src/expanded.rs".to_string());
    expanded.insert("src/missing_patch.rs".to_string());

    let mut patches = HashMap::new();
    patches.insert("src/expanded.rs".to_string(), "+added\n-context".to_string());
    let element = build_code_review_view_from_diff(
        Default::default(),
        &expanded,
        vec![
            "src/collapsed.rs".to_string(),
            "src/expanded.rs".to_string(),
            "src/missing_patch.rs".to_string(),
        ],
        patches,
    );

    let scrollable_children = element.as_widget().children();

    assert_eq!(scrollable_children.len(), 1);
}
