use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    // 验证一行内多个动作的提取、重复去除及 Windows 路径保留。
    fn strips_and_parses_git_action_directives() {
        let parsed = parse_assistant_markdown(
            "Done\n\n::git-stage{cwd=\"/repo\"} ::git-push{cwd=\"/repo\" branch=\"feat/x\"} ::git-stage{cwd=\"C:\\repo\\\"}",
            Path::new("/repo"),
        );

        assert_eq!(parsed.visible_markdown, "Done");
        assert_eq!(
            parsed.git_actions,
            vec![
                GitActionDirective::Stage { cwd: "/repo".to_string() },
                GitActionDirective::Push { cwd: "/repo".to_string(), branch: "feat/x".to_string() },
                GitActionDirective::Stage { cwd: "C:\\repo\\".to_string() },
            ]
        );
    }

    #[test]
    // 缺少必填分支的完整指令形状应被隐藏，但不得生成动作行。
    fn hides_malformed_directives_without_materializing_rows() {
        let parsed = parse_assistant_markdown("Done ::git-push{cwd=\"/repo\"}", Path::new("/repo"));

        assert_eq!(parsed.visible_markdown, "Done");
        assert!(parsed.git_actions.is_empty());
    }

    #[test]
    // 代码评论指令应转换为普通 Markdown，并保留正文中的类指令文本。
    fn renders_code_comment_directives_as_markdown() {
        let parsed = parse_assistant_markdown(
            concat!(
                "Found two issues.\n\n",
                r#"::code-comment{title="Fix body= parsing" body="Keep role=\"tab\", ::git-stage{cwd=/tmp}, file=, and \n literal." file="/repo/src/app.ts" start=10 end=12 priority="P2"}"#,
                "\n\n",
                r#":::code-comment{title="[P1] Clamp the range" body="The line range should match the App." file="codex/src/range.ts" start=8 end=2 priority=3}"#,
            ),
            Path::new("/repo"),
        );

        // VibeWindow 适配：v3 crate 改名后仍沿用复制来源的 Codex 快照名。
        // VibeWindow adaptation: retain the copied Codex snapshot name after
        // the v3 crate rename.
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.bind(|| {
            insta::assert_snapshot!(
                "codex_tui__git_action_directives__tests__code_comment_directive_fallback",
                parsed.visible_markdown
            );
        });
        assert!(parsed.git_actions.is_empty());
    }

    #[test]
    // 行内示例和缺少必填属性的代码评论必须原样展示。
    fn preserves_non_directive_and_malformed_code_comment_text() {
        let markdown = "Mention `::code-comment{title=\"Example\"}` inline.\n::code-comment{title=\"Missing body\" file=\"/repo/src/app.ts\"}";
        let parsed = parse_assistant_markdown(markdown, Path::new("/repo"));

        assert_eq!(parsed.visible_markdown, markdown);
    }

    #[test]
    // 多次创建分支时，工作目录查询应返回最后一个匹配指令。
    fn last_created_branch_cwd_uses_the_last_matching_directive() {
        let parsed = parse_assistant_markdown(
            "::git-create-branch{cwd=\"/first\" branch=\"first\"}\n::git-push{cwd=\"/repo\" branch=\"first\"}\n::git-create-branch{cwd=\"/second\" branch=\"second\"}",
            Path::new("/repo"),
        );

        assert_eq!(parsed.last_created_branch_cwd(), Some("/second"));
    }
