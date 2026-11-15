use super::*;
#[test]
fn voice_task_11_local_reads_only_final_prose() {
    assert_eq!(speech_text("<think>内部思考</think>你好。\n```rust\nsecret();\n```"), "你好。");
}
#[test]
fn voice_task_11_local_does_not_read_inline_code() {
    assert_eq!(speech_text("请查看 `secret_token()` 后继续。"), "请查看  后继续。");
}
