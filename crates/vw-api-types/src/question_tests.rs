//! 验证问题查询与回复 DTO 的空筛选默认值和协议枚举名称。

use crate::question::{ListQuestionsRequest, QuestionKind, QuestionStatus, ReplyQuestionRequest};
use serde_json::json;

#[test]
fn question_requests_default_to_no_filter_or_answer() {
    let list: ListQuestionsRequest = serde_json::from_value(json!({})).expect("valid list");
    assert_eq!(list.session_id, None);
    assert_eq!(list.status, None);

    let reply: ReplyQuestionRequest = serde_json::from_value(json!({})).expect("valid reply");
    assert!(reply.selected_option_ids.is_empty());
    assert_eq!(reply.text, None);

    assert_eq!(serde_json::to_value(QuestionKind::Approval).expect("serialize"), json!("approval"));
    assert_eq!(serde_json::to_value(QuestionStatus::Pending).expect("serialize"), json!("pending"));
}
