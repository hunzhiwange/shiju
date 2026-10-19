//! HTTP 请求节点的安全可视化编辑面板。

use super::*;
use iced::widget::{column, row};

fn http_message(edit: WorkflowHttpRequestEdit) -> Message {
    Message::WorkflowTool(WorkflowMessage::NodeEditorHttpRequestEdit(edit))
}

fn headers_action(action: text_editor::Action) -> WorkflowMessage {
    WorkflowMessage::NodeEditorHttpRequestEdit(WorkflowHttpRequestEdit::Headers(action))
}
fn params_action(action: text_editor::Action) -> WorkflowMessage {
    WorkflowMessage::NodeEditorHttpRequestEdit(WorkflowHttpRequestEdit::Params(action))
}
fn body_action(action: text_editor::Action) -> WorkflowMessage {
    WorkflowMessage::NodeEditorHttpRequestEdit(WorkflowHttpRequestEdit::Body(action))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_http_request_section<'a>(
    validation: &'a super::state::WorkflowNodeEditorValidation,
    method: &'a str,
    url: &'a str,
    auth_type: &'a str,
    auth_username: &'a str,
    auth_password: &'a str,
    auth_token: &'a str,
    headers: &'a text_editor::Content,
    params: &'a text_editor::Content,
    body_type: &'a str,
    body: &'a text_editor::Content,
    timeout_connect: &'a str,
    timeout_read: &'a str,
    timeout_write: &'a str,
    ssl_verify: bool,
    retry: WorkflowNodeRetryDraft,
    error_strategy: &'a str,
) -> Element<'a, Message> {
    let auth_fields: Element<'a, Message> = match auth_type {
        "basic" => row![
            workflow_text_input("用户名", auth_username, |v| http_message(
                WorkflowHttpRequestEdit::AuthUsername(v)
            )),
            workflow_text_input("密码", auth_password, |v| http_message(
                WorkflowHttpRequestEdit::AuthPassword(v)
            )),
        ]
        .spacing(8)
        .into(),
        "bearer" | "api-key" => {
            workflow_text_input("凭据（不会出现在反馈消息中）", auth_token, |v| {
                http_message(WorkflowHttpRequestEdit::AuthToken(v))
            })
        }
        _ => Space::new().height(Length::Shrink).into(),
    };
    column![
        row![
            build_editor_field(
                "方法",
                workflow_text_input("get", method, |v| http_message(
                    WorkflowHttpRequestEdit::Method(v)
                ))
            ),
            build_editor_field_validated(
                "URL",
                workflow_text_input("https://example.com/api", url, |v| http_message(
                    WorkflowHttpRequestEdit::Url(v)
                )),
                validation.first_error_for("http.url")
            ),
        ]
        .spacing(8),
        build_editor_field_validated(
            "鉴权类型",
            workflow_text_input("no-auth / basic / bearer / api-key", auth_type, |v| http_message(
                WorkflowHttpRequestEdit::AuthType(v)
            )),
            validation.first_error_for("http.authorization")
        ),
        auth_fields,
        build_editor_field(
            "请求头 YAML",
            build_embedded_text_editor(headers, "请求头 YAML", headers_action, 90.0)
        ),
        build_editor_field(
            "查询参数 YAML",
            build_embedded_text_editor(params, "查询参数 YAML", params_action, 90.0)
        ),
        build_editor_field(
            "请求体类型",
            workflow_text_input("none / json / raw-text / form-data", body_type, |v| http_message(
                WorkflowHttpRequestEdit::BodyType(v)
            ))
        ),
        build_editor_field(
            "请求体 YAML",
            build_embedded_text_editor(body, "body.data YAML", body_action, 120.0)
        ),
        row![
            build_editor_field_validated(
                "连接超时",
                workflow_text_input("10", timeout_connect, |v| http_message(
                    WorkflowHttpRequestEdit::TimeoutConnect(v)
                )),
                validation.first_error_for("http.timeout.connect")
            ),
            build_editor_field_validated(
                "读取超时",
                workflow_text_input("60", timeout_read, |v| http_message(
                    WorkflowHttpRequestEdit::TimeoutRead(v)
                )),
                validation.first_error_for("http.timeout.read")
            ),
            build_editor_field_validated(
                "写入超时",
                workflow_text_input("20", timeout_write, |v| http_message(
                    WorkflowHttpRequestEdit::TimeoutWrite(v)
                )),
                validation.first_error_for("http.timeout.write")
            ),
        ]
        .spacing(8),
        row![
            toggler(ssl_verify).on_toggle(|v| http_message(WorkflowHttpRequestEdit::SslVerify(v))),
            text("校验 TLS 证书").size(12)
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        row![
            toggler(retry.enabled)
                .on_toggle(|v| http_message(WorkflowHttpRequestEdit::RetryEnabled(v))),
            text(format!("重试 {} 次 / {} ms", retry.max_retries, retry.retry_interval)).size(12),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        build_editor_field(
            "错误策略",
            workflow_text_input("none / fail-branch", error_strategy, |v| http_message(
                WorkflowHttpRequestEdit::ErrorStrategy(v)
            ))
        ),
    ]
    .spacing(12)
    .into()
}
