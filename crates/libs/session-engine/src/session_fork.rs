//! Native fork copying and checkpoint inheritance.
use crate::session_io::{SessionIo, CreateInput};
use session_info::{Info, Error};
use agent_storage as storage;
use ordered_id as id;
use agent_project::instance;
use session_store::ui_store;
use std::sync::LazyLock;
fn instance_directory_opt() -> Option<String> { let d = instance::directory(); if d.is_empty() {None} else {Some(d)} }
fn checkpoint_scope() -> String { crate::compaction_checkpoint::scope_for_directory(&ui_store::current_tenant_scope(), &instance::directory()) }
pub fn forked_title(title: &str) -> String {
    // 匹配已有的分支编号格式
    static RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^(.+) \(fork #(\d+)\)$").unwrap());
    if let Some(caps) = RE.captures(title) {
        // 提取基础标题和当前编号
        let base = caps.get(1).map(|m| m.as_str()).unwrap_or(title);
        let num = caps.get(2).and_then(|m| m.as_str().parse::<u64>().ok()).unwrap_or(0);
        // 递增编号
        return format!("{} (fork #{})", base, num + 1);
    }
    // 首次分支
    format!("{} (fork #1)", title)
}

pub async fn fork_with_parent<H: SessionIo>(
    session_id: &str,
    message_id: Option<&str>,
    parent_id: Option<String>,
) -> Result<Info, Error> {
    // 读取原始会话信息
    let original = H::get(session_id).await?;
    let binding =
        crate::binding_store::load_scoped(H::home(), ui_store::current_tenant_scope(), &original.project_id, session_id).await?;
    // 生成分支标题
    let title = forked_title(&original.title);
    let directory = instance_directory_opt().unwrap_or_else(|| original.directory.clone());
    // 在创建目标前确认截点确实属于源会话。
    let mut msgs = crate::message::messages(session_id, None).await?;
    if let Some(until) = message_id {
        if !msgs.iter().any(|msg| msg.info.id() == until) {
            return Err(Error::Storage(storage::Error::NotFound(storage::NotFoundError {
                message: format!("message {until} not found in source thread"),
            })));
        }
    }
    // 创建新会话
    let created = H::create_next(CreateInput {
        parent_id,
        title: Some(title),
        directory,
        permission: original.permission.clone(),
    })
    .await?;
    let _history = crate::message::lock_history(&created.id).await;

    if let Some(binding) = binding {
        crate::binding_store::save_scoped(H::home(), ui_store::current_tenant_scope(), &created.project_id, &created.id, &binding)
            .await?;
    }
    #[cfg(not(target_arch = "wasm32"))]
    ui_store::with_dynamic_tool_store(&original.project_id, |store, scope| store.copy_dynamic_tools(scope, session_id, &created))
        .map_err(Error::Runtime)?;
    // 获取原始会话的所有消息，按时间正序复制，保证父消息映射可用。
    msgs.sort_by(|a, b| a.info.id().cmp(b.info.id()));
    // 建立旧消息 ID 到新消息 ID 的映射
    let mut id_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // 复制消息和部分
    for msg in msgs {
        let reached_target = message_id.is_some_and(|until| msg.info.id() == until);

        // 生成新的消息 ID
        let new_id = id::ascending(id::Prefix::Message, None)?;
        id_map.insert(msg.info.id().to_string(), new_id.clone());

        // 对于助手消息，需要更新父消息 ID 的映射
        let parent_id = match &msg.info {
            crate::message::Info::Assistant(a) => id_map.get(&a.parent_id).cloned(),
            _ => None,
        };

        // 克隆并更新消息信息
        let mut cloned_info = msg.info.clone();
        cloned_info.set_session_id(&created.id);
        cloned_info.set_id(&new_id);
        if let Some(parent_id) = parent_id {
            cloned_info.set_parent_id(&parent_id);
        }
        crate::message::update_message(&cloned_info).await?;

        // 复制消息的所有部分
        for part in msg.parts {
            let mut new_part = part.clone();
            #[cfg(not(target_arch = "wasm32"))]
            session_store::dynamic_tool_store::interrupt_part(&mut new_part);
            new_part.set_session_id(&created.id);
            new_part.set_message_id(&new_id);
            new_part.set_id(&id::ascending(id::Prefix::Part, None)?);
            crate::message::update_part(&new_part).await?;
        }

        if reached_target {
            break;
        }
    }

    crate::compaction_checkpoint::copy_scoped(&checkpoint_scope(), &checkpoint_scope(), session_id, &created.id, None)
        .await
        .map_err(|error| Error::Runtime(anyhow::anyhow!(error)))?;
    Ok(created)
}
