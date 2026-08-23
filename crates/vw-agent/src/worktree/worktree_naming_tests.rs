//! 验证 worktree 名称的 slug 归一化与随机名称格式。

use super::naming::{random_name, slug};

#[test]
fn slug_normalizes_user_visible_names() {
    assert_eq!(slug("  Feature/Test 42! "), "feature-test-42");
    assert_eq!(slug("___"), "");
    assert_eq!(slug("Already--Slug"), "already-slug");
    assert_eq!(slug("release/v1.2.3"), "release-v1-2-3");
}

#[test]
fn random_name_uses_expected_shape() {
    let name = random_name();
    assert_eq!(name.split('-').count(), 2);
    assert!(name.chars().all(|ch| ch.is_ascii_lowercase() || ch == '-'));
}
