# 任务 4: GitHub Copilot 安装与编辑器验收

批次：【批次 4】 依赖：批次 3 已结束（可阻塞）

状态：已阻塞

目的：从系统安装 GitHub Copilot，使用该 ACP 和指定模型完成同一编辑器需求。

来源任务：用户指定的网关流式编辑器请求。

预计会话范围：只验证 copilot 的安装、连接、流式聊天和独立产物。与其他 ACP 共用安装器和网关，按批次串行。

## 文件

- 产物：`/Users/xiangmin/worker/code/ruejs/acpdemo/copilot/`。
- 证据：本任务文件的实测记录；原始响应仅留在本机私有临时目录。

## 上下文

- 安装、安装状态、连接诊断和模型来源是独立信号。本任务走现有网关，不修改 Rust，不运行 Cargo，槽位不适用。
- 按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

## 验证计划

1. 确认目标目录无既有改动；从 `/v1/global/config/acp/installed` 和 `/v1/agent?include_disabled=true` 取基线；用 `POST /v1/desktop/acp-install/copilot` 测系统安装。记录 HTTP、退出码、前后安装状态、解析的可执行文件和可核实版本。安装返回失败而已有可执行文件时，分别报告并继续。
2. 从 `GET /v1/global/config/acp` 提取该项的配置，传给 `POST /v1/desktop/acp-diagnostic/acp-smoke-copilot` 验证握手；不要回显或提交配置环境值。用 `GET /v1/acp-model-source/copilot` 记录客户端模型支持、版本及拒绝原因。
3. 若预设关闭，用 `PATCH /v1/global/config` 的 `{"agents":{"acp-copilot":{"enabled":true}}}` 临时启用当前租户预设。本项结束恢复原值，并从 `GET /v1/agent?include_disabled=true` 复查。本项会话使用 `full_access: true`。验证编辑器的写入范围为本项的目标目录。不得静默换模型、账号或引擎。
4. 发送下列真实请求。完整 SSE 响应写到私有临时文件，再摘录事件类型和脱敏错误；同时检查 curl 退出码、HTTP 和最终事件。用响应头的 `x-vw-session-id` 读取 `/v1/session/{session_id}/ui/summary` 的绑定和 `/v1/session/{session_id}/acp-options` 的选定模型，不能用管道最后一个命令的退出码判成功。

```bash
curl -sS -N --max-time 180 -H 'Content-Type: application/json' \
  -H 'X-Platform-ID: 10000' \
  -H 'X-Company-ID: 100000000' \
  -H 'X-VibeWindow-Directory: /Users/xiangmin/worker/code/ruejs' \
  --data-binary '{"model":"alibaba-token-plan-cn/deepseek-v4.1-flash-1017","agent":"acp-copilot","acp_agent":"copilot","acp_model_source":"session_model","options":{"cwd":"/Users/xiangmin/worker/code/ruejs","full_access":true},"messages":[{"role":"user","content":"帮我用 Rue.js 实现一个纯文本编辑器，支持输入、实时显示字数、刷新后恢复内容。只写到 acpdemo/copilot/，不修改目录外文件。使用本仓库的 @rue-js/rue；在目标目录内提供可运行页面与运行说明。"}]}' \
  http://127.0.0.1:42617/v1/chat/stream
```

5. 用产物的运行说明启动页面，检查使用 Rue、实际输入后字数更新、刷新后内容恢复，无阻断性页面错误，检查未写目标目录外文件。

## 完成证据

系统安装成功、真实 ACP 连接、指定模型发起的请求成功结束，且浏览器行为全部通过，方可标记已完成。逐段记录实测 HTTP/退出码、终态、产物和验证结果；失败明确区分安装、启动、授权、模型来源、超时和产物问题。

## 实测记录

- 系统安装（沿用切换模型前实测）：HTTP 200，curl 0，0.12 秒。copilot 安装完成，请测试连接；首次使用可能需要登录。
- 启动命令探测：True → True；`/Users/xiangmin/.local/share/shiju/acp-runtime/copilot-1.0.105-model-source-v1/node_modules/.bin/copilot`。独立诊断：authentication_required。
- 模型能力：{'status': 'supported', 'source': 'session_model'}；已审查版本：1.0.105。
- 聊天：HTTP 200，curl 28，180.13 秒；超时。
- 事件数量：`{"chat.context_loaded": 1, "chat.step_start": 1, "chat.delta": 11, "chat.tool": 28}`；服务端绑定：`copilot`；选定模型：`alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。
- 产物：0 个文件；无文件。
- 超时后调用会话取消：HTTP 200。
- 复跑条件：补齐未通过的安装、聊天终态或页面行为证据。
