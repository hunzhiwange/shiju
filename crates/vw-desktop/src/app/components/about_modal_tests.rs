//! 验证关于弹窗的信息复制内容、哈希判定与复制状态视图构造。
//! 已复制状态以内容哈希而非布尔值判定，fixture 因此同时覆盖内容变化后的失效路径。

use super::*;

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn copied_app_info_preserves_visible_metadata_and_layout() {
    let copy_text = about_copy_text();

    assert_eq!(
        copy_text,
        format!("{APP_NAME}\n{ABOUT_SUBTITLE}\n\n{ABOUT_DESCRIPTION}\n{AUTHOR_INFO}")
    );
    assert_eq!(copy_text.lines().count(), 5);
    assert_eq!(copy_text.lines().nth(2), Some(""));
    assert!(!copy_text.contains(ABOUT_TITLE));
}

#[test]
fn copy_text_hash_is_deterministic_and_content_sensitive() {
    let copy_text = about_copy_text();

    assert_eq!(hash_copy_text(&copy_text), hash_copy_text(&copy_text));
    assert_ne!(hash_copy_text(&copy_text), hash_copy_text(APP_NAME));
    assert_ne!(hash_copy_text(&copy_text), hash_copy_text(&(copy_text + "\n")));
}

#[test]
fn view_builds_with_copy_action_for_uncopied_content() {
    let mut app = App::new().0;

    keep_element(view(&app));
    app.last_copied_code_hash = Some(hash_copy_text("different content"));
    keep_element(view(&app));
}

#[test]
fn view_builds_with_confirmation_for_matching_copy_hash() {
    let mut app = App::new().0;
    app.last_copied_code_hash = Some(hash_copy_text(&about_copy_text()));

    keep_element(view(&app));
}
