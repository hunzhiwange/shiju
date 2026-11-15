//! 提问领域的宿主适配：只传递当前可信租户上下文。

pub use agent_question::{event, Error};
pub use question_types::*;

pub async fn ask(input: AskInput) -> Result<Vec<Answer>, Error> {
    agent_question::ask(input, crate::session::ui_store::current_tenant_scope()).await
}

pub(crate) async fn ask_sensitive(
    input: AskInput,
    form: question_types::McpForm,
) -> Result<Vec<Answer>, Error> {
    agent_question::ask_sensitive(input, form, crate::session::ui_store::current_tenant_scope()).await
}

pub fn reply(input: ReplyInput) {
    agent_question::reply(input, crate::session::ui_store::current_tenant_scope());
}

pub fn reject(request_id: impl Into<String>) {
    agent_question::reject(request_id, crate::session::ui_store::current_tenant_scope());
}

pub fn list() -> Vec<Request> {
    agent_question::list(crate::session::ui_store::current_tenant_scope())
}
