use vw_shared::question;

use super::GatewayClient;

impl GatewayClient {
    /// 列出当前待处理的问题请求。
    pub async fn question_list(&self) -> Result<Vec<question::Request>, String> {
        self.get_json("/v1/question", &[]).await
    }

    /// 回答指定问题请求，并提交多组选项答案。
    pub async fn question_reply(
        &self,
        request_id: &str,
        answers: Vec<Vec<String>>,
    ) -> Result<bool, String> {
        let request_id = required_request_id(request_id)?;
        self.post_json(
            &format!("/v1/question/{}/reply", encode_path_segment(request_id)),
            &[],
            &serde_json::json!({ "answers": answers }),
        )
        .await
    }

    /// 拒绝指定问题请求。
    pub async fn question_reject(&self, request_id: &str) -> Result<bool, String> {
        let request_id = required_request_id(request_id)?;
        self.post_json(
            &format!("/v1/question/{}/reject", encode_path_segment(request_id)),
            &[],
            &serde_json::json!({}),
        )
        .await
    }
}

fn required_request_id(request_id: &str) -> Result<&str, String> {
    let request_id = request_id.trim();
    if request_id.is_empty() {
        return Err("question request id must not be empty".to_string());
    }
    Ok(request_id)
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
#[path = "question_api_tests.rs"]
mod question_api_tests;
