    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    // 常规截断应为省略号预留三个字素位置。
    fn test_truncate_text() {
        let text = "Hello, world!";
        let truncated = truncate_text(text, /*max_graphemes*/ 8);
        assert_eq!(truncated, "Hello...");
    }

    #[test]
    // 空字符串在任意正限制下保持为空。
    fn test_truncate_empty_string() {
        let text = "";
        let truncated = truncate_text(text, /*max_graphemes*/ 5);
        assert_eq!(truncated, "");
    }

    #[test]
    // 零限制不能返回任何字素或省略号。
    fn test_truncate_max_graphemes_zero() {
        let text = "Hello";
        let truncated = truncate_text(text, /*max_graphemes*/ 0);
        assert_eq!(truncated, "");
    }

    #[test]
    // 一字素限制直接返回首字素。
    fn test_truncate_max_graphemes_one() {
        let text = "Hello";
        let truncated = truncate_text(text, /*max_graphemes*/ 1);
        assert_eq!(truncated, "H");
    }

    #[test]
    // 两字素限制直接返回前两个字素。
    fn test_truncate_max_graphemes_two() {
        let text = "Hello";
        let truncated = truncate_text(text, /*max_graphemes*/ 2);
        assert_eq!(truncated, "He");
    }

    #[test]
    // 三字素边界恰好只能容纳省略号。
    fn test_truncate_max_graphemes_three_boundary() {
        let text = "Hello";
        let truncated = truncate_text(text, /*max_graphemes*/ 3);
        assert_eq!(truncated, "...");
    }

    #[test]
    // 短于限制的文本不得发生变化。
    fn test_truncate_text_shorter_than_limit() {
        let text = "Hi";
        let truncated = truncate_text(text, /*max_graphemes*/ 10);
        assert_eq!(truncated, "Hi");
    }

    #[test]
    // 恰好等于限制的文本不得误判为溢出。
    fn test_truncate_text_exact_length() {
        let text = "Hello";
        let truncated = truncate_text(text, /*max_graphemes*/ 5);
        assert_eq!(truncated, "Hello");
    }

    #[test]
    // 表情按字素而非 UTF-8 字节截断。
    fn test_truncate_emoji() {
        let text = "👋🌍🚀✨💫";
        let truncated = truncate_text(text, /*max_graphemes*/ 3);
        assert_eq!(truncated, "...");

        let truncated_longer = truncate_text(text, /*max_graphemes*/ 4);
        assert_eq!(truncated_longer, "👋...");
    }

    #[test]
    // 带组合标记的字符应作为完整字素保留。
    fn test_truncate_unicode_combining_characters() {
        // 下方示例字符各自包含组合标记。
        let text = "é́ñ̃"; // Characters with combining marks
        let truncated = truncate_text(text, /*max_graphemes*/ 2);
        assert_eq!(truncated, "é́ñ̃");
    }

    #[test]
    // 超长 ASCII 文本应满足精确输出长度并带省略号。
    fn test_truncate_very_long_text() {
        let text = "a".repeat(1000);
        let truncated = truncate_text(&text, /*max_graphemes*/ 10);
        assert_eq!(truncated, "aaaaaaa...");
        // 下方长度由七个 `a` 加三个点组成。
        assert_eq!(truncated.len(), 10); // 7 'a's + 3 dots
    }

    #[test]
    // 简单对象压成单行，并保留分隔符后的可读空格。
    fn test_format_json_compact_simple_object() {
        let json = r#"{ "name": "John", "age": 30 }"#;
        let result = format_json_compact(json).unwrap();
        assert_eq!(result, r#"{"name": "John", "age": 30}"#);
    }

    #[test]
    // 嵌套对象同样保持层级括号和单行空格规则。
    fn test_format_json_compact_nested_object() {
        let json = r#"{ "user": { "name": "John", "details": { "age": 30, "city": "NYC" } } }"#;
        let result = format_json_compact(json).unwrap();
        assert_eq!(result, r#"{"user": {"name": "John", "details": {"age": 30, "city": "NYC"}}}"#);
    }

    #[test]
    // 未超宽路径原样返回。
    fn test_center_truncate_doesnt_truncate_short_path() {
        let sep = std::path::MAIN_SEPARATOR;
        let path = format!("{sep}Users{sep}codex{sep}Public");
        let truncated = center_truncate_path(&path, /*max_width*/ 40);

        assert_eq!(truncated, path);
    }

    #[test]
    // 长路径优先保留多个首尾片段，在中部放置省略号。
    fn test_center_truncate_truncates_long_path() {
        let sep = std::path::MAIN_SEPARATOR;
        let path = format!("~{sep}hello{sep}the{sep}fox{sep}is{sep}very{sep}fast");
        let truncated = center_truncate_path(&path, /*max_width*/ 24);

        assert_eq!(truncated, format!("~{sep}hello{sep}the{sep}…{sep}very{sep}fast"));
    }

    #[test]
    // Windows 盘符形式也遵守同一首尾片段策略。
    fn test_center_truncate_truncates_long_windows_path() {
        let sep = std::path::MAIN_SEPARATOR;
        let path = format!(
            "C:{sep}Users{sep}codex{sep}Projects{sep}super{sep}long{sep}windows{sep}path{sep}file.txt"
        );
        let truncated = center_truncate_path(&path, /*max_width*/ 36);

        let expected = format!("C:{sep}Users{sep}codex{sep}…{sep}path{sep}file.txt");

        assert_eq!(truncated, expected);
    }

    #[test]
    // 单个过长片段从前方截断，以保留更具辨识度的尾部。
    fn test_center_truncate_handles_long_segment() {
        let sep = std::path::MAIN_SEPARATOR;
        let path = format!("~{sep}supercalifragilisticexpialidocious");
        let truncated = center_truncate_path(&path, /*max_width*/ 18);

        assert_eq!(truncated, format!("~{sep}…cexpialidocious"));
    }

    #[test]
    // 数组及其中对象按同一紧凑空格规则格式化。
    fn test_format_json_compact_array() {
        let json = r#"[ 1, 2, { "key": "value" }, "string" ]"#;
        let result = format_json_compact(json).unwrap();
        assert_eq!(result, r#"[1, 2, {"key": "value"}, "string"]"#);
    }

    #[test]
    // 已紧凑 JSON 也会补充分隔符后的可读空格。
    fn test_format_json_compact_already_compact() {
        let json = r#"{"compact":true}"#;
        let result = format_json_compact(json).unwrap();
        assert_eq!(result, r#"{"compact": true}"#);
    }

    #[test]
    // 多行和缩进空白应折叠为单行。
    fn test_format_json_compact_with_whitespace() {
        let json = r#"
        {
            "name": "John",
            "hobbies": [
                "reading",
                "coding"
            ]
        }
        "#;
        let result = format_json_compact(json).unwrap();
        assert_eq!(result, r#"{"name": "John", "hobbies": ["reading", "coding"]}"#);
    }

    #[test]
    // 无效 JSON 明确返回 `None`。
    fn test_format_json_compact_invalid_json() {
        let invalid_json = r#"{"invalid": json syntax}"#;
        let result = format_json_compact(invalid_json);
        assert!(result.is_none());
    }

    #[test]
    // 空对象不引入多余空格。
    fn test_format_json_compact_empty_object() {
        let json = r#"{}"#;
        let result = format_json_compact(json).unwrap();
        assert_eq!(result, "{}");
    }

    #[test]
    // 空数组不引入多余空格。
    fn test_format_json_compact_empty_array() {
        let json = r#"[]"#;
        let result = format_json_compact(json).unwrap();
        assert_eq!(result, "[]");
    }

    #[test]
    // JSON 原始值无需结构重排，保持规范字面量。
    fn test_format_json_compact_primitive_values() {
        assert_eq!(format_json_compact("42").unwrap(), "42");
        assert_eq!(format_json_compact("true").unwrap(), "true");
        assert_eq!(format_json_compact("false").unwrap(), "false");
        assert_eq!(format_json_compact("null").unwrap(), "null");
        assert_eq!(format_json_compact(r#""string""#).unwrap(), r#""string""#);
    }

    #[test]
    // 英文列表连接覆盖零到多项的标点与 `and` 位置。
    fn test_proper_join() {
        let empty: Vec<String> = vec![];
        assert_eq!(proper_join(&empty), "");
        assert_eq!(proper_join(&["apple"]), "apple");
        assert_eq!(proper_join(&["apple", "banana"]), "apple and banana");
        assert_eq!(proper_join(&["apple", "banana", "cherry"]), "apple, banana and cherry");
        assert_eq!(
            proper_join(&["apple", "banana", "cherry", "date"]),
            "apple, banana, cherry and date"
        );
    }
