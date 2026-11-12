# 任务 5: Gemini CLI 安装与编辑器验收

批次：【批次 5】 依赖：批次 4 已结束（可阻塞）

状态：已阻塞

目的：从系统安装 Gemini CLI，使用该 ACP 和指定模型完成同一编辑器需求。

来源任务：用户指定的网关流式编辑器请求。

预计会话范围：只验证 gemini 的安装、连接、流式聊天和独立产物。与其他 ACP 共用安装器和网关，按批次串行。

## 文件

- 产物：`/Users/xiangmin/worker/code/ruejs/acpdemo/gemini/`。
- 证据：本任务文件的实测记录；原始响应仅留在本机私有临时目录。

## 上下文

- 安装、安装状态、连接诊断和模型来源是独立信号。本任务走现有网关，不修改 Rust，不运行 Cargo，槽位不适用。
- 按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

## 验证计划

1. 确认目标目录无既有改动；从 `/v1/global/config/acp/installed` 和 `/v1/agent?include_disabled=true` 取基线；用 `POST /v1/desktop/acp-install/gemini` 测系统安装。记录 HTTP、退出码、前后安装状态、解析的可执行文件和可核实版本。安装返回失败而已有可执行文件时，分别报告并继续。
2. 从 `GET /v1/global/config/acp` 提取该项的配置，传给 `POST /v1/desktop/acp-diagnostic/acp-smoke-gemini` 验证握手；不要回显或提交配置环境值。用 `GET /v1/acp-model-source/gemini` 记录客户端模型支持、版本及拒绝原因。
3. 若预设关闭，用 `PATCH /v1/global/config` 的 `{"agents":{"acp-gemini":{"enabled":true}}}` 临时启用当前租户预设。本项结束恢复原值，并从 `GET /v1/agent?include_disabled=true` 复查。本项会话使用 `full_access: true`。验证编辑器的写入范围为本项的目标目录。不得静默换模型、账号或引擎。
4. 发送下列真实请求。完整 SSE 响应写到私有临时文件，再摘录事件类型和脱敏错误；同时检查 curl 退出码、HTTP 和最终事件。用响应头的 `x-vw-session-id` 读取 `/v1/session/{session_id}/ui/summary` 的绑定和 `/v1/session/{session_id}/acp-options` 的选定模型，不能用管道最后一个命令的退出码判成功。

```bash
curl -sS -N --max-time 180 -H 'Content-Type: application/json' \
  -H 'X-Platform-ID: 10000' \
  -H 'X-Company-ID: 100000000' \
  -H 'X-Directory: /Users/xiangmin/worker/code/ruejs' \
  --data-binary '{"model":"alibaba-token-plan-cn/deepseek-v4.1-flash-1017","agent":"acp-gemini","acp_agent":"gemini","acp_model_source":"session_model","options":{"cwd":"/Users/xiangmin/worker/code/ruejs","full_access":true},"messages":[{"role":"user","content":"帮我用 Rue.js 实现一个纯文本编辑器，支持输入、实时显示字数、刷新后恢复内容。只写到 acpdemo/gemini/，不修改目录外文件。使用本仓库的 @rue-js/rue；在目标目录内提供可运行页面与运行说明。"}]}' \
  http://127.0.0.1:42617/v1/chat/stream
```

5. 用产物的运行说明启动页面，检查使用 Rue、实际输入后字数更新、刷新后内容恢复，无阻断性页面错误，检查未写目标目录外文件。

## 完成证据

系统安装成功、真实 ACP 连接、指定模型发起的请求成功结束，且浏览器行为全部通过，方可标记已完成。逐段记录实测 HTTP/退出码、终态、产物和验证结果；失败明确区分安装、启动、授权、模型来源、超时和产物问题。

## 实测记录

- 系统安装（沿用切换模型前实测）：HTTP 200，curl 0，5.69 秒。gemini 安装完成，请测试连接；首次使用可能需要登录。
- 启动命令探测：True → True；`/Users/xiangmin/.local/share/shiju/acp-runtime/gemini-0.66.0-model-source-v1/node_modules/.bin/gemini`。独立诊断：authentication_required。
- 模型能力：{'status': 'supported', 'source': 'session_model'}；已审查版本：0.66.0。
- 聊天：HTTP 200，curl 0，67.32 秒；chat.error。首轮无内容；session/resume 方法不存在；session/load 拒绝会话 ID
- 事件数量：`{"chat.context_loaded": 1, "chat.step_start": 2, "chat.model_recovery": 2, "chat.session_updated": 1, "chat.error": 1}`；服务端绑定：`gemini`；选定模型：`alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。
- 产物：0 个文件；无文件。
- 复跑条件：处理请求错误「首轮无内容；session/resume 方法不存在；session/load 拒绝会话 ID」。


### 2026-11-12 重测（编辑器创建即停止）

- 按用户最新要求，以编辑器文件创建完成为终点，不执行浏览器、构建或 Cargo 检查。本次仍未创建成功，状态保持已阻塞；未向 `docs/dev/acp/ACP验收.md` 写入成功结论。
- 沿用最近验收确认的租户 `10001 / 100010005`，网关 `127.0.0.1:42617`，指定模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`；未更换模型、账号或引擎。
- 安装接口 HTTP 200、curl 0、0.028653 秒，安装前后均为 true；可执行文件 `/Users/xiangmin/.local/share/shiju/acp-runtime/gemini-0.66.0-model-source-v1/node_modules/.bin/gemini`，`--version` 退出码 0，版本 0.66.0。
- 独立诊断 HTTP 200、curl 0、`authentication_required`；模型来源 HTTP 200、curl 0，`session_model` supported。
- 流式请求 HTTP 200、curl 0、47.366920 秒，终态 `chat.error`。首轮模型未返回内容；恢复时 `session/resume` 返回 Method not found，`session/load` 返回 Invalid session identifier。与前次错误一致，不能将 HTTP 200 记为成功。
- 事件数量：`chat.context_loaded` ×1、`chat.step_start` ×2、`chat.model_recovery` ×2、`chat.session_updated` ×1、`chat.error` ×1；无 `chat.done`、无工具事件。
- 会话 `ses_1cf25bbf7001rcbNnhYEk6v1iW`：同租户及工作目录头读取 `ui/summary` 与 `acp-options` 均为 HTTP 200、curl 0；绑定 `acp-gemini / gemini`、`full_access=true`，选定模型为 `session_model / alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，连接状态为 false。
- 产物 0 个文件。请求前后比较 Rue.js 仓库普通文件 SHA-256（排除 `.git`、`node_modules`、`target`），无新增或内容变化；目标目录不存在。
- 本机历史文件仅有会话元数据、用户消息和更新记录，没有助手产物，支持首轮无内容的观察；尚不能据此断言首轮无内容的底层根因。网关为现有 `target-dir/debug/shiju gateway` 进程，工作区已有他人 Gemini 中继修改；本任务不修改 Rust、不重启该实例、不重复同一失败请求。
- 预设 `acp-gemini` 原本 enabled=true，结束复查仍为 true，本轮未修改配置，无需恢复。
- 原始请求、SSE、配置快照和文件基线仅保存于本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/gemini-acceptance-mg_8s1pt`，未提交凭据或配置环境值。
- 后续动作：排查并修复 Gemini 首轮无内容及会话恢复失败后，使用同租户、同模型重新验收；编辑器创建成功后再更新正式验收文档。
