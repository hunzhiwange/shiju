//! HTTP 请求节点的安全可视化编辑面板。

use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn build_http_request_section<'a>(
    i18n: crate::app::i18n::I18n,
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
    desktop_workflow_ui::view::http_request::build_http_request_section(i18n, validation, method, url, auth_type, auth_username, auth_password, auth_token, headers, params, body_type, body, timeout_connect, timeout_read, timeout_write, ssl_verify, retry, error_strategy).map(Message::WorkflowTool)
}
