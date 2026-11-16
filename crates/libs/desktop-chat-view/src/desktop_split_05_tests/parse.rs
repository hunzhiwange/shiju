//! 验证原始消息拆分为文本、思考和工具块时的标签及容错边界。
//! 不闭合标签、名称前缀碰撞、多行 JSON 和行数上限用于约束解析器的保守回退。

use crate::message_view::parse::owned_blocks_from_raw;
use crate::message_view::ParsedChatBlock;

#[test]
fn acp_search_keeps_its_title_and_receives_sparse_completion() {
    let raw = concat!(
        "检查样式。tool search\n",
        "{\n",
        r#""status":"in_progress","toolCallId":"call-search","input":"{}","title":"Search for 'code-surface|code-gutter' in style.css""#,
        "\n}\n",
        "继续检查。\n",
        "tool search\n",
        r#"{"status":"running","toolCallId":"call-other","input":"{}","title":"Search in main.ts"}"#,
        "\n",
        "tool tool_call\n",
        r#"{"status":"completed","toolCallId":"call-search","input":"{}","title":"tool_call","output":"{\"exit_code\":0}"}"#,
        "\n",
    );
    let blocks = owned_blocks_from_raw(raw);
    let [
        ParsedChatBlock::Text { content: before },
        ParsedChatBlock::Tool { raw: search },
        ParsedChatBlock::Text { content: between },
        ParsedChatBlock::Tool { raw: other },
    ] = blocks.as_slice()
    else {
        panic!("expected both search cards and surrounding text");
    };

    assert_eq!(before, "检查样式。");
    assert_eq!(between, "继续检查。\n");
    let (header, body) = search.split_once('\n').unwrap();
    let payload: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(header, "tool search");
    assert_eq!(payload["title"], "Search for 'code-surface|code-gutter' in style.css");
    assert_eq!(payload["status"], "completed");
    assert_eq!(payload["output"], r#"{"exit_code":0}"#);
    let identity = desktop_chat_tools::tools::tool_parse::tool_identity_from_raw;
    assert_eq!(identity(search).as_deref(), Some("acp:call-search"));
    assert_eq!(identity(other).as_deref(), Some("acp:call-other"));
}
