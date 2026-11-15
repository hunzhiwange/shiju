//! changes.rs 测试模块。
//!
//! 这些测试固定相邻解析器、视图辅助函数或状态计算的行为，防止后续 UI 重排时破坏边界契约。

use super::{parse_changes_file_summaries, parse_changes_files};

/// 解析 changes file summaries extracts metadata without file bodies 的输入文本，返回后续视图可以直接消费的结构化结果。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn parse_changes_file_summaries_extracts_metadata_without_file_bodies() {
    let output = concat!(
        "Success. Updated the following files:\nM src/main.rs\n\n",
        "<changes>\n",
        "{\"files\":[{\"path\":\"src/main.rs\",\"additions\":3,\"deletions\":1,",
        "\"before\":\"old body\",\"after\":\"new body\"}]}",
        "\n</changes>"
    );

    let files = parse_changes_file_summaries(output);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].kind, 'M');
    assert_eq!(files[0].path, "src/main.rs");
    assert_eq!(files[0].additions, 3);
    assert_eq!(files[0].deletions, 1);
}

/// 解析 changes files keeps full file bodies for expanded rows 的输入文本，返回后续视图可以直接消费的结构化结果。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn parse_changes_files_keeps_full_file_bodies_for_expanded_rows() {
    let output = concat!(
        "Success. Updated the following files:\nA src/new.rs\n\n",
        "<changes>\n",
        "{\"files\":[{\"path\":\"src/new.rs\",\"additions\":2,\"deletions\":0,",
        "\"before\":\"\",\"after\":\"fn main() {}\\n\"}]}",
        "\n</changes>"
    );

    let files = parse_changes_files(output);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "src/new.rs");
    assert!(files[0].before.is_empty());
    assert_eq!(files[0].after, "fn main() {}\n");
}

/// 解析 changes files ignores closing tag text before changes block 的输入文本，返回后续视图可以直接消费的结构化结果。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn parse_changes_files_ignores_closing_tag_text_before_changes_block() {
    let output = concat!(
        "<diff>\n",
        "-before\n",
        "+after with </changes> literal\n",
        "</diff>\n\n",
        "<changes>\n",
        "{\"files\":[{\"path\":\"src/main.rs\",\"additions\":1,\"deletions\":1,\"before\":\"before\\n\",\"after\":\"after\\n\"}]}\n",
        "</changes>"
    );

    let files = parse_changes_files(output);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "src/main.rs");
}

#[test]
fn parse_changes_files_returns_empty_for_missing_or_invalid_blocks() {
    assert!(parse_changes_files("no changes").is_empty());
    assert!(parse_changes_files("<changes>\nnot-json\n</changes>").is_empty());
    assert!(parse_changes_files("<changes>\n{\"files\":{}}\n</changes>").is_empty());
    assert!(
        parse_changes_files("<changes>\n{\"files\":[{\"additions\":1}]}\n</changes>").is_empty()
    );
}

#[test]
fn parse_changes_file_summaries_classifies_added_deleted_and_modified() {
    let output = concat!(
        "<changes>\n",
        "{\"files\":[",
        "{\"path\":\"src/new.rs\",\"additions\":2,\"deletions\":0,\"before\":\"\",\"after\":\"new\"},",
        "{\"path\":\"src/old.rs\",\"additions\":0,\"deletions\":2,\"before\":\"old\",\"after\":\"\"},",
        "{\"path\":\"src/mod.rs\",\"additions\":1,\"deletions\":1,\"before\":\"old\",\"after\":\"new\"}",
        "]}\n",
        "</changes>"
    );

    let summaries = parse_changes_file_summaries(output);
    assert_eq!(summaries.iter().map(|item| item.kind).collect::<Vec<_>>(), vec!['A', 'D', 'M']);
}
