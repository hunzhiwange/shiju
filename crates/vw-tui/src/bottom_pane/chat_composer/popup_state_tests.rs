//! 验证已关闭补全弹窗按令牌出现次数跟踪，而不是依赖易漂移的字节偏移。

use super::*;
use crate::bottom_pane::textarea::TextArea;
use pretty_assertions::assert_eq;

#[test]
// 两个相同令牌分别紧邻不同原子元素时，关闭记录只能命中原来的那一次出现。
fn dismissed_tokens_after_adjacent_elements_are_occurrence_scoped() {
    let first_bound = "$bound1";
    let second_bound = "$bound2";
    let token = "$other";
    let text = format!("{first_bound}{token} {second_bound}{token}");
    let first_start = first_bound.len();
    let second_start = text.rfind(token).expect("second token");
    let second_bound_start = text.find(second_bound).expect("second bound mention");
    let mut textarea = TextArea::new();
    textarea.insert_str(&text);
    textarea.add_element_range(0..first_start);
    textarea.add_element_range(second_bound_start..second_bound_start + second_bound.len());
    let dismissed = DismissedToken::new(
        &textarea,
        first_start..first_start + token.len(),
        "other".to_string(),
    );

    assert!(!dismissed.matches(
        &textarea,
        &(second_start..second_start + token.len()),
        "other",
    ));
}

#[test]
// 嵌套在较长 `@` 查询中的同文片段不算一个边界完整的已关闭令牌。
fn nested_at_query_does_not_count_as_complete_dismissed_token() {
    let text = "@ma@latest @ma";
    let second_start = text.rfind("@ma").expect("second token");
    let mut textarea = TextArea::new();
    textarea.insert_str(text);

    assert_eq!(
        complete_token_occurrences_before(&textarea, "@ma", second_start),
        0
    );
}

#[test]
// 非提及类原子元素同样构成边界，并保持前后相同令牌的出现身份互不混淆。
fn non_sigil_elements_delimit_dismissed_token_occurrences() {
    let token = "$other";
    let first_element = "[one]";
    let text = format!("{token}{first_element} {token}");
    let second_start = text.rfind(token).expect("second token");
    let mut textarea = TextArea::new();
    textarea.insert_str(&text);
    textarea.add_element_range(token.len()..token.len() + first_element.len());
    let dismissed = DismissedToken::new(&textarea, 0..token.len(), "other".to_string());

    assert!(!dismissed.matches(
        &textarea,
        &(second_start..second_start + token.len()),
        "other",
    ));
}
