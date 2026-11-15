//! 单轮对话编排；宿主只提供记忆、模型、技能与工具执行动作。
use anyhow::Result;
use model_types::ChatMessage;
use tokio::sync::mpsc::Sender;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait TurnAgent {
    fn auto_save(&self) -> bool;
    async fn store_user_message(&self, message: &str) -> Result<()>;
    async fn load_memory_context(&self, message: &str) -> Result<String>;
    fn configured_datetime(&self) -> String;
    fn turn_history(&mut self) -> &mut Vec<ChatMessage>;
    async fn select_model(&self, message: &str) -> Result<String>;
    fn system_prompt(&self) -> Result<String>;
    fn begin_skill_turn(&mut self, message: &str, instructions: &str);
    async fn set_skill_model(&mut self, model: &str) -> Result<()>;
    async fn execute_loop(
        &mut self,
        history: &mut Vec<ChatMessage>,
        model: &str,
        on_delta: Option<Sender<String>>,
    ) -> Result<String>;
    fn trim_turn_history(&mut self);
}

/// 调用方在进入本函数前建立原请求项目作用域。
pub async fn turn<A: TurnAgent>(
    agent: &mut A,
    user_message: &str,
    on_delta: Option<Sender<String>>,
) -> Result<String> {
    if agent.auto_save() {
        let _ = agent.store_user_message(user_message).await;
    }
    let context = agent.load_memory_context(user_message).await.unwrap_or_default();
    let now = agent.configured_datetime();
    let stamped_user_message = format!("[{now}] {user_message}");
    let enriched = match &context {
        c if !c.is_empty() => format!("{c}{stamped_user_message}"),
        _ => stamped_user_message,
    };
    agent.turn_history().push(ChatMessage::user(enriched));
    let effective_model = agent.select_model(user_message).await?;
    let persistent_len = agent.turn_history().len();
    let protocol = agent.system_prompt()?;
    let mut request_history = vec![ChatMessage::user(format!(
        "<system-reminder>\n{protocol}\n</system-reminder>"
    ))];
    agent.begin_skill_turn(user_message, &request_history[0].content);
    Box::pin(agent.set_skill_model(&effective_model)).await?;
    let transient_len = request_history.len();
    request_history.extend(agent.turn_history().iter().cloned());
    let final_response = agent.execute_loop(&mut request_history, &effective_model, on_delta).await?;
    agent.turn_history().extend(request_history.into_iter().skip(transient_len + persistent_len));
    agent.trim_turn_history();
    Ok(final_response)
}
