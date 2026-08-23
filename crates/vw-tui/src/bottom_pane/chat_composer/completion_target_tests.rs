//! 验证补全目标解析在换行、分隔符和原子文本元素附近只选中可编辑令牌。

use super::*;
use crate::bottom_pane::textarea::TextArea;
use pretty_assertions::assert_eq;

#[test]
// 光标停在换行附近时，左右行的 `@` 或 `$` 令牌都不能获得补全亲和性。
fn current_prefixed_token_affinity_does_not_cross_line_break() {
    for (text, prefix, allow_empty) in [
        ("@file\n  continue", '@', false),
        ("continue  \n@file", '@', false),
        ("$skill\n  continue", '$', true),
        ("continue  \n$skill", '$', true),
    ] {
        let mut textarea = TextArea::new();
        textarea.insert_str(text);
        textarea.set_cursor(text.find("  ").expect("indentation present") + 1);

        assert_eq!(
            current_prefixed_token_range(&textarea, prefix, allow_empty),
            None
        );
    }
}

#[test]
// 分隔符右侧是普通文本时，不应回退选择左侧旧的前缀令牌。
fn current_prefixed_token_affinity_does_not_cross_separator_before_plain_text() {
    for (text, prefix, allow_empty) in [("@old  word", '@', false), ("$old  word", '$', true)] {
        let mut textarea = TextArea::new();
        textarea.insert_str(text);
        textarea.set_cursor(text.find("  ").expect("separator present") + 1);

        assert_eq!(
            current_prefixed_token_range(&textarea, prefix, allow_empty),
            None
        );
    }
}

#[test]
// 左侧相邻令牌已绑定为原子元素时，应越过它选择分隔符右侧的新令牌。
fn current_prefixed_token_prefers_right_after_adjacent_target() {
    let bound = "@bound";
    let left = "@bound@old";
    let text = "@bound@old  @new";
    let right_start = left.len() + 2;
    let mut textarea = TextArea::new();
    textarea.insert_str(text);
    textarea.add_element_range(0..bound.len());
    textarea.set_cursor(left.len());

    assert_eq!(
        current_prefixed_token_range(&textarea, '@', /*allow_empty*/ false),
        Some((right_start..text.len(), "new".to_string()))
    );
}

#[test]
// 光标正位于新令牌起点时，补全目标应采用右侧令牌而非左侧旧令牌。
fn current_prefixed_token_prefers_right_at_token_start() {
    let text = "$old $new";
    let right_start = "$old ".len();
    let mut textarea = TextArea::new();
    textarea.insert_str(text);
    textarea.set_cursor(right_start);

    assert_eq!(
        current_prefixed_token_range(&textarea, '$', /*allow_empty*/ true),
        Some((right_start..text.len(), "new".to_string()))
    );
}

#[test]
// 右侧候选是已绑定的 `@` 元素时，应回退到左侧仍可编辑的提及。
fn current_prefixed_token_falls_back_from_bound_at_token_on_right() {
    let text = "@left  @bound";
    let bound_start = text.find("@bound").expect("bound mention");
    let mut textarea = TextArea::new();
    textarea.insert_str(text);
    textarea.add_element_range(bound_start..text.len());
    textarea.set_cursor("@left ".len());

    assert_eq!(
        current_prefixed_token_range(&textarea, '@', /*allow_empty*/ true),
        Some((0.."@left".len(), "left".to_string()))
    );
}

#[test]
// 已绑定技能后的路径或配置后缀不能抢占左侧可编辑的 `$` 候选。
fn current_prefixed_token_falls_back_from_bound_dollar_suffix_on_right() {
    for suffix in ["/path", ".config"] {
        let text = format!("$left  $bound{suffix}");
        let bound_start = text.find("$bound").expect("bound skill mention");
        let bound_end = bound_start + "$bound".len();
        let mut textarea = TextArea::new();
        textarea.insert_str(&text);
        textarea.add_element_range(bound_start..bound_end);
        textarea.set_cursor("$left  ".len());

        assert_eq!(
            current_prefixed_token_range(&textarea, '$', /*allow_empty*/ true),
            Some((0.."$left".len(), "left".to_string()))
        );
    }
}

#[test]
// 多个相邻原子元素之间，只应返回光标所在的后段纯文本令牌。
fn current_prefixed_token_targets_later_segment_between_bound_elements() {
    let text = "$bound1$one$bound2$two$bound3";
    let bound2_start = text.find("$bound2").expect("second bound mention");
    let two_start = text.find("$two").expect("later plaintext target");
    let bound3_start = text.find("$bound3").expect("third bound mention");
    let mut textarea = TextArea::new();
    textarea.insert_str(text);
    textarea.add_element_range(0.."$bound1".len());
    textarea.add_element_range(bound2_start..two_start);
    textarea.add_element_range(bound3_start..text.len());
    textarea.set_cursor(two_start + "$t".len());

    assert_eq!(
        current_prefixed_token_range(&textarea, '$', /*allow_empty*/ true),
        Some((two_start..bound3_start, "two".to_string()))
    );
}

#[test]
// 右侧候选的范围必须在下一个已绑定元素之前截断。
fn current_prefixed_token_bounds_right_candidate_before_bound_element() {
    let text = "continue @other@bound";
    let other_start = text.find("@other").expect("editable mention");
    let bound_start = text.find("@bound").expect("bound mention");
    let mut textarea = TextArea::new();
    textarea.insert_str(text);
    textarea.add_element_range(bound_start..text.len());
    textarea.set_cursor(other_start);

    assert_eq!(
        current_prefixed_token_range(&textarea, '@', /*allow_empty*/ false),
        Some((other_start..bound_start, "other".to_string()))
    );
}

#[test]
// 候选后紧邻粘贴占位元素时，范围不得覆盖该原子元素。
fn current_prefixed_token_stops_before_overlapping_element() {
    let editable = "$other";
    let placeholder = "[Pasted Content 1001 chars]";
    let text = format!("{editable}{placeholder}");
    let mut textarea = TextArea::new();
    textarea.insert_str(&text);
    textarea.add_element_range(editable.len()..text.len());
    textarea.set_cursor(editable.len());

    assert_eq!(
        current_prefixed_token_range(&textarea, '$', /*allow_empty*/ true),
        Some((0..editable.len(), "other".to_string()))
    );
}

#[test]
// 令牌后的行尾空格不应让补全解析跨到下一行。
fn current_prefixed_token_stops_at_trailing_space_before_line_break() {
    let text = "$figma \nnext";
    let mut textarea = TextArea::new();
    textarea.insert_str(text);
    textarea.set_cursor("$figma ".len());

    assert_eq!(
        current_prefixed_token_range(&textarea, '$', /*allow_empty*/ true),
        None
    );
}

#[test]
// 同一 shell 风格令牌内嵌套的 `$` 保持为查询正文的一部分。
fn current_prefixed_token_keeps_nested_dollar_prefix_in_same_token() {
    let text = "$HOME/$USER";
    let mut textarea = TextArea::new();
    textarea.insert_str(text);
    textarea.set_cursor(text.find("$USER").expect("nested prefix present"));

    assert_eq!(
        current_prefixed_token_range(&textarea, '$', /*allow_empty*/ true),
        Some((0..text.len(), "HOME/$USER".to_string()))
    );
}

#[test]
// 已绑定的普通技能提及追加限定后缀后，仍保持不可编辑身份。
fn typed_qualified_suffix_preserves_bound_plain_mention() {
    // 限定技能绑定与历史持久化有意延后；在已绑定普通提及后输入后缀时，
    // 不得把它重新解释为可编辑的限定提及。
    // Qualified skill bindings and history persistence are intentionally deferred. Typing a
    // suffix after a bound plain mention must not reinterpret it as an editable qualified one.
    let bound = "$google-calendar";
    let text = "$google-calendar:availability";
    let mut textarea = TextArea::new();
    textarea.insert_str(text);
    textarea.add_element_range(0..bound.len());
    textarea.set_cursor(text.len());

    let (range, token) = current_prefixed_token_range(&textarea, '$', /*allow_empty*/ true)
        .expect("dollar token should be present");
    assert_eq!((range.clone(), token.as_str()), (0..text.len(), &text[1..]));
    assert!(!prefixed_token_range_is_editable(
        &textarea, '$', &range, &token,
    ));
}

#[test]
// `$` 查询分类必须区分技能语法、环境变量和含位置参数的 shell 语法。
fn dollar_query_classifies_shell_and_skill_syntax() {
    assert!(!dollar_query_is_completable("{CODEX_HOME}/config.toml"));
    assert!(!dollar_query_is_completable("(pwd)"));
    assert!(dollar_query_is_completable("home:search"));
    assert!(dollar_query_is_completable("home"));
    assert!(!dollar_query_is_completable("HOME"));
    for query in ["0", "1", "12", "-", "_"] {
        assert!(!dollar_query_is_completable(query));
        assert_eq!(
            dollar_query_kind(query),
            DollarQueryKind::DefiniteShellParameter
        );
    }
    for query in ["1_suffix", "1foo", "-x"] {
        assert!(!dollar_query_is_completable(query));
        assert_eq!(
            dollar_query_kind(query),
            DollarQueryKind::AmbiguousShellParameter
        );
    }
}
