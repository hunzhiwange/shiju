---
name: cli-workbench
description: 在识矩 CLI 工作台查找同项目会话、启动辅助 ACP Agent、发送协作任务并按请求 ID 核对结果。
---

# CLI 工作台协作

在工作台内有权调用 `shiju` 的 Agent 可使用此技能。手动安装时，请用户将本目录复制到项目的 `.agents/skills/cli-workbench/` 或用户的 `~/.agents/skills/cli-workbench/`。如果所用 Agent 使用自己的技能目录，请由用户将本目录放到它支持的位置。

工作台子进程使用网关提供的当前会话连接。保留继承的环境，避免打印、复制或转发连接凭据。工作台外的调用需明确提供网关用户与项目 ID，并按网关配置选择凭据来源。网关会校验访问权限。

```sh
shiju workbench --host 127.0.0.1 --port 3000 --platform local --company local \
  --user "$GATEWAY_USER_ID" --skey-env GATEWAY_SKEY list --project "$PROJECT_ID"
```

切换项目需要用户授予相应网关权限。请在目标项目的工作台内发起协作，或由用户在外部显式指定已授权的身份和项目。

## 发送并核对结果

1. 运行 `shiju workbench list`，从 `result` 选择同项目可用会话。使用返回的 `id`，并检查 `capabilities.delivery` 与 `state`。
2. 需要辅助 Agent 时，经用户确认运行 `shiju workbench start acp acp-claude --yes`。将示例中的 `acp-claude` 替换为用户已配置的 ACP 配置 ID。记录 `result.id`。ACP 进程可在第一次投递时启动。
3. 经用户确认发送后，记录网关回传的 `result.id`。发送前生成唯一的请求 ID，以便响应丢失时仍可查询。

   ```sh
   session_id='替换为 list 或 start 返回的会话 ID'
   request_id="$(uuidgen)"
   printf '%s\n' '请检查当前项目的改动，并总结尚未覆盖的边界。' |
     shiju workbench send "$session_id" --request-id "$request_id" --yes
   shiju workbench wait "$session_id" --request-id "$request_id" --timeout-secs 60
   shiju workbench read "$session_id" --request-id "$request_id"
   ```

4. `completed` 时读取 `result.output`，并查看 `output_truncated`。若收到 `failed`，查看 `result.error`。核对返回的 `id.target_session_id` 和 `id.request_id`，确认结果属于本次投递。

## 遇到审批、草稿或超时

- `waiting_for_approval` 或 `awaiting_approval` 时，请用户在工作台处理权限请求，然后用同一请求 ID 继续查询。
- `busy` 表示目标正忙。等用户确认目标可用后重试，保持目标、请求 ID 和消息原文一致。
- `pending_submission` 表示原生 CLI 收到了草稿。提示用户在目标终端审阅并手动提交。Shell 或不支持投递的目标会返回 `unavailable`。
- `timed_out` 或发送连接中断后，先运行 `shiju workbench status "$session_id" --request-id "$request_id"`。已受理的投递可用原 ID 继续 `wait` 或 `read`。确认需要重试时，复用原目标、请求 ID 和消息原文。
- 调用失败时，CLI 以非零状态退出，并在 JSON 的 `error.code` 中报告原因。等待超时时，如果已收到快照，`message` 会保留该快照。成功返回的 `result.state` 表示投递自身状态。
