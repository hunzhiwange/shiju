# 任务 7: Auggie CLI 安装与编辑器验收

批次：【批次 7】 依赖：批次 6 已结束（可阻塞）

状态：已完成（按本轮用户要求，仅验收编辑器创建）

目的：从系统安装 Auggie CLI，使用该 ACP 和指定模型完成同一编辑器需求。

来源任务：用户指定的网关流式编辑器请求。

预计会话范围：只验证 auggie 的安装、连接、流式聊天和独立产物。与其他 ACP 共用安装器和网关，按批次串行。

## 文件

- 产物：`/Users/xiangmin/worker/code/ruejs/acpdemo/auggie/`。
- 证据：本任务文件的实测记录；原始响应仅留在本机私有临时目录。

## 上下文

- 安装、安装状态、连接诊断和模型来源是独立信号。本任务走现有网关，不修改 Rust，不运行 Cargo，槽位不适用。
- 按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

## 验证计划

1. 确认目标目录无既有改动；从 `/v1/global/config/acp/installed` 和 `/v1/agent?include_disabled=true` 取基线；用 `POST /v1/desktop/acp-install/auggie` 测系统安装。记录 HTTP、退出码、前后安装状态、解析的可执行文件和可核实版本。安装返回失败而已有可执行文件时，分别报告并继续。
2. 从 `GET /v1/global/config/acp` 提取该项的配置，传给 `POST /v1/desktop/acp-diagnostic/acp-smoke-auggie` 验证握手；不要回显或提交配置环境值。用 `GET /v1/acp-model-source/auggie` 记录客户端模型支持、版本及拒绝原因。
3. 若预设关闭，用 `PATCH /v1/global/config` 的 `{"agents":{"acp-auggie":{"enabled":true}}}` 临时启用当前租户预设。本项结束恢复原值，并从 `GET /v1/agent?include_disabled=true` 复查。本项会话使用 `full_access: true`。验证编辑器的写入范围为本项的目标目录。不得静默换模型、账号或引擎。
4. 发送下列真实请求。完整 SSE 响应写到私有临时文件，再摘录事件类型和脱敏错误；同时检查 curl 退出码、HTTP 和最终事件。用响应头的 `x-vw-session-id` 读取 `/v1/session/{session_id}/ui/summary` 的绑定和 `/v1/session/{session_id}/acp-options` 的选定模型，不能用管道最后一个命令的退出码判成功。

```bash
curl -sS -N --max-time 180 -H 'Content-Type: application/json' \
  -H 'X-Platform-ID: 10000' \
  -H 'X-Company-ID: 100000000' \
  -H 'X-Directory: /Users/xiangmin/worker/code/ruejs' \
  --data-binary '{"model":"alibaba-token-plan-cn/deepseek-v4.1-flash-1017","agent":"acp-auggie","acp_agent":"auggie","acp_model_source":"session_model","options":{"cwd":"/Users/xiangmin/worker/code/ruejs","full_access":true},"messages":[{"role":"user","content":"帮我用 Rue.js 实现一个纯文本编辑器，支持输入、实时显示字数、刷新后恢复内容。只写到 acpdemo/auggie/，不修改目录外文件。使用本仓库的 @rue-js/rue；在目标目录内提供可运行页面与运行说明。"}]}' \
  http://127.0.0.1:42617/v1/chat/stream
```

5. 用产物的运行说明启动页面，检查使用 Rue、实际输入后字数更新、刷新后内容恢复，无阻断性页面错误，检查未写目标目录外文件。

## 完成证据

系统安装成功、真实 ACP 连接、指定模型发起的请求成功结束，且浏览器行为全部通过，方可标记已完成。逐段记录实测 HTTP/退出码、终态、产物和验证结果；失败明确区分安装、启动、授权、模型来源、超时和产物问题。

## 实测记录

- 系统安装（沿用切换模型前实测）：HTTP 200，curl 0，4.96 秒。auggie 安装完成，请测试连接；首次使用可能需要登录。
- 启动命令探测：False → True；`/opt/homebrew/bin/auggie`。独立诊断：connected。
- 模型能力：{'status': 'unsupported', 'source': 'session_model', 'reason': 'unsupported_cli'}；已审查版本：。
- 聊天：HTTP 200，curl 0，22.79 秒；chat.error。acp: ACP auggie session_model launch has not been verified: fixed upstream rejects extension Provider registration; arbitrary model injection is unavailable
- 事件数量：`{"chat.context_loaded": 1, "chat.step_start": 1, "chat.model_recovery": 1, "chat.session_updated": 1, "chat.error": 1}`；服务端绑定：`auggie`；选定模型：`未取得`。
- 产物：0 个文件；无文件。
- 预设恢复：启用 HTTP 200，运行前 True；恢复 HTTP 200，复查 False。
- 复跑条件：处理请求错误「acp: ACP auggie session_model launch has not been verified: fixed upstream rejects extension Provider registration; arbitrary model injection is unavailable」。

## 2026-11-12 复测（创建文件即结束）

- 本轮按用户要求，以实际编辑器文件创建为终点，不执行浏览器、构建或 Cargo 检查。仍为**已阻塞**，未更新 `docs/dev/acp/ACP验收.md` 为成功。
- 原 42617 端口未监听，使用现有 `target/rust-slot-8/debug/shiju` 与 `/Users/xiangmin/.shiju-harness-dev` 启动临时网关 `127.0.0.1:56858`。沿用最近验收租户 `10001 / 100010005`，指定模型、引擎及 `session_model` 保持不变，`full_access=true`。
- 安装 HTTP 200、curl 0、2.305644 秒；安装前后均 true。可执行文件 `/opt/homebrew/bin/auggie`，版本命令退出码 0，`0.37.1 (commit e8d38687)`。独立诊断 HTTP 200、curl 0、connected。
- 模型来源接口 HTTP 200、curl 0，明确 `session_model` unsupported / unsupported_cli，仅 `cli_local` supported。没有静默切换到 CLI 自带模型。
- 仍实测指定模型请求：HTTP 200、curl 0、5.642566 秒，终态 `chat.error`，错误 `acp: ACP session_model driver is not verified`。事件计数：`{"chat.context_loaded": 1, "chat.step_start": 1, "chat.model_recovery": 1, "chat.session_updated": 1, "chat.error": 1}`。
- 会话 `ses_1cff7726c001JfSENoWh61eZPN` 的 ui/summary 与 acp-options 均 HTTP 200、curl 0；绑定 acp-auggie / auggie，full_access=true，connected=false，未取得选定模型。
- 产物 0 个文件；Rue.js 仓库请求前后普通文件 SHA-256 比较（排除 .git、node_modules、target），无新增或变更。
- acp-auggie 预设基线 false；临时启用、恢复均 HTTP 200、curl 0，最终复查 false。临时网关已停止，未修改 Rust。
- 原始请求、SSE、配置及文件基线仅保存在私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/auggie-acceptance-o7tc5vyd/`，未提交凭据或环境值。
- 下一步：补齐并验证 Auggie session_model 驱动后，使用相同模型重试编辑器创建。

## 2026-11-12 驱动修复与最终验收

- 本轮用户明确授权修复 Auggie session_model 驱动，并要求编辑器实际创建成功即结束；本轮范围覆盖前文“不修改 Rust”和浏览器验收要求。
- 已增加固定版原生 Auggie 0.37.1 的受管模型绑定补丁、安装校验、Rust 驱动和子进程隔离。保留原生 ACP 与工具，不替换引擎、不切换指定模型。
- 临时修复版网关 50928，指定租户 10001 / 100010005；指定模型请求 HTTP 200、curl 0、72.391644 秒，chat.done，无 chat.error。
- 实际创建 7 个编辑器文件，目标目录外无文件变化；未执行编辑器构建或浏览器测试。
- 5 个转换测试及真实原生 CLI 工具续接回归通过；网关构建通过。完整 Clippy 退出码 101，受桌面 worktree 无关未使用代码阻塞，不计为通过。
- 预设已恢复 false；临时网关已停止，槽位已释放。完整证据、路径与后续建议已更新至 `docs/dev/acp/ACP验收.md` 的 Auggie CLI 验收节。
