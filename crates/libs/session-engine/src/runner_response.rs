//! Session runner implementation; instantiated with the concrete IO host.
use super::*;

pub fn should_handle_docs_request_locally(query: &str) -> bool {
    {
        let q = query.to_lowercase();
        if !q.contains("docs") {
            return false;
        }
        let list = q.contains("文件列表") || q.contains("文件 列表") || q.contains("列表");
        let read = q.contains("读取") || q.contains("查看") || q.contains("列出");
        list && read
    }
}

pub fn handle_docs_request<H: RunnerHost>(
    session: &mut Session,
    ctx: &ToolRuntimeContext<H::CtxHost>,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
) {
    match H::list_docs(ctx.root.as_ref()) {
        Ok(files) => {
            let mut out = String::new();
            out.push_str(
                "<think>\n思考中\n任务分解：\n- 读取 docs 目录\n- 整理文件列表\n</think>\n\n",
            );
            out.push_str("docs 目录下的文件：\n");
            for path in files {
                out.push_str("- ");
                out.push_str(&path);
                out.push('\n');
            }
            session.push(Role::Assistant, out.clone());
            if !on_event(StreamEvent::Delta(out)) {
                return;
            }
            on_event(StreamEvent::Done(models::TokenUsage::default()));
        }
        Err(message) => {
            session.push(Role::System, message.clone());
            on_event(StreamEvent::Error(message));
        }
    }
}

pub fn handle_step_error(
    session: &mut Session,
    llm_messages: &mut Vec<Value>,
    empty_response_retries: &mut usize,
    step: &mut usize,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    message: String,
) -> bool {
    if message.trim() != EMPTY_RESPONSE_ERR {
        session.push(Role::System, message.clone());
        on_event(StreamEvent::Error(message));
        return false;
    }

    *empty_response_retries = empty_response_retries.saturating_add(1);
    if *empty_response_retries <= EMPTY_RESPONSE_RETRY_LIMIT {
        session.push(
            Role::System,
            format!(
                "{}，自动重试中（{}/{}）",
                EMPTY_RESPONSE_ERR, empty_response_retries, EMPTY_RESPONSE_RETRY_LIMIT
            ),
        );
        llm_messages.push(json!({
            "role": "system",
            "content": EMPTY_RESPONSE_RETRY_PROMPT
        }));
        *step += 1;
        return true;
    }

    let final_message = format!(
        "{}，自动重试 {} 次后仍为空，任务终止",
        EMPTY_RESPONSE_ERR, EMPTY_RESPONSE_RETRY_LIMIT
    );
    session.push(Role::System, final_message.clone());
    on_event(StreamEvent::Error(final_message));
    false
}

#[cfg(test)]
#[path = "runner_response_tests.rs"]
mod tests;
