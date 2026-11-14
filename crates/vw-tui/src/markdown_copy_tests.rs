use super::*;
use pretty_assertions::assert_eq;

#[test]
// 复制目标应保留围栏语言及代码正文的精确字节内容。
fn copy_targets_preserve_fenced_code_language_and_exact_content() {
    let markdown =
        "```python title=example\nprint('hi')\n```\n\n    ignored()\n\n~~~\nplain()\n~~~";
    assert_eq!(
        extract_copy_targets(markdown),
        vec![
            CopyTarget::Code {
                language: Some("python".to_string()),
                content: Arc::from("print('hi')\n"),
            },
            CopyTarget::Code { language: None, content: Arc::from("plain()\n") },
        ]
    );
    assert_eq!(
        extract_copy_targets("```powershell\r\nGet-Item .\r\nWrite-Output ok\r\n```\r\n"),
        vec![CopyTarget::Code {
            language: Some("powershell".to_string()),
            content: Arc::from("Get-Item .\r\nWrite-Output ok\r\n"),
        }]
    );
}

#[test]
// 嵌套引用内的 Markdown 复制目标应维持源码顺序。
fn copy_targets_preserve_nested_quote_markdown_and_source_order() {
    let markdown = "> outer **bold**\n> > inner *quote*\n> ```sh\n> nested()\n> ```\n";
    assert_eq!(
        extract_copy_targets(markdown),
        vec![
            CopyTarget::Quote(Arc::from(
                "outer **bold**\n> inner *quote*\n```sh\nnested()\n```\n"
            )),
            CopyTarget::Code {
                language: Some("sh".to_string()),
                content: Arc::from("nested()\n"),
            },
        ]
    );
}

#[test]
// 只有引用标记而无正文的块不应生成复制目标。
fn copy_targets_exclude_blockquotes_without_prose() {
    let markdown = concat!(
        "> > ::git-stage{cwd=\"/repo\"}\n\n",
        "> ::git-stage{cwd=\"/repo\"}\n> ```sh\n> code()\n> ```\n\n",
        "> quoted prose\n"
    );
    assert_eq!(
        extract_copy_targets(markdown),
        vec![
            CopyTarget::Code {
                language: Some("sh".to_string()),
                content: Arc::from("code()\n"),
            },
            CopyTarget::Quote(Arc::from("quoted prose\n")),
        ]
    );
}
