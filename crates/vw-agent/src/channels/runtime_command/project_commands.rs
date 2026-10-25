//! 当前聊天的项目查询和选择；校验全部成功后同步提交，不自动恢复旧上下文。
use super::super::*;
use super::command::{ChannelRuntimeCommand, parse_runtime_command};

pub(crate) async fn handle_project_command(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
) -> String {
    match project_command(ctx, msg).await {
        Ok(response) => response,
        Err(err) => {
            format!("项目操作失败：{err}。使用 /projects 查看稳定 ID；/project default 恢复默认。")
        }
    }
}

async fn project_command(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
) -> Result<String> {
    let command = parse_runtime_command(&msg.channel, &msg.content);
    let chat = channel_chat_scope(ctx, msg);
    if matches!(command, Some(ChannelRuntimeCommand::ListProjects)) {
        let mut projects = crate::project::list(&chat.tenant).await?;
        projects.sort_by(|a, b| a.id.cmp(&b.id));
        let mut response = String::from("项目（使用 /project <稳定ID> 选择）：\n");
        // 先过滤失效目录，再限制条数，避免旧登记挤掉可访问项目。
        projects.retain(|project| {
            std::fs::canonicalize(&project.worktree).is_ok_and(|path| path.is_dir())
        });
        let total = projects.len();
        for project in projects.into_iter().take(30) {
            let label = project
                .name
                .as_deref()
                .unwrap_or(&project.worktree)
                .chars()
                .filter(|c| !c.is_control())
                .take(80)
                .collect::<String>();
            response.push_str(&format!("{}  {}\n", project.id, label));
        }
        if total > 30 {
            response.push_str("列表最多显示 30 项；可直接使用已知稳定 ID 选择。\n");
        }
        return Ok(response);
    }
    // 与模型执行共用聊天锁；拒绝繁忙请求，不排队应用过期选择。
    let gate = channel_chat_gate(ctx, msg);
    let _guard = gate.try_write().map_err(|_| anyhow::anyhow!("当前聊天繁忙，请先使用 /stop"))?;
    let mut candidate = ctx.clone();
    candidate.message_snapshot = None;
    if matches!(command, Some(ChannelRuntimeCommand::ShowProject)) {
        let snapshot = resolve_channel_message_snapshot(&candidate, msg).await?;
        let source = match snapshot.source {
            ChannelProjectSource::Chat => "聊天选择",
            ChannelProjectSource::Instance => "实例默认",
            ChannelProjectSource::Workspace => "工作目录默认",
        };
        return Ok(format!(
            "当前项目：{}\n来源：{source}\n/projects 查看项目；/project <稳定ID> 选择；/project default 恢复默认。",
            snapshot.project.id
        ));
    }
    let Some(ChannelRuntimeCommand::SelectProject(id)) = command else {
        anyhow::bail!("未知项目命令");
    };
    let mut state = ctx.project_state.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if id == "default" {
        state.selections.remove(&chat);
    } else {
        // 只接受精确 ID，绝不把目录或名称交给项目发现 API。
        state.selections.insert(chat.clone(), id.clone());
    }
    candidate.project_state = Arc::new(Mutex::new(state));
    let snapshot = resolve_channel_message_snapshot(&candidate, msg).await?;
    // 校验完成后没有 await：历史、会话映射与选择在同一聊天锁内原子更新。
    let key = snapshot.scope.key();
    ctx.conversation_histories.lock().unwrap_or_else(|e| e.into_inner()).remove(&key);
    ctx.route_overrides.lock().unwrap_or_else(|e| e.into_inner()).remove(&key);
    sender_session_store().lock().unwrap_or_else(|e| e.into_inner()).remove(&snapshot.scope);
    let mut state = ctx.project_state.lock().unwrap_or_else(|e| e.into_inner());
    if id == "default" {
        state.selections.remove(&chat);
    } else {
        state.selections.insert(chat, id);
    }
    Ok(format!("已切换项目：{}；下一条消息将开启新会话，旧会话已保留。", snapshot.project.id))
}
