# Codex CLI 验收

```bash
curl -sS -N --max-time 180 -H 'Content-Type: application/json' \
  -H 'X-Platform-ID: 10000' \
  -H 'X-Company-ID: 100000000' \
  -H 'X-Directory: /Users/xiangmin/worker/code/ruejs' \
  --data-binary '{"model":"alibaba-token-plan-cn/deepseek-v4.1-flash-1017","agent":"acp-codex","acp_agent":"codex","acp_model_source":"session_model","options":{"cwd":"/Users/xiangmin/worker/code/ruejs","full_access":true},"messages":[{"role":"user","content":"帮我用 Rue.js 实现一个纯文本编辑器，支持输入、实时显示字数、刷新后恢复内容。只写到 acpdemo/codex/，不修改目录外文件。使用本仓库的 @rue-js/rue；在目标目录内提供可运行页面与运行说明。"}]}' \
  http://127.0.0.1:42617/v1/chat/stream
```

# OpenCode 验收

2026-11-12：**真实 ACP 与浏览器验收通过**。最新指定租户 `10001 / 100010005`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。

验收状态：功能验收已完成，额外聚焦回归待补跑。该回归编译已停止，退出码 130，未执行断言；不计为测试通过。

## 复现与验收步骤

1. 使用租户 `10001 / 100010005`，从 `GET /v1/global/config/acp/installed` 与 `GET /v1/agent?include_disabled=true` 记录安装与预设基线；调用 `POST /v1/desktop/acp-install/opencode`，分别核对 HTTP、请求退出码、可执行文件与版本。
2. 从 `GET /v1/global/config/acp` 提取 OpenCode 配置，传给 `POST /v1/desktop/acp-diagnostic/acp-smoke-opencode`；通过 `GET /v1/acp-model-source/opencode` 核对 `session_model` 支持。配置环境值仅保存在本机私有目录，不回显或提交。
3. 若 `acp-opencode` 未启用，临时通过 `PATCH /v1/global/config` 设置 `{"agents":{"acp-opencode":{"enabled":true}}}`，结束后恢复基线。请求使用 `full_access: true`，产物只允许写入 `acpdemo/opencode/`。
4. 启动包含中继修复的网关后执行下列请求。示例使用原开发实例端口 `42617`；本次成功验收实际使用独立端口 `52383`。为覆盖本次 220.613 秒的处理时长，示例客户端限时设为 300 秒；原始实测仍为下文记录的 180 秒限时及续传。

```bash
curl -sS -N --max-time 300 -H 'Content-Type: application/json' \
  -H 'X-Platform-ID: 10001' \
  -H 'X-Company-ID: 100010005' \
  -H 'X-Directory: /Users/xiangmin/worker/code/ruejs' \
  --data-binary '{"model":"alibaba-token-plan-cn/deepseek-v4.1-flash-1017","agent":"acp-opencode","acp_agent":"opencode","acp_model_source":"session_model","options":{"cwd":"/Users/xiangmin/worker/code/ruejs","full_access":true},"messages":[{"role":"user","content":"帮我用 Rue.js 实现一个纯文本编辑器，支持输入、实时显示字数、刷新后恢复内容。只写到 acpdemo/opencode/，不修改目录外文件。使用本仓库的 @rue-js/rue；在目标目录内提供可运行页面与运行说明。"}]}' \
  http://127.0.0.1:42617/v1/chat/stream
```

5. 将响应头与完整 SSE 保存在本机私有目录，分别检查 curl 退出码、HTTP 与最终事件；HTTP 200 本身不代表成功。通过响应头 `x-vw-session-id` 查询 `/v1/session/{session_id}/ui/summary` 与 `/v1/session/{session_id}/acp-options`，确认绑定、完全访问授权与实际选定模型，不能只凭 `/v1/models` 列表判断模型是否可用。
6. 按产物 README 启动页面，验证 Rue API 使用、实际输入后的字数更新、刷新恢复、控制台及目标目录外文件变化。只有安装、真实 ACP 请求成功终态与浏览器行为均通过，才判定功能验收通过。

## 前序阻塞与结论边界

- 首轮在 ACP 初始化阶段拒绝完全访问授权，未产生文件；后续指定租户的真实会话已确认 `full_access=true`，该拒绝未复现。
- 租户 `10000 / 100000000` 的模型解析失败只代表当时该网关与租户配置，不能据此认定上游模型不存在。用户随后明确指定租户 `10001 / 100010005`，最终模型为 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。
- 中间使用 `deepseek-v4-flash` 的请求在 99.446 秒以 `chat.error` / `Bad Gateway` 结束，产物为 0，浏览器检查未执行。该模型的 403 结论不适用于最终通过验收的模型。
- 前序模型解析与 502 排查的原始证据分别保存在本机私有目录 `/tmp/shiju-opencode-5dCvw1/` 与 `/tmp/shiju-opencode-hv6SKK/`。

## 502 根因与修复

原模型 `alibaba-token-plan-cn/deepseek-v4-flash` 的上游请求返回 `403 AccessDenied.Unpurchased`，中继把所有非成功响应改写成 502，导致 OpenCode 重试。`crates/vw-agent/src/model_relay/transport.rs` 已改为保留上游 4xx 状态，继续丢弃错误正文和响应头；重定向与服务端错误仍按原策略屏蔽。双 HTTP 边界回归场景覆盖 403、401、400、429、307、503，检查状态、无错误正文/响应头泄漏与无中继内部重试。

用户更新密钥并最终指定 `deepseek-v4.1-flash-1017` 后，该模型最小调用 HTTP 200。当前租户该提供商的 Chat API 地址已从误设的 `/api/v1` 修正为 `/compatible-mode/v1`，未修改密钥。地址依据 [阿里云官方 Base URL 文档](https://help.aliyun.com/zh/model-studio/base-url)；原 403 的解释参见 [官方 FAQ](https://help.aliyun.com/zh/model-studio/token-plan-personal-faq)。

## 实测证据

- 修复版构建：`cargo build -p vw-cli --bin shiju` 退出码 0。为不打断用户实例，使用本机独立网关 `127.0.0.1:52383` 验收，二进制为 `target/rust-slot-8/debug/shiju`。
- 系统安装：HTTP 200，0.357 秒，安装前后均为 True，版本 1.19.5。安装、基线与诊断请求脚本退出码 0。独立诊断仍为 authentication_required，但真实会话成功连接，不能仅凭独立诊断推断会话认证失败。
- 预设 acp-opencode 原本启用，结束仍启用，本轮未修改预设。模型来源 session_model supported。
- 请求使用指定租户、指定模型、acp-opencode / opencode、session_model、full_access=true，写入范围为 `acpdemo/opencode/`。
- 原 SSE HTTP 200，在 180.006 秒客户端限时处断开，curl 28；使用相同请求体、原 x-vw-stream-request-id 与 Last-Event-ID=486 断点续传，HTTP 200、curl 0、0.034 秒，取得 chat.done。续传没有重新发起模型任务。服务端助手记录 finish=stop，处理时长 220.613 秒，无错误。
- 原请求与续传合计事件：chat.context_loaded ×1、chat.step_start ×1、chat.delta ×324、chat.tool ×214、chat.session_updated ×2、chat.step_finish ×1、chat.done ×1，无 chat.error。
- 会话 `ses_1cef12a4b001gCfSRnJbGysTs1`：ui/summary HTTP 200，绑定 agent=acp-opencode、acp_agent=opencode、full_access=true；acp-options HTTP 200，执行期间 connected=true，selected_model 为 session_model / alibaba-token-plan-cn/deepseek-v4.1-flash-1017。
- 产物为 `/Users/xiangmin/worker/code/ruejs/acpdemo/opencode/`，8 个源码/说明/配置文件及 3 个 dist 文件。App.tsx 实际导入本仓库 @rue-js/rue 的 ref、computed、生命周期 API；Vite 配置复用仓库 Rue 编译插件与别名。
- 按 README 启动 Vite 页面，浏览器真实按键输入 `你好 Rue\n第二行 123` 后显示 11 字、14 字符、2 行；刷新后正文、统计与“已恢复上次编辑的内容”提示均正确。浏览器控制台 error/warn 列表为空。
- 独立执行 `pnpm exec tsc -p acpdemo/opencode/tsconfig.json` 与 `pnpm exec vite build --config acpdemo/opencode/vite.config.mjs`，均退出码 0。
- 请求前后比较 Rue.js 仓库普通文件 SHA-256（排除 .git、node_modules、target），全部新增与变更均在 acpdemo/opencode/ 内，无目录外文件变化。
- `cargo clippy --all-targets --all-features -- -D warnings` 退出码 0。首次聚焦测试编译被当时工作区无关错误阻塞，未取得预期红测；修复后的额外聚焦回归 `acp_model_source_relay_chat_roundtrip_stream_tools_and_account_isolation`（all-features）编译耗时过长，经用户催促收尾后已停止（退出码 130），未执行断言，不计为通过。
- Cargo 使用共享槽位 8，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-8`。原始请求、SSE、日志及配置仅保存在私有目录 `/tmp/shiju-opencode-fix-Nl5QNb/`，未提交凭据或配置环境值。

## 运行产物

在 Rue.js 仓库根目录执行：

```sh
pnpm exec vite --config acpdemo/opencode/vite.config.mjs
```

本次验收使用独立修复版网关；用户原 42617 实例未被重启。后续重启到包含本次修改的构建后即可使用修复后的错误映射。临时测试服务已停止，构建槽位已释放，缓存保留。

# GitHub Copilot 验收

2026-11-12：**编辑器创建成功**。按用户本轮要求，以编辑器文件创建完成为验收终点；未执行浏览器行为验证、构建或 Cargo 检查，不将这些检查记为通过。

- 网关：`127.0.0.1:42617`；沿用上一项确认的租户 `10001 / 100010005`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，`acp-copilot / copilot`、`session_model`、`full_access=true`。
- 系统安装接口 HTTP 200，0.10 秒；安装前后均为 true。可执行文件为 `/Users/xiangmin/.local/share/shiju/acp-runtime/copilot-1.0.105-model-source-v1/node_modules/.bin/copilot`，`--version` 退出码 0，版本 `1.0.105`。安装、诊断与状态读取脚本退出码均为 0。
- 独立诊断 HTTP 200、`authentication_required`；模型来源接口 HTTP 200、`session_model` supported。真实会话随后确认 `connected=true`，独立诊断不代表本次托管模型请求失败。
- 流式请求 HTTP 200、curl 0，耗时 107.320 秒，终态 `chat.done`，无 `chat.error`。事件：`chat.context_loaded` ×1、`chat.step_start` ×1、`chat.delta` ×97、`chat.tool` ×51、`chat.session_updated` ×2、`chat.step_finish` ×1、`chat.done` ×1。请求额外要求“完成文件后简短回复即结束，不运行额外测试”。
- 会话 `ses_1cf1d9258001nPQC5k9ykxrsjo`：带相同租户及工作目录头读取 `ui/summary` 返回 HTTP 200，绑定 `agent=acp-copilot`、`acp_agent=copilot`、`full_access=true`；`acp-options` HTTP 200，执行期间连接成功，选定模型与请求一致。
- 产物：`/Users/xiangmin/worker/code/ruejs/acpdemo/copilot/` 下的 `index.html`、`vite.config.mjs`、`README.md`、`src/main.tsx`、`src/App.tsx`、`src/style.css`。源码导入本仓库 `@rue-js/rue`，包含输入处理、响应式字数统计、`localStorage` 保存与恢复逻辑；运行说明已提供。
- 请求前后比较 Rue.js 仓库普通文件 SHA-256（排除 `.git`、`node_modules`、`target`）：仅上述 6 个文件新增，目标目录外无文件内容变化。
- `acp-copilot` 预设原本 enabled=true，结束复查仍为 true；本轮未修改配置，无需恢复。未修改 Rust、未启动额外测试服务。
- 原始请求、SSE、配置快照与文件基线仅保存在本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/copilot-acceptance-na3515th/`，未提交凭据或配置环境值。

在 Rue.js 仓库根目录启动编辑器：

```sh
pnpm exec vite --config acpdemo/copilot/vite.config.mjs
```

# Gemini CLI 验收

2026-11-12：**编辑器创建成功**。按用户要求，以实际创建编辑器文件为验收终点；未运行编辑器构建、浏览器或功能测试，不将这些检查记为通过。

- 租户 `10001 / 100010005`；模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`；`acp-gemini / gemini`、`session_model`、`full_access=true`。未更换账号、模型或引擎。
- 使用独立修复版网关 `127.0.0.1:57849`，用户原 `42617` 实例未重启。系统安装接口 HTTP 200、curl 0、0.025095 秒，安装前后均为 true；CLI 版本 `0.66.0`，版本命令退出码 0。可执行文件为 `/Users/xiangmin/.shiju-harness-dev/acp-runtime/gemini-0.66.0-model-source-v1/node_modules/.bin/gemini`。
- 独立诊断 HTTP 200、`authentication_required`；模型来源接口 HTTP 200、`session_model` supported。真实流式工具执行和最终产物另行核实，未用独立诊断结果替代真实会话验收。

## 首轮与工具续接修复

- 已确认模型传入原生 CLI：实际请求路径包含 `/v1beta/models/deepseek-v4.1-flash-1017:streamGenerateContent`。不是模型名未传递。
- 该上游默认返回非空 `reasoning_content`，而当前 Gemini→Chat 文本/工具转换不能保留可恢复的推理状态。实际调用对照：默认请求包含推理，显式 `enable_thinking=false` 后正常返回正文且无推理内容。中继现在对该转换路径统一应用模型已支持的关闭开关，覆盖 Gemini 省略 `thinkingConfig` 的工具续接；原生 Gemini 路径不变，不丢弃不支持的推理数据冒充成功。
- 同时沿用工作区已有的显式 `thinkingBudget=0` 与空推理字段兼容处理。真实历史进一步暴露出合法的混合消息：同一 Gemini `user` Content 中先有两个 `functionResponse`，后接用户文字；旧转换返回 422。现将完整工具结果后面的文字保留为独立 Chat `user` 消息，顺序不变；工具结果尚未齐全时插入文字仍拒绝。捕获的真实请求转换已由 422 变为通过，交错顺序仍为 422。
- 没有通过替换会话绕过恢复失败。修复后继续原失败会话 `ses_1cf3f2d82001tiUMUcLrYHpKuO`，恢复前后原生 ACP 会话 ID 和记录 ID 均为 `6e43b46d-b517-41ad-b541-ccaa54b90b23`。
- 中间一轮虽取得 `chat.done`，模型仅文字宣称已创建，实际文件为 0，未计成功。随后在同一会话明确要求实际调用写入工具，最终取得下述真实产物。

## 最终实测证据

- 最终请求 HTTP 200、curl 0、115.606537 秒，终态 `chat.done`，无 `chat.error`。事件：`chat.context_loaded` ×1、`chat.step_start` ×1、`chat.delta` ×23、`chat.tool` ×46、`chat.session_updated` ×2、`chat.step_finish` ×1、`chat.done` ×1。
- 带相同租户及工作目录头读取 `ui/summary`、`acp-options` 均为 HTTP 200、curl 0：绑定 `acp-gemini / gemini`、`full_access=true`，选定模型与请求一致。请求结束后 `connected=false`；这不代表上述真实执行失败。
- 产物目录 `/Users/xiangmin/worker/code/ruejs/acpdemo/gemini/` 实际新增 6 个文件：`index.html`、`vite.config.mjs`、`README.md`、`src/main.tsx`、`src/App.tsx`、`src/style.css`。源码导入本仓库 `@rue-js/rue`，包含输入处理、响应式统计、`localStorage` 保存与恢复逻辑；Vite 配置复用仓库 Rue 插件和别名，README 提供运行说明。仅做文件及源码核对，未验证浏览器行为。
- 请求前后比较 Rue.js 仓库普通文件 SHA-256（排除 `.git`、`node_modules`、`target`）：只有上述 6 个文件新增，目标目录外无文件内容变化。
- 聚焦集成回归 `cargo test -p vw-cli -p vw-agent --test acp_gemini_chat --features vw-agent/default` 退出码 0：1 个父测试通过，辅助用例由父测试在私有子进程执行。覆盖流式/非流式工具续接、实际模型 ID、工具关联、后续文字保留及拒绝交错顺序。`cargo build -p vw-cli --bin shiju --features vw-agent/default` 退出码 0。
- 验证使用隔离工作树；6 个 Gemini 相关源码/清单/测试文件与主工作区逐字一致。主工作区原有 `worktree/cleanup.rs` SHA-256 格式化错误、文件树测试未使用代码及 cron 测试缺字段阻塞了构建/完整 Clippy；未修改这些无关文件。隔离完整 Clippy 又被既有 `revert_tests.rs` 缺少 `fork_execution_started` 初始化阻塞，明确错误后停止（退出码 130）。最终代码完成后再次执行 `cargo clippy --all-targets --all-features -- -D warnings`，在编辑器创建成功时按用户要求停止（退出码 -2）；完整 Clippy 不记为通过。
- Cargo 使用共享槽位 8，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-8`。预设 `acp-gemini` 原本及结束时均 enabled=true，无需恢复配置。临时测试网关已停止，未启动编辑器测试服务。
- 最终请求、SSE 与文件基线保存在本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/gemini-acceptance-uf1d_tm7/`；修复、编译及回归证据在 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-gemini-fix-tg44jvml/`，未提交凭据或配置环境值。

在 Rue.js 仓库根目录启动编辑器：

```sh
pnpm exec vite --config acpdemo/gemini/vite.config.mjs
```

后续建议：将原网关重启到包含这些修复的构建；其他任务的编译问题处理完成后再补跑完整 Clippy。

# OpenClaw 验收

2026-11-12：**编辑器创建成功**。按用户本轮要求，以实际创建编辑器文件为验收终点；未启动页面、未运行浏览器行为验证、构建或 Cargo 检查，不将这些检查记为通过。

- 网关 `127.0.0.1:42617`；沿用最近验收租户 `10001 / 100010005`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`；`acp-openclaw / openclaw`、`session_model`、`full_access=true`。未更换模型、账号或引擎。
- 系统安装 HTTP 200、curl 0、9.005154 秒，安装状态 false → true。解析后的可执行文件为 `/Users/xiangmin/.local/share/shiju/acp-runtime/openclaw-2026.10.10-model-source-v1/node_modules/.bin/openclaw`，参数 `acp`；版本命令退出码 0，输出 `OpenClaw 2026.10.10 (7dc3350)`。
- 独立诊断 HTTP 200、curl 0，结果 `handshake_failed`；模型来源接口 HTTP 200、curl 0，`session_model` supported，已审查版本 `2026.10.10`。真实会话随后确认连接并完成工具执行，独立诊断结果与真实会话分别记录。
- 真实流式请求 HTTP 200、curl 0、115.922140 秒，终态 `chat.done`，无 `chat.error`。事件：`chat.context_loaded` ×1、`chat.step_start` ×1、`chat.delta` ×3、`chat.tool` ×58、`chat.session_updated` ×2、`chat.step_finish` ×1、`chat.done` ×1。请求明确要求实际调用工具创建文件，完成后简短回复，不运行额外测试。
- 会话 `ses_1cfeffacf001qs3sQHwZr6e2An`：执行期间带相同租户和工作目录头读取 `ui/summary` 与 `acp-options` 均为 HTTP 200、curl 0，绑定 `acp-openclaw / openclaw`、`full_access=true`、`connected=true`，选定模型与请求一致。结束后复查上述接口及预设接口时网关已无法连接（curl 7、HTTP 000），未将旧响应记为结束后的复查结果。
- 实际产物位于 `/Users/xiangmin/worker/code/ruejs/acpdemo/openclaw/`，共 8 个文件：`index.html`、`vite.config.mjs`、`tsconfig.json`、`README.md`、`src/main.tsx`、`src/App.tsx`、`src/env.d.ts`、`src/style.css`。源码实际导入本仓库 `@rue-js/rue`，包含输入处理、响应式统计、`localStorage` 保存与恢复；Vite 配置复用仓库 Rue 插件和别名，README 提供运行说明。仅核对文件和源码，未验证运行行为。
- 请求前后比较 Rue.js 仓库普通文件 SHA-256（排除 `.git`、`node_modules`、`target`），仅上述 8 个文件新增，目标目录外无文件内容变化。
- `acp-openclaw` 预设基线 enabled=true，本轮未修改预设或配置，无需恢复；结束复查因网关不可连接未完成。未修改 Rust，未启动编辑器测试服务。
- 原始请求、SSE、配置快照和文件基线仅保存在本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/openclaw-acceptance-3oqah_qx/`，未提交凭据或配置环境值。

在 Rue.js 仓库根目录启动编辑器：

```sh
pnpm exec vite --config acpdemo/openclaw/vite.config.mjs
```

# Auggie CLI 验收

2026-11-12：**编辑器创建成功**。按用户本轮要求，以实际创建编辑器文件为验收终点；未启动页面、未执行编辑器构建或浏览器行为测试，不将这些检查记为通过。

- 租户 `10001 / 100010005`；模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`；`acp-auggie / auggie`、`session_model`、`full_access=true`。未切换模型、账号或代理引擎。
- 原生 Auggie 0.37.1 没有可用的 OpenAI 兼容配置入口，原驱动为拒绝占位。本轮为该固定版本增加受管补丁：在原生聊天入口转换文本、工作目录、推理及工具历史到本机会话中继，返回原生节点，保留 Auggie 自身 ACP、文件工具与会话历史。模型目录只包含所选模型，不回退官方模型；未知内容、错误/截断流及未匹配的工具结果明确拒绝。会话模型模式关闭远程代码索引，桌面原生工具要求当前完全访问授权。
- 安装器校验 npm 归档完整性、原始 bundle SHA-256 和补丁后文件 SHA-256，安装在独立目录，不修改全局 CLI。子进程隔离继承的认证、Node 注入和代理环境；短期中继令牌仅进入私有配置和子进程环境，上游密钥不交给 CLI。
- 使用现有配置启动临时修复版网关 `127.0.0.1:50928`，用户原 42617 实例未重启。安装接口 HTTP 200、curl 0、0.682955 秒，安装前后均 true；解析后的入口为 `/Users/xiangmin/.shiju-harness-dev/acp-runtime/auggie-0.37.1-model-source-v1/node_modules/.bin/auggie`，版本命令退出码 0，`0.37.1 (commit e8d38687)`。
- 独立诊断 HTTP 200、curl 0、connected；模型来源接口 HTTP 200、curl 0，`session_model` supported，已审查版本 `0.37.1`，平台 Darwin arm64。
- 真实流式请求 HTTP 200、curl 0、72.391644 秒，终态 `chat.done`，无 `chat.error`。事件：`chat.context_loaded` ×1、`chat.step_start` ×1、`chat.delta` ×17、`chat.tool` ×36、`chat.session_updated` ×2、`chat.step_finish` ×1、`chat.done` ×1。请求明确要求实际调用工具写入文件，完成后简短回复，不运行额外测试。
- 会话 `ses_1d00a47a5001aELzGqN4562kBv`：执行期间与结束后，带相同租户和工作目录头读取 ui/summary、acp-options 均 HTTP 200、curl 0。绑定 acp-auggie / auggie、full_access=true；执行期间 connected=true，选定模型与请求一致。结束后 connected=false。
- 产物 `/Users/xiangmin/worker/code/ruejs/acpdemo/auggie/` 下实际新增 7 个文件：`index.html`、`vite.config.mjs`、`README.md`、`src/main.tsx`、`src/App.tsx`、`src/env.d.ts`、`src/style.css`。源码导入本仓库 `@rue-js/rue`，包含输入、响应式字数统计、localStorage 保存和恢复逻辑；Vite 复用仓库 Rue 插件及别名，README 提供运行说明。只做文件与源码核对，未验证运行行为。
- 请求前后比较 Rue.js 仓库普通文件 SHA-256（排除 .git、node_modules、target）：仅上述 7 个文件新增，目标目录外无文件内容变化。
- 创建验收前，`node --test patches/acp-model-source/auggie/session-model.test.mjs` 退出码 0，5 个测试通过；`python3 scripts/acp_model_source_auggie_tests.py <受管 augment.mjs>` 退出码 0，真实原生 ACP 完成指定模型选择、文件工具写入、推理历史续接及中文流回传。独立受管安装脚本退出码 0。
- `cargo build -p vw-cli --bin shiju --features vw-agent/default` 退出码 0。`cargo clippy --all-targets --all-features -- -D warnings` 退出码 101，被工作区无关的 `vw-desktop` worktree 未使用代码阻塞：`app/message/settings/worktrees.rs::apply`、`app/state/settings/worktrees.rs::current` 及 generation/identity 字段；未修改这些无关文件。完整 Clippy 不计为通过，未额外执行 Rust 单元测试。
- Cargo 使用共享槽位 1，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。acp-auggie 基线 enabled=false，临时启用与恢复均 HTTP 200、curl 0，结束复查 false。临时网关已停止，槽位已释放并保留缓存。
- 原始请求、SSE、配置与文件基线保存在私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/auggie-driver-ftgk74ve/acceptance/`；补丁探测、安装与构建记录在 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/auggie-driver-ftgk74ve/`。未提交凭据或配置环境值。

在 Rue.js 仓库根目录启动编辑器：

```sh
pnpm exec vite --config acpdemo/auggie/vite.config.mjs
```

后续建议：将常用网关重启到包含本次驱动与受管安装器的构建；工作区无关错误修复后补跑完整 Clippy。

# CodeBuddy 验收

2026-11-12：**编辑器创建成功**。按用户本轮要求，以实际创建编辑器文件为验收终点；未启动页面、未执行构建、浏览器行为测试或 Cargo 检查，不将这些检查记为通过。

- 原端口 42617 不可连接；使用现有 `target/rust-slot-1/debug/shiju` 构建及现有开发配置启动临时网关 `127.0.0.1:57723`，本轮未修改 Rust 或重新编译。
- 沿用最近验收租户 `10001 / 100010005`；指定模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`；`acp-codebuddy / codebuddy`、`session_model`、`full_access=true`。未更换模型、账号或引擎。
- 系统安装 HTTP 200、curl 0、5.899198 秒，安装状态 false → true。入口为 `/Users/xiangmin/.shiju-harness-dev/acp-runtime/codebuddy-2.179.3-model-source-v1/node_modules/.bin/codebuddy`；版本命令退出码 0，版本 `2.179.3`。模型来源接口 HTTP 200、curl 0，session_model supported，已审查版本 2.179.3，平台 Darwin arm64。
- 安装后全局解析配置的 args 为空，独立诊断 HTTP 200、curl 0、timed_out；首轮真实请求明确拒绝 `ACP codebuddy custom launch arguments has not been verified`，未创建文件。驱动仅接受 `--acp`，随后通过配置接口将当前租户的 codebuddy 参数显式设为 `["--acp"]`，保留受管入口及环境配置，HTTP 200、curl 0。该有效租户配置保留；未削弱驱动校验。全局 ACP 列表不读取租户覆盖，不能用该列表的空参数替代真实会话结果。
- 重试真实请求 HTTP 200、curl 0、96.792175 秒，终态 `chat.done`，无 `chat.error`。事件数量：`{"chat.context_loaded": 1, "chat.step_start": 1, "chat.delta": 145, "chat.tool": 286, "chat.session_updated": 2, "chat.step_finish": 1, "chat.done": 1}`。请求要求实际调用工具创建文件，完成后简短回复，不执行额外测试。
- 会话 `ses_1d017ac36001ZQLQX43qNO4dVY`：执行期间及结束后读取 ui/summary、acp-options 均 HTTP 200；绑定 acp-codebuddy / codebuddy，full_access=true，selected_model 与请求一致。执行期间 connected=true，结束后 connected=false。访问状态同时报告原生权限支持 partial；未将 full_access 请求标志解释为原生权限完全支持。
- 产物 `/Users/xiangmin/worker/code/ruejs/acpdemo/codebuddy/` 实际新增 8 个文件：`index.html`、`vite.config.mjs`、`tsconfig.json`、`README.md`、`src/main.tsx`、`src/App.tsx`、`src/env.d.ts`、`src/style.css`。源码导入本仓库 `@rue-js/rue`，包含输入、computed 字数统计、localStorage 保存与恢复；Vite 复用仓库 Rue 插件及别名，README 提供运行说明。仅核对文件与源码，未验证页面运行行为。
- 请求前后比较 Rue.js 仓库普通文件 SHA-256（排除 .git、node_modules、target），仅上述 8 个文件新增，目标目录外无文件内容变化。
- acp-codebuddy 基线 enabled=false，临时启用 HTTP 200、curl 0；恢复 HTTP 200，结束复查 enabled=false。临时网关已停止，未启动编辑器服务。
- 原始请求、SSE、配置及文件基线仅保存在本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/codebuddy-acceptance-nslscvwx/`，未提交凭据或环境值。

在 Rue.js 仓库根目录启动编辑器：

```sh
pnpm exec vite --config acpdemo/codebuddy/vite.config.mjs
```

# Cursor CLI 验收

2026-11-12：**编辑器创建成功**。按用户本轮要求，以实际创建编辑器文件为验收终点；仅核对文件、源码及运行说明，未启动页面、未执行编辑器构建或浏览器行为测试，不将这些检查记为通过。

- 沿用租户 `10001 / 100010005`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`；`acp-cursor / cursor`、`session_model`、`full_access=true`。未切换模型、账号或代理引擎。
- 原驱动明确拒绝 session_model。现对固定 Cursor `2026.11.11-b912975` 增加独立受管补丁，在原生 AgentClient.run 边界接入 Chat 推理和工具续接，保留 Cursor ACP、模型选择、原生 shell 解析器、权限审批/执行器与 protobuf/blob 会话历史。仅公布前台 shell 工具用于读写文件；未接入的远程服务、MCP、图片、非 agent 模式、总结历史、并行/未知工具及异常或截断流明确拒绝，不宣称支持完整远程工具集。补丁边界见 `patches/acp-model-source/cursor/README.md`。
- 官方固定归档、原始补丁锚点与全部 351 个最终运行文件均做 SHA-256 校验；不修改全局 Cursor CLI。受管子进程不继承云认证、Node 注入及代理环境，使用独立配置/历史目录和内存凭据存储。私有 0600 配置仅包含本机会话中继与可撤销短期令牌，上游密钥不交给 Cursor；未添加 force/yolo 或跳过审批。
- 使用现有开发配置及本轮修复构建启动临时网关 `127.0.0.1:62290`；原 42617 实例未被重启。系统安装 HTTP 200、curl 0、24.492262 秒，安装状态 true → true；前者来自原全局安装，后者已解析到受管入口 `/Users/xiangmin/.shiju-harness-dev/acp-runtime/cursor-2026.11.11-b912975-model-source-v1/dist-package/cursor-agent`，参数 `acp`。版本命令退出码 0，输出 `2026.11.11-b912975`。
- 独立诊断 HTTP 200、curl 0、1.530356 秒，结果 authentication_required；模型来源接口 HTTP 200、curl 0，session_model supported，已审查版本 `2026.11.11-b912975`，平台 Darwin arm64。真实连接与工具执行另由以下会话核实。
- 真实流式请求 HTTP 200、curl 0、71.934220 秒，终态 `chat.done`，无 `chat.error`。事件：chat.context_loaded ×1、chat.step_start ×1、chat.delta ×103、chat.tool ×18、chat.session_updated ×2、chat.step_finish ×1、chat.done ×1。请求要求实际调用工具创建文件，完成后简短回复，不运行额外测试。
- 会话 `ses_1d0506831001wpW2h6VepGAadt`：执行期间及结束后，带相同租户及工作目录头读取 ui/summary 与 acp-options 均 HTTP 200、curl 0；绑定 acp-cursor / cursor，full_access=true，选定模型与请求一致。执行期间 connected=true，结束后 connected=false。
- 产物 `/Users/xiangmin/worker/code/ruejs/acpdemo/cursor/` 实际新增 7 个文件：`index.html`、`vite.config.mjs`、`README.md`、`src/main.tsx`、`src/App.tsx`、`src/env.d.ts`、`src/style.css`。源码导入本仓库 `@rue-js/rue`，包含输入处理、computed 字数统计、localStorage 防抖保存与初始化恢复；Vite 复用仓库 Rue 插件及别名，README 提供运行说明。仅确认这些逻辑存在，未验证页面运行行为。
- 请求前后比较 Rue.js 仓库普通文件 SHA-256（排除 .git、node_modules、target），仅上述 7 个文件新增，目标目录外无文件内容变化。
- 驱动验证：`node --test patches/acp-model-source/cursor/session-model.test.cjs` 退出码 0，4 个测试通过。`python3 scripts/acp_model_source_cursor_tests.py <受管入口>` 及追加 `--deny` 的回合均退出码 0，真实原生 ACP 验证选定模型、中文流、推理/工具结果续接、session/load 后历史恢复，以及 allow_once 实际写入、reject_once 不写入。独立受管安装脚本退出码 0；未用这些本地 HTTP fixture 代替真实模型验收。
- `cargo build -p vw-cli --bin shiju --features vw-agent/default` 退出码 0；最后一次增量构建耗时 9 分 26 秒，运行前逐字核对二进制内嵌补丁、安装器及校验清单与当前源码一致。`cargo clippy --all-targets --all-features -- -D warnings` 退出码 101，被工作区既有 vw-desktop worktree 的未使用 import、函数和字段阻塞（包括 open_existing_checkout、ManagedWorktreesState 等）；未修改这些无关文件，完整 Clippy 不计为通过。未额外执行 Rust 单元测试。
- Cargo 使用共享槽位 6，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-6`；槽位已释放、缓存保留。acp-cursor 基线 enabled=false，临时启用与恢复均 HTTP 200、curl 0，结束复查 false。临时网关已停止，未启动编辑器测试服务。
- 原始请求、SSE、配置及文件基线仅保存在私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/cursor-driver-vxho34f5/acceptance/`；补丁探测、安装与构建记录在 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/cursor-driver-vxho34f5/`，未提交凭据或配置环境值。

在 Rue.js 仓库根目录启动编辑器：

```sh
pnpm exec vite --config acpdemo/cursor/vite.config.mjs
```

后续建议：将常用网关重启到包含本次驱动与受管安装器的构建；无关编译问题处理完成后补跑完整 Clippy。

# Factory Droid 验收

日期：2026-11-12。按最新授权，以指定模型实际创建编辑器文件为本项终点，不执行浏览器或编辑器构建测试。

- 固定 Factory Droid 1.9.0 / Darwin arm64 官方制品，SHA-256 为 `b62ce8de0ed7ffaada1abb2a017fe182021003694e85fe7f3b1136ad43628a5d`。使用原生 `exec --output-format acp`、Airgap 与 BYOK，不修改供应商二进制，不替换模型、账号或引擎。模式依据：[Airgapped Deployment](https://docs.factory.com/enterprise/airgapped-deployment)、[Custom Models](https://docs.factory.com/model-independence/byok)；固定制品实测确认环境入口及 ACP 会话只公布隔离自定义模型。
- 原普通安装请求 HTTP 408、curl 0、30.005419 秒；直接执行同一官方安装器并通过其校验后安装原生 1.9.0。必要底层修复将 ACP 安装路由的外层时限延至 310 秒，覆盖安装器既有的 300 秒期限。修复版系统安装 HTTP 200、curl 0、30.657898 秒；本次基线已发现原生 CLI，安装后解析到受管制品，安装状态 true → true。受管路径：`/Users/xiangmin/.shiju-harness-dev/acp-runtime/droid-1.9.0-model-source-v1/droid`；版本探测退出码 0，输出 `1.9.0`。
- 普通 CLI 诊断 HTTP 200、curl 0、2.113923 秒，返回 `authentication_required`，不将其记为已登录。隔离 Airgap 运行时实测 initialize / session/new 成功；真实请求的 `acp-options` 确认 connected=true、版本 1.9.0、来源 session_model，选定模型为 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，provider 为 `alibaba-token-plan-cn`。
- 使用本次构建的独立副本网关 `127.0.0.1:53624`，租户 `10001 / 100010005`，`acp-droid / droid`、session_model、full_access=true。会话 `ses_1d06de20f001XRsXQQZ5yiRurg` 的 summary 确认绑定及完全访问。未重启用户原 42617 服务。
- 配置根权限 0700、运行设置权限 0600；推理地址仅指向本机会话中继，认证使用短期令牌；不将上游密钥交给 Droid，不修改 HOME 或全局 Factory 配置。子进程不继承云认证、Node/Bun 注入或代理凭据。原生 ACP 默认会自动执行部分工具，因此桌面入口要求当前完全访问授权；不注入跳过审批参数。
- 真实请求 HTTP 200；180.001459 秒后 curl 28。已收到指定模型的 `chat.step_finish`、finish_reason=stop；事件计数为 context_loaded 1、step_start 1、delta 19、tool 78、session_updated 1、step_finish 1，无 chat.error。连接没有在客户端期限内关闭，未收到 chat.done；这项传输限制单独保留，不表述为 curl 成功。按最新创建文件终点完成验收，不续传、不另跑编辑器测试。
- 模型实际创建 `acpdemo/droid/index.html`、`vite.config.mjs`、`tsconfig.json`、`README.md`、`src/main.tsx`、`src/env.d.ts`、`src/App.tsx`、`src/style.css`，共 8 个文件。App.tsx 引用 `@rue-js/rue` 并包含输入、计数、localStorage 持久化逻辑。请求前后 SHA-256 快照（排除 .git、node_modules、target）只发现这 8 个新增文件，覆盖的 Rue 仓库内容未发现目标目录外变更；未宣称浏览器行为已验证。
- acp-droid 基线 enabled=false；临时启用和恢复均 HTTP 200、curl 0，结束复查 false。验收网关及其子进程已停止。原始响应、配置及完整文件快照仅存于本机私有临时目录，未提交。
- 验证：`cargo clippy --all-targets --all-features -- -D warnings` 退出码 0；`cargo build -p vw-cli --bin shiju --features vw-agent/default` 退出码 0，二者使用槽位 8，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-8`。`cargo test -p vw-acp --lib droid_native_tools_require_current_full_access` 使用槽位 7、对应绝对目录 `/Users/xiangmin/worker/code/shiju/target/rust-slot-7`，退出码 0、1 项通过。驱动聚焦回归在编译期间按用户“创建成功即停止”的最新指示终止，未取得结果；超时/协议聚焦回归及第二轮 Clippy 未运行，不记通过。额外回归进程及本任务槽位锁已清理，缓存保留。


# Goose 验收

2026-11-13：**指定模型实际创建编辑器成功**。按最新授权，以编辑器文件创建为本项终点；未执行浏览器行为、编辑器构建或额外回归，不将这些检查记为通过。

- 批次 11；前一批 Droid 已结束并归档。沿用租户 `10001 / 100010005`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，`acp-goose / goose`、`session_model`、`full_access=true`，未更换账号、模型或引擎。
- 使用前一批已验证 CLI 副本与现有开发配置启动独立网关 `127.0.0.1:57480`；未重启用户原服务。本项未修改 Rust、安装器或模型补丁，复用现有固定 Goose v1.57.0 源码与模型锁定补丁。
- 系统安装 HTTP 200、curl 0、234.038691 秒；安装状态 false → true。安装器核对固定源码及补丁 SHA-256，真实编译上游 Goose CLI 后生成并校验构建回执。解析命令为 `/Users/xiangmin/.shiju-harness-dev/acp-runtime/goose-v1.57.0-model-source-v1/goose acp`；版本命令退出码 0，输出 1.57.0。
- 安装器执行 `cargo build -p goose-cli --bin goose --no-default-features --features rustls-tls,bundled-mcp --locked` 成功；使用共享槽位 9，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-9`。本项无仓库代码变更，未运行仓库 Clippy 或单元测试；外部 CLI 编译成功不代表这些检查通过。
- 独立诊断 HTTP 200、curl 0、0.469719 秒，结果 authentication_required；模型来源 HTTP 200、curl 0，session_model supported、已验证版本 1.57.0。真实会话另行确认 connected=true，不用独立诊断代替托管模型验收。
- 真实请求 HTTP 200、curl 0、97.107318 秒，终态 chat.done，无 chat.error；事件为 chat.context_loaded ×1、chat.step_start ×1、chat.delta ×68、chat.tool ×38、chat.session_updated ×2、chat.step_finish ×1、chat.done ×1。提示要求实际创建文件，完成后简短结束、不运行额外测试。
- 会话 `ses_1d07cdd690015fbHACHvVNztYn` 的 ui/summary 与 acp-options 均 HTTP 200、curl 0；确认绑定 agent=acp-goose、acp_agent=goose、full_access=true，执行期间 connected=true，选定模型来源及模型与请求一致。客户端访问能力为 client_only，不将其表述为后端原生免审批已验证。
- 目标目录原先不存在。模型实际创建 `acpdemo/goose/` 下 `index.html`、`vite.config.mjs`、`tsconfig.json`、`README.md`、`src/main.tsx`、`src/env.d.ts`、`src/App.tsx`、`src/style.css`，共 8 个文件。源码引用本仓库 `@rue-js/rue`，包含输入、响应式计数与 localStorage 保存恢复逻辑，并提供运行说明；未宣称浏览器行为已验证。
- Rue 仓库请求前后普通文件 SHA-256 快照（排除 .git、node_modules、target）仅发现上述 8 个新增文件，目标目录外无文件内容变化。acp-goose 基线 enabled=false，临时启用与恢复均 HTTP 200、curl 0；结束复查 false。
- 独立网关及其会话子进程已停止，槽位 9 已释放，缓存保留。原始请求、SSE、配置、安装资产与文件快照仅存于本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-goose-t5og8vpt/`，未提交凭据或配置环境值。

在 Rue.js 仓库根目录启动：

```sh
pnpm exec vite --config acpdemo/goose/vite.config.mjs
```

# Hermes Agent 验收

2026-11-13：**指定模型实际创建编辑器成功**。按最新授权，以实际创建文件为终点；未执行浏览器行为、编辑器构建或额外回归，不将这些检查记为通过。

- 批次 12；批次 11 Goose 已结束并归档。沿用租户 `10001 / 100010005`、模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，`acp-hermes / hermes`、`session_model`、`full_access=true`，未替换模型、账号或引擎。
- 使用现有 `target/rust-slot-8/debug/shiju` 的独立副本及现有开发配置启动临时网关 `127.0.0.1:59651`；未重启用户原网关。本项未修改 Rust、安装器或补丁，未运行 Cargo、Clippy 或单元测试，复用现有受管 Hermes 驱动。
- 系统安装 HTTP 200、curl 0、6.024820 秒；安装状态 false → true。解析入口为 `/Users/xiangmin/.shiju-harness-dev/acp-runtime/hermes-0.19.0-model-source-v1/bin/hermes`，参数 `["acp"]`；版本命令退出码 0，`Hermes Agent v0.19.0 (2026.7.20)`，Python 3.12.16。
- 独立诊断 HTTP 200、curl 0、10.799680 秒，结果 timed_out；模型来源接口 HTTP 200、curl 0，session_model supported，已审查版本 0.19.0、Darwin arm64。独立诊断超时与真实会话连接分开记录。
- 会话 `ses_1d081b70b001VZxLXLi52CyNdT` 的 ui/summary、acp-options 均 HTTP 200、curl 0；绑定 acp-hermes / hermes，full_access=true，真实运行期间 connected=true，选定模型与来源均与请求一致。访问能力报告 partial，不将完全访问请求标志解释为所有原生工具权限均已验证。旧租户的完全访问拒绝在本次未复现，无需放宽权限校验。
- 真实请求 HTTP 200，180.006854 秒后 curl 28；事件为 chat.context_loaded ×1、chat.step_start ×1、chat.delta ×336、chat.tool ×43，无 chat.error，未收到 chat.step_finish 或 chat.done。指定模型已实际创建下列完整文件，按最新创建终点停止，不宣称流式请求正常结束。
- 目标目录原先不存在；实际新增 `acpdemo/hermes/` 下 `index.html`、`vite.config.mjs`、`tsconfig.json`、`README.md`、`src/main.tsx`、`src/env.d.ts`、`src/App.tsx`、`src/style.css`，共 8 个文件。源码引用本仓库 `@rue-js/rue`，包含输入处理、computed 字数统计及 localStorage 保存恢复逻辑；README 提供运行说明。仅核对文件与源码，未验证运行行为。
- 请求前与服务停止后的 Rue 仓库普通文件 SHA-256 快照（排除 .git、node_modules、target）只发现上述 8 个新增文件，目标目录外无文件内容变化。acp-hermes 基线 enabled=false，临时启用与恢复均 HTTP 200、curl 0，结束复查 false。临时网关进程组已停止，未启动编辑器服务。
- 原始请求、响应、配置与文件快照仅保存在本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-hermes-pqbeea1p`，未提交配置环境值或凭据。

在 Rue.js 仓库根目录启动：

```sh
pnpm exec vite --config acpdemo/hermes/vite.config.mjs
```

# iFlow 验收

2026-11-13：**指定模型实际创建编辑器成功**。批次 13，批次 12 Hermes 已结束归档。按最新授权，以创建完整编辑器文件为终点；未执行浏览器、编辑器构建或额外回归，不将这些检查记为通过。

- 沿用租户 `10001 / 100010005`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，`acp-iflow / iflow`、`session_model`、`full_access=true`；未更换模型、账号或引擎。
- 复用前一批已验证 CLI 副本和现有开发配置，启动独立网关 `127.0.0.1:61508`。本项未修改 Rust、安装器或补丁，未运行 Cargo、Clippy 或单元测试；复用现有受管 iFlow 驱动，无需新增补丁。
- 系统安装 HTTP 200、curl 0、1.538436 秒，安装状态 false → true。受管入口 `/Users/xiangmin/.shiju-harness-dev/acp-runtime/iflow-0.5.19-model-source-v1/node_modules/.bin/iflow`，参数 `["--experimental-acp"]`；版本命令退出码 0，输出 `0.5.19`。
- 独立诊断 HTTP 200、curl 0、1.542084 秒，结果 authentication_required；模型来源 HTTP 200、curl 0，session_model supported，已审查版本 0.5.19、Darwin arm64。普通诊断未登录不妨碍本次托管模型会话，真实连接另行核实。
- 会话 `ses_1d0873848001V6SUhXIJNRLmde` 的 ui/summary、acp-options 在执行期间及结束后均 HTTP 200、curl 0。执行期间 connected=true，绑定 acp-iflow / iflow，full_access=true，选定模型及来源与请求一致；原生权限能力为 partial，不将请求标志解释为所有原生权限均已验证。
- 真实请求 HTTP 200、curl 0、142.259781 秒，终态 chat.done，无 chat.error。事件：chat.context_loaded ×1、chat.step_start ×1、chat.delta ×10、chat.tool ×54、chat.session_updated ×2、chat.step_finish ×1、chat.done ×1。提示要求实际调用工具创建文件，完成后简短结束，不运行额外测试。
- 目标目录原先不存在，实际新增 `acpdemo/iflow/` 下 `index.html`、`vite.config.mjs`、`tsconfig.json`、`README.md`、`src/main.tsx`、`src/env.d.ts`、`src/App.tsx`、`src/style.css` 共 8 个文件。源码引用本仓库 `@rue-js/rue`，包含输入、computed 字数统计、localStorage 保存与恢复逻辑；README 提供运行说明。仅核对源码及文件，不宣称页面行为已验证。
- 请求前与网关停止后比较 Rue 仓库普通文件 SHA-256（排除 .git、node_modules、target），仅上述 8 个新增文件，未发现目标目录外内容变化。预设基线 enabled=false；临时启用和恢复均 HTTP 200、curl 0，结束复查 false。临时网关及其进程组已停止。
- 原始请求、配置、SSE 和文件快照仅存于本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-iflow-mrs32dq1/`，未提交凭据或配置环境值。

在 Rue.js 仓库根目录启动：

```sh
pnpm exec vite --config acpdemo/iflow/vite.config.mjs
```


# Kilocode 验收

2026-11-13：**指定模型实际创建编辑器成功**。批次 14，批次 13 iFlow 已结束归档。按最新授权，以创建完整编辑器文件为终点；未执行浏览器、编辑器构建或额外回归，不将这些检查记为通过。

- 沿用租户 `10001 / 100010005`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，`acp-kilocode / kilocode`、`session_model`、`full_access=true`；未更换模型、账号或引擎。
- 复用前一批已验证 CLI 副本与现有开发配置，启动独立网关 `127.0.0.1:62912`。本项未修改 Rust、安装器或补丁，未运行 Cargo、Clippy 或单元测试；复用现有受管 Kilocode 驱动，无需新增补丁。
- 系统安装 HTTP 200、curl 0、3.646785 秒，安装状态 false → true。入口 `/Users/xiangmin/.shiju-harness-dev/acp-runtime/kilocode-7.8.18-model-source-v1/node_modules/.bin/kilocode`，参数 `["acp", "--pure"]`；版本命令退出码 0，输出 `7.8.18`。
- 独立诊断 HTTP 200、curl 0、1.687216 秒，结果 authentication_required；模型来源 HTTP 200、curl 0，session_model supported，已审查版本 7.8.18、Darwin arm64。普通诊断未登录与真实托管模型连接分别记录。
- 会话 `ses_1d08be76a001MU4eZXB1wbLQxt` 的 ui/summary 和 acp-options 执行期间及结束后均 HTTP 200、curl 0；绑定 acp-kilocode / kilocode，full_access=true，执行期间 connected=true，选定模型及来源与请求一致。访问能力为 client_only，不将完全访问请求标志解释为后端原生免审批已验证。旧租户的完全访问拒绝未复现，无需放宽权限检查。
- 真实请求 HTTP 200、curl 0、122.864950 秒，终态 chat.done，无 chat.error。事件为 chat.context_loaded ×1、chat.step_start ×1、chat.delta ×155、chat.tool ×113、chat.session_updated ×2、chat.step_finish ×1、chat.done ×1。提示要求实际调用工具创建文件，完成后简短结束，不运行额外测试。
- 目标目录原先不存在；实际新增 `acpdemo/kilocode/` 下 `index.html`、`vite.config.mjs`、`tsconfig.json`、`README.md`、`src/main.tsx`、`src/env.d.ts`、`src/App.tsx`、`src/style.css` 共 8 个文件。源码引用本仓库 `@rue-js/rue`，包含输入、computed 字数统计、localStorage 防抖保存与初始化恢复逻辑；README 提供运行说明。仅核对文件与源码，不宣称页面行为已验证。
- 请求前与网关停止后比较 Rue 仓库普通文件 SHA-256（排除 .git、node_modules、target），仅上述 8 个新增文件，未发现目标目录外内容变化。预设基线 enabled=false；临时启用和恢复均 HTTP 200、curl 0，结束复查 false。临时网关及其进程组已停止。
- 原始请求、配置、SSE 与文件快照仅保存在本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-kilocode-5s99ctd4/`，未提交凭据或配置环境值。

在 Rue.js 仓库根目录启动：

```sh
pnpm exec vite --config acpdemo/kilocode/vite.config.mjs
```


# Kimi Code CLI 验收

2026-11-13：**指定模型实际创建编辑器成功**。批次 15，批次 14 Kilocode 已结束归档。按最新授权，以实际创建完整文件为终点；未执行浏览器、编辑器构建或额外回归，不将这些检查记为通过。

- 沿用租户 `10001 / 100010005`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，`acp-kimi / kimi`、`session_model`、`full_access=true`；未更换模型、账号或引擎。
- 复用前批已验证 CLI 副本与现有开发配置启动独立网关 `127.0.0.1:64195`。本项未修改 Rust、安装器或补丁，未运行 Cargo、Clippy 或单元测试；复用现有 Kimi 原生驱动，无需新增补丁。
- 系统安装 HTTP 200、curl 0、2.641986 秒；安装状态 true → true。解析命令 `kimi acp`，版本命令退出码 0，输出 `2.5.0`。独立诊断 HTTP 200、curl 0、1.390971 秒，结果 authentication_required；模型来源接口 HTTP 200、curl 0，session_model supported、已审查版本 2.5.0、Darwin arm64。普通诊断与真实托管模型连接分别记录。
- 会话 `ses_1d090fa67001ag9Tf7rDNMOfiJ` 的 ui/summary 与 acp-options 执行期间及结束后均 HTTP 200、curl 0；绑定 acp-kimi / kimi，full_access=true。启动阶段 connected=false，随后真实执行期间 connected=true，选定模型及来源均与请求一致。访问能力报告 partial，不将请求标志解释为所有原生权限均已验证。
- 真实请求 HTTP 200、curl 0、155.673370 秒，终态 chat.done，无 chat.error。事件为 chat.context_loaded ×1、chat.step_start ×1、chat.delta ×33、chat.tool ×1184、chat.session_updated ×2、chat.step_finish ×1、chat.done ×1。提示要求实际调用工具创建文件，完成后简短结束，不运行额外测试。旧租户 180 秒超时且无产物的问题本次未复现。
- 目标目录原先不存在；实际新增 `acpdemo/kimi/` 下 `index.html`、`vite.config.mjs`、`tsconfig.json`、`README.md`、`src/main.tsx`、`src/env.d.ts`、`src/App.tsx`、`src/style.css`，共 8 个文件。源码引用本仓库 `@rue-js/rue`，包含输入、computed 字数统计、localStorage 防抖保存与初始化恢复逻辑；README 提供运行说明。仅核对源码与文件，不宣称页面行为已验证。
- 请求前与网关停止后比较 Rue 仓库普通文件 SHA-256（排除 .git、node_modules、target），仅上述 8 个新增文件，未发现目标目录外内容变化。预设基线 enabled=false；临时启用 HTTP 200、curl 0，恢复 HTTP 200、curl 0，结束复查 false。临时网关进程组已停止。
- 原始请求、配置、SSE 与文件快照仅存于本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-kimi-lxuf9smm/`，未提交配置环境值或凭据。

在 Rue.js 仓库根目录启动：

```sh
pnpm exec vite --config acpdemo/kimi/vite.config.mjs
```

# Kiro 验收

2026-11-13：**指定模型实际创建编辑器成功**。批次 16，批次 15 Kimi 已结束归档。按最新授权，以完整编辑器文件及运行说明实际落盘为终点；未执行浏览器、编辑器构建或创建后的额外回归，不将这些检查记为通过。

- 沿用租户 `10001 / 100010005`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，`acp-kiro / kiro`、`session_model`、`full_access=true`；未更换模型、账号或引擎。
- 原 42617 未监听，使用现有开发配置启动独立网关 `127.0.0.1:49344`。最初复用的旧槽位 1 CLI 安装 HTTP 408、curl 0、30.073425 秒；换用前批含安装超时修复的 CLI 副本后，系统安装 HTTP 200、curl 0、28.699284 秒，状态 false → true。解析入口 `/Users/xiangmin/.shiju-harness-dev/acp-runtime/kiro-3.0.0-model-source-v1/kiro-cli acp`，版本命令退出码 0、`kiro-cli 3.0.0`。独立诊断 HTTP 200、curl 0、0.260415 秒，结果 authentication_required；模型来源 session_model supported，已审查 Darwin arm64 / 3.0.0。普通诊断不代表托管模型连接结果。
- 实际旧构建会话 HTTP 200、chat.error，内部 HTTP 422，无文件。根因为 Kiro 的 GetFeatureConfiguration 有 5000ms 时限，而中继先解析 provider/model，超时后原生引擎采用默认 GenerateAssistantResponse 旧协议。相同会话的最小文本及工具 Responses 探针均 HTTP 200，排除了指定模型本身不可用。修复仅让有效会话令牌校验后的固定版本静态协议标志直接返回；模型目录、推理的模型状态及原有授权检查保持不变。
- 使用共享槽位 8 构建当前源码 CLI，`cargo build -p vw-cli --bin shiju --features vw-agent/default` 退出码 0、6 分 55 秒。CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-8`。45 个中继协议源码及运行时清单哈希在构建期间未变化；实际运行独立 CLI 副本，SHA-256 `ddaeacf1128d6da1e03fe914f0227fc278a2db9bc5956b027e2254c550647255`。本轮未修改 Kiro 引擎、受管补丁或安装器。
- 新构建安装接口 HTTP 200、curl 0、0.273043 秒，状态 true → true。真实原生日志记录 ExperimentConfigService Resolved、keyCount=1，随后 q.converse.dispatch 使用 CreateResponse；旧协议回退及 HTTP 422 不再出现。
- 会话 `ses_1d0b802c7001w1GlLIErX0sQb2` 的 ui/summary 与 acp-options 执行期间及请求后均 HTTP 200、curl 0。绑定 acp-kiro / kiro，full_access=true，执行期间 connected=true，选定模型与来源均与请求一致。访问能力报告 partial，不将请求标志解释为所有原生权限均已验证。
- 真实创建请求 HTTP 200、curl 28、180.002985 秒，客户端达到原 180 秒期限；未收到 chat.done、chat.error 或最终结束事件，不能宣称流式请求正常结束。事件为 chat.context_loaded ×1、chat.step_start ×1、chat.delta ×9、chat.tool ×133。超时前原生工具已实际创建完整文件；依照本轮“实际创建即停止”终点验收，不重试或追加回归。
- 目标目录原先不存在；实际新增 `acpdemo/kiro/` 下 `index.html`、`vite.config.mjs`、`tsconfig.json`、`README.md`、`src/main.tsx`、`src/env.d.ts`、`src/App.tsx`、`src/style.css`，共 8 个文件。源码导入本仓库 `@rue-js/rue`，包含输入、computed 实时字数/字符/行数、localStorage 防抖保存与初始化恢复逻辑；README 提供运行说明。仅核对文件与源码，不宣称页面行为已验证。
- 请求前与网关停止后比较 Rue 仓库普通文件 SHA-256（排除 .git、node_modules、target），仅上述 8 个新增文件，目标目录外无内容变化。预设基线 enabled=false；本次启用 HTTP 200、curl 0、9.168435 秒，恢复 HTTP 200、curl 0、8.689470 秒，结束复查 false。临时网关已停止，槽位 8 已释放、缓存保留。
- `cargo clippy --all-targets --all-features -- -D warnings` 退出码 101，仅报告共享工作区 `crates/vw-desktop/src/app/state/settings/cron_frequency.rs:14` 的 `CronFrequency::ALL` dead_code；未修改无关模块，不计完整 Clippy 通过。新增聚焦回归覆盖静态标志无需模型目录、无令牌/撤销令牌拒绝及缺失模型目录拒绝；`cargo test -p vw-agent --lib kiro_protocol_flag_is_local_but_catalog_still_requires_selected_model` 在大型 lib test 编译阶段终止以优先当前构建与真实验收，未执行用例、未取得通过结果。创建后未补跑回归。
- 原始请求、SSE、配置、探针与文件快照仅存于本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-kiro-7k_kt464/`，未提交凭据、环境值或敏感载荷。

在 Rue.js 仓库根目录启动：

```sh
pnpm exec vite --config acpdemo/kiro/vite.config.mjs
```

后续建议：常用网关使用包含本次修复的构建；处理无关 Clippy 错误后补跑该检查与未完成的聚焦回归。


# PI 验收

2026-11-13：**指定模型实际创建编辑器成功**。批次 17，批次 16 Kiro 已结束归档。按最新授权，以完整文件及运行说明实际落盘为终点；未执行浏览器、编辑器构建或创建后的额外回归，不将这些检查记为通过。

- 沿用租户 `10001 / 100010005`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，`acp-pi / pi`、`session_model`、`full_access=true`；未替换模型、账号或引擎。
- 使用批次 16 已验证的 CLI 副本和现有开发配置启动独立网关 `127.0.0.1:58747`；未重启用户原服务。本项未修改 Rust、安装器或补丁，未运行 Cargo、Clippy 或单元测试；复用现有受管 PI 驱动，无需新增补丁。
- 系统安装 HTTP 200、curl 0、7.362939 秒，安装状态 false → true。受管入口 `/Users/xiangmin/.shiju-harness-dev/acp-runtime/pi-0.0.22-model-source-v1/node_modules/.bin/pi-acp`，无启动参数。`--version` 退出码 0 但无输出；独立核对安装包 package.json，pi-acp 为 0.0.22，@mariozechner/pi-coding-agent 为 0.73.1，不把空版本输出记为版本证据。
- 独立诊断 HTTP 200、curl 0、0.355479 秒，结果 authentication_required；模型来源接口 HTTP 200、curl 0，session_model supported，已审查版本 0.0.22 (Pi 0.73.1)、Darwin arm64。普通诊断与真实托管模型连接分别记录。
- 会话 `ses_1d0bef06e001OJfz2YUT84fXDY` 的 ui/summary 与 acp-options 在执行期间及结束后均 HTTP 200、curl 0（查询需带相同租户与 X-Directory，最初遗漏目录头的 summary 返回 404，补齐后成功）。确认绑定 acp-pi / pi，full_access=true，执行期间 connected=true，选定模型及来源与请求一致。访问能力报告 partial，不将请求标志解释为所有原生权限均已验证。旧租户的完全访问授权拒绝未复现，无需放宽权限检查。
- 真实创建请求 HTTP 200、curl 0、133.633731 秒，终态 chat.done，无 chat.error。事件为 chat.context_loaded ×1、chat.step_start ×1、chat.tool ×1810、chat.delta ×15、chat.session_updated ×2、chat.step_finish ×1、chat.done ×1。提示要求实际调用工具创建文件，完成后简短结束，不运行额外测试。
- 目标目录原先不存在，实际新增 `acpdemo/pi/` 下 `index.html`、`vite.config.mjs`、`tsconfig.json`、`README.md`、`src/main.tsx`、`src/env.d.ts`、`src/App.tsx`、`src/style.css`，共 8 个文件。源码导入本仓库 `@rue-js/rue`，包含输入处理、computed 实时字数统计、localStorage 防抖保存与初始化恢复逻辑；README 提供运行说明。仅核对文件与源码，不宣称页面行为已验证。
- 请求前与网关停止后比较 Rue 仓库普通文件 SHA-256（排除 .git、node_modules、target），仅上述 8 个文件新增，目标目录外无内容变化。预设基线 enabled=false；临时启用 HTTP 200、curl 0、8.095596 秒，恢复 HTTP 200、curl 0、7.407091 秒，结束复查 false。临时网关进程组已停止，未启动编辑器服务。
- 原始请求、SSE、配置、安装版本与文件快照仅存于本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-pi-257t9d9u/`，未提交凭据或配置环境值。

在 Rue.js 仓库根目录启动：

```sh
pnpm exec vite --config acpdemo/pi/vite.config.mjs
```

# Qoder CLI 验收

2026-11-13：**指定模型实际创建编辑器成功**。批次 18，批次 17 PI 已结束归档。按最新授权，以完整编辑器文件和运行说明落盘为终点；未执行浏览器、编辑器构建或创建后的额外回归，不将这些检查记为通过。

- 沿用租户 `10001 / 100010005`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，`acp-qoder / qoder`、`session_model`、`full_access=true`。没有更换模型、账号或引擎。
- 初次复验的普通 Qoder 入口安装成功（HTTP 200、curl 0、8.292494 秒），从 1.1.86 更新为 1.1.88，但原生 status 为 logged_in=false、allow_byok=0，原生 session/new 返回 Authentication required；旧网关真实请求 HTTP 200、curl 0、60.755587 秒，终态 chat.error / `ACP session_model driver is not verified`，无文件。该失败只适用于普通账号入口，不作为必须新增账号授权才能完成本项的结论。
- 继续调查固定 1.1.88 bundle，发现其本地 ACP 框架 `P7e/ahe` 使用与 Qoder 账号框架相同的 `e5/O1` 原生会话、AgentLoop、工具与权限控制。新增 `patches/acp-model-source/qoder/`，仅在显式私有会话绑定下选择该本地框架，只注册绑定的 OpenAI-compatible Provider；没有换入其他代理引擎，没有使用 fakeResponses，也没有伪造官方身份、模型目录或 BYOK 权益。官方云 Provider/账号框架不在该分支构造，普通未绑定入口保留原逻辑。
- Rust 驱动通过请求拥有者的 0700 目录、0600 配置文件传入原模型、准确回环端点及短期中继令牌，供应商密钥不交给 CLI；固定 npm 归档完整性、原 bundle、补丁后 bundle 与绑定模块哈希均受校验。仅接受受管入口、固定 ACP 参数和隔离 user 配置源；启动前将 Node TCP 限制在本次准确回环端口并拒绝外部 dns.lookup，阻止上游后台官方网络尝试。官方账号状态及全局 bundle 在实现前后保持原样，普通账号入口仍要求真实登录。
- 创建前的验证：3 项 Node 边界测试通过（私有文件/会话引用拒绝、官方主机和无关回环端口拒绝、绑定撤销拒绝）；固定受管 CLI 的真实 initialize/session/new 成功，真实推理请求到达本机 HTTP 探针（503 仅是可达性证据）；原生 mode `default → yolo → default` 成功，模型不变。新增 Rust 拒绝场景通过 Clippy 编译检查，未单独执行 Cargo 聚焦测试；创建后未补跑。
- `cargo clippy --all-targets --all-features -- -D warnings` 退出码 0、4 分 45 秒。`cargo build -p vw-cli --bin shiju --features vw-agent/default` 退出码 0、11 分 11 秒。使用共享槽位 7，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `target/rust-slot-7` 的绝对路径；槽位已释放、缓存保留。真实运行独立 CLI 副本，SHA-256 `ae98ea828c3d489fd0409ea31d6d6fefb325bb1159f33f28972582e2b5399739`。
- 新构建使用现有开发配置启动临时网关 `127.0.0.1:49459`，未重启用户原服务。受管安装器固定 npm 安装退出码 0（3 秒）；随后系统安装 API HTTP 200、curl 0、0.016839 秒，状态 true → true，解析入口为 `/Users/xiangmin/.shiju-harness-dev/acp-runtime/qoder-1.1.88-model-source-v1/node_modules/.bin/qodercli --acp`。已核实版本 1.1.88；普通独立诊断 HTTP 200、curl 0、3.748527 秒，authentication_required；模型来源 HTTP 200、curl 0，session_model supported、已审查 Darwin arm64 / 1.1.88。普通诊断与私有绑定会话分别记录。
- 会话 `ses_1d0dfe692001pft4ovsCg1I26q` 的 ui/summary 与 acp-options 在运行期间及结束后均 HTTP 200、curl 0；绑定 acp-qoder / qoder，full_access=true。真实执行期间 connected=true，selected_model 的 source、provider_id 和完整模型 ID 与请求一致。访问能力报告 partial，不将请求标志解释为全部原生权限均已验证；结束后请求拥有者关闭连接，connected=false 不代表执行失败。
- 真实创建请求 HTTP 200、curl 0、148.170801 秒，终态 chat.done，无 chat.error。事件为 chat.context_loaded ×1、chat.step_start ×1、chat.delta ×37、chat.tool ×64、chat.session_updated ×2、chat.step_finish ×1、chat.done ×1。提示要求实际调用工具创建文件，完成后简短结束，不运行额外测试。
- 目标目录原先不存在；实际新增 `acpdemo/qoder/` 下 `index.html`、`vite.config.mjs`、`tsconfig.json`、`README.md`、`src/main.tsx`、`src/env.d.ts`、`src/App.tsx`、`src/style.css` 共 8 个文件。源码导入本仓库 `@rue-js/rue`，包含 textarea 输入、computed 字数统计、localStorage 防抖保存与初始化恢复；Vite 复用仓库 Rue 插件，README 提供运行说明。仅核对文件与源码，不宣称页面行为已验证。
- 请求前与网关停止后的 Rue 普通文件 SHA-256 快照（排除 .git、node_modules、target）仅发现上述 8 个新增文件，目标目录外无内容变化。预设基线 enabled=false，临时启用 HTTP 200、curl 0、6.116960 秒；恢复 HTTP 200、curl 0、10.305573 秒，结束 GET agents HTTP 200、curl 0、复查 false。临时网关进程组已停止，端口关闭；未启动编辑器服务。
- 原始请求、配置、SSE、探针、源码证据与文件快照仅保存在本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-qoder-v48l1xmr/`（最终成功证据在 `acceptance/`），未提交凭据、配置环境值或原始供应商 bundle。

在 Rue.js 仓库根目录启动：

```sh
pnpm exec vite --config acpdemo/qoder/vite.config.mjs
```

后续建议：常用网关切换到包含本次受管入口的构建；浏览器行为与编辑器构建仍待维护者按需验证。

# Qwen Code 验收

2026-11-13：**指定模型实际创建编辑器成功**。批次 19；批次 18 Qoder 已结束并归档。按最新授权，以文件创建为验收终点，未运行浏览器、编辑器构建或额外回归。

- 沿用租户 `10001 / 100010005`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，`acp-qwen / qwen`、`session_model`、`full_access=true`，未更换账号、模型或引擎。
- 原 42617 未监听；复用上一批已构建 CLI 的独立副本及现有开发配置，启动临时网关 `127.0.0.1:50855`。二进制 SHA-256 `ae98ea828c3d489fd0409ea31d6d6fefb325bb1159f33f28972582e2b5399739`。本项未修改 Rust、安装器或补丁，未运行 Cargo、Clippy 或单元测试。
- 系统安装 HTTP 200、curl 0、1.493188 秒，安装状态 false → true。解析入口 `/Users/xiangmin/.shiju-harness-dev/acp-runtime/qwen-0.26.7-model-source-v1/node_modules/.bin/qwen --acp`；版本命令退出码 0，版本 0.26.7。独立诊断 HTTP 200、curl 0、1.388422 秒，authentication_required；模型来源 HTTP 200、curl 0，session_model supported，已审查 Darwin arm64 / 0.26.7。普通独立诊断与真实私有绑定会话分别记录。
- 真实创建请求 HTTP 200、curl 0、111.953304 秒，终态 `chat.done`，无 chat.error。事件：chat.context_loaded ×1、chat.step_start ×1、chat.delta ×28、chat.tool ×80、chat.session_updated ×2、chat.step_finish ×1、chat.done ×1。提示要求实际调用工具创建文件，完成后简短回复，不运行额外测试。
- 会话 `ses_1d0e670af0011mSDT1ee2xuBja`：执行期间及结束后的 ui/summary 与 acp-options 均 HTTP 200、curl 0，绑定 acp-qwen / qwen、full_access=true；选定模型的 source、provider_id、完整模型 ID 与请求一致。执行期间 connected=true，原生权限支持 native；结束后连接关闭。
- 旧目标目录为前轮未跟踪产物，已完整移至本机私有目录 `previous-qwen/` 保留；本轮从空目标目录创建 `acpdemo/qwen/` 下 `README.md`、`index.html`、`vite.config.mjs`、`tsconfig.json`、`src/App.tsx`、`src/main.tsx`、`src/env.d.ts`、`src/style.css` 共 8 个文件。源码导入本仓库 `@rue-js/rue`，包含 textarea 输入、computed 字数统计和 localStorage 保存恢复；README 提供运行说明。仅确认文件与源码存在，不宣称页面行为通过。
- 请求前与网关停止后的 Rue 普通文件 SHA-256 快照（排除 .git、node_modules、target），仅上述 8 个文件新增，目标目录外无内容变化。预设基线 enabled=false；临时启用 HTTP 200、curl 0、6.000155 秒；恢复 HTTP 200、curl 0、7.171790 秒，结束 GET agents HTTP 200、curl 0、复查 false。自己的临时网关进程组已停止，未启动编辑器服务。
- 原始配置、SSE、会话查询、旧产物备份和文件快照仅保存在本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-qwen-ndlkd7wp`，未提交凭据或配置环境值。

后续建议：按需启动 `pnpm exec vite --config acpdemo/qwen/vite.config.mjs` 检查页面行为。

# Snow CLI 验收

2026-11-13：**指定模型实际创建编辑器成功**。批次 20；批次 19 Qwen 已结束归档。按最新授权，以完整文件创建为终点，未运行浏览器、编辑器构建或创建后的额外回归。

- 沿用租户 `10001 / 100010005`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，`acp-snow / snow`、`session_model`、`full_access=true`，未更换模型、账号或引擎。
- 复用前批已构建 CLI 的独立副本及现有开发配置启动临时网关 `127.0.0.1:52132`；二进制 SHA-256 `ae98ea828c3d489fd0409ea31d6d6fefb325bb1159f33f28972582e2b5399739`。本项未修改 Rust、安装器或补丁，未运行 Cargo、Clippy 或单元测试。
- 系统安装 HTTP 200、curl 0、2.171400 秒，安装状态 false → true。解析入口 `/Users/xiangmin/.shiju-harness-dev/acp-runtime/snow-0.9.14-model-source-v1/node_modules/.bin/snow --acp`；版本命令退出码 0，输出 0.9.14。独立诊断 HTTP 200、curl 0、0.995240 秒，connected；模型来源 HTTP 200、curl 0，session_model supported、已审查 Darwin arm64 / 0.9.14。
- 真实创建请求 HTTP 200、curl 0、127.753565 秒，终态 `chat.done`，无 chat.error。事件：chat.context_loaded ×1、chat.step_start ×1、chat.delta ×31、chat.tool ×52、chat.session_updated ×2、chat.step_finish ×1、chat.done ×1。提示要求实际调用工具创建文件，完成后简短回复，不运行额外测试。旧租户空参数审批后未生成文件的问题本次未复现，无需削弱权限检查。
- 会话 `ses_1d0ecb9c1001VWusv3H6GCUf4Z` 的 ui/summary 与 acp-options 在执行期间及结束后均 HTTP 200、curl 0，绑定 acp-snow / snow、full_access=true；执行期间 connected=true，选定模型的 source、provider_id、完整模型 ID 与请求一致。访问能力为 client_only，不将请求标志解释为原生工具全部免审批已验证。
- 目标目录原先不存在；实际新增 `acpdemo/snow/` 下 `README.md`、`index.html`、`vite.config.mjs`、`tsconfig.json`、`src/App.tsx`、`src/main.tsx`、`src/env.d.ts`、`src/style.css` 共 8 个文件。源码导入本仓库 `@rue-js/rue`，包含 textarea 输入、computed 字数统计、localStorage 防抖保存与初始化恢复；README 提供运行说明。仅核对文件与源码，不宣称页面行为已验证。
- 请求前与网关停止后的 Rue 普通文件 SHA-256 快照（排除 .git、node_modules、target）仅发现上述 8 个文件新增，目标目录外无内容变化。预设基线 enabled=false；临时启用 HTTP 200、curl 0、9.106885 秒，恢复 HTTP 200、curl 0、6.000420 秒，结束 GET agents HTTP 200、curl 0、复查 false。自己的临时网关进程组已停止，未启动编辑器服务。
- 原始配置、SSE、会话查询与文件快照仅保存在本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-snow-za7laocj/`，未提交凭据或配置环境值。

后续建议：按需启动 `pnpm exec vite --config acpdemo/snow/vite.config.mjs` 检查页面行为。

# TRAE CLI 验收

2026-11-13：**指定模型实际创建编辑器成功**。批次 21；批次 20 Snow 已完成归档。按最新授权，以完整编辑器文件与运行说明实际落盘为终点；未运行浏览器、编辑器构建或创建后的额外回归，不将页面行为记为通过。

- 沿用租户 `10001 / 100010005`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，`acp-trae / trae`、`session_model`、`full_access=true`；未更换模型、账号或执行引擎。
- 初次复验旧网关：系统安装 HTTP 200、curl 0、3.619042 秒，状态 true → true，官方入口版本 0.121.3；诊断 HTTP 200、curl 0、0.454664 秒、connected。真实请求 HTTP 200、curl 0、40.856944 秒，终态 chat.error，仍被未验证驱动拒绝，目标目录不存在且无产物。这些信号不计为指定模型连接成功。
- 继续调查固定发行物：实际配置文件名为 `.trae/trae_cli.yaml`，与该包内置手册中的 `traecli.yaml` 不同。正确配置后，原生 ACP 可枚举和选择自定义模型；未登录的 session/prompt 在平台账号中间件报 `failed to get value from keyring: secret not found in keyring`，本机模型探针请求数为 0。没有导入 IDE 令牌、交换平台身份或写入登录凭据。
- 新增 `patches/acp-model-source/trae/`，从官方固定归档生成独立受管本地组合。保留真实 TRAE 原生循环、工具、权限处理、ACP 服务；仅不装配平台模型的 CommercializationPreCheck 与 ReportCustomModelTokenUsage 中间件，并让原生用户目录和缓存目录定位到请求私有目录。未更改登录、账号类型、企业功能或配额检查的实现及返回值；原始官方二进制逐字节保留，未绑定的普通入口仍运行原版。没有替换引擎或伪造平台授权、配额结果。
- 启动器要求 0700 目录、0600 文件与匹配会话引用，绑定唯一原模型、准确 IPv4 回环中继端点和短期令牌，禁止模型回退；不修改 HOME。Darwin 原生进程启动前加载内核网络策略，仅允许绑定的回环 TCP 端口，拒绝官方平台、DNS、遥测和无关回环连接；策略也覆盖子进程。供应商模型密钥未交给 CLI，原生配置在模型选择后仍保留令牌环境变量占位符。归档、官方二进制、本地组合和启动器摘要均受安装器及启动器校验。官方归档 SHA-256 `30fd43892949dba6d1bff5fb464c8ec3a936b61cb97ed5a9d5af3a97b6e3f3f8`，原始二进制 SHA-256 `7648c66ffb535e83e4b1f5bbd820f0384d77234763cfb32700b1be3289358758`。
- 创建前验证：`scripts/acp_model_source_trae_tests.py` 三项通过，25.018 秒，覆盖私有文件/会话引用/端点拒绝、真实内核端口隔离、真实原生 ACP initialize/session/new/session/set_model 与模型 HTTP 请求。模型探针故意返回 HTTP 400，仅证明原生请求到达私有端点，不计为编辑器创建。Rust 新增启动拒绝场景由 Clippy all-targets 编译，未单独执行 Rust 单元测试。
- 使用共享槽位 7，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `target/rust-slot-7` 的绝对路径。`cargo clippy --all-targets --all-features -- -D warnings` 退出码 0，4 分 43 秒；`cargo build -p vw-cli --bin shiju --features vw-agent/default` 退出码 0，10 分 40 秒。相关源码摘要在构建期间无变化。真实运行独立 CLI 副本 SHA-256 `49cdde7237d6c3267520f484b36f4d096e41266e49e31f5ece40eea94893c0e2`；槽位已释放、缓存保留。
- 新构建使用现有开发配置启动临时网关 `127.0.0.1:59563`，未重启用户原服务。固定受管安装器退出码 0，系统安装 API HTTP 200、curl 0、0.082881 秒，状态 true → true，入口 `/Users/xiangmin/.shiju-harness-dev/acp-runtime/trae-0.121.3-model-source-v1/traecli acp serve`，核实版本 0.121.3。普通独立诊断 HTTP 200、curl 0、0.740548 秒、connected；模型来源接口 HTTP 200、curl 0、session_model supported，已审查 Darwin arm64 / 0.121.3。普通诊断与私有模型请求分别记录。
- 会话 `ses_1d10f575d001coW29r630ktu2X` 的 ui/summary 与 acp-options 在执行期间及结束后均 HTTP 200、curl 0；绑定 acp-trae / trae、full_access=true。执行期间 connected=true，selected_model 的来源、provider_id 与完整模型 ID 均匹配请求。访问能力报告 client_only，仅证明客户端工具审批自动批准，不声称原生工具全部免审批已验证。
- 真实创建请求 HTTP 200、curl 0、148.020811 秒，终态 chat.done，无 chat.error。事件：chat.context_loaded ×1、chat.step_start ×1、chat.delta ×72、chat.tool ×36、chat.session_updated ×2、chat.step_finish ×1、chat.done ×1。提示要求实际调用工具创建文件，完成后简短结束，不运行额外测试。
- 目标目录原先不存在；实际新增 `acpdemo/trae/` 下 `README.md`、`index.html`、`vite.config.mjs`、`tsconfig.json`、`src/Editor.tsx`、`src/main.tsx`、`src/env.d.ts`、`src/style.css` 共 8 个文件。源码导入本仓库 `@rue-js/rue`，包含 textarea 输入、computed 实时字数/字符/行数统计、localStorage 400ms 防抖保存与初始化恢复；README 提供运行说明。仅核对产物与源码，不宣称浏览器行为通过。
- 请求前与网关停止后的 Rue 普通文件 SHA-256 快照（排除 .git、node_modules、target），仅上述 8 个文件新增，目标目录外无内容变化。预设基线 enabled=false，临时启用 HTTP 200、curl 0、9.695698 秒；恢复 HTTP 200、curl 0、8.371216 秒，结束 GET agents HTTP 200、curl 0、复查 false。自己的临时网关进程组已停止，端口关闭，未启动编辑器服务。
- 原始配置、SSE、会话查询、原生探针和文件快照仅保存在本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-trae-3fmuyjp0/`，最终成功证据位于 `acceptance/`。未提交凭据、配置环境值、原始供应商归档或二进制。

后续建议：常用网关切换至包含本次受管入口的构建；按需运行 `pnpm exec vite --config acpdemo/trae/vite.config.mjs` 检查页面行为。

# Mistral Vibe 验收

2026-11-13：**指定模型实际创建编辑器成功**。批次 22，批次 21 TRAE 已归档。按最新授权，以完整文件创建为终点；未启动页面，未执行浏览器、编辑器构建或创建后的额外回归，不将页面行为记为通过。

- 租户 `10001 / 100010005`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，`acp-vibe / vibe`、`session_model`、`full_access=true`。未更换模型、账号或执行引擎。
- 修复底层 Vibe 模型绑定：将目录的上下文上限写入原生 `max_context_length`，同时用于主模型与压缩模型；零值沿用共享默认值。保留原输出上限，不通过降低模型能力绕开旧错误。目录 context=1000000 / output=384000，此前仅传输出值，Vibe 回退到 262144 上下文而拒绝 session/new。原生 3.0.4 ModelConfig 离线校验接受原目录值（退出码 0）。回归断言覆盖主模型与压缩模型的两项上限，由 Clippy all-targets 编译，未单独执行 Rust 单元测试。
- 创建前使用共享槽位 7，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为该槽位绝对路径。`cargo clippy --all-targets --all-features -- -D warnings` 退出码 0，215.98 秒；`cargo build -p vw-cli --bin shiju --features vw-agent/default` 退出码 0，367.73 秒。本项源码摘要核对无变化，槽位已释放、缓存保留。独立 CLI 副本 SHA-256 `c4309537b44682686570fc3ba7eecd896e955d973a17f1908e7e7a6023b63d11`。
- 使用现有开发配置启动临时网关 `127.0.0.1:63051`。系统安装 HTTP 200、curl 0、8.176781 秒，状态 false → true；解析入口 `/Users/xiangmin/.shiju-harness-dev/acp-runtime/vibe-3.0.4-model-source-v1/bin/vibe-acp`，版本命令退出码 0、vibe-acp 3.0.4。独立诊断 HTTP 200、curl 0、3.692232 秒，authentication_required；模型来源接口 HTTP 200、curl 0，session_model supported、已审查 Darwin arm64 / 3.0.4。普通诊断不计为会话模型连接成功。
- 真实请求 HTTP 200、curl 0、134.730681 秒，终态 chat.done，无 chat.error。事件：chat.context_loaded ×1、chat.step_start ×1、chat.delta ×28、chat.tool ×40、chat.session_updated ×2、chat.step_finish ×1、chat.done ×1。提示要求实际调用工具创建文件，完成后简短回复，不运行额外测试。
- 会话 `ses_1d120551b001YH48b1QJeyVdFn` 的 ui/summary 与 acp-options 在执行期间及结束后均 HTTP 200、curl 0，绑定 acp-vibe / vibe、full_access=true；执行期间 connected=true，selected_model 的来源、provider_id 与完整模型 ID 均匹配请求。访问能力为 client_only，仅证明客户端工具审批自动批准，不声称原生工具全部免审批已验证。
- 目标目录原先不存在。实际新增 `acpdemo/vibe/` 下 `README.md`、`index.html`、`vite.config.mjs`、`src/App.tsx`、`src/main.tsx`、`src/env.d.ts`、`src/style.css` 共 7 文件。源码导入本仓库 `@rue-js/rue`，包含 textarea 输入、computed 实时统计、localStorage 400ms 防抖保存、pagehide/beforeunload 补写和初始化恢复；README 提供运行说明。仅检查文件与源码存在。
- 请求前与网关停止后的 Rue 普通文件 SHA-256 快照（排除 .git、node_modules、target）仅发现上述 7 文件新增，目标目录外无内容变化。预设基线 enabled=false，临时启用 HTTP 200、curl 0、9.306666 秒；恢复 HTTP 200、curl 0、7.130865 秒，结束 GET agents HTTP 200、curl 0、复查 false。自己的临时网关进程组已停止，端口关闭，未启动编辑器服务。
- 原始配置、SSE、会话查询、构建日志及快照仅保存在本机私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-vibe-kbsumlda/`，未提交凭据或配置环境值。

后续建议：常用网关更新至含此修复的构建；按需运行 `pnpm exec vite --config acpdemo/vibe/vite.config.mjs` 检查页面行为。

# Autohand Code 验收

2026-11-13：**已阻塞，未通过**。任务 22 恢复后核对当前实现与接入证据，指定 `session_model` 尚无驱动，不能进行指定模型文件创建验收。

- 已交付注册 `autohand`（固定 autohand-cli@0.9.11、--acp）、系统安装白名单和自动派生 `acp-autohand` builtin 预设。接入证据 `docs/dev/evidence/acp-onboarding-autohand.md` 明确保留会话模型中继拒绝门控；其中本地 fixture 成功不能替代本次指定模型验收。
- 本次源码检查：`crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs` 没有 autohand configure 分支或 verified_release 条目；capabilities 因此返回 SessionModel / UnsupportedCli。该文件 SHA-256 为 `1d1dea379f3d787468888a1e320aa0ca00cd7f119201ced32455ba99b76158f7`。这是静态阻塞证据，未冒充运行时接口响应。
- Rue.js 的 `packages/rue/package.json` 存在；目标 `acpdemo/autohand/` 不存在。未创建产物、未覆盖文件。未启动网关、安装或诊断进程，未变更租户/预设配置，无临时配置或自有进程需要恢复清理；未操作其他任务环境。
- 未执行：安装 API、模型来源 GET、诊断、启用保存重读、真实聊天 SSE、会话绑定查询、工作区完整快照及页面检查。没有 HTTP、客户端退出码、运行版本或构建摘要可报告；不使用旧二进制判定当前接入成功或失败。仅文档变更，未运行 Cargo、Clippy 或测试。
- 本次静态源码与工作区状态证据目录：`/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-autohand-blocked-091xoi7s`（0700），未保存凭据或配置环境值。

重跑条件：先完成并验证 Autohand 会话模型中继驱动，模型来源明确支持后，使用包含该驱动的已验证构建从任务步骤 1 重新执行；保持指定模型、租户、引擎和目录边界。

# DeepAgents 验收

2026-11-13：**已阻塞，未通过**。任务 23 核对当前实现与接入证据后，确认指定 `session_model` 未交付，未执行指定模型文件创建。

- 前置任务 22 已明确记录阻塞，未启动服务或变更配置，无待恢复环境。来源任务 `2-DeepAgents.md` 已不存在；以当前注册、安装白名单、builtin 派生及 `docs/dev/evidence/acp-onboarding-deepagents.md` 核对交付边界，没有将任务文件缺失当成通过。
- 当前注册为 `deepagents`，固定 `deepagents-acp@0.1.42`，本地候选 `deepagents-acp`；系统安装映射存在，`merged_agent_catalog` 派生 `acp-deepagents`、builtin=true、backend=deepagents，默认停用。这些为源码检查结果，未冒充安装 API 或配置保存重读结果。
- 既有接入证据只验证 CLI 本地模型 gpt-4o，并明确保留 UnsupportedCli 门控。当前 `crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs` 没有 DeepAgents configure、source_protocol 或 verified_release 分支；因此 capabilities 的 SessionModel 为 UnsupportedCli。该文件本次 SHA-256：`1d1dea379f3d787468888a1e320aa0ca00cd7f119201ced32455ba99b76158f7`。不改用 cli_local、CLI 默认模型或其他 backend 绕过指定模型要求。
- 新增整个私有模型绑定、协议路由及审核驱动超出本项验收小修范围；未修改产品实现。指定租户 `10001 / 100010005`、模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 与引擎要求保持原样。
- Rue.js 工作区及 `packages/rue/package.json` 存在，已保存初始 Git 状态；`acpdemo/deepagents/` 不存在，未写入 Rue.js 工作区或生成产物。未做完整文件 SHA-256 快照，不能声明经过全目录差异验证。
- 未启动网关、供应商进程或安装器，未 PATCH 预设/租户配置，无本次临时配置、自有进程或槽位需要恢复清理。未执行安装/诊断/模型来源 API、启用保存重读、聊天 SSE、会话绑定及文件验收；没有本次 HTTP、退出码、运行版本或二进制构建摘要可报告。未运行页面、浏览器、构建、Cargo、Clippy 或测试，本次仅文档变更。
- 本次源码副本、SHA-256 与工作区初始状态仅存本机 0700 私有目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-deepagents-blocked-a3la5u4n`；没有保存凭据、配置环境值或供应商二进制。

重跑条件：先交付并验证 DeepAgents 的 session_model 私有中继驱动与真实模型请求证据；使用包含该实现的已验证构建，从任务 23 步骤 1 重新执行。任务文件保留为已阻塞。


# Autohand Code 验收（2026-11-17 扩围恢复）

**重跑通过**：保持租户 `10001 / 100010005`、模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`、原生 `autohand` 引擎与宿主权限策略。真实请求取得 `chat.done`，无 `chat.error`，完整文件实际落盘。首轮失败及目录外变化保留在下文，不将首轮记为通过，也不声称最初工作区没有目录外变化。

## 实现与版本

- 用户授权扩围后补齐 `llm-runtime` 的 Autohand Chat 模型驱动、协议路由及固定版本能力条目。私有 0700 启动目录、0600 配置只含短期中继令牌；显式 OpenAI provider/model、环回地址校验、启动参数白名单、环境冲突拒绝和撤销沿用宿主边界。不借用 CLI 默认模型，不传入供应商长期密钥。
- 固定 CLI `0.9.11 (46a28b16)`，真实入口 `/opt/homebrew/bin/autohand --acp`；入口 JS SHA-256 `339bdb0c8f4f8b7f9076108434d910b2cd5e14f2593b48c0e5858e04e91950eb`。安装后的 OpenAI 提供商源码与本机固定 npm 版本探测源码一致。
- 补充 Autohand 官方 ACP 扩展 `clientCapabilities._meta.autohandWorkspaceSetup=agent-managed`，将工作区准备及质量检查交由宿主管理，避免完成文件后擅自执行 CLI 默认质量流程；不选择 yolo/automode，不改变权限回调。修复共享消息的带标签 JSON 费用字段在 `serde_json/arbitrary_precision` 下出现 `invalid type: map, expected f64` 的反序列化问题，仍拒绝字符串、普通对象及非有限数值。
- CLI 构建另遇既有 `gateway-app-skills/src/claude_api.rs` 中无调用者的 `claude_token` 残留函数引用不存在变量 `value`；仅删除该死函数，调用者继续使用已有统一请求上下文。
- 本轮使用共享工作区的新调试构建，不使用旧二进制判断驱动。最终 `shiju` SHA-256：`180df604cc793f5265bca057861ca01e45e3b29b40c0f4399331833e37f2741b`。构建包含本任务源码；其他并发任务差异未纳入本任务提交，源码摘要另存私有证据。

## 安装、预设及真实会话

- 首次系统安装 API HTTP 200、curl 0、11.580975 秒，安装基线为 false，之后为 true；真实入口版本命令退出 0。新构建重读 installed、agent、acp 配置及模型来源接口均 HTTP 200；重复系统安装 HTTP 200。普通诊断 HTTP 200、`authentication_required`；与私有会话实际连接分开记录。
- 能力接口确认 Darwin arm64 / `0.9.11 (46a28b16)` 的 session_model supported。内置预设 `acp-autohand`、builtin=true、backend=autohand，原有效 enabled=false；临时 PATCH 为 true 并 GET 确认，结束恢复 false。重跑结束整份配置与本轮基线相等，agent 有效配置也相等。首轮启用前没有该显式覆盖项，保存后留下等效的停用内置配置，不声称首轮配置字节完全一致。
- 使用任务原请求体及三个指定租户/目录头，不替换模型。成功重跑 HTTP 200、curl 0、149.211335 秒；事件为 context_loaded×1、step_start×1、chat.tool×70（状态事件，非 70 次独立工具调用）、delta×1、session_updated×2、step_finish×1、chat.done×1，chat.error 为 0。
- 实际响应头会话 `ses_1e8ff4fb2001ecTOwXzRMGyFMW`：运行中及结束后的 summary/options 均 HTTP 200，绑定 `acp-autohand / autohand`、full_access=true；selected_model.source=session_model、provider_id=alibaba-token-plan-cn、完整 model ID 与请求一致。运行中 connected=true，结束 connected=false。未把完全访问请求误称为已审核的供应商 Native 权限支持。
- 重跑采集器早期误读首轮遗留响应头；已从本轮实际响应头重新提取 ID，绑定结论只采用 `running-*` 和 `actual-final-*` 新查询，不引用旧会话响应；未因此重发聊天请求。

## 产物、边界与清理

- 目标目录 `/Users/xiangmin/worker/code/ruejs/acpdemo/autohand/` 共 8 文件：`README.md`、`index.html`、`vite.config.mjs`、`tsconfig.json`、`src/main.tsx`、`src/Editor.tsx`、`src/env.d.ts`、`src/style.css`。已读取实际源码：本仓库 Rue Vite 别名及编译插件、`@rue-js/rue` 的 useApp/ref/computed/lifecycle、textarea 输入、实时字数及 localStorage 防抖保存和启动恢复均在文件中。说明给出运行入口；没有运行浏览器、页面启动、页面构建或额外页面测试，不将静态检查冒充实际渲染。
- 初始 SHA-256 快照含 4,378 普通文件，排除 .git/node_modules/target。首轮真实创建文件后收到原生 `session/prompt` internal error，SSE 为 chat.error，没有 chat.done；结束 options 又因费用类型错误返回 HTTP 400。首轮 8 文件与原始证据已归档；确认这些任务自建文件未被外部修改后，移出目标目录再按修复后的实现重跑，未覆盖用户文件。
- 首轮期间观察到目标外 1,855 文件变化：1,854 新文件、1 个既有文件修改，涉及根 `dist/`、`packages/**/dist/` 等生成输出、`.autohand/memory/index.json`，修改项为 `packages/runtime/__tests__/node-plan-test-utils.d.ts`。固定 CLI 源码确认有默认自动质量流程及项目记忆初始化，但未取得完整原生 stderr，无法排除并发变化，不把全部生成文件强行归因给本会话。完整具体路径与前后摘要在 `first-boundary.json`、`first-after.json`、`first-run/before.json`；未擅自删除这些目录外文件，也不宣称恢复了最初全工作区字节状态。
- 成功重跑前基线为 6,232 文件，结束 SHA-256 对比仅目标目录 8 文件新增，目录外 0 差异，详见 `before.json`、`after.json`、`boundary.json`。这是本次成功重跑的边界证据，不能替代首轮变化记录。
- 两次临时网关分别使用独立端口 64991、50326，均已退出 0；结束自有进程存活数 0，自有槽位锁随控制进程退出释放，缓存保留。临时 enabled 配置已恢复并 GET 重读确认。没有主动停止或重启原服务；结束检查原 `127.0.0.1:42617` 连接被拒绝，原 PID 已不存在，其消失原因不在本次已掌握证据内，不能声称原服务健康。
- 全部原始请求、响应、SSE、配置、快照、首轮产物与探测脚本仅保存在本机 `/tmp/shiju-autohand-20261117-retry`（0700），未提交凭据、完整配置、敏感原始载荷、供应商二进制或 Rue.js 工作区文件。

## Rust 验证

通过 `scripts/cargo_slot.py` 的原子槽位入口执行，未裸运行 Cargo。首个驱动红灯使用槽位 3，之后串行复用槽位 4；两个构建环境变量均为对应绝对路径 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3` 或 `rust-slot-4`。恢复批次实际入口为私有证据中的 `recover.py`（内部持有 `slot(number=4)`），未争用他人锁或清理缓存。下列为槽位内原生子命令；后续复跑应加 `python3 scripts/cargo_slot.py` 入口。

|子命令|退出码|有效结果|
|---|---:|---|
|`cargo test -p llm-runtime --lib autohand_selected_model_uses_chat_protocol -- --test-threads=1`|101|1 失败，缺失模型驱动路由|
|`cargo test -p llm-runtime --lib autohand_ -- --test-threads=1`|0|3 通过：协议、私有配置撤销、覆盖/非环回拒绝|
|`cargo test -p message-types --features serde_json/arbitrary_precision --lib assistant_cost_roundtrips_ -- --test-threads=1`|101|1 失败，重现 map/expected f64；早期夹具编译错误已修正，不作为业务红灯|
|`cargo test -p message-types --features serde_json/arbitrary_precision --lib cost_roundtrip -- --test-threads=1`|0|2 通过：助手及步骤费用数值回读，错误类型拒绝|
|`cargo test -p acp-client --lib autohand_initialize_ -- --test-threads=1`|101 → 0|先以正确 agent 身份重现缺失 metadata 的 1 失败，再 1 通过；早期错误夹具不作为业务证据|
|`cargo clippy -p message-types -p acp-client --lib --no-deps -- -D warnings`|0|受影响库严格检查通过|
|`cargo clippy -p llm-runtime --lib --no-deps -- -D warnings`|101|101 项既有诊断（如 result_large_err/collapsible_if），无本次 Autohand 文件诊断；未扩大修复这些无关 lint|
|`cargo build -p vw-cli --bin shiju`|101 → 0 → 0|首次被既有死函数阻断；删除后通过；消息/握手修复后的最终构建再次通过|

仅格式化修改文件；未运行桌面编译、整仓构建、全量测试、全仓格式检查、全特性 Clippy、wasm 或 Makefile。功能重跑通过不代表上述严格 llm-runtime lint 已通过。

补充检查：删除死函数所涉 `gateway-app-skills` 的定向严格 Clippy 通过 `python3 scripts/cargo_slot.py cargo clippy -p gateway-app-skills --lib --no-deps -- -D warnings` 在槽位 9 执行（两个构建目录变量均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-9`），退出 101，15 项既有 unused token 等诊断；没有修改这些无关调用。此前等待槽位 4 的同一检查尚未取得锁，已仅中断持有本任务私有日志文件的自有排队进程，再经标准入口取得槽位 9；未操作其他任务进程或锁。最后检查结束后自有编译控制进程均已退出，槽位缓存保留。普通诊断首次 curl 退出 0、HTTP 200、2.263259 秒；首轮失败流 curl 退出 0、HTTP 200、155.432666 秒，明确不计为成功。


## 2026-11-17 DeepAgents 验收复核

结论：**已阻塞，未通过**。任务 22 已于本日完成实机验收并记录配置恢复、自有进程退出；来源任务 2 已删除，接入证据仍明确 session_model 未发布。

本次先检查 RustCodeGraph 索引（12,367 文件），explore 未及时返回，转查当前生产源码；注册与模型驱动已迁至独立 crate。`crates/libs/acp-agent-registry/src/agent_registry.rs` 保留 deepagents 与 deepagents-acp@0.1.42；`crates/libs/config-acp-install/src/lib.rs` 保留固定安装映射；`crates/libs/agent-config/src/catalog.rs` 派生 builtin=true、backend=deepagents、默认停用的 acp-deepagents。

当前实际生产 `crates/libs/llm-runtime/src/acp/model_source/drivers/mod.rs` 的 verified_release、source_protocol、configure 均无 deepagents 分支，capabilities 因未验证版本返回 SessionModel/UnsupportedCli。vw-agent 兼容层委托此驱动，迁移没有解除阻塞。生产驱动 SHA-256：`b189cc94de2873cbf41b92dd07cf9c4e7daa5881133676d58a6940170c807767`。新增完整私有中继驱动超出验收小修范围，未扩大实现或替换指定模型。

Rue.js 源码 packages/rue/package.json 存在，目标 acpdemo/deepagents/ 不存在；初始 Git 状态、源码副本及摘要存于本机 0700 目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-deepagents-20261117-8iuu7mzw`。未写 Rue.js 文件，未生成完整普通文件快照，不能声称全目录边界验证通过。

未执行安装、诊断、模型来源 API、配置 PATCH、SSE 或会话绑定及产物验收；未启动网关或供应商进程、未领取编译槽位，未改配置，无本次运行环境需恢复。无运行版本或二进制摘要可报告。仅文档变更，未运行 Cargo、Clippy、测试、页面构建或浏览器。恢复条件仍为交付并验证 DeepAgents session_model 驱动，再用包含该实现的已验证构建从步骤 1 重跑；任务 23 保持已阻塞并保留。


# DeepAgents 验收（2026-11-17 扩围恢复）

结论：**指定模型文件创建验收通过**。用户追加授权扩大范围后，补齐宿主 session_model 私有中继驱动，并修复真实执行发现的两处底层问题。此结论替代本文件此前的能力缺失阻塞；严格 Clippy 的既有失败单独保留，不声称全仓检查通过。

## 实现与安全边界

- 当前实现位于 `crates/libs/llm-runtime/src/acp/model_source/drivers/deepagents.rs`。使用现有 RelayBinding 的租户、项目、会话、provider/model 身份及短期令牌；明确选择 Chat 协议，传入 `--model openai:<选定模型>` 和 `--workspace <请求工作区>`，拒绝自定义 args/env、非私有回环端点及不可执行模型参数。不替换模型、租户或 DeepAgents 引擎。
- acp-client 对受管 DeepAgents 复用现有 OS 环境白名单，不合并账号凭据或继承 Node 注入、代理及原生模型端点。本次自有网关设置了无效的继承 OPENAI_API_BASE，真实指定模型仍成功；没有通过 CLI 默认模型绕过中继。未扩大工具权限、替换 HOME 或增加 Native 完全访问能力。
- 真实首轮 session/new 因启动工作区与请求不一致而拒绝；新增独立失败测试后，显式传入工作区，保留原真实路径校验。首轮 HTTP 200、curl 退出 0、48.62 秒，但 SSE chat.error，无模型文件产物，明确不计通过。
- 第二轮模型绑定成功，但读取目录的宿主文件 RPC 错误被 LangGraph 当成 superstep 异常，134.72 秒后 chat.error。守卫现在将已授权文件 handler 的失败转换为明确 ToolMessage error，不向模型泄露底层异常详情，不回退本地文件，取消仍抛出并中止。未知/拒绝权限、未审核工具限制保持。独立守卫测试先红后绿；原生 CLI 夹具也明确断言宿主写入拒绝无文件且会话可继续。
- 新守卫的审核摘要为 `82e173b8284d3496e2032be5bec1c155241164f2f4c07aeff5d36e6f27e2faff`，系统安装使用独立 `deepagents-0.1.42-model-source-v2` 目录，保留旧 v1 安装。CLI 文件仍为 `d65ca9295356f90c11402e3ec9f39856324329a8a7acbdfe79592444e471abce`。入口为该受管目录下 `node_modules/.bin/deepagents-acp`，实际包版本及 --version 均为 0.1.42；握手 agentInfo.version=0.0.1 是上游独立字段。

## 本次真实执行

- 使用新构建 shiju 0.6.0，`cargo build -p vw-cli --bin shiju` 成功；最终 SHA-256 `183680133a28dbad5987e7343c6ec12803fe5696f6f7c6080cec2ec0cc5130bd`。构建使用当前共享源码，关键源码摘要与构建时 HEAD 保存在私有 source-hashes.json；未使用旧二进制判定修复成功。最后自有网关监听 127.0.0.1:55084，未重启用户服务。
- 所有 API 请求均带 X-Platform-ID=10001、X-Company-ID=100010005、X-Directory=/Users/xiangmin/worker/code/ruejs。读取 installed、完整 agent 目录与 acp 配置基线。新合同的安装状态 false→true；系统 POST 安装 HTTP 200、客户端退出 0、1.687 秒，回读受管包版本与摘要匹配。
- 普通诊断 HTTP 200、客户端退出 0、1.558 秒，内容为 authentication_required；此结果仅作普通诊断记录，未当成模型会话通过。模型来源 API 明确 session_model supported、Darwin arm64、审核 0.1.42。builtin acp-deepagents、backend deepagents 经临时 enabled=true 保存及 GET 重读确认。
- 原任务 JSON 请求体保持指定 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`、acp-deepagents、deepagents、session_model、full_access=true 及 Rue.js 工作区。第三轮初始 curl 在 300.011 秒超时退出 28，HTTP 200，未见终态；没有重提创建请求。
- 查询原会话状态后，根据现有生产代码与续传验收用例确认支持 Last-Event-ID。使用响应的原 x-vw-stream-request-id、原始请求体和游标 206 续传；HTTP 200、curl 退出 0、266.898 秒，响应仍是同一 session `ses_1e9490491001DBBQ6T1C1YapmA`。合并原流与续传流取得 chat.done，chat.error 为 0；7 次成功宿主 write_file 更新实际生成下列 5 文件。API 将这些写入事件展示为 edit，不代表启用了被拒绝的原生 edit_file。
- 运行中 summary 确认 acp-deepagents/deepagents/full_access=true；options connected=true，selected_model.source=session_model、provider_id=alibaba-token-plan-cn、完整模型 ID=alibaba-token-plan-cn/deepseek-v4.1-flash-1017。成功结束再 GET summary/options 均 HTTP 200，身份与选定模型仍匹配；最终 connected=false，未把已清理的连接描述为持续在线。access_status 为 client_only，未宣称 Native 免审批。

## 实际文件与环境恢复

- 产物位于 `/Users/xiangmin/worker/code/ruejs/acpdemo/deepagents/`：`index.html`、`vite.config.ts`、`src/main.tsx`、`src/style.css`、`README.md`。起初目录不存在，没有覆盖用户文件。
- 已阅读源码：main.tsx 从 @rue-js/rue 导入 ref/computed/useApp，以受控 textarea 的 onInput 更新状态与实时字数，localStorage 写入及初始化读回实现刷新恢复；HTML、样式、Vite 配置与运行说明齐备。node_modules 的 @rue-js/rue 和对应 Vite 插件真实路径分别指向本仓库 packages/rue 与 packages/vite-plugin-rue。README 推荐从仓库根运行 `pnpm exec vite --config acpdemo/deepagents/vite.config.ts`。
- 本次只验收真实文件创建与源码内容，没有启动页面、浏览器、页面构建或额外页面回归，不把静态审阅宣称为实际渲染通过。
- 初始普通文件 SHA-256 快照为 6,240 项，排除 .git/node_modules/target；首轮、第二轮结束均 0 差异。成功后 6,245 项，仅上述目标目录新增 5 文件，目录外差异 0，详见 before.json、first-boundary.json、second-boundary.json、after.json、boundary.json。
- 每轮 enabled 配置均在 finally 恢复，PATCH 与 GET 均成功，最终 agent 目录与最初有效配置相等、enabled=false。最初没有 acp-deepagents 覆盖条目；内置预设校验禁止 null 删除，因此保留显式 enabled=false 覆盖项，**有效停用状态已恢复，不声称存储字节完全还原**。没有改动其他预设、默认模型或权限策略。
- 三轮自有网关均退出 0；最终自有网关及槽位控制进程存活数 0，自有槽位锁已释放，缓存保留。受管 v2 安装作为系统安装结果保留。原始请求、响应、完整 SSE、配置、快照和探测脚本仅存本机 `/tmp/shiju-deepagents-20261117-retry`（0700）；未提交凭据、敏感原始载荷、供应商二进制或 Rue.js 工作区文件。

## 定向验证

Rust 重型命令全部经 scripts/cargo_slot.py，串行复用槽位 3；CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。私有 build.py 在同一槽位内完成严格检查、CLI 构建并持有网关生命周期；外部任务占槽期间正常排队，未清理他人锁或缓存。

| 原生子命令（均在槽位入口内） | 结果 |
| --- | --- |
| cargo test -p llm-runtime --lib deepagents_selected_model_ -- --test-threads=1 | 1 项业务失败，确认缺失 Chat 路由 |
| cargo test -p llm-runtime --lib deepagents_ -- --test-threads=1 | 初次及工作区修复后均 4 通过，含既有缓存集合身份回归 |
| cargo test -p llm-runtime --lib deepagents_private_model_configuration_ -- --test-threads=1 | 1 失败，确认未传工作区；随后上述完整过滤转绿 |
| cargo test -p acp-agent-registry --lib deepagents_recoverable_host_errors_ -- --test-threads=1 | 1 失败，确认旧安装目录不能发布新合同 |
| cargo test -p acp-agent-registry --lib managed_runtime::tests -- --test-threads=1 | 2 通过、2 明确忽略（需供应商安装/网络）；真实系统安装另由 API 完成 |
| cargo clippy -p llm-runtime -p acp-client -p acp-agent-registry --lib --no-deps -- -D warnings | 退出 101，llm-runtime 101 项既有诊断，无本次 DeepAgents/守卫/安装目录/进程隔离文件诊断；未扩大修复无关 lint |
| cargo build -p vw-cli --bin shiju | 三轮均成功，最终含工作区与宿主文件错误修复 |

`node patches/acp-model-source/deepagents/guard_tests.mjs <审核包或私有夹具目录>`：新增宿主错误用例先以原守卫失败，再以新守卫通过；覆盖 5 类权限结果、取消、宿主撤销及 7 类未审核工具不执行。`python3 scripts/acp_onboarding_deepagents_acceptance.py <私有新审核包入口>` 最终退出 0，覆盖真实拒绝/允许写入、错误工具、MCP/工作区扩张拒绝、同进程恢复、取消续用和认证/模型错误不回退。夹具此前期待宿主拒绝导致整轮 fatal 的旧断言失败，已改为明确检查工具错误、无敏感详情、无越权文件及后续成功，并重新取得通过证据。

未运行桌面构建、整仓测试/构建、全特性 Clippy、全仓格式检查、wasm 或 Makefile；仅格式化修改文件。任务 23 文件创建验收完成，严格 lint 既有失败与配置存储差异如上保留。


# DimCode 验收（2026-11-17）

结论：**已阻塞，未通过**。任务 23 最新扩围验收已通过并记录配置恢复、自有进程退出；来源任务 3 已删除，接入证据明确 session_model 尚未开放。本次新核对不能由历史接入通过替代。

- RustCodeGraph 索引 12,369 文件；经 node 读取当前生产驱动及注册、派生目录。注册位于 `crates/libs/acp-agent-registry/src/agent_registry.rs`，固定 dimcode@0.5.28，本地 dim acp；安装白名单位于 `crates/libs/config-acp-install/src/lib.rs`，固定 npm 安装同版本。`crates/libs/agent-config/src/catalog.rs` 派生 builtin=true、backend=dimcode、默认停用的 acp-dimcode。以上是源码核对，未冒称本次系统安装或配置保存通过。
- 当前生产 `crates/libs/llm-runtime/src/acp/model_source/drivers/mod.rs` 无 DimCode 的 verified_release、source_protocol 和 configure 分支；capabilities 对 session_model 返回 UnsupportedCli，协议路由亦明确拒绝未验证驱动。该文件 SHA-256 为 `3646c08fed609baee841dc211de5b649bc9ead5014af961d846ef3ef37ff302d`。新增完整私有中继驱动超出本验收小修范围；没有替换指定模型、租户、引擎或改用 CLI 默认模型。
- Rue.js 的 packages/rue/package.json 存在，acpdemo/dimcode/ 起初不存在。初始 Git 状态及排除 .git/node_modules/target 的 6,245 个普通文件 SHA-256 快照已采集；没有写入 Rue.js。未做结束快照，不将静态只读检查声称为完整目录边界验收。
- 未执行安装、普通诊断、模型来源 API、配置 PATCH、SSE、会话绑定、产物创建验收；没有运行版本和二进制摘要可报告。未启动网关、供应商进程或领取编译槽位，未改配置，无本次运行环境需恢复。纯文档记录，未运行 Cargo、定向测试、Clippy、桌面或页面构建、浏览器；没有把历史测试结果算作本次通过。
- 本次源码副本、摘要、初始快照及工作区状态仅存本机 0700 目录 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-dimcode-20261117-wzy8qvop`，未提交原始配置、敏感请求或供应商二进制。任务引用的 .plans/references/rust.md 不存在，本轮未调用 Cargo。

重跑条件：交付并验证 DimCode session_model 私有中继驱动，使用包含该实现的已验证构建，从任务 24 步骤 1 重跑。任务文件保持已阻塞并保留。


# DimCode 验收（2026-11-17 扩围恢复）

结论：**指定模型文件创建验收通过**。用户追加扩围授权后，本任务补齐宿主 session_model 驱动与 DimCode 只读初始化的底层适配；此结论替代此前能力缺失阻塞。严格 Clippy 的既有失败单独保留。

## 实现与权限边界

- `crates/libs/llm-runtime/src/acp/model_source/drivers/dimcode.rs` 使用现有 RelayBinding 的租户、项目、会话、provider/model 身份与短期中继令牌，明确 OpenAI Chat 协议；仅写请求私有 DIMCODE_HOME/v2 的 0600 config/providers 文件，目录 0700。设置主模型、辅助/子代理模型及原生 session-selected/<选定模型>，不借用 CLI 默认模型。拒绝自定义参数/环境、非回环中继和 .dimcode/providers.json 项目覆盖。受管子进程沿用 OS 环境白名单，保留 HOME，隔离模型凭据、代理及注入变量。
- 固定发行物的 ACP session/new 硬编码 read-only；首轮正确绑定模型但写入被 capability_rule 拒绝。新增 `acp-client/src/client/dimcode_access.rs`：仅当前 full_access + ApproveAll 授权时同步 workspace-write，否则同步 read-only；拒绝原生 full-access。回读不确认则阻止 prompt，权限控制不得改变模型，保存权限不能恢复撤销授权。文件读写继续通过宿主 RPC 与原有校验；仍报告 client_only，未发布 Native 免审批能力。
- 原生私有配置探针确认 Chat 请求确实使用 selected-model，两轮 end_turn；宿主拒绝写入后仍可继续，允许写入后私有夹具实际落盘。只用于配置/宿主边界验证，不替代真实模型验收。

## 系统安装与构建

- 固定 DimCode 0.5.28。系统 POST 安装两轮均 HTTP 200、客户端退出 0；实际入口 `/opt/homebrew/Cellar/node/26.13.0_2/bin/dim`，解析到同目录 lib/node_modules/dimcode/bin/dim.mjs，--version 退出 0。入口 SHA-256 `ba7e8e41d23b53ca01c984336e54bfb2813601cf9e6f80748f98804d0443407f`，Darwin arm64 原生文件 SHA-256 `0d6298cc8881b15ae8cdf1901b6f39674b74dbf5c14caedadfe509ab5ae114b0`。
- 首轮安装基线 false，安装后发现仍 false：npm 使用实际 Homebrew Node 的 Cellar 全局前缀，入口未在网关 PATH。已定位文件而非误报安装成功；结束首轮后，仅为第二轮自有网关 PATH 补入该已安装 Node bin 目录。未改主机 npm 配置、用户服务或绕开系统安装。第二轮安装前后均 true，重读确认。原正常启动环境若未包含该目录，仍可能无法发现 dim，只能使用固定 npx 回退；未声称这一通用发现问题已修复。
- 普通诊断均 HTTP 200、connected；模型来源接口明确 session_model supported，审核平台 Darwin arm64 / 0.5.28。普通诊断没有计为真实模型成功。
- 使用槽位 4 新构建 shiju 0.6.0，最终 `cargo build -p vw-cli --bin shiju` 退出 0，2m16s；二进制 SHA-256 `fd909abe08985bac9cd36f2fbafc465cecaa213f330e367efd29f5b68c1d152c`。源码摘要与构建时 HEAD 存在 source-hashes-final.json，共享工作区其他差异未纳入本任务提交。

## 真实会话与文件

- 所有请求使用 X-Platform-ID=10001、X-Company-ID=100010005、X-Directory=/Users/xiangmin/worker/code/ruejs。读取 installed、完整 agent 目录及 acp 配置基线；确认 acp-dimcode 的 builtin=true、backend=dimcode，临时 enabled=true 后 GET 重读成功。请求 JSON 沿用任务指定模型、预设、后端、session_model、cwd 和 full_access=true。
- 首轮会话 `ses_1e9713408001WlO2ME4hafpWWK`：初始 HTTP 200、curl 退出 28、300.011222 秒；查询状态并核对续传实现后，使用原请求标识与 Last-Event-ID=296 续传，HTTP 200、curl 0、176.801317 秒。原流合并取得 chat.done，无 chat.error，但只读权限拒绝全部写入，目标不存在，**明确不计通过**。没有因超时重新提交创建任务。该轮结束普通文件快照 0 差异，配置恢复，网关退出 0。
- 修复后另行重跑，原目标仍不存在，没有覆盖用户产物。HTTP 200、curl 0、134.517361 秒；chat.context_loaded×1、chat.step_start×1、chat.delta×84、chat.tool×66（状态事件，非 66 个独立调用）、chat.session_updated×2、chat.step_finish×1、chat.done×1，chat.error=0。
- 实际会话 `ses_1e9822d26001wN9tt0onLDZeLM` 的运行中及结束 summary/options 均 HTTP 200。绑定 acp-dimcode/dimcode、full_access=true，运行中 connected=true，selected_model.source=session_model、provider_id=alibaba-token-plan-cn、完整模型 ID=alibaba-token-plan-cn/deepseek-v4.1-flash-1017；结束 connected=false，选定模型仍匹配。access_status 为 client_only，未把完全访问请求当成 Native 权限认证。
- 产物 `/Users/xiangmin/worker/code/ruejs/acpdemo/dimcode/` 共 7 文件：README.md、index.html、vite.config.mjs、src/main.tsx、src/App.tsx、src/env.d.ts、src/style.css。已阅读实际源码：Vite 复用仓库 examples/shared/rue-vite.mjs，其 Rue 别名指向 packages/rue/src；useApp 挂载、ref/computed 状态、textarea onInput 实时统计、localStorage 初始化恢复和 400ms 防抖保存均在文件内，README 提供根目录运行命令。没有运行页面、浏览器、页面构建或额外页面回归，不声称已实际渲染。

## 环境恢复与证据

- 成功重跑前 6,245 个普通文件，结束 6,252 个，快照排除 .git/node_modules/target；仅上述目标目录新增 7 文件，目录外 0 差异，见 before.json、after.json、boundary.json。首轮快照另存 first-run/before.json、first-after.json。
- 两轮临时 enabled 均在 finally 恢复并 GET 重读。最终第二轮整份 config 与其基线相等，agent 有效配置相等、enabled=false。最初首轮无显式预设覆盖项，首次保存后保留等效 enabled=false 覆盖，因此不声称恢复首轮存储字节；未改其他预设、默认模型或权限策略。
- 两轮自有网关分别监听 60110、53600，均退出 0；最终自有进程已结束、端口关闭、槽位控制进程退出，锁释放，缓存保留。没有重启用户服务，系统安装结果保留。
- 原始请求、响应、SSE、配置、原生探针、快照及源码摘要仅存本机 `/tmp/shiju-dimcode-20261117-retry`（0700），首轮证据在 first-run/；此前静态核对证据目录也保留。未提交凭据、敏感原始载荷、供应商二进制或 Rue.js 工作区。

## 定向验证

Rust 重型命令均经 scripts/cargo_slot.py 的 slot(number=4)，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-4`，串行执行。首次选择被占槽位 3 时仅中断尚未取得锁的自有排队进程；没有操作他人锁或缓存。任务旧引用 .plans/references/rust.md 不存在，使用现有槽位入口。

| 原生子命令（槽位内） | 结果 |
| --- | --- |
| cargo test -p llm-runtime --lib dimcode_selected_model_ -- --test-threads=1 | 1 失败，重现无驱动协议路由 |
| cargo test -p llm-runtime --lib dimcode_ -- --test-threads=1 | 初始 3 通过；项目覆盖红灯修复后 4 通过 |
| cargo test -p llm-runtime --lib dimcode_rejects_project_ -- --test-threads=1 | 1 失败，重现项目 provider 覆盖 |
| cargo test -p acp-client --lib dimcode_ -- --test-threads=1 | 先 3 项业务失败，再 3 通过，补充控制失败/只读同步后最终 5 通过；早期夹具 collect 类型编译错误已修正，不作为业务红灯 |
| cargo clippy -p llm-runtime -p acp-client -p acp-agent-registry --lib --no-deps -- -D warnings | 两轮均退出 101，llm-runtime 101 项既有诊断，无本次 DimCode/权限/环境隔离修改文件诊断 |
| cargo build -p vw-cli --bin shiju | 两轮均退出 0，最终含权限同步修复 |

只格式化新增 Rust 文件，未运行桌面编译、整仓构建/测试、全特性 Clippy、全仓格式检查、wasm 或 Makefile。上述既有严格 lint 失败不影响本次实机文件创建结论，但不被描述为通过。

成功重跑 API 耗时补充：new-install：HTTP 200，客户端退出 0，1.341 秒；new-diagnostic：HTTP 200，客户端退出 0，0.780 秒；capability：HTTP 200，客户端退出 0，0.001 秒；enable：HTTP 200，客户端退出 0，8.208 秒；restore：HTTP 200，客户端退出 0，6.489 秒。


# Dirac 验收（2026-11-17）

结论：通过本次“完整文件创建为终点”验收。普通诊断返回 authentication_required，未计作通过；指定 session_model 的真实会话独立成功。

- 前置核对：来源任务已不存在，已读 acp-onboarding-dirac.md、差异清单、当前注册/安装契约及模型驱动；任务24已有完成记录，环境已清理。当前代码拆分到 acp-agent-registry、config-acp-install、llm-runtime；旧 .plans/references/rust.md 不存在，沿用 scripts/cargo_slot.py。
- 复用任务24的 shiju 0.6.0 槽位4构建，SHA-256 `fd909abe08985bac9cd36f2fbafc465cecaa213f330e367efd29f5b68c1d152c`；构建证据在 /tmp/shiju-dimcode-20261117-retry/build.log，来源 HEAD f72f5c2e8f00ad7a0927eb8b2cb740e102ba893b。Dirac 驱动、注册及安装文件与该来源无差异，本次源码摘要存 source-hashes.json。未重新编译或运行无关 Cargo 检查。
- 系统安装 POST /v1/desktop/acp-install/dirac：HTTP200、客户端退出0、6.734秒，installed 从 false 到 true。真实入口 /opt/homebrew/lib/node_modules/dirac-cli/dist/cli.mjs，版本0.5.24，SHA-256 `c88e043f590a520da3345050e32bbe6930a68cefad60945ef4a7d14648217429`。临时网关 PATH 使用已存在 Node24.14.0，未替换模型或安装链路。
- 普通诊断 HTTP200、退出0、1.829秒，结果 authentication_required；模型来源接口 session_model=supported，审核平台 Darwin arm64、版本0.5.24。acp-dirac 的 builtin=true、backend=dirac，基线 disabled；临时启用 PATCH HTTP200、9.218秒，并 GET 重读确认。
- 请求严格沿用任务JSON及三个指定租户/目录头。首次流 HTTP200、curl退出28、300.006774秒。查询会话已 idle，使用原 x-vw-stream-request-id 与 Last-Event-ID=346 续传；HTTP200、退出0、0.023489秒，没有重复提交创建任务。合并事件：chat.context_loaded=1、chat.step_start=1、chat.delta=213、chat.tool=131（状态事件）、chat.session_updated=2、chat.step_finish=1、chat.done=1、chat.error=0。
- 实际会话 `ses_1e98f062f001U6qDpLkF0H3EWc`：summary确认 acp-dirac/dirac、full_access=true；运行中 options connected=true，selected_model.source=session_model、provider_id=alibaba-token-plan-cn、完整模型ID=alibaba-token-plan-cn/deepseek-v4.1-flash-1017。结束 connected=false，选定模型保持。access_status=client_only，不宣称 Native 免审批已认证。
- 实际产物 /Users/xiangmin/worker/code/ruejs/acpdemo/dirac/：README.md、index.html、vite.config.mjs、src/main.tsx、src/App.tsx、src/env.d.ts、src/style.css。已读源码与说明：useApp挂载、ref/computed、textarea onInput、实时字数、localStorage同步保存与初始化恢复均存在；Vite复用 examples/shared/rue-vite.mjs，别名指向 packages/rue/src。README含根目录运行命令，末尾文件树代码块未闭合（排版瑕疵，不影响源码/运行说明）。未运行浏览器、页面构建或页面测试，不声称实际渲染通过。
- before.json/after.json为排除.git、node_modules、target的普通文件SHA-256快照；仅目标目录新增上述7文件，目录外0差异。未覆盖既有目标，未提交Rue.js工作区。
- 恢复：urllib路径多次返回408，首次恢复脚本因空响应JSON解析退出；重启自有网关后仍408。最终 curl --noproxy '*' 直连 PATCH enabled=false 返回HTTP200、退出0，GET重读disabled，与基线完整有效智能体配置相等。未将中间失败隐藏，尚未定位408根因；基线无显式覆盖项，最终保留等效disabled覆盖，不宣称存储字节一致。系统安装保留，未改其他预设或默认模型。
- 清理：自有网关端口61356、49398均已退出0，slot4锁释放、缓存保留，未重启用户服务。全部原始配置、请求、响应、SSE、摘要和快照仅存本机 /tmp/shiju-dirac-accept-20261117（0700）；未提交凭据或供应商二进制。
- 本次无产品代码修复，复用已有验证产物；仅做文档轻量差异检查，不重新运行单测、Clippy、桌面编译、全仓构建或额外页面回归。历史单测不是本次新增验证证据；本次证据为系统API、真实模型工具调用、落盘源码和恢复重读。


# GLM Agent 验收（2026-11-17）

结论：通过本次“完整文件创建为终点”验收。普通诊断 authentication_required 单独记录，不作为真实模型成功依据。

- 前置：来源任务5已不存在，已核对接入证据、差异清单、当前注册/安装白名单、派生预设及模型驱动；任务25已有通过及清理记录。代码已拆分至 acp-agent-registry、config-acp-install、agent-config、llm-runtime。旧 .plans/references/rust.md 不存在，使用现有槽位约定。
- 构建：复用任务24/25的 shiju 0.6.0，SHA-256 `fd909abe08985bac9cd36f2fbafc465cecaa213f330e367efd29f5b68c1d152c`；构建日志与源码摘要见 /tmp/shiju-dimcode-20261117-retry。当前 GLM 驱动/注册/安装链路已核对，drivers/mod.rs 与构建源码摘要一致；该文件相对构建时 HEAD 的差异仅为当时已纳入构建的 DimCode 接入。槽位4被其他编译占用，退出尚未取得锁的自有排队进程，复制并复核上述既有二进制到0700私有目录运行；没有使用可被并发替换的槽位路径启动网关，没有操作他人锁或缓存。
- 所有请求使用 X-Platform-ID=10001、X-Company-ID=100010005、X-Directory=/Users/xiangmin/worker/code/ruejs。保存 installed、agent、acp及config基线。系统安装 POST HTTP200、客户端退出0、1.817秒，installed false→true；真实入口 /opt/homebrew/lib/node_modules/glm-acp-agent/dist/index.js，package.json版本1.17.1，SHA-256 `d0d9237bffcbd5017a0085aa3c4625c29e2b750f87c358fa9c455d3cd4cdbd98`。未采用其他安装链路。
- 普通诊断 HTTP200、退出0、0.998秒，authentication_required。模型来源接口确认 session_model supported、审核版本1.17.1、Darwin arm64。确认 acp-glm-acp-agent 的 builtin=true、backend=glm-acp-agent，基线disabled；临时启用PATCH HTTP200、11.951秒，GET重读enabled。
- 请求严格采用任务指定JSON。首流HTTP200、curl退出28、300.005292秒；查询实际会话已idle，核对现有续传实现后以原 x-vw-stream-request-id 和 Last-Event-ID=66 续传，HTTP200、退出0、91.986313秒，没有重复提交创建任务。合并事件：chat.context_loaded=1、chat.step_start=1、chat.delta=23、chat.tool=60（状态事件数）、chat.session_updated=2、chat.step_finish=1、chat.done=1、chat.error=0。
- 会话 `ses_1e9a518ad001uwV3EiO1cB8m3e`：运行中summary确认 acp-glm-acp-agent/glm-acp-agent、full_access=true；options connected=true，selected_model.source=session_model、provider_id=alibaba-token-plan-cn、完整模型ID=alibaba-token-plan-cn/deepseek-v4.1-flash-1017。结束及重启自有网关后回读connected=false，selected_model仍匹配。权限support=client_only，不宣称Native免审批通过。
- 实际产物 /Users/xiangmin/worker/code/ruejs/acpdemo/glm-acp-agent/ 共8文件：README.md、index.html、vite.config.mjs、tsconfig.json、src/main.tsx、src/App.tsx、src/env.d.ts、src/style.css。已检查源码与说明：useApp挂载、ref/computed、textarea onInput实时统计、localStorage逐次保存和初始化恢复；Vite复用仓库examples/shared/rue-vite.mjs，别名指向packages/rue/src。README提供根目录启动命令。未运行浏览器、页面构建或额外页面测试，未宣称实际渲染通过。
- 初始目标不存在，未覆盖既有文件。普通文件SHA-256快照排除.git/node_modules/target：6259→6267，仅目标目录新增上述8文件，目录外0差异；before.json、after.json、boundary.json保留。未提交Rue.js工作区。
- 恢复与清理：启动早期首次配置GET超时60秒、curl探测超时15秒，网关就绪后重试成功。首次恢复urllib超时，curl恢复返回HTTP408，均未计成功；重启自有网关后curl PATCH返回HTTP200、退出0、20.886秒，GET重读enabled=false，与基线完整有效智能体配置相等。基线无显式覆盖项，最终保留等效disabled覆盖，不宣称存储字节一致。未改其他预设或默认模型，系统安装保留。自有网关均正常退出0，未重启用户服务、未持有槽位锁。
- 本次无产品代码修复，复用已验证构建，只运行本次系统API、真实会话、源码/目录快照及文档轻量检查；未运行新单测、Cargo检查、Clippy、桌面编译、整仓构建或额外页面回归，不将历史单测描述为本次证据。
- 全部原始请求、配置、响应、SSE、快照、源码摘要与本机进程采样仅存 /tmp/shiju-glm-accept-20261117（0700）。没有提交凭据、原始敏感载荷、供应商或宿主二进制。配置接口早期超时/408根因尚未确定，已保留失败证据与成功恢复回读。


# Grok Build 验收（2026-11-17）

结论：**已阻塞，未通过**。本次卡在临时网关启动阶段，没有完成系统安装、指定模型会话或文件创建；不将来源接入证据或前序验收当作本次成功。

- 前置：任务26已有 GLM 通过及环境清理记录，来源任务6已不存在；已读 Grok 接入证据、差异清单、只读 plan.md。通过 RustCodeGraph 核对当前 acp-agent-registry 注册、config-acp-install 白名单、agent-config 派生预设及 llm-runtime 驱动。grok 的固定备用/系统安装版本为 @xai-official/grok@1.0.77，本地入口优先 grok agent stdio；acp-grok 派生 builtin=true、backend=grok、默认停用。源码声明 session_model、Responses 中继及 Darwin arm64 审核边界，以上未冒称本次 API 或真实模型通过。
- 构建：复用任务24/25/26已验证的 shiju 0.6.0，SHA-256 `fd909abe08985bac9cd36f2fbafc465cecaa213f330e367efd29f5b68c1d152c`，复制到本次0700私有目录后复核摘要。构建来源 HEAD f72f5c2e8f00ad7a0927eb8b2cb740e102ba893b，日志在 /tmp/shiju-dimcode-20261117-retry/build.log。Grok 驱动、注册、白名单与派生目录逐文件核对与该来源相同，drivers/mod.rs 摘要与构建时 source-hashes-final.json 相同；未用旧实现推断新实现结果，未占用槽位或重复编译。
- 两轮自有临时网关使用现有开发配置与回环空闲端口51639、52783，启动参数先由当前 gateway --help 核实。配置基线请求携带 X-Platform-ID=10001、X-Company-ID=100010005、X-Directory=/Users/xiangmin/worker/code/ruejs。两轮 GET /v1/global/config 均客户端退出28、HTTP000，60秒无响应；首轮另一次重试也超时，/health探测15秒超时。重启交接时一次过早连接退出7，未计作有效接口结果。没有取得配置基线，没有提交任何安装、诊断、PATCH 或创建请求。
- 两次原生进程采样均显示 serve_gateway → all_tools_with_runtime → SkillTool::new_with_runtime_config → accessible_skills → block_on(SkillHost::permission_rules) → scoped thread join 等待；子线程等待 Runtime::block_on 的权限规则 future。当前对应位置为 crates/libs/tool-services/src/skill/mod.rs:684，启动调用在 crates/vw-agent/src/gateway/bootstrap.rs。此为复现等待路径的采样证据，尚未证明完整死锁根因；未关闭技能、降低权限或替换租户/模型来绕过。通用网关启动问题超出本接入验收小修范围，未修改产品代码。
- Rue.js packages/rue/package.json 存在，acpdemo/grok/ 起初不存在且结束仍不存在。初始 Git 状态已保存；普通文件 SHA-256 快照排除 .git、node_modules、target，前后均6267文件、0差异，未覆盖或创建用户产物。
- 未执行 installed/agent/acp API 基线、系统安装与真实入口版本核验、普通诊断、模型来源 API、预设启用保存重读、SSE/续传、会话身份/模型绑定及产物源码检查；不能报告供应商安装版本或实际会话终态。未改 enabled 或其他配置，无临时配置需恢复，也没有成功 GET 的恢复证明。没有运行单测、Cargo、Clippy、桌面构建、浏览器、页面构建或额外页面测试；本次仅复用已验证构建并执行API探测、进程采样和文件快照。任务旧引用 .plans/references/rust.md 不存在，本轮未调用 Cargo。
- 原始请求、响应指标、两次采样、源码/二进制摘要、工作区状态及前后快照仅存本机 /tmp/shiju-grok-accept-20261117（0700），首轮失败文件在 startup-failed/。未提交凭据、完整配置、原始敏感载荷、供应商二进制或 Rue.js 工作区。

清理：首轮自有网关收到SIGTERM后15秒未退出，脚本对自有进程组升级SIGKILL，退出-9；第二轮收到SIGTERM退出0。两个自有PID均已不存在、端口均关闭，没有重启用户服务，没有领取或遗留槽位锁。未修改临时enabled配置，无待恢复配置。

重跑条件：先解决并验证现有开发配置下网关启动的权限规则等待，再使用来源可核验构建从任务27步骤1重跑；保留任务文件为已阻塞，不以静态能力声明或历史测试替代本次端到端证据。


# Grok Build 验收（2026-11-17 扩围重跑）

结论：**指定模型完整文件创建验收通过**，替代前述启动阻塞结论。用户追加继续排障及扩围修复授权后，本次修复了两处 Grok 模型驱动问题；未替换指定模型、租户或引擎，未降低权限策略。严格 Clippy 的既有失败单独保留。

## 启动排障与底层修复

- 复用原已验证构建重试时，仅为自有临时网关设置 TOKIO_WORKER_THREADS=4，并将调试日志限制在0700私有目录，网关及配置API恢复响应。没有修改持久配置或重启用户服务；这是本次成功启动的运行条件，不声称默认线程数下的等待根因已修复。
- 首轮会话 ses_1e9c18db0001uIlg1e6MhjC9yX 的 Grok 在 initialize 前退出1。隔离原生探针复现 `models.max_retries` 类型错误：serde_json arbitrary_precision 的数字0跨格式序列化成 `$serde_json::private::Number` 表。grok.rs 改为原生 TOML 表与整数生成配置，恢复校验同步使用 TOML 值，保留私有目录/文件权限、环境令牌及模型绑定校验。新增独立 grok_tests.rs，先取得真实失败，再通过。
- 该修复构建 SHA-256 06c9d349d8f5b0bd5c7df4a240880898fae9d6e6dda155ace9d085086699642e。第二轮 ses_1e9dc224d001a6QwrngQx1nKpQ 已 connected 且指定模型绑定正确，但 prompt 因 Responses 转换层422 Unsupported Responses capability结束，明确不计通过。真实1.0.77的隔离请求捕获确认默认 include、prompt_cache_key、reasoning.summary=concise 超出现有通用转换契约，未放宽转换器白名单或借用 Codex 特例。
- [官方自定义模型说明](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/11-custom-models.md)提供原生 chat_completions；本地真实1.0.77探针确认实际 Chat 请求。grok.rs 的受管模型配置与 drivers/mod.rs 的 source_protocol 同步改为原生 Chat，所选模型仍绑定相同宿主中继；supports_reasoning_effort 按所选模型能力设置，不制造能力支持。凭据仍仅进入子进程环境，主/辅助模型继续绑定同一选定模型，CLI子代理与共享leader继续禁用。
- 两轮失败均已正常终止，未创建目标文件；普通文件快照均0差异，配置恢复并GET确认有效预设与基线相等。首轮基线无显式覆盖，恢复后保留等效disabled覆盖，不宣称其存储字节一致。最终成功轮使用新的基线，完整config恢复相等。

## 最终构建、系统链路与指定模型

- 槽位6最终构建 shiju 0.6.0，cargo build -p vw-cli --bin shiju 退出0，49.43秒；二进制 SHA-256 `791a91fc449e433c470d9999344161d681bcc7a15bbc5b54c65949f1e315ebea`。原生TOML首轮构建亦退出0、5m59s。构建前后核对 Grok 驱动/测试、模型分派、注册、安装白名单、派生目录、客户端与网关启动源码摘要，无变化；来源HEAD及摘要存 source-hashes-chat-build.json。复制产物到私有目录后运行，未从可被并发替换的槽位路径启动网关。
- 所有验收API使用 X-Platform-ID=10001、X-Company-ID=100010005、X-Directory=/Users/xiangmin/worker/code/ruejs。保存 installed、agents、acp和config基线。POST /v1/desktop/acp-install/grok：HTTP200、客户端退出0、2.917秒，installed true→true；实际入口 /Users/xiangmin/.grok/bin/grok，解析到 grok-1.0.77，--version为 grok 1.0.77 (50ca579fa996)、退出0，文件 SHA-256 `197c66a63e5ac46e07450b0ca21d141ce665050a412180f185dadfae1c7a5b2a`。安装结果保留，没有使用替代安装链路或任意URL。
- POST /v1/desktop/acp-diagnostic/acp-smoke-grok 使用当前对应ACP配置：HTTP200、退出0、7.886秒，authentication_required，仅作为普通诊断结果。模型来源API独立确认session_model supported；本次真实执行版本1.0.77，不将目录列出的历史1.0.24当成本轮实测。
- acp-grok 的 builtin=true、backend=grok，基线disabled；临时PATCH enabled=true：HTTP200、退出0、8.862秒，GET重读enabled。真实请求严格沿用任务JSON、指定模型与full_access=true，未改为CLI默认模型。
- POST /v1/chat/stream：HTTP200、curl退出0、207.714567秒，无超时或续传，无重复提交同一运行中的创建任务。事件：chat.context_loaded=1、chat.step_start=1、chat.delta=564、chat.tool=75（状态事件数）、chat.session_updated=2、chat.step_finish=1、chat.done=1、chat.error=0。
- 成功会话 `ses_1e9e43208001LJpsZIhvgFJPLl` 的运行中及结束summary均确认 acp-grok/grok、full_access=true。运行中options connected=true、selected_model.source=session_model、provider_id=alibaba-token-plan-cn、完整模型ID=alibaba-token-plan-cn/deepseek-v4.1-flash-1017；结束connected=false，选定模型仍匹配。access_status.support=client_only，未声称Native免审批认证。

## 实际文件、目录边界与恢复

- /Users/xiangmin/worker/code/ruejs/acpdemo/grok/ 新增8文件：README.md、index.html、vite.config.mjs、tsconfig.json、src/main.tsx、src/App.tsx、src/env.d.ts、src/style.css。已逐文件读取：useApp挂载、ref/computed、textarea onInput 实时统计、localStorage 初始化恢复、250ms防抖保存及pagehide/卸载补写存在；README提供根目录启动与运行说明。Vite复用examples/shared/rue-vite.mjs，Rue别名指向packages/rue/src，确实引用本仓库 @rue-js/rue。
- 成功轮目标起初不存在，未覆盖既有产物。排除.git/node_modules/target的普通文件SHA-256快照6267→6275，仅目标目录新增上述8文件，目录外0差异；before.json、after.json、boundary.json保留。没有提交Rue.js工作区。
- 恢复PATCH：HTTP200、退出0、7.849秒；GET重读enabled=false，完整config及有效智能体配置均与成功轮基线相等。未修改其他预设、默认模型或权限策略。
- 本轮扩围自有网关端口55805、51639、55158均退出0，三个自有PID均不存在、端口均关闭；原生探针与捕获服务亦已退出。两个槽位6检查控制进程均正常结束并释放自有锁，缓存保留，未操作他人进程/锁。最初等待槽位4时仅取消尚未取得锁的自有排队进程，130不算测试证据。
- 全部原始配置、请求/响应、SSE、探针、日志、源码/二进制摘要及快照仅存 /tmp/shiju-grok-retry-20261117（0700）：根目录为首次握手失败，final/为Responses转换失败，chat-final/为最终成功轮，capture*/为隔离原生请求探针。此前阻塞证据仍保留 /tmp/shiju-grok-accept-20261117。未提交凭据、完整配置、原始敏感载荷或二进制。

## 定向验证与边界

Rust重型命令均经scripts/cargo_slot.py的slot(number=6)串行执行，CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为 /Users/xiangmin/worker/code/shiju/target/rust-slot-6。任务旧引用.plans/references/rust.md不存在，使用现有槽位入口。

| 槽位内原生命令 | 本次结果 |
| --- | --- |
| cargo test -p llm-runtime --lib grok_generated_config_ -- --test-threads=1 | 修复前1失败；原生TOML修复后1通过 |
| cargo test -p llm-runtime --lib grok_ -- --test-threads=1 | 原生Chat修复前2通过/2失败；修复后4通过，含既有私有绑定冲突与恢复设置测试 |
| cargo clippy -p llm-runtime --lib --no-deps -- -D warnings | 两轮均退出101，101项既有诊断，无本次Grok驱动、测试或分派文件诊断；未计通过 |
| cargo build -p vw-cli --bin shiju | 两轮均退出0，最终49.43秒 |

仅格式化修改的Grok驱动与新测试文件。未运行桌面编译、整仓测试/构建、全特性Clippy、全仓格式检查、wasm或Makefile；没有运行浏览器、页面构建或创建后的额外页面回归，不宣称实际渲染或页面执行已通过。任务27按完整文件创建验收完成；默认线程数启动等待与既有严格lint问题仍保留其明确边界。

# Nova 验收（2026-11-17）

结论：**已阻塞，未通过**。本次模型来源API明确拒绝指定的 session_model，不以安装成功、普通诊断或历史受控模型测试替代真实指定模型文件创建。

- 前置：任务27已有完整文件创建通过及配置恢复、进程清理记录；来源任务7已不存在，已读差异清单、只读plan.md及docs/dev/evidence/acp-onboarding-nova.md。使用RustCodeGraph核对模型驱动与安装白名单、注册目录；当前模块已迁移至acp-agent-registry、config-acp-install、agent-config、llm-runtime。历史Nova接入明确只验证CLI本地模型路径，任意宿主模型绑定保持UnsupportedCli。
- 构建：复用任务27最终shiju 0.6.0，复制到本次0700私有目录后核对SHA-256 `791a91fc449e433c470d9999344161d681bcc7a15bbc5b54c65949f1e315ebea`。来源HEAD `888e83caa02341506b8e52bce0b52d9e17361170`，构建日志位于/tmp/shiju-grok-retry-20261117/chat-build.log；当前模型分派、注册、安装白名单、派生目录、进程启动与网关bootstrap摘要全部匹配source-hashes-chat-build.json，没有用未知来源旧产物判断新行为。
- 自有网关使用现有开发配置、回环端口64076，启动前读取gateway --help；沿用前序成功运行条件TOKIO_WORKER_THREADS=4，未修改持久线程配置。所有API携带X-Platform-ID=10001、X-Company-ID=100010005、X-Directory=/Users/xiangmin/worker/code/ruejs，取得config、installed、agents、acp基线。
- POST /v1/desktop/acp-install/nova：HTTP200、客户端退出0、11.164126秒，installed false→true。实际入口 /opt/homebrew/lib/node_modules/@compass-ai/nova/scripts/bin-wrapper.mjs，package.json版本1.1.65，入口SHA-256 `82f4ea07714bc0227d9c7206420acc1fdb41cd2f4a2886d1146db47af456119b`。使用系统固定安装链路，没有临时替换URL或shell安装。
- 使用本次对应ACP配置POST /v1/desktop/acp-diagnostic/acp-smoke-nova：HTTP200、退出0、2.102539秒，authentication_required。GET /v1/acp-model-source/nova：HTTP200、退出0，session_model=unsupported、reason=unsupported_cli；cli_local=supported、connected=false。驱动verified_release没有nova，source_protocol/configure也没有nova分派；这是与历史能力边界一致的确定阻塞，不是诊断认证结果的推测。
- GET预设确认acp-nova、builtin=true、runtime.kind=acp、runtime.backend=nova，基线enabled=false。能力拒绝后未临时启用、未PATCH任何配置、未提交chat/stream创建请求；未执行SSE、会话summary/acp-options绑定核验、实际模型工具调用或文件内容验收。没有会话成功终态，也没有可宣称通过的页面产物；未改用CLI本地默认模型冒充指定模型。
- Rue.js packages/rue/package.json存在；acpdemo/nova/起初不存在，结束仍不存在。普通文件SHA-256快照排除.git、node_modules、target，前后均6275文件、0差异；初始Git状态保留。未覆盖用户文件，未提交Rue.js工作区。
- 清理：自有网关收到停止信号后退出0，PID40938不存在、端口64076关闭；未重启用户服务，没有取得槽位锁或清理缓存。系统安装保留；未改enabled或其他配置，无临时配置需恢复，未执行恢复PATCH或取得恢复GET证据。
- 原始配置、请求、响应、日志、构建/入口摘要、前后快照及清理证据仅存本机 /tmp/shiju-nova-accept-20261117（0700）。本次没有产品代码修复，复用已验证产物，仅执行系统API、入口元数据核验、进程及快照检查；未运行新单测、Cargo、Clippy、桌面编译、全仓构建、浏览器、页面构建或额外页面回归。未把历史单测描述为本次验证。

重跑条件：先独立实现并验证Nova的宿主指定模型中继，再从任务28步骤1完整重跑。任务文件保留已阻塞；未绕过当前能力门控或扩大本次验收为新驱动实现。

# Nova 验收（2026-11-17–18 扩围补丁重跑）

结论：**指定模型完整文件创建验收通过**，替代前述模型能力阻塞结论。用户明确授权参考 Cursor 使用补丁突破，并另行明确同意仅在私有验收目录通过官方 API 授予最小数据处理同意；未替换模型、租户或引擎，未降低审批策略。

## 补丁与失败修复

- 固定 `@compass-ai/nova@1.1.65`，沿用系统受管安装链路，在独立安装中校验官方 npm archive integrity、原始 bundle SHA-256及补丁上下文；安装后验证入口、bundle和辅助模块摘要。没有修改全局供应商文件，没有使用任意URL或shell替代安装。只有本机Darwin arm64受管产物通过校验才开放session_model，其他情况保持显式拒绝。
- 新增Nova驱动及受管补丁：MC模型目录固定为所选模型，主/辅助模型解析绑定同一模型；供应商ACP保留的Auto项不作为验收依据。使用原生Messages协议连接宿主的私有回环中继，凭据仅进入子进程环境，JSON绑定文件0600、目录0700。启动参数、环境冲突、非回环URL和未审核入口失败关闭。通过ACP RPC显式选定 `anthropic/<宿主模型ID>`；anthropic为供应商协议目录命名空间，实际上游仍为所选宿主provider/model，不是换用Claude模型。
- 原有owner approval握手、供应商permission/request到标准审批的转换和失败终态处理保持不变。原生补丁探针两轮均完成，实际/v1/messages只请求selected-host-model且携带正确中继Bearer；HTTP401得到failed终态，没有回退。拒绝edit_file时文件保持before，后续轮仍完成；最终隔离探针的工作区仅有fixture.txt。
- 空白环境先复现原生Consent not granted，没有绕过。用户授权后，通过官方导出的getConsentManager/createMinimalConsent/grantConsent在 `/tmp/shiju-nova-patch-20261117/native-home/.nova/` 授权，回读valid=true、dataCollection=true，telemetry/analytics/errorReporting均false。显式状态目录经过0700、属主与符号链接检查；未修改用户全局Nova/Compass同意记录，最终确认两处全局consent.json仍不存在。补丁本身不自动授予同意。
- 首次真实模型轮已正常完成并创建7个页面文件，但发现供应商另在工作区新建7个.nova缓存、日志、学习记录和shell产物，因此该轮不计完整边界通过。逐文件确认均不在before快照、内容摘要仍匹配后，将本轮全部14个新文件移入私有first-run-owned-files，工作区恢复到6275文件、0差异；没有覆盖或删除既有用户文件。随后补丁将项目状态定位到私有目录，增加目录边界/符号链接回归；保留首轮失败证据。首次受管安装也移入私有first-managed-install，最终由系统API重新安装当前补丁，没有就地修改旧安装或用户全局CLI。

## 最终系统安装与指定模型

- 最终槽位6构建 `cargo build -p vw-cli --bin shiju` 退出0，13m19s。复制到私有目录后运行，二进制SHA-256 `b507ddebf25b76f2341a9e77c181415cbbbe964643d86389f9046e318082fad6`；Nova驱动、分派、受管安装器、契约和补丁的构建前后摘要一致，见source-hashes-final.json。不是未含补丁的历史产物；源码存在其他会话改动，不宣称clean HEAD构建。
- 全部验收API使用X-Platform-ID=10001、X-Company-ID=100010005、X-Directory=/Users/xiangmin/worker/code/ruejs，保存config、installed、agents、acp基线。最终系统安装POST HTTP200、客户端退出0、10.739381秒，installed true→true；本任务最初系统安装曾false→true。受管目录为现有开发配置下的acp-runtime/nova-1.1.65-model-source-v1，入口node_modules/.bin/nova解析到scripts/bin-wrapper.mjs，package.json版本1.1.65。
- 最终入口SHA-256 `82f4ea07714bc0227d9c7206420acc1fdb41cd2f4a2886d1146db47af456119b`，dist/cli.js为 `23ea8eeb9f3fca0a5fa2a9b1200aa1efa6a0f3a172e6c766b3388991dd135c93`，dist/shiju-session-model.mjs为 `21cd7af09568a2a915fedc08d53a576f0df2badf7e7ff5b2979e2a1d47e3c76e`；均与审核契约一致。
- 普通诊断HTTP200、退出0、0.654875秒，authentication_required，独立记录。模型来源API确认session_model supported、审核1.1.65、Darwin arm64；不能把普通诊断作为真实模型成功依据。
- acp-nova的builtin=true、runtime.kind=acp、backend=nova，基线disabled；临时启用PATCH HTTP200、退出0、6.899301秒，GET重读enabled。
- 请求严格沿用任务JSON，指定alibaba-token-plan-cn/deepseek-v4.1-flash-1017与full_access=true。POST /v1/chat/stream：HTTP200、curl退出0、176.668669秒，无超时、续传或重复提交。事件：chat.context_loaded=1、chat.step_start=1、chat.delta=3、chat.tool=48（状态事件）、chat.session_updated=2、chat.step_finish=1、chat.done=1、chat.error=0。
- 会话 `ses_1ea41e6ae001rpN1rPUXhvFkSa` 的运行中/结束summary确认acp-nova/nova、full_access=true；运行中options connected=true，selected_model.source=session_model、provider_id=alibaba-token-plan-cn、完整模型ID=alibaba-token-plan-cn/deepseek-v4.1-flash-1017。结束connected=false，所选模型保持匹配。供应商ACP版本自报1.0.0，npm实际包为1.1.65；权限support=client_only，不宣称Native免审批认证。

## 文件、恢复与清理

- 最终实际产物 `/Users/xiangmin/worker/code/ruejs/acpdemo/nova/` 共7文件：README.md、index.html、vite.config.mjs、src/main.tsx、src/App.tsx、src/env.d.ts、src/style.css。已检查源码和说明：useApp挂载、ref/computed、textarea onInput实时统计、每次输入localStorage保存及初始化恢复；Vite复用仓库examples/shared/rue-vite.mjs与指向packages/rue/src的别名，实际使用本仓库@rue-js/rue。README提供根目录运行说明。没有运行浏览器、页面构建或额外页面测试，不宣称实际渲染/刷新交互已实测。
- 最终目标起初不存在；排除.git/node_modules/target的普通文件SHA-256快照6275→6282，仅新增上述7文件，目录外0差异。修复后的.nova运行状态均进入私有目录；保留首轮越界失败及其产物，不将首轮计为通过。未提交Rue.js工作区。
- 配置接口多次408完整保留：首轮启用/恢复408，重启自有网关后启用200，首轮恢复重启重试200。首轮最初无显式覆盖，恢复保留等效disabled覆盖，不宣称存储字节一致。最终轮启用200；结束恢复PATCH HTTP408（48.659633秒），重启后的重试亦408（30.003708秒），随后GET证实Nova disabled且完整config与本轮基线相等，说明不能仅凭408推断未保存。
- 再次重启回读确认Nova仍disabled，全部agents与最终轮基线完全相等。此时仅channels_config.feishu_instances从空列表变为1项，故不宣称重启后的完整config仍相等；本任务PATCH只涉及agents.acp-nova，未请求修改飞书配置，未覆盖这项非本任务差异，也不臆断其外部写入/运行时投影来源。
- 六个自有网关PID96914/38290/91837/18438/43829/51893分别对应回环端口53279/56288/60225/54642/57921/59890，均正常退出0；最终核对PID不存在、端口关闭，见cleanup-final.json。没有重启用户服务。所有使用槽位产物的编译/测试进程均已结束并释放自有锁，缓存保留。
- 全部真实请求、配置、SSE、探针、源码/二进制摘要、快照和恢复证据仅保留本机0700目录 `/tmp/shiju-nova-patch-20261117/`：accept-final为首次文件完成但边界失败轮，boundary-final为最终通过轮；原始模型能力阻塞证据仍在/tmp/shiju-nova-accept-20261117。未提交凭据、完整配置、原始敏感载荷或供应商/宿主二进制。

## 定向验证

Rust重型命令经scripts/cargo_slot.py的slot(number=6)串行执行，CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-6`。仅格式化新增/修改的Nova Rust文件；旧任务引用.plans/references/rust.md不存在，使用现有槽位入口。一次切换排队入口时遇到获锁竞态，自有入口中断，核实其进程与编译子进程已退出后清理自己的遗留锁；该次不计通过，没有清理他人锁或缓存。

| 命令/检查 | 本次结果 |
| --- | --- |
| cargo test -p llm-runtime --lib nova_ -- --test-threads=1 | 驱动接入前1失败；最终1通过，483过滤 |
| python3 scripts/acp_model_source_nova_tests.py | 最终4通过，含供应商拒绝、绑定文件权限/地址、状态目录及项目目录/符号链接边界 |
| cargo test -p vw-acp --test onboarding_nova -- --test-threads=1 | 6通过、3需专用环境的真实CLI测试ignored；原生补丁探针和指定模型会话另行实际执行 |
| cargo clippy -p acp-agent-registry --lib --no-deps -- -D warnings | 最终退出0 |
| cargo clippy -p llm-runtime --lib --no-deps -- -D warnings | 退出101，101项既有诊断，未见本次Nova文件诊断，不计通过；未宣称最终全量严格lint通过 |
| cargo build -p vw-cli --bin shiju | 最终退出0，13m19s；此前阶段构建均退出0，最终复制产物及源码摘要已核对 |

没有运行桌面编译、全仓测试/构建、全特性Clippy、全仓格式检查、wasm或Makefile，也没有在页面创建后追加浏览器、页面构建或页面测试。完整文件创建、实际模型绑定、目录边界及本任务配置恢复均有最终轮新证据；原生免审批、其他操作系统和飞书配置变化归因仍不在本次通过声明内。


# siGit Code 验收（2026-11-18）

结论：**指定模型完整文件创建验收通过**。首次客户端300秒超时后，通过现有续传接口取得原请求chat.done；不把HTTP200或诊断结果当成成功终态。

- 前置：已核对siGit接入证据、当前注册/安装/派生预设/模型驱动及Nova最终验收与恢复记录。旧路径已迁移至crates/libs/acp-agent-registry、config-acp-install、agent-config及llm-runtime；使用RustCodeGraph检查。来源任务已移除不作为通过依据。
- 复用Nova最终shiju 0.6.0构建，二进制SHA-256 `b507ddebf25b76f2341a9e77c181415cbbbe964643d86389f9046e318082fad6`，构建来源d6fef4272d346339ca0bbcf00c04af6d944601b5及既有source-hashes-final.json；当前摘要逐项匹配，siGit驱动、注册、安装与预设源码对该构建HEAD无差异。不宣称clean HEAD构建。
- 全部API携带X-Platform-ID=10001、X-Company-ID=100010005、X-Directory=/Users/xiangmin/worker/code/ruejs。取得config、installed、agents、acp基线。自有回环网关先读gateway --help，使用现有开发配置、TOKIO_WORKER_THREADS=4。
- 首次系统安装HTTP400、curl退出0、0.197120秒；受审源码安装要求仓库Cargo槽位环境，首个网关未继承该环境。停止该自有网关后，经scripts/cargo_slot.py的slot(number=6)入口启动，两个构建目录均为target/rust-slot-6；系统安装重试HTTP200、退出0、16.371460秒，installed false→true。未用任意URL、shell或未修补CLI替代系统安装链路。槽位4被其他进程占用，等待入口终止后改用空闲槽位6，未清理他人锁。
- 真实入口为开发配置acp-runtime/sigit-1.7.10-model-source-v1/sigit，受审版本1.7.10。入口SHA-256 `b98bdfdc85884922297704df67d5b9441a8fd399352457bfd75cc09f86934520`；收据源码SHA-256 `088809cfc43ceda419913488390a65e830ac8ff66f1b5dd14a8d833d6b46369e`、补丁SHA-256 `4d775d40748eb3a4fc5656034d591052c4d8e7d99cd58e238ea9f6230f9f9256`均回读核对。
- 普通诊断HTTP200、退出0、4.022415秒，authentication_required，独立记录。模型来源接口HTTP200确认session_model supported、审核版本1.7.10、Darwin arm64。内置acp-sigit的builtin=true、runtime.kind=acp、backend=sigit，基线disabled；启用PATCH和GET重读均HTTP200。
- 请求严格使用任务JSON及指定alibaba-token-plan-cn/deepseek-v4.1-flash-1017。首次chat/stream HTTP200，curl退出28、300秒限时结束，无chat.done；先查询原会话summary/options。确认现有stream_acceptance与transport支持续传后，以响应x-vw-stream-request-id及最后SSE id发送Last-Event-ID，原请求续传HTTP200、退出0、0.008489秒，未重新提交创建任务。合计chat.context_loaded=1、chat.step_start=1、chat.delta=283、chat.tool=132（状态事件）、chat.session_updated=2、chat.step_finish=1、chat.done=1、chat.error=0。
- 会话ses_1ea5ef4bb0013SlCxrrDHv6w7y：运行中summary确认acp-sigit/sigit、full_access=true；运行中options connected=true，selected_model.source=session_model、provider_id=alibaba-token-plan-cn、完整模型ID=alibaba-token-plan-cn/deepseek-v4.1-flash-1017。结束后connected=false、所选模型保持匹配；以实际接口证据判断绑定，不依赖助手自述。
- 实际产物acpdemo/sigit/共8文件：README.md、index.html、vite.config.mjs、tsconfig.json、src/App.tsx、src/main.tsx、src/env.d.ts、src/style.css。已读源码和说明：useApp挂载、textarea onInput、ref/computed实时统计非空白字数及字符数；300ms防抖写localStorage、初始化恢复，pagehide/beforeunload补齐待保存内容。Vite复用examples/shared/rue-vite.mjs，别名指向packages/rue/src，实际使用本仓库@rue-js/rue；README提供运行说明。没有运行浏览器、页面构建、类型检查或额外页面测试，不宣称实际渲染/刷新交互实测。
- 目标目录初始不存在，未覆盖用户文件。排除.git/node_modules/target的普通文件SHA-256快照6282→6290，仅新增上述8文件，目录外0差异；保留初始Git状态，未提交Rue.js工作区。
- 恢复PATCH客户端60秒超时（HTTP000、退出28），随后GET确认全部agents与基线相等、siGit disabled。完整config差异仅agents.acp-sigit：基线无显式覆盖，恢复后保留等效disabled预设覆盖；不宣称存储字节一致，未覆盖其他配置。系统安装保留。
- 自有网关PID52270/61882均停止并退出0；最终端口55582关闭、PID61882不存在。槽位6入口正常退出释放自有锁、缓存保留；未重启用户服务。
- 原始配置、请求、响应、SSE、日志、入口收据/构建摘要与快照仅存本机0700目录 `/tmp/shiju-sigit-accept-20261118/`。本次无产品代码修复，复用已验证构建；执行系统安装、诊断、模型/预设/会话API、续传、文件内容与快照检查。未运行新Rust单测、Cargo检查、Clippy、桌面编译、全仓构建/测试、wasm或Makefile；历史单测不计本次证据。旧任务引用.plans/references/rust.md不存在，实际使用现有槽位入口完成安装。


# Amp 验收（2026-11-18）

结论：**已阻塞**。系统安装与预设保存重读成功，但指定模型会话返回 chat.error，未创建文件；不以 HTTP200 或能力声明认定通过。

- 已核对来源接入证据、差异清单、当前注册/派生预设/模型驱动，以及任务29 siGit验收完成及恢复记录。RustCodeGraph索引可用；旧源码路径已迁移到crates/libs。历史Amp原生测试通过不替代本次生产链路验收。
- 复用Nova最终shiju 0.6.0构建，SHA-256 `b507ddebf25b76f2341a9e77c181415cbbbe964643d86389f9046e318082fad6`；构建来源HEAD d6fef4272d346339ca0bbcf00c04af6d944601b5，既有source-hashes-final.json全部摘要与当前文件一致；Amp驱动、注册和catalog对构建HEAD无差异。不宣称clean HEAD构建。
- 全部API携带X-Platform-ID=10001、X-Company-ID=100010005、X-Directory=/Users/xiangmin/worker/code/ruejs。保存installed、agents、acp和config基线。读取gateway帮助后经scripts/cargo_slot.py槽位6启动自有回环网关，两个构建目录均为target/rust-slot-6，使用现有开发配置。
- 系统安装POST HTTP200、curl退出0、1.284865秒，amp-acp installed false→true。真实入口acp-runtime/amp-acp-0.12.0-model-source-v1/node_modules/.bin/amp-acp解析到amp-acp/dist/index.js，SHA-256 `fc3b527cad139098f2f120992e91414e4e0b45af6a71fc479d5e4b79c1122934`；受审桥接0.12.0、历史本地ThreadWorker引擎0.0.1778732631-gc5875c，非当前原生CLI声明。引擎dist/main.js摘要 `2cca8fdfbad3fdc9cec0d4de31946279c51a47adb9e6dd0242131f3cac7a5a1d`。
- 普通诊断HTTP200、退出0、0.009185秒，结果startup_failed，独立记录。模型来源API声明session_model supported、cli_local unsupported，Darwin arm64。acp-amp-acp的builtin=true、runtime.kind=acp、backend=amp-acp，基线disabled；启用PATCH与GET回读成功。
- 请求严格使用任务JSON、指定alibaba-token-plan-cn/deepseek-v4.1-flash-1017和full_access=true。chat/stream HTTP200、curl退出0、49.378849秒；chat.context_loaded、chat.step_start、chat.model_recovery（phase=stopped）、chat.session_updated、chat.error各1，chat.done=0、chat.tool=0。未超时、续传或重复提交。
- 会话ses_1ea742ec9001YpbZ32JEevjrhR的summary确认acp-amp-acp/amp-acp、full_access=true；最终options.selected_model.source=session_model、provider_id=alibaba-token-plan-cn、完整模型ID匹配。轮询与最终connected均false，未取得connected=true证据。错误为“Amp selected model execution failed”。底层可定位到patches/acp-model-source/amp-acp/apply.py:83将引擎result.is_error映射为该错误；现有证据不足以判断供应商原始原因，不归因为认证失败或模型不支持。未绕过或修改产品行为；需独立排查引擎原始失败后重跑本任务。
- 目标acpdemo/amp-acp初始及结束均不存在，无页面或README。排除.git/node_modules/target的普通文件SHA-256快照6290→6290，0差异；保存初始Git状态，未提交Rue.js工作区。未运行浏览器、页面构建或页面测试。
- 恢复PATCH HTTP200，GET确认全部agents与基线相等、Amp disabled。完整config仅agents.acp-amp-acp增加等效disabled覆盖，不宣称存储字节一致。保留系统安装。自有网关PID77770退出0、回环端口51791关闭；槽位入口退出并释放自有锁、保留缓存，未重启用户服务。
- 原始配置、请求、SSE、日志、构建/入口摘要及快照仅存0700本机目录 `/tmp/shiju-amp-accept-20261118/`。本次无产品代码修复，复用已验证构建，未运行新Rust单测、Cargo检查、Clippy、桌面编译、全仓测试/构建、wasm或Makefile；历史单测不计新证据。旧.plans/references/rust.md不存在，实际使用现有槽位入口。任务文件保留已阻塞；重跑条件为真实指定模型成功终态、connected绑定及完整文件落盘均取得新证据。


# Amp 修复后复验（2026-11-18）

结论：**指定模型完整文件创建验收通过**。保留前述阻塞记录；本节为完成两项底层修复后的最终新证据。客户端首次300秒超时后续传原请求取得chat.done，不把HTTP200或诊断结果当成通过。

## 根因与修复

- 原始失败由临时诊断构建取得引擎原始错误：模型中继拒绝422。真实请求含旧Opus默认thinking.display=summarized，中继只接受type/budget_tokens。Amp控制桥仅剔除这一旧模型默认值，保留所选模型思考配置，未知display仍拒绝；同步更新受审模块摘要。提交 `5ef9d07f8`。
- 去除display后，中间轮曾写出7个文件但未完成README；另有正式轮在首次工具后失败。诊断捕获adaptive模式下assistant仅有tool_use、下一轮422的请求。中继原先把没有思考块的合法自适应工具轮一律视为删除思考，导致结果取决于该轮是否返回思考。
- 修复为仅回放服务端已记录的无思考消息：完整消息和部署摘要、租户/项目/会话/供应商/模型/generation作用域及900秒有效期均校验。带签名工具ID仍强制保留思考块；较早的无思考记录不能绕过后续签名要求。没有伪造思考、关闭思考、接受任意历史或放宽权限。提交 `6a33d6415`。
- 当前宿主首次构建另遇gateway-computer-control状态接口跨await持有读锁的E0277。权限查询移到取读锁之前后恢复编译，未改变权限判断，仅提交对应一行移动，未包含共享文件其他工作。提交 `0f45452ba`。

## 最终正式构建、安装与会话

- 正式shiju 0.6.0构建退出0，367.550228秒；SHA-256 `c01a466afc21a06e643db6341457e404981f06833833c6c327e7a02774758b78`。构建起点HEAD `df9f114345adf4f843a0929e88750cb48122ba22` 加上述工作区修复；源码摘要前后无差异，见final-fixed-source-before.json和final-fixed-source-changes.json。不宣称clean HEAD构建。最终构建未使用诊断源码包装器。
- 全部网关API使用X-Platform-ID=10001、X-Company-ID=100010005、X-Directory=/Users/xiangmin/worker/code/ruejs，保留config、installed、agents和acp基线。读取gateway帮助后使用既有开发配置、自有回环端口及Node v24.14.0；未重启用户服务。
- 最终系统安装POST HTTP200、curl退出0、3.008644秒，installed false→true；本任务最初亦从false安装为true。更新契约后，旧自建产物校验曾正确返回HTTP400 checksum mismatch；逐项校验后将本任务自己安装的旧版本/临时诊断版本完整移到0700私有目录，再由系统接口重新安装，未绕过校验、就地修改安装文件或覆盖用户全局CLI。
- 受管目录acp-runtime/amp-acp-0.12.0-model-source-v1，入口node_modules/.bin/amp-acp解析到amp-acp/dist/index.js；桥接0.12.0、历史本地ThreadWorker引擎0.0.1778732631-gc5875c。最终四个产物摘要全部匹配契约：index.js `fc3b527cad139098f2f120992e91414e4e0b45af6a71fc479d5e4b79c1122934`，shiju-local.mjs `141b55200e4d9239a467d5c8f84fb5a5b2ca91523eb22f361d6367028a76fed9`，amp-cli `f3076f4f083423e1215e366e18ec7808a4405e973cfe66d0135b1bd10592a1d5`，引擎main.js `2cca8fdfbad3fdc9cec0d4de31946279c51a47adb9e6dd0242131f3cac7a5a1d`。已确认诊断写入代码不在正式安装中，不冒充当前原生Amp或跨平台认证。
- 普通诊断HTTP200、退出0、0.016814秒，startup_failed，独立记录；模型来源接口确认session_model supported、cli_local unsupported及Darwin arm64。内置acp-amp-acp的builtin=true、runtime.kind=acp、backend=amp-acp，基线disabled；启用PATCH及GET重读均成功。
- 最终请求严格沿用任务JSON，指定alibaba-token-plan-cn/deepseek-v4.1-flash-1017、session_model、full_access=true。首次chat/stream HTTP200、curl退出28、300.011149秒，尚无终态；先查询原会话状态，核对现有transport续传协议，再用响应x-vw-stream-request-id和Last-Event-ID续传同一请求，HTTP200、退出0、0.012599秒。没有重复提交最终创建任务。
- 合并原流和续传：chat.context_loaded=1、chat.step_start=1、chat.delta=12、chat.tool=34（状态事件）、chat.session_updated=2、chat.step_finish=1、chat.done=1、chat.error=0。请求标识168373f5-0326-4927-9bba-602822a40e72，会话ses_1ead3a641001txKdyfPum0PcTN。
- 运行中summary确认acp-amp-acp/amp-acp、full_access=true；运行中options connected=true，selected_model.source=session_model、provider_id=alibaba-token-plan-cn、完整模型ID=alibaba-token-plan-cn/deepseek-v4.1-flash-1017。结束connected=false，所选模型保持匹配。依据实际接口和终态判断绑定，不依赖助手自述。

## 文件、恢复与清理

- 最终acpdemo/amp-acp/共7文件：README.md、index.html、vite.config.mjs、src/App.tsx、src/main.tsx、src/env.d.ts、src/style.css。已逐项检查：useApp挂载、textarea输入、ref/computed实时非空白字数及字符数、400ms防抖localStorage写入、初始化恢复、pagehide/beforeunload补写；README提供运行说明。Vite复用examples/shared/rue-vite.mjs，别名实际指向本仓库packages/rue/src，使用本仓库@rue-js/rue。未运行浏览器、页面构建或额外页面测试，不宣称渲染、刷新交互已经实测。
- 最终目标初始不存在，文件SHA-256快照（排除.git/node_modules/target）6290→6297，仅新增上述7文件，无符号链接、目录外0差异。保留初始Git状态，未提交Rue.js工作区。中间失败轮7文件和诊断成功轮8文件均在逐项确认属于本次新产物、快照无目录外变化后完整移入私有目录；未覆盖用户文件，也未将诊断轮当作最终通过证据。
- 最终恢复PATCH HTTP408、curl退出0、48.275102秒；随后GET确认所有agents和完整config均与最终轮基线相等，Amp disabled。任务最初无显式覆盖，早期恢复保留等效disabled覆盖，不宣称自最初基线起存储字节完全相同。正式系统安装保留。
- 九个自有网关PID77770/32095/95306/19491/22893/36324/41754/46406/69149均退出；最终核对对应PID不存在、回环端口均关闭，见cleanup.json。自有Cargo入口均已退出并释放自己的槽位锁，缓存保留；未清理他人锁或结束他人进程。
- 主证据目录 `/tmp/shiju-amp-complete-20261118/`；修复、红绿灯、构建、原生探针和临时诊断源码在 `/tmp/shiju-amp-debug-20261118/`。前序accept/retry/retry2/final/diagnostic/fixed-accept/diagnostic2/final-pass/diagnostic3各轮证据保留在对应 `/tmp/shiju-amp-*-20261118/` 目录，全部根目录0700。原始请求、配置、SSE、敏感载荷及供应商/宿主二进制仅留本机，未提交。临时诊断源码最终通过私有副本构建，仓库中的临时诊断改动已还原，未保留诊断写入逻辑。

## 定向验证

| 检查 | 本次结果 |
| --- | --- |
| node --test patches/acp-model-source/amp-acp/local.test.mjs | 修复前2通过/1失败（422≠200）；最终3通过，未知display仍422 |
| cargo test -p vw-acp --test onboarding_amp_acp -- --test-threads=1 | 槽位6，4通过、0 ignored |
| cargo test -p model-protocol --lib model_protocol_015_unreasoned_ -- --test-threads=1 | 槽位1红灯，0通过/2失败、152过滤 |
| cargo test -p model-protocol --lib messages_thinking_ -- --test-threads=1 | 槽位1最终13通过、142过滤，含既有签名删除/篡改拒绝及新增无思考消息身份、内容、部署和期限检查 |
| cargo test -p gateway-computer-control --lib cc_control_18_ -- --test-threads=1 | 槽位6，11通过、2过滤 |
| 原生Amp受控探针 | 真实安装入口，4个完成回合、13次模型边界请求，验证写入、MCP/选项显式拒绝、重启load、取消后继续、401/404失败；受控模型结果不替代真实云模型验收 |
| cargo clippy -p acp-agent-registry --lib --no-deps -- -D warnings | 槽位6，退出0 |
| cargo clippy -p gateway-computer-control --lib --no-deps -- -D warnings | 槽位6，退出0 |
| cargo clippy -p model-protocol --lib --no-deps -- -D warnings | 槽位6，退出101，27项既有诊断位于本次未修改的messages_gemini_stream、responses、search_citations、signature_rectify、tool_media、tool_namespace、xai等文件；无本次messages_thinking文件诊断，不计通过 |
| cargo build -p vw-cli --bin shiju | 槽位6最终退出0，6m07s，正式二进制和源码摘要已核对 |

Rust重型命令经scripts/cargo_slot.py槽位入口串行执行，对应CARGO_TARGET_DIR和CARGO_BUILD_BUILD_DIR均为target/rust-slot-N绝对路径。共31项定向单测通过；仅格式化修改的两个思考协议Rust文件，保留其他文件格式。执行相关差异git diff --check；未运行桌面编译、全仓构建/测试、全特性Clippy、全仓格式检查、wasm或Makefile。旧.plans/references/rust.md缺失，沿用现有槽位入口，不修改plan.md或prompt.md。


# Cortex Code 验收（2026-11-18）

结论：已阻塞，未通过。任务30已有完整验收及环境恢复记录；任务10文件与 `docs/dev/evidence/acp-onboarding-cortex-code.md` 当前均不存在，不以文件缺失推定接入通过。当前注册存在，启动参数为 `cortex acp serve`；安装策略 `crates/libs/config-acp-install/src/lib.rs` 对 cortex-code 返回 None，独立 installer_contract 测试也明确断言 Unix/Windows 均不支持系统安装。模型来源驱动未列入 Cortex Code 的 verified_release，实际接口明确拒绝 session_model。补齐系统安装及指定模型接入、交付脱敏接入证据后重跑任务31。

- 本次复用任务30正式 shiju 0.6.0 产物 `/tmp/shiju-amp-complete-20261118/shiju`，SHA-256 `c01a466afc21a06e643db6341457e404981f06833833c6c327e7a02774758b78`；构建来源沿用上一节的 df9f114345adf4f843a0929e88750cb48122ba22 加工作区修复，未重新构建，不宣称对应当前 clean HEAD。当前相关源码摘要另存私有证据，当前安装拒绝源码与实测一致，不单凭旧二进制判定当前实现失败。
- 使用自有回环网关 127.0.0.1:51431、现有开发配置；全部 API 均携带任务规定的三个租户/目录头。GET installed、agent?include_disabled=true、acp 与完整配置均 HTTP200；Cortex 安装前后均 false；内置 acp-cortex-code 为 builtin=true、runtime.kind=acp、backend=cortex-code、enabled=false。
- POST /v1/desktop/acp-install/cortex-code：HTTP400，curl退出0，0.001179秒，明确返回“该 ACP 暂无一键安装命令，请使用官方安装器”。未改用外部安装脚本，无可核验入口或上游版本。
- POST /v1/desktop/acp-diagnostic/acp-smoke-cortex-code 使用当前 GET 的对应 ACP 配置：HTTP200，curl退出0，0.008343秒，结果 startup_failed，未视为握手或会话成功。GET /v1/acp-model-source/cortex-code：HTTP200，curl退出0，0.000679秒；session_model=unsupported、reason=unsupported_cli，cli_local=supported，connected=false。
- Rue.js packages/rue/src 存在，目标 acpdemo/cortex-code 初始不存在；已保存 Git 初始状态及排除 .git/node_modules/target 的普通文件 SHA-256 快照，前后6297项、0差异。未创建目标产物，未覆盖任何用户文件。
- 安装与指定模型能力为前置阻塞，未临时启用预设、未发 chat/stream、无会话标识或 chat.done；未执行会话 summary/options、页面文件功能验收、浏览器或页面构建。未执行 Rust 单测、Clippy 或编译：没有行为修复，遇到明确外部能力阻塞后停止执行，不声称测试通过。
- 结束 GET 全部配置与基线相等，无需恢复 PATCH。自有网关 PID91090 退出0，端口已关闭；未重启或终止用户服务，未申请 Cargo 槽位。
- 原始配置、请求、响应、指标、快照及清理记录仅存本机0700目录 `/tmp/shiju-cortex-accept-20261118/`，不提交敏感载荷或供应商二进制。此次仅追加本节并更新任务31阻塞状态。


## Cortex Code 追加排查（2026-11-18）

- 按扩大排查授权重新核对共享工作区；安装仍为显式 None，模型来源仍无 Cortex verified_release。进一步核对官方 [安装说明](https://docs.snowflake.com/en/user-guide/cortex-code/cortex-code-cli)、[ACP前置条件](https://docs.snowflake.com/en/user-guide/cortex-code/cortex-code-acp) 和 [设置](https://docs.snowflake.com/en/user-guide/cortex-code/settings)：ACP要求先配置 Snowflake 连接，模型选项并不能证明可接入任意外部供应商。文档、设置及原生CLI帮助未提供已确认的外部模型端点配置路径；不把未发现入口夸大为上游永久不支持。
- 已突破“无可核验官方发行物”的调查限制：只下载并阅读官方 https://ai.snowflake.com/static/cc-scripts/install.sh，未执行安装脚本；其固定发行前缀为 cortex-code-cli/a4643c4278，stable指向 `1.2.46+175654.6f8561fd271f`。按脚本的URL编码规则获取manifest及Darwin arm64包，下载退出0，SHA-256 `76a37a37b432fa0b684d67efdaba0b2753b40d34924836a445f6597b24e665ee` 与官方manifest一致。首次未编码“+”的manifest请求404；编码后成功，不把404当作发行物不可用。
- 发行物仅解压到0700私有目录；原生 `--version`、`--help`、`acp serve --help` 全部退出0，版本 Cortex Code v1.2.46。ACP帮助支持 --connection、--model、--workdir 及权限工具选项，没有已确认的外部模型base URL入口。未安装到系统、未修改shell配置或权限策略，未把临时下载代替系统安装验收。
- 使用独立 SNOWFLAKE_HOME/COCO_LOG_DIR、本仓库Rue.js cwd及完整指定模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 进行原生ACP探针，未添加bypass。initialize成功，protocolVersion=1（agentInfo报告1.0.0，与发行版本分别记录）；随后session/new因没有Snowflake连接退出。首次解析被供应商非JSON错误输出中断，已重跑将原始stdout/stderr直接写入私有文件；第二次退出7、耗时2.025069秒，明确报错 `No Snowflake connection available`。没有发送prompt或创建任务，没有取得指定模型请求证据。
- 本机默认 ~/.snowflake 不存在，SNOWFLAKE_/CORTEX_ 环境变量数量0；只核对存在性与变量名，未读取或记录凭据。当前真实外部前置为获授权且可用的Snowflake连接；此条件即使补齐，也仍需证明指定Alibaba模型能保持完整绑定，不能用Snowflake默认模型冒充。安全替代路径已覆盖官方校验发行物、原生参数/环境选模及ACP直连诊断；没有注册账号、伪造认证、重定向身份服务或替换引擎。
- 两个自有探针PID已退出，原自有网关51431端口仍关闭，无新增网关、系统安装、宿主Snowflake配置或配置PATCH。Rue.js与上一轮结束快照比较仍6297项、0差异；无目标产物。当前仅文档变化，未运行Cargo或页面测试。任务保持已阻塞，保留任务文件。
- 新私有证据 `/tmp/shiju-cortex-retry-20261118/`：官方脚本、版本、manifest、校验后的发行包、CLI帮助、原生协议请求/响应/错误、退出指标、目录边界和清理记录。供应商二进制及原始载荷不提交。


## Corust Agent 验收（2026-11-18）

结论：已阻塞，未通过。来源任务 `11-Corust-Agent.md` 与 `docs/dev/evidence/acp-onboarding-corust-agent.md` 均不存在。任务31已明确记录阻塞并完成清理，未争用其网关。

- 当前注册位于 `crates/libs/acp-agent-registry/src/agent_registry.rs`，命令 `corust-agent-acp`、空参数；旧路径已迁移。`config-acp-install/src/lib.rs:59–61` 明确返回None，`llm-runtime/src/acp/model_source/drivers/mod.rs` 无Corust已验证版本和驱动；`agent-config/src/catalog.rs` 派生内置预设。现有onboarding测试使用Python协议fixture，不能证明真实CLI或指定模型成功，未据此宣称通过。
- 复用任务30正式shiju 0.6.0产物 `/tmp/shiju-amp-complete-20261118/shiju`，SHA-256 `c01a466afc21a06e643db6341457e404981f06833833c6c327e7a02774758b78`，构建来源为 `df9f114345adf4f843a0929e88750cb48122ba22` 加当时工作区修复；未宣称当前clean HEAD构建。当前相关源码摘要存私有证据，拒绝行为与当前源码一致，不单凭旧产物判定实现失败。
- 新建自有回环网关，使用现有开发配置。全部HTTP请求携带指定平台10001、公司100010005与Rue.js目录头；安装、预设、ACP配置及全局配置基线GET均200。安装前后corust-agent均false；POST系统安装HTTP400、客户端退出0、耗时0.007392秒，明确“该 ACP 暂无一键安装命令，请使用官方安装器”。无实际入口或CLI版本可核验，未使用替代下载或shell安装。
- 按接口传入对应ACP配置，诊断HTTP200、客户端退出0、0.068754秒，但响应为startup_failed。模型能力GET200、客户端退出0，session_model明确unsupported_cli，cli_local supported、connected=false。未用CLI默认模型替代指定 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。
- 本次GET确认 `acp-corust-agent` 的builtin=true、backend=corust-agent、enabled=false。安装与模型前置已阻塞，未临时启用、未PATCH、未提交chat/stream，无SSE、chat.done、会话绑定或完整文件创建证据。未取得selected_model、full_access或真实模型请求证明。
- Rue.js源码 `packages/rue/src` 存在；目标 `acpdemo/corust-agent/` 初始不存在且结束未创建。排除.git、node_modules、target的普通文件SHA-256快照6297项、零差异。全局配置结束GET200，完整内容与基线相等；无需恢复写入。仅停止自有网关，退出0、端口关闭。
- 未运行Cargo、单测、Clippy、页面构建、浏览器或创建后的额外回归；本轮无代码修改与行为修复，前置阻塞，仅文档轻量检查。旧 `.plans/references/rust.md` 不存在，本轮无需编译槽位。任务32保留已阻塞，待系统安装、真实session_model驱动和脱敏接入证据交付后完整重跑。
- 本次私有证据 `/tmp/shiju-corust-accept-20261118/`，根目录0700，包含原始响应、指标、配置基线及重读、网关日志、构建与当前源码摘要、快照和清理记录；原始配置与敏感载荷不提交。首次基线连接早于网关就绪失败，网关就绪后重新读取成功，未重复提交创建任务。

### Corust 扩大范围重试（2026-11-18）

- 当前共享源码仍无Corust系统安装命令或session_model驱动，未改变模型、租户、引擎或权限策略。本轮进一步查询官方发行源：原Corust-ai仓库重定向到rustingai，官方extension.toml和GitHub最新发行均为v0.6.0。下载Darwin arm64包，按GitHub release asset digest核验SHA-256 `6a74d46f645774f0a7e2a95371bad39d63927e231a34d52d2800c79f508f025f` 一致，仅解出普通文件至0700私有目录，未安装到宿主PATH。官方来源：[发行页](https://github.com/rustingai/corust-agent-release/releases/tag/v0.6.0)、[扩展清单](https://github.com/rustingai/corust-agent-release/blob/main/extension.toml)。
- 原生--help/--version均退出0但无输出，不能据此推断参数支持。ACP initialize实际成功，protocolVersion=1，agentInfo版本0.1.0（与发行v0.6.0分别记录），唯一认证方法oauth_browser。隔离HOME、本仓库Rue.js cwd下，session/new携带原完整指定模型，返回JSON-RPC -32000 Authentication required。此处model字段不构成选模被支持的证据，未发送prompt或创建任务。
- 仅检查宿主存在性与环境变量名：无~/.corust目录，CORUST_环境变量数量0；二进制显示CORUST_API_KEY/CORUST_HOME及OAuth入口，但这些静态字符串不能证明外部Alibaba模型可配置。未伪造认证、重定向身份服务、注册账号、自动开启浏览器登录或替换默认模型。当前真正外部前置为获授权的有效Corust认证；即使补齐，仍须证明原指定模型真实请求、系统安装及完整会话产物。
- 自有原生探针已退出，未启动新网关、未PATCH配置、未系统安装；旧自有网关端口仍关闭。Rue.js与前轮结束快照比较6297项零差异，未创建目标目录。仅文档轻量检查，无代码变更，不运行Cargo、单测、Clippy或页面验证。私有证据 `/tmp/shiju-corust-retry-20261118/`（0700）保存官方manifest、release API、校验包、原生协议请求/响应、退出指标、边界及清理记录；供应商二进制与原始载荷不提交。任务仍已阻塞，待有效认证与模型入口证据后继续。


## Devin 验收（2026-11-18）

结论：已阻塞，未通过。任务32已有明确阻塞及环境恢复记录；来源任务 `12-Devin.md` 和 `docs/dev/evidence/acp-onboarding-devin.md` 均不存在，不以缺失推定接入通过。

- 当前注册与安装代码已迁移到 `crates/libs/acp-agent-registry/src/agent_registry.rs`、`crates/libs/config-acp-install/src/lib.rs`；注册为 `devin acp`，安装策略明确返回None。`agent-config/src/catalog.rs` 派生builtin预设；`llm-runtime/src/acp/model_source/drivers/mod.rs` 无Devin verified_release及驱动。现有onboarding测试含协议fixture，不能证明真实供应商会话通过。
- 复用任务30正式shiju 0.6.0产物 `/tmp/shiju-amp-complete-20261118/shiju`，SHA-256 `c01a466afc21a06e643db6341457e404981f06833833c6c327e7a02774758b78`。构建来源沿用任务30的df9f114345adf4f843a0929e88750cb48122ba22加工作区修复，不宣称当前clean HEAD构建；另存当前相关源码摘要，当前源码与拒绝行为一致，不单凭旧产物判定失败。
- 自有回环网关127.0.0.1:50568，现有开发配置；所有请求携带规定平台10001、公司100010005及Rue.js目录头。installed、agents、ACP及全局配置基线GET均200；安装前后Devin均未安装。POST系统安装HTTP400、客户端退出0、0.000575秒，明确“该 ACP 暂无一键安装命令，请使用官方安装器”；无真实入口及CLI版本可核验，未使用任意脚本替代系统链路。
- 传入当前对应ACP配置的诊断HTTP200、客户端退出0、0.009044秒，结果startup_failed。模型能力GET HTTP200、客户端退出0、0.000645秒，session_model=unsupported、reason=unsupported_cli，cli_local=supported、connected=false；未改用默认模型冒充指定 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。
- GET确认内置预设acp-devin、builtin=true、runtime.kind=acp、backend=devin、enabled=false。前置安装及模型能力阻塞后未PATCH启用、未发送chat/stream，无SSE、chat.done、会话summary/options或selected_model/full_access绑定证据，未创建编辑器文件。
- Rue.js `packages/rue/src` 存在；目标acpdemo/devin初始不存在，结束未创建。排除.git、node_modules、target的普通文件SHA-256快照前后6297项零差异。结束GET完整配置与基线相等，无临时配置需要恢复；自有网关已停止、端口关闭，未重启用户服务、未占用Cargo槽位。
- 未运行Cargo、定向单测、Clippy、桌面编译、浏览器或页面构建：本次无代码修复，复用既有产物，遇到明确能力阻塞后仅完成独立检查与文档轻量检查；不宣称单测或真实会话成功。旧 `.plans/references/rust.md` 不存在，本次无需编译。
- 本机0700私有证据 `/tmp/shiju-devin-accept-20261118/` 保存原始请求/响应、指标、配置基线与重读、构建与源码摘要、快照、网关日志及清理记录，不提交敏感载荷或供应商二进制。任务33保留已阻塞，待交付系统安装、真实session_model驱动和脱敏接入证据后完整重跑。


### Devin 扩大范围重试（2026-11-18）

- 重查共享源码，系统安装仍显式None，session_model驱动仍无Devin已验证版本。进一步核对官方[安装说明](https://docs.devin.ai/cli)、[模型说明](https://docs.devin.ai/cli/models)、[配置格式](https://docs.devin.ai/cli/reference/configuration/config-file)及[认证说明](https://docs.devin.ai/cli/enterprise/devin-auth)。官方支持选模，但上述文档未提供已确认的任意外部供应商base URL配置路径；不把未找到入口夸大为永久不支持。
- 已安全突破“无已校验发行物”的调查限制：只读取官方install.sh，未执行；按官方current/manifest.json下载3000.13.4 Darwin arm64包，SHA-256 `24b52c2bb312f8ab3d71545a173a69446901b27da5a44300bd03464ab066525b` 与manifest一致。仅将普通文件解出到0700私有目录，未系统安装或写入宿主PATH。初次解压因本机Python不支持tarfile filter参数停止，随后改为逐项校验路径并只提取普通文件成功。
- 原生--version、--help、acp --help均退出0，版本3000.13.4 (d9a56503ce88)。ACP支持--model/DEVIN_MODEL以及in-process执行。使用隔离HOME/XDG目录、Rue.js cwd、完整原指定模型启动原生ACP，无权限绕过或云引擎替换。initialize成功，protocolVersion=1、agentInfo为affogato/Devin Agent 3000.13.4，认证方式devin-browser。
- session/new可返回本地会话，但stderr明确报告指定模型未解析并回退默认；响应model.currentValue为swe-1-6-fast、options为空。因此没有提交prompt或把该会话视为成功。继续显式调用session/set_config_option，设置完整原模型，得到-32602 Invalid params（Invalid value for config option model）。第二次探针退出0、耗时0.232574459秒；协议成功或进程退出0不等于指定模型调用成功。
- auth status退出0但明确Not logged in，models list退出1并要求devin auth login；宿主默认~/.config/devin、~/.local/share/devin、~/.devin不存在，DEVIN_/WINDSURF_/COGNITION_环境变量数量0。当前真实外部前置是有效且获授权的Devin认证；补齐后仍须确认该原模型的可用路由，不能使用默认模型冒充。没有自动开启登录、注册账号、伪造认证或重定向身份服务。未在缺认证及模型绑定证据时发布伪支持驱动。
- 两次原生探针均已退出（in-process，无共享daemon），未新增网关、未PATCH全局配置、未系统安装或修改权限策略；原自有网关50568端口仍关闭。Rue.js与前轮结束快照比较6297项零差异，目标目录未创建。无代码修复，未运行Cargo、单测、Clippy或页面验证，仅文档轻量检查。任务保持已阻塞，待有效认证及原模型接入条件具备后继续。
- 新私有证据 `/tmp/shiju-devin-retry-20261118/`（0700）：官方脚本、manifest、校验包、原生帮助、认证状态、模型列表错误、ACP请求/响应/退出指标、边界及清理记录。供应商二进制和原始敏感载荷不提交。


## Harn 验收（2026-11-18）

结论：已阻塞，未通过。任务33已明确阻塞并恢复环境；来源任务 `13-Harn.md` 与 `docs/dev/evidence/acp-onboarding-harn.md` 均不存在，缺失不视为已交付。

- 当前注册迁移至 `crates/libs/acp-agent-registry/src/agent_registry.rs`，入口为 `harn serve acp --session-environment isolated`，隔离子进程环境；`config-acp-install/src/lib.rs` 明确返回None，模型来源驱动无Harn verified_release。`agent-config/src/catalog.rs` 派生builtin预设。现有onboarding生命周期与拒绝测试使用Python协议fixture，不能证明真实供应商或指定模型成功。
- 复用任务30正式shiju 0.6.0产物 `/tmp/shiju-amp-complete-20261118/shiju`，SHA-256 `c01a466afc21a06e643db6341457e404981f06833833c6c327e7a02774758b78`；构建来源为df9f114345adf4f843a0929e88750cb48122ba22加当时工作区修复，不宣称当前clean HEAD构建。保存当前相关源码摘要，当前源码与拒绝行为一致，不单凭旧产物判定失败。
- 自有回环网关127.0.0.1:57877，沿用现有开发配置；所有请求携带平台10001、公司100010005及规定Rue.js目录头。installed、agents、ACP和全局配置基线GET均200。安装前后Harn未安装；POST系统安装HTTP400、客户端退出0、耗时0.000518秒，明确“该 ACP 暂无一键安装命令，请使用官方安装器”。无真实CLI入口和版本可核验，未使用任意下载或shell替代系统链路。
- 传入当前ACP配置的诊断HTTP200但结果startup_failed；模型能力GET200，session_model=unsupported、reason=unsupported_cli，cli_local=supported、connected=false。未以HTTP200或默认模型冒充指定 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 成功。
- GET确认acp-harn、builtin=true、backend=harn、enabled=false。安装及选模前置阻塞后未PATCH启用、未提交chat/stream；无SSE、chat.done、summary/options或selected_model/full_access绑定证据，未创建编辑器文件。
- Rue.js `packages/rue/src` 存在；目标acpdemo/harn初始不存在，结束未创建。排除.git、node_modules、target的普通文件SHA-256快照6297项，前后零差异。结束GET完整配置与基线一致，无临时写入需要恢复；自有网关退出0、端口已关闭，未重启用户服务。
- 无代码变更或行为修复，复用既有产物；未运行Cargo、定向单测、Clippy、桌面编译、浏览器或页面构建，仅做文档轻量检查。旧 `.plans/references/rust.md` 不存在，本次无需编译槽位。任务34保留已阻塞；系统安装、真实session_model驱动和脱敏接入证据交付后完整重跑。
- 本次0700私有证据 `/tmp/shiju-harn-accept-20261118/` 保存原始响应及指标、配置基线和重读、源码与构建摘要、网关日志、快照及清理记录；敏感环境值、原始载荷和供应商二进制不提交。


### Harn 扩大范围重试（2026-11-18）

- 重查共享安装策略仍为None，session_model仍无已验证驱动。进一步核对官方[发行v0.10.201](https://github.com/burin-labs/harn/releases/tag/v0.10.201)及[供应商配置](https://github.com/burin-labs/harn/blob/v0.10.201/docs/src/llm/providers.md)。Darwin arm64 gzip包SHA-256 `332a7a94162e8c9d3345bbc34fcc6e1043e952b707f1ba44ba79fceb2c21a414` 与GitHub release asset digest一致，原生版本0.10.201。旧注册清单未提供摘要不等于官方发行缺少校验；本次已取得可校验发行物，仅解压至私有目录，未系统安装。
- 隔离HOME/XDG目录，以现有 `serve acp --session-environment isolated` 启动，initialize和session/new均成功。未配置overlay时，完整原模型设置返回-32602 invalid_model，提示provider不在目录；使用官方HARN_PROVIDERS_CONFIG机制注册同名供应商后，完整 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 被session/set_config_option接受。该结果只证明选择器接受，不证明凭据、请求或实际模型绑定成功。探针进程未携带供应商密钥，也未输出凭据。
- 进一步发送不写文件的普通文本提示，返回JSON-RPC -32000，terminalClass=generic_throw，Compilation error；中文编辑器简化提示亦返回 Unexpected character '。'。没有得到成功模型调用、chat.done或编辑器产物。原生[源码prompt.rs](https://github.com/burin-labs/harn/blob/v0.10.201/crates/harn-serve/src/adapters/acp/prompt.rs)第323–355行确认：有pipeline时加载该程序；无文件时将prompt直接包装为pipeline main程序编译。因此当前无文件入口是程序执行语义，不能仅注册供应商或安装二进制就宣称自然语言编码agent可用。未通过改写用户请求为代码、替换引擎或放宽权限伪造验收。
- 本轮确认新的实质前置：需要可审查、兼容自然语言请求的Harn agent pipeline/上游入口，再补系统安装及session_model驱动并验证真实请求。未把缺少宿主Harn认证当作已证实根因；选模接受后的编译失败发生在供应商调用之前。兼容pipeline尚未交付，当前继续保留阻塞；完整请求及模型绑定仍需后续端到端验收。
- 所有自有原生探针已退出（overlay探针关闭stdin后未自然退出，5秒后仅终止自有进程，退出-15；其他探针退出0）。未新增网关、未PATCH全局配置，旧57877端口仍关闭。原生进程在Rue.js目标目录外创建9项`.harn`数据库及锁文件；已逐项确认此前不存在、摘要未变化后移至私有证据，移除新建空目录，恢复原6297项快照，无目标产物。未删除其他文件或宣称执行期间无越界。
- 私有证据 `/tmp/shiju-harn-retry-20261118/`（0700）保存发行API与校验包、文档、原生请求/响应和退出指标、上游源码、运行文件与恢复记录。无本仓库代码变更，未运行Cargo、单测、Clippy或页面测试；文档轻量检查。不提交供应商包、完整配置或原始载荷。


### Harn 实现补齐与最终实机验收（2026-11-18）

结论：最终编辑器文件创建验收通过。保留前述失败记录；原请求创建5项文件后因轮次预算失败，修复后通过显式补全会话完成全部6项文件，不宣称首次请求一次通过。

- 在本任务的扩围授权内补齐实现：固定官方Harn 0.10.201 Darwin arm64发行物、摘要校验安装、原生 `std/agent/loop` 自然语言pipeline、私有会话模型overlay，以及将宿主工具调用转成标准ACP文件读写的传输适配。推理与agent循环仍由官方Harn执行，工厂key保持harn。普通引擎内fs写入曾绕开宿主回调，已改为仅经宿主文件接口；未开放进程工具或放宽宿主审批。
- 官方gzip发行包SHA-256 `332a7a94162e8c9d3345bbc34fcc6e1043e952b707f1ba44ba79fceb2c21a414`；系统安装校验原生二进制、launcher及pipeline，保留许可证。只跳过官方包中指向harn的两个未使用DAP/LSP别名，不提取链接；其他链接仍拒绝。仅已验证的Darwin arm64提供此管理安装，CLI-local模型来源明确不支持。
- 正式CLI为shiju 0.6.0，最终二进制SHA-256 `245d9857ee0005fba1936d1a8bea31719406ef6c550bfe0cc53cb46115ce130e`。构建来源为f0d2a6d56a43ea6d1ff2673ebea254db698dd2fa附近共享工作区加本任务实现，使用Cargo槽位11 `cargo build -p vw-cli --bin shiju`；不宣称clean HEAD构建。使用复制后的自有二进制运行网关，未重启用户服务或占用槽位产物运行。
- 所有HTTP请求保留平台10001、公司100010005和Rue.js目录头。新安装HTTP200（实际下载安装15.250594秒；后续校验重读安装200），安装状态从false变true；真实入口是管理runtime中的harn launcher，官方引擎 `harn-native --version` 为0.10.201。普通诊断HTTP200但结果authentication_required，单独记录，未把它算作诊断通过。实际session_model初始化、选模及模型调用另有成功证据。
- 内置acp-harn的builtin=true、backend=harn；临时PATCH enabled=true并GET确认保存。最初没有显式acp-harn覆写，API拒绝删除builtin条目，已恢复enabled=false并重读确认有效预设与原始基线一致，保留显式disabled覆写。最终补全会话各自结束时完整配置与会前基线相等，其他预设和默认配置未改写。
- 首次真实调用揭示客户端旧session/set_model与Harn当前协议不匹配；已专门使用session/set_config_option(model)并要求返回值精确确认，不做失败回退。模型relay凭据只授予Harn进程内调用，子命令和MCP进程不继承；overlay、pipeline及运行状态放入请求私有目录，拒绝外部启动参数、环境及项目供应商覆写。
- 原创建请求300秒超时后，先调查状态，再按源码确认的接口用同一请求ID及Last-Event-ID续传，没有重复创建任务。续传最终记录budget_exhausted及5项部分文件。新增原生回归复现40轮上限问题后，将pipeline上限调整为80，并证明超过40轮读取仍可经宿主完成写入。旧原生会话跨网关重启恢复返回unknown session，未自动替换；该恢复路径未验收通过，保留限制。
- 已保留部分产物，显式新建同模型、同后端补全会话 `ses_1eb8e9a8c001trQR138CGlci3R`：HTTP200、客户端退出0、100.001998秒，chat.done且无chat.error，实际补README并修复250ms保存防抖导致的立即刷新丢稿风险。静态复核又发现README的workspace filter不匹配，最终会话 `ses_1eb937736001SbYKa1LXS8gE5L` 由Harn工具修正说明：HTTP200、客户端退出0、99.168283秒，chat.done且无chat.error。
- 两次成功会话均查询执行中及结束后的summary/options；执行中connected=true，绑定agent=acp-harn、backend=harn、full_access=true；selected_model.source=session_model、provider_id=alibaba-token-plan-cn、model完整为 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。权限状态如实为client_only，未宣称原生免审批。结束后连接释放时connected=false，保留原selected_model，不将其描述为仍有活动连接。
- 最终产物：`acpdemo/harn/index.html`、`package.json`、`vite.config.ts`、`src/main.tsx`、`src/style.css`、`README.md`。9条成功Harn写工具记录覆盖全部6个文件。静态核对本仓库@rue-js/rue导入、指向源码的Vite aliases及其真实文件、textarea输入、computed实时计数、localStorage读取恢复与每次输入同步保存；README使用 `pnpm --dir acpdemo/harn dev`，没有修改workspace列表。未运行页面构建、浏览器、物理刷新或创建后的页面回归，不宣称这些行为已实测。
- Rue.js普通文件快照从6297项到6303项，最终差异仅上述6个目标文件。Harn期间新建的`.burin/index/snapshot.bin`与`.harn-tmp/.gitignore`已保留到私有证据并移除新建空目录，不宣称执行期间没有运行缓存。自有网关全部退出0、端口关闭，无残留Harn进程；未删除他人改动或提交Rue.js工作区。
- 聚焦验证：7项Rust测试（注册1、配置2、模型驱动2、确认选模2）和6项Python/原生测试（自然语言1、宿主读写/拒绝/40轮预算3、安装档案2）通过，均有有效测试数及红绿证据。acp-client定向Clippy通过。受影响库Clippy仍有既有失败：首次依赖agent-log的3项lint；--no-deps后llm-runtime已有101项lint，未涉及新增Harn文件；未修复无关问题，也未宣称Clippy全通过。未运行全仓测试、桌面客户端编译、wasm或页面检查。
- 私有证据 `/tmp/shiju-harn-complete-20261118/`、`/tmp/shiju-harn-retry-20261118/`（根目录0700）保存各轮请求/响应、SSE及续传、构建与版本、安装失败复现、模型绑定、产物SHA-256、前后快照、恢复与清理记录。原始配置、凭据、敏感载荷和供应商二进制均不提交。详见新增 `docs/dev/evidence/acp-onboarding-harn.md`。


## Junie 验收（2026-11-18）

结论：已阻塞，未通过。前序 Harn 已完成验收及环境恢复；来源任务14与脱敏证据 `docs/dev/evidence/acp-onboarding-junie.md` 均不存在，不以任务缺失推定接入通过。

- 当前实现已迁移至 `acp-agent-registry`、`config-acp-install`、`agent-config` 和 `llm-runtime`。Junie 注册入口为 `junie --acp=true`，安装白名单显式返回 None；模型来源无 Junie verified_release 或驱动。现有 onboarding 协议 fixture 不等于真实供应商验收。
- 复用前序正式 shiju 0.6.0 产物 `/tmp/shiju-harn-complete-20261118/shiju`，SHA-256 `245d9857ee0005fba1936d1a8bea31719406ef6c550bfe0cc53cb46115ce130e`，构建来源沿用 Harn 章节（共享工作区加 Harn 实现），不宣称当前 clean HEAD 构建。保存当前相关源码摘要，源码和实测拒绝一致，不单凭旧二进制认定失败。
- 自有回环网关127.0.0.1:51629，使用现有开发配置；全部请求携带平台10001、公司100010005及Rue.js目录头。安装、agents、ACP及完整配置基线GET均200；Junie安装前后均false。系统安装POST HTTP400、客户端退出0、0.002037秒，明确“该 ACP 暂无一键安装命令，请使用官方安装器”。无真实入口和版本可核验，未使用任意脚本替代系统安装。
- 传入对应ACP配置的普通诊断HTTP200、客户端退出0、0.018792秒，结果startup_failed。模型能力GET HTTP200，session_model明确unsupported/unsupported_cli，cli_local支持、connected=false；未切换到默认模型。
- GET确认acp-junie、builtin=true、runtime.kind=acp、backend=junie、enabled=false。安装与指定模型前置阻塞，未PATCH启用或提交chat/stream；无SSE终态、会话summary/options、full_access或selected_model绑定证据，未创建编辑器产物。
- Rue.js `packages/rue/src` 存在，目标acpdemo/junie前后不存在。普通文件SHA-256快照排除.git、node_modules、target，前后6303项零差异；完整配置结束GET与基线一致，无临时配置待恢复。自有网关退出0、端口已关闭，未重启用户服务、未使用Cargo槽位。
- 未运行Cargo、定向单测、Clippy、桌面编译、浏览器或页面构建：本次无代码修复，复用已有产物，明确能力阻塞后仅完成独立API检查与文档轻量检查，不宣称真实会话或单测通过。旧 `.plans/references/rust.md` 不存在，本次无需编译。
- 本机0700私有证据 `/tmp/shiju-junie-accept-20261118/` 保存原始请求/响应、HTTP/耗时/退出指标、配置基线与重读、源码/二进制摘要、快照及网关清理记录，不提交原始敏感载荷或供应商二进制。保留任务35已阻塞；需补齐经审查的系统安装、真实session_model驱动与脱敏接入证据后完整重跑。


### Junie 官方发行与原生 ACP 重试（2026-11-18）

结论仍为已阻塞。已验证官方发行物、原生 ACP、自定义模型的请求路由，以及 Safe 模式的审批和写入行为。原指定模型的真实推理与编辑器端到端验收尚未通过。

- 官方 Darwin arm64 发行 26.11.2（4100.5）压缩包 SHA-256 为 `ad103a996f0a4f5cee051f858bdf0fc33d4ee83b7c9219971d485046f084cd3c`，与 JetBrains 更新清单及 GitHub release asset digest 一致。检查 ZIP 条目后用 ditto 解出应用到私有目录，原生 `--version`、`--help` 均退出0。宿主 PATH 和 ShiJu 系统安装均未更改。[官方发行](https://github.com/JetBrains/junie/releases/tag/4100.5)
- 隔离 Junie 配置，以 `--acp=true` 启动，initialize 返回 ACP v1 和原生版本。空配置的 session/new 返回 -32000 authentication required。用官方 JSON 自定义模型配置后，session/new 成功并报告 `v1:6:custom:custom:shiju`。回环 HTTP 收到 `OpenAICompletion` 请求，model 精确为 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。回环 stub 用于定向验证，未用其响应充当真实推理。[自定义模型说明](https://junie.jetbrains.com/docs/custom-llm-models.html)
- 真实模型探针先读取指定租户配置并确认原模型在目录中。初次因探针误用旧 profile 目录超时，修正后网关基线读取超时。另核对网关 OpenAI 兼容处理器使用进程默认 provider，且只转发文本历史，因此该入口不能作为本次固定 provider 的工具调用中继。本轮未向外部推理端发请求，未提交目标 chat/stream。
- 原生 Safe 配置 `brave_mode=off` 经 ACP 重读确认。测试分别触发 shell 和 create 工具，宿主取消审批，两项私有文件均未写入。随后在私有目录批准一次 create，即使客户端声明支持 `fs.writeTextFile`，Junie 仍直接写入文件并报告工具完成，未发 `fs/write_text_file`。同一发行版加 `--sandbox` 后也复现，且未公布 Sandbox 配置项。探针仅在私有目录使用合成工具返回。该结果不能用审批回调代替宿主的文件边界。完整验收需先证明可由宿主执行的文件边界，再接入固定版本系统安装及租户绑定模型中继。[Junie EAP 沙箱说明](https://junie.jetbrains.com/docs/junie-cli-eap.html)
- 本轮无配置 PATCH。结束配置 GET 超时，无法确认快照重读；未产生待恢复的临时配置。两个自有网关分别退出0和在宽限期后退出-9，端口均关闭，原生探针均已退出。移除了原生探针初始化产生的空宿主实例目录。Rue.js 6303项普通文件与原快照一致，目标目录仍不存在。未运行 Cargo、浏览器或页面检查。
- 原始发行证据、探针帧、审批结果、配置和清理记录保存在本机0700目录 `/tmp/shiju-junie-retry-20261118/`。未提交供应商二进制、凭据或原始敏感载荷。


## Poolside 验收（2026-11-18）

结论：已阻塞。当前系统安装拒绝 Poolside，模型来源拒绝 `session_model`。前序 Junie 已记录阻塞并停止自有进程，未留临时配置写入。历史提交 `00f4d9c1d` 交付了 `pool acp` 注册、内置预设和协议夹具，来源任务文件已移除，当前缺少 `docs/dev/evidence/acp-onboarding-poolside.md`。

- 当前安装策略 `crates/libs/config-acp-install/src/lib.rs` 对 `poolside` 返回 `None`，`crates/libs/llm-runtime/src/acp/model_source/drivers/mod.rs` 未登记其已验证版本或驱动。复用 Harn 验收构建的 shiju 0.6.0，构建来源为 `f0d2a6d56a43ea6d1ff2673ebea254db698dd2fa` 附近的工作区加 Harn 实现，构建基线已包含 Poolside 注册提交。二进制 SHA-256 为 `245d9857ee0005fba1936d1a8bea31719406ef6c550bfe0cc53cb46115ce130e`，当前相关源码摘要另存私有证据。
- 在自有回环网关 `127.0.0.1:56388` 使用现有开发配置，请求均携带平台 10001、公司 100010005 和 Rue.js 目录头。安装、预设、ACP 及完整配置基线 GET 均为 HTTP 200。安装前后 `poolside=false`。系统安装 POST 返回 HTTP 400，curl 退出 0、耗时 0.000566 秒，响应为“该 ACP 暂无一键安装命令，请使用官方安装器”。宿主未找到 `pool` 入口，未取得 CLI 版本。
- 按当前 ACP 配置发送诊断，HTTP 200、curl 退出 0、耗时 0.011112 秒，结果为 `startup_failed`。模型能力 GET 返回 HTTP 200、curl 退出 0、耗时 0.000773 秒，`session_model=unsupported`、`reason=unsupported_cli`、`cli_local=supported`、`connected=false`。
- GET 确认 `acp-poolside` 的 `builtin=true`、`runtime.kind=acp`、`backend=poolside`、`enabled=false`。安装与模型前置阻塞后未 PATCH，也未提交指定 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 的创建请求。无 SSE 终态、会话绑定或产物验收证据。
- Rue.js 的 `packages/rue/src` 已确认存在。排除 `.git`、`node_modules`、`target` 的普通文件 SHA-256 快照前后各 6303 项，零差异，`acpdemo/poolside/` 前后均不存在。结束 GET 全局配置与基线一致，无临时配置待恢复。自有网关退出 0，端口关闭。
- 槽位 1 中 `cargo test -j 1 -p config-acp-install --lib acp_onboarding_poolside -- --test-threads=1` 通过 1 项，`cargo test -j 1 -p agent-config --lib acp_onboarding_poolside -- --test-threads=1` 通过 2 项，均退出 0。无代码变更，未重跑协议夹具、Clippy、桌面编译、页面构建或浏览器验证。
- 原始响应、指标、诊断请求、配置和快照存于本机 0700 目录 `/tmp/shiju-poolside-accept-20261118.aGudrk/`。补齐受审核的系统安装、指定模型驱动及脱敏接入证据后重跑任务 36。


### Poolside 官方发行与原生 ACP 重试（2026-11-18）

结论仍为已阻塞。官方 Poolside 1.0.16 可启动，原生 ACP 已证明能向 OpenAI 兼容端点发送完整指定模型 ID，并在隔离工作区验证了只读沙箱的应用审批。真实租户模型调用、ShiJu 系统安装及编辑器产物仍待验收。

- 官方安装脚本与 GitHub 发行均指向 v1.0.16。按脚本路径下载 Darwin arm64 包，SHA-256 `0932af3eb2b57a863acacb664ec8b2b1d3a76c2570a788b086001608cc585f74` 与官方 checksums.txt 和 GitHub 资产摘要均一致。压缩包仅含一个普通可执行文件，解出至 0700 私有目录。原生 `--version`、`--help`、`acp --help` 均退出 0。未执行官方安装脚本或修改宿主 PATH。[官方发行](https://github.com/poolsideai/pool/releases/tag/v1.0.16)
- 隔离 HOME 启动 `pool acp`，`initialize` 返回协议版本 1 和 agentInfo 1.0.16，空配置的 `session/new` 返回 -32000 Authentication required。通过官方 standalone 设置接入回环探针后，`session/new`、`session/set_config_option(model)` 均成功，重读模型值为 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。回环收到带工具定义的 `POST /v1/chat/completions`，请求的 model 精确匹配，ACP 返回 `end_turn`。分别验证目录自动选模、环境变量指定模型，以及 `GET /v1/models` 返回 404 时显式模型仍可发送。探针使用合成回复，未向真实推理端发送请求。[官方 OpenAI 兼容配置](https://github.com/poolsideai/pool/tree/v1.0.16#openai-compatible-api)
- 原生权限探针禁用 shell、subagent 和 web_fetch。允许路径调用了宿主 `fs/write_text_file`，宿主拒绝后仍残留一个零字节文件，允许该调用的独立探针则写入预期的 14 字节。明确拒绝的目录外路径未落盘。因此仅靠 ACP 写回调还不能保证宿主拒绝时无副作用。
- `--sandbox required` 在未配置或仅提供额外设置文件时以“no sandbox is configured”退出 1。改为官方文档指定的隔离 HOME 配置位置并复用本机已有 Docker 镜像后，原生 ACP 启动成功。只读沙箱把允许文件暂存在沙箱，目录外写入报 Permission denied。取消应用和保留在沙箱两种选择均未写入宿主，明确批准 `sandbox-apply` 后仅允许文件写入私有工作区，内容 14 字节，目录外文件仍不存在。未给工具挂载 Docker socket、宿主配置或凭据。[官方沙箱配置](https://docs.poolside.ai/sandboxes)
- 使用原 shiju 0.6.0 构建启动独立网关，按任务租户及目录头只读复查。安装状态仍为 false，模型来源仍为 `unsupported_cli`。租户配置目录中存在指定模型，当前进程无该供应商的 API key 环境变量。既有公开 OpenAI 兼容处理器使用进程默认 provider，且仅转换 role/content，无法为此次测试保证租户绑定和工具协议。原生回环结果表明 Poolside 登录并非自定义模型入口的必要条件。重跑需交付固定摘要的 ShiJu 系统安装、使用原租户授权凭据的请求级模型中继和经验证的宿主权限边界，再用完整指定模型走原创建请求。仅下载发行、向原生 CLI 设置模型或登录 Poolside 均无法解除当前 ShiJu 阻塞。
- 本轮未写配置或提交编辑器请求。网关结束配置与本轮及首次验收基线一致，自有网关和原生进程均退出，回环端口已关闭，沙箱重试前后 Docker 容器及网络列表相等。Rue.js 前后各 6303 项普通文件 SHA-256 无变化，目标目录仍不存在。无代码变更，未重跑 Cargo 或页面检查。原始发行核验、协议帧、回环请求、审批结果、配置重读和清理证据保存在本机 0700 目录 `/tmp/shiju-poolside-retry-20261118.7IKBGI/`。


## Stakpak 验收（2026-11-18）

结论为已阻塞。前序 Poolside 已记录阻塞并完成环境恢复。历史提交 `137e91a83` 交付了 `stakpak acp` 注册、内置预设和协议夹具。已从提交前的版本核对来源任务 16，当前缺少 `docs/dev/evidence/acp-onboarding-stakpak.md`。

- 当前注册位于 `crates/libs/acp-agent-registry/src/agent_registry.rs`，安装白名单 `crates/libs/config-acp-install/src/lib.rs` 对 Stakpak 返回 `None`。预设由 `crates/libs/agent-config/src/catalog.rs` 派生，模型分派 `crates/libs/llm-runtime/src/acp/model_source/drivers/mod.rs` 无 Stakpak 已验证版本及驱动。复用 Harn 验收产物 shiju 0.6.0，SHA-256 为 `245d9857ee0005fba1936d1a8bea31719406ef6c550bfe0cc53cb46115ce130e`。其构建基线 `f0d2a6d56a43ea6d1ff2673ebea254db698dd2fa` 已包含 Stakpak 注册提交，构建含前序 Harn 实现。当前相关源码摘要另存私有证据。
- 使用现有开发配置启动自有回环网关 `127.0.0.1:65243`。请求均携带平台 10001、公司 100010005 及 Rue.js 目录头。安装、预设、ACP 和全局配置基线 GET 均为 HTTP 200。系统安装 POST 返回 HTTP 400，curl 退出 0，耗时 0.000609 秒，提示暂无一键安装命令。安装前后均为 `false`，宿主 PATH 无 `stakpak`，未取得可执行入口或版本。
- 将 GET 返回的 Stakpak ACP 配置用于诊断。诊断 HTTP 200，curl 退出 0，耗时 0.011782 秒，结果为 `startup_failed`。模型能力 GET 返回 HTTP 200，curl 退出 0，耗时 0.000725 秒，报告 `session_model=unsupported`、`reason=unsupported_cli`、`cli_local=supported`、`connected=false`。
- 预设 GET 确认 `acp-stakpak` 的 `builtin=true`、`runtime.kind=acp`、`backend=stakpak`、`enabled=false`。因安装和模型能力阻塞，未 PATCH 启用或提交指定 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 的创建请求。无流终态、会话身份及模型绑定证据，编辑器产物未生成。
- 已确认 Rue.js 的 `packages/rue/src` 存在。排除 `.git`、`node_modules`、`target` 后，前后各 6303 项普通文件的 SHA-256 零差异，Git 状态一致，`acpdemo/stakpak/` 始终不存在。结束 GET 的全局配置与基线相同，无临时配置需恢复。自有网关退出 0，端口已关闭。
- 通过 `scripts/cargo_slot.py` 在槽位 1 串行运行定向测试。安装契约 1 项、预设与目录绑定 2 项通过，均退出 0。无代码变更，未重跑协议夹具、Clippy、桌面编译、页面构建或浏览器验证。该任务引用的旧 `.plans/references/rust.md` 不存在，沿用仓库现行槽位入口。
- 原始响应、诊断请求、网关日志、源码与构建摘要、测试日志及快照存于本机 0700 目录 `/tmp/shiju-stakpak-accept-20261118.BFPoyZ/`。补齐经审核的系统安装、指定会话模型驱动及脱敏接入证据后重跑任务 37。


### Stakpak 官方发行与原生 ACP 重试（2026-11-18）

结论仍为已阻塞。原生 Stakpak 可通过私有模型目录使用完整指定模型 ID 并发送带工具定义的请求。ShiJu 系统安装、真实租户模型调用与编辑器产物仍待验收。

- 官方 v0.3.88 Darwin arm64 发行包 SHA-256 `9c18808f7a3352a5ca2cbb9f123f2b2fafda03a5abef446c20065bb6ec629d1f` 与 GitHub release asset digest 一致。归档仅含一个普通可执行文件，解出至 0700 私有目录。原生 `--version` 返回 0.3.88，`acp --help` 正常退出。宿主 PATH 和系统安装状态未变。[官方发行](https://github.com/stakpak/agent/releases/tag/v0.3.88)
- 空配置 `initialize` 返回 ACP v1 及 agentInfo 0.3.88，`session/new` 返回 -32000 Authentication required。使用官方自定义 OpenAI 兼容 profile 和回环测试凭据后，`session/new` 成功，按短 ID 或完整 ID 提供 `/v1/models` 均未被原生 ACP 读取。它公布的目录为空，精确选模返回 -32602，直接提示在请求前因 `Provider not found: stakpak` 失败。固定版源码确认兼容端点读取本地 models.dev 缓存，ACP 选模要求精确匹配目录 ID。[官方配置](https://github.com/stakpak/agent/blob/v0.3.88/README.md#manual-configuration) · [兼容模型目录实现](https://github.com/stakpak/agent/blob/v0.3.88/libs/ai/src/providers/openai/provider.rs) · [ACP 选模实现](https://github.com/stakpak/agent/blob/v0.3.88/cli/src/commands/acp/server.rs)
- 在隔离 HOME 的模型缓存中登记完整 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，使目录条目的 provider 指向同名自定义端点。原生 `session/new` 公布精确的 provider 和模型 ID，`session/set_model` 成功，回环收到携带完整 ID 及 19 项工具定义的 `POST /v1/chat/completions`，合成回复以 `end_turn` 结束。另验证 OpenAI 类型的兼容 profile 配合私有缓存也能发出同一完整模型 ID。两种探针均只访问回环，无真实供应商推理、租户凭据或浏览器认证。[缓存读取实现](https://github.com/stakpak/agent/blob/v0.3.88/libs/ai/src/registry/models_dev.rs)
- 使用同名 provider 的隔离探针验证文件审批。取消审批时未收到写回调，文件未落盘。批准工具后拒绝 `fs/write_text_file`，原生返回工具错误，文件仍未落盘。独立批准探针由宿主回调写入私有目录中预期的 14 字节。仅验证了受测的原生创建路径，Rue.js 编辑器任务未提交。
- 原 shiju 0.6.0 构建的自有回环网关复查仍为 `installed=false` 和 `session_model=unsupported_cli`。租户配置目录含指定模型，探针环境无配置声明的供应商密钥。既有公开 OpenAI 兼容处理器使用进程默认 provider，并将消息转换为文本历史，无法保证本任务的租户绑定和工具协议。复跑需要受审查的固定摘要系统安装、使用原租户授权的请求级模型中继，以及由 Stakpak 驱动生成的私有模型目录并验证宿主权限边界。安装发行物或设置原生 profile 本身无法解除 ShiJu 的能力门控。
- 两轮全局配置重读一致，无 PATCH 或临时配置需恢复。自有网关和原生探针均退出 0，两轮网关端口已关闭。Rue.js 快照前后各 6303 项普通文件零差异，目标目录始终不存在。未重跑 Cargo、页面构建或浏览器验证。发行校验、原生帧、回环请求、审批和清理记录存于本机 0700 目录 `/tmp/shiju-stakpak-retry-20261118.w71m2q/`。


## VT Code 验收（2026-11-18）

结论为已阻塞。ShiJu 暂无 VT Code 一键安装，指定的 `session_model` 路径返回 `unsupported_cli`。

- 前序 Stakpak 已记录阻塞和环境恢复。历史提交 `eb8ff9d4a` 交付 VT Code 注册、内置预设和协议夹具。已从提交前版本核对来源任务 17，并阅读脱敏接入记录。当前注册入口为 `vtcode acp`，安装白名单对 `vtcode` 返回 `None`，模型来源分派中缺少 VT Code 的已验证版本及驱动。
- 复用前序 Harn 验收的 shiju 0.6.0 二进制，SHA-256 为 `245d9857ee0005fba1936d1a8bea31719406ef6c550bfe0cc53cb46115ce130e`。构建基线 `f0d2a6d56a43ea6d1ff2673ebea254db698dd2fa` 包含 VT Code 接入提交，构建还含前序 Harn 实现。当前相关源码摘要另存私有证据。
- 根据 CLI 帮助使用现有开发配置启动自有网关 `127.0.0.1:55539`。所有请求携带平台 10001、公司 100010005 及 Rue.js 目录头。安装、预设、ACP 和全局配置基线 GET 均为 HTTP 200。系统安装 POST 返回 HTTP 400，curl 退出 0，耗时 0.000583 秒，提示暂无一键安装命令。安装前后均为 `false`，宿主 PATH 无 `vtcode`，没有取得真实入口或版本。
- 普通诊断使用 GET 读出的 VT Code ACP 配置，返回 HTTP 200，curl 退出 0，耗时 0.008809 秒，结果为 `startup_failed`。独立模型能力 GET 返回 HTTP 200，curl 退出 0，耗时 0.000657 秒，报告 `session_model=unsupported`、`reason=unsupported_cli`、`cli_local=supported`、`connected=false`。
- GET 确认 `acp-vtcode` 的 `builtin=true`、`runtime.kind=acp`、`backend=vtcode`、`enabled=false`。因安装和模型能力阻塞，未 PATCH 启用或提交指定 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 的创建请求。会话终态、权限及模型绑定仍待验证，编辑器产物未生成。
- 已确认 Rue.js 的 `packages/rue/src` 和 `@rue-js/rue` 包。排除 `.git`、`node_modules`、`target` 后，前后各 6303 项普通文件的 SHA-256 零差异，Git 状态一致，`acpdemo/vtcode/` 始终不存在。结束 GET 的全局配置与基线相等，自有网关退出 0，端口已关闭。
- 通过 `scripts/cargo_slot.py` 在槽位 1 串行执行 `cargo test -j 1 -p config-acp-install --lib acp_onboarding_vtcode -- --test-threads=1` 和 `cargo test -j 1 -p agent-config --lib acp_onboarding_vtcode -- --test-threads=1`。安装契约 1 项、预设及目录绑定 2 项通过，均退出 0。任务引用的旧 `.plans/references/rust.md` 缺失，沿用仓库现行槽位入口。未运行协议夹具、Clippy、桌面编译、页面构建或浏览器验证。
- 原始请求、响应、日志、摘要及快照保存在本机 0700 目录 `/tmp/shiju-vtcode-accept-20261118.wwdBSD/`。补齐经审核的系统安装和指定会话模型驱动后重跑任务 38。


### VT Code 官方发行与原生 ACP 重试（2026-11-18）

结论仍为已阻塞。官方发行版已在隔离环境证明原生 ACP 可使用完整指定模型 ID 和工具协议。ShiJu 系统安装、真实租户推理和 Rue.js 编辑器产物仍待验收。

- 官方 Darwin arm64 发行 0.183.0 的归档 SHA-256 为 `ceb895c724cd666f283169cf2c400a2a3988dfc97cc8b62d75882ffedc218f2d`，与 GitHub 资产摘要和官方 .sha256 一致。检查归档后只提取普通可执行文件到 0700 私有目录。原生 `--version` 返回 `0.183.0+4fd8e47a885c`，`acp --help` 正常运行。宿主 PATH 和 ShiJu 安装状态未变。[官方发行](https://github.com/vinhnx/VTCode/releases/tag/0.183.0)
- 使用隔离 HOME、私有工作目录及限制写入和网络的 macOS 沙箱执行 `vtcode acp`。空配置下，`initialize` 返回 ACP v1，`session/new` 成功但当前 provider 为 OpenRouter，指定模型被拒绝。旧 `session/set_model` 返回 -32601。私有 TOML 配置同名 OpenAI 兼容 provider 后，`session/set_config_option` 的 model 项重读为 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，provider 项为 `alibaba-token-plan-cn`。配置的推理档位 low 被无档位能力的合成 provider 明确拒绝，探针改为 none 后可请求。[固定版本 ACP 指南](https://github.com/vinhnx/VTCode/blob/0.183.0/docs/guides/zed-acp.md) · [固定版本配置示例](https://github.com/vinhnx/VTCode/blob/0.183.0/vtcode.toml.example)
- 回环收到带完整 ID 和 5 项工具定义的非流式 `POST /v1/chat/completions`。显式模型配置没有触发 `GET /v1/models`，常规与目录不可用探针均用合成响应取得 `end_turn`。工具探针取消 `apply_patch` 审批后文件不存在，独立批准探针写出预期的 15 字节。批准后没有收到 `fs/write_text_file`，说明该路径由原生工具直接写入，集成时需用外部边界约束原生工具写入。
- 自有网关按原租户及目录头只读复查，`installed=false`、`session_model=unsupported_cli`。全局租户配置存在原模型条目，探针进程缺少该 provider 声明的密钥环境变量。网关公开 OpenAI 兼容端点使用进程默认 provider，且只转换文本消息，不能作为本次固定租户及工具协议的中继。未替换 provider 或模型，未向真实上游发请求。重跑需在 ShiJu 补齐固定摘要系统安装、使用原租户可用授权的请求级模型中继、现代 ACP 选模确认及原生工具写入边界，再用原创建请求验证。
- 本轮未写配置或提交编辑器请求。全局配置重读与首轮基线一致。自有网关退出 0，原生探针由自有进程组终止，复查的 7 个回环端口均关闭。Rue.js 前后 6303 项普通文件的 SHA-256 零差异，Git 状态一致，目标目录不存在。无代码变更，未重跑 Cargo 或页面验证。发行校验、ACP 帧、回环请求、审批结果及清理证据保存在本机 0700 目录 `/tmp/shiju-vtcode-retry-20261118.RGuroQ/`。


## MiMo Code 验收（2026-11-18）

结论为已阻塞。系统安装器装入了 MiMo Code 0.1.17，网关仍报告未安装。普通 ACP 诊断需要认证，指定的 `session_model` 被当前 ShiJu 能力门控拒绝。

- 前序任务 38 已记录阻塞和环境恢复。从 `6ef10d08d^` 核对来源任务 18，提交 `6ef10d08d` 交付了 MiMo Code 注册、系统安装契约、内置预设和协议夹具。当前缺少 `docs/dev/evidence/acp-onboarding-mimo-code.md`。注册以 `mimo acp` 为本地入口，备用入口为 `npx -y @mimo-ai/cli@0.1.17 acp`。安装白名单固定版本 0.1.17，`llm-runtime` 的模型来源分派尚无 MiMo Code 已验证版本及驱动。
- 复用 Harn 验收的 shiju 0.6.0 二进制，SHA-256 为 `245d9857ee0005fba1936d1a8bea31719406ef6c550bfe0cc53cb46115ce130e`。构建使用 `f0d2a6d56a43ea6d1ff2673ebea254db698dd2fa` 附近工作区及 Harn 实现，该基线包含 MiMo Code 接入提交。当前相关源码摘要存于私有证据。
- 按 CLI 帮助使用现有开发配置启动自有回环网关 `127.0.0.1:64943`。所有请求携带平台 10001、公司 100010005 和 Rue.js 目录头。安装、预设、ACP、全局配置基线 GET 均为 HTTP 200。
- 系统安装 POST 返回 HTTP 200，curl 退出 0，耗时 6.067971 秒，npm 全局安装退出 0。安装前后网关均返回 `installed=false`。包入口位于 `/opt/homebrew/Cellar/node/26.13.0_2/bin/mimo`，当前 PATH 缺少该目录。直接执行该入口的 `--version` 返回 0.1.17，退出 0，入口脚本 SHA-256 为 `1e7ec982f15b2c193497d34b0a70ceecb9af2ceba6c4faf60ed71d49d2c96378`。网关读取的合并配置仍走固定版 `npx`，`gateway-global::acp_command_installed` 对该下载入口返回 false。保留本次系统安装的包。
- 用网关读出的 ACP 配置执行诊断，HTTP 200，curl 退出 0，耗时 1.616698 秒，返回 `authentication_required`。模型能力 GET 返回 HTTP 200，curl 退出 0，耗时 0.000658 秒，报告 `session_model=unsupported`、`reason=unsupported_cli`、`cli_local=supported`、`connected=false`。
- 预设 GET 确认 `acp-mimo-code` 的 `builtin=true`、`runtime.kind=acp`、`backend=mimo-code`、`enabled=false`。安装发现和模型来源前置条件未满足，未 PATCH 启用、验证启用持久化或发送指定 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 的创建请求。无 SSE、会话 summary/options、权限和选模绑定证据，编辑器产物未生成。
- 已确认 Rue.js 的 `packages/rue/src` 和 `@rue-js/rue` 包。排除 `.git`、`node_modules`、`target` 后，前后各 6303 项普通文件 SHA-256 零差异，Git 状态一致，`acpdemo/mimo-code/` 始终不存在。结束配置 GET 与基线相等，自有网关退出 0、端口已关闭。
- 槽位 1 串行执行 `cargo test -j 1 -p config-acp-install --lib acp_onboarding_mimo_code -- --test-threads=1` 和 `cargo test -j 1 -p agent-config --lib acp_onboarding_mimo_code -- --test-threads=1`。安装契约 1 项、预设和目录绑定 2 项通过，均退出 0。仓库缺少旧 `.plans/references/rust.md`，本次使用现行 `scripts/cargo_slot.py`。未重跑协议夹具、Clippy、桌面编译、页面构建或浏览器验证。
- 原始请求与响应、npm 与运行日志、源码与产物摘要、测试及两次快照存于本机 0700 目录 `/tmp/shiju-mimo-code-accept-20261118.pHdvAm/`。修复安装入口发现并补齐经审核的指定会话模型驱动及接入证据后，按任务 39 完整重跑。


### MiMo Code 官方发行与原生 ACP 重试（2026-11-18）

结论仍为已阻塞。已验证官方发行、网关进程内入口发现和原生工具协议。ShiJu 尚无 MiMo Code 的指定会话模型驱动，真实租户推理和 Rue.js 编辑器仍待验收。

- 官方 Darwin arm64 v0.1.17 归档的 SHA-256 `90ce4ce3457d581a4262c42f62c5e20b8d9dedec0ddc05a051b695f2732fe381` 与 GitHub 资产摘要一致。归档只含普通可执行文件 `mimo`，其 SHA-256 `254d1372465ae60e28a2684b97d088a105d0e34672110345a7a87ed152be5270` 与系统安装器装入 npm 包的原生文件完全一致。该二进制的 `--version` 与 `acp --help` 正常退出。[官方发行](https://github.com/XiaomiMiMo/MiMo-Code/releases/tag/v0.1.17)
- 在私有 HOME、私有工作目录及仅允许回环通信的 macOS 沙箱中启动原生 ACP，`initialize` 自报 `OpenCode` 0.1.17、协议 v1。空配置虽然接受 `session/set_model` 并回显精确 ID，但 `session/new` 公布的初始值为 Xiaomi 模型，空配置探针未发送 prompt。隔离配置登记同名 OpenAI 兼容 provider 后，短 ID 模型键会让 ACP 会话显示完整 ID，但 HTTP 请求使用短 ID。将完整 ID 用作模型键后，原生目录使用 `alibaba-token-plan-cn/alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。使用该原生目录值选模并复查配置，回环收到完整原始模型 ID、流式请求和 4 项工具定义，合成应答产生 `end_turn`。回显和辅助模型请求均不足以证明主提示路由。[固定版本自定义端点说明](https://github.com/XiaomiMiMo/MiMo-Code/blob/v0.1.17/README.md#custom-openai-compatible-endpoints)
- 合成 `write` 探针在宿主取消 `session/request_permission` 后没有落盘。允许一次审批、拒绝 `fs/write_text_file` 时，原生工具仍写出 15 字节并报告成功。独立探针允许宿主写回也产出相同内容。另一探针批准权限但由外部沙箱禁止私有工作目录之外的测试路径，写入失败、文件未落盘。接入需保证原生回退写入也受宿主文件边界约束。
- 两个自有网关仅在自身 PATH 添加 npm 安装所在目录。指定租户的合并配置解析到 `mimo`，安装状态从首轮的 `false` 变为 `true`。初次全局配置 GET 在 30 秒超时，第二次以 90 秒时限重读成功。能力 GET 返回 `session_model=unsupported_cli`，`connected=false`，预设仍停用。租户目录有原模型，探针进程缺少该 provider 声明的密钥环境变量，未验证其他凭据来源。公开 OpenAI 兼容端点使用进程默认 provider 并转换纯文本消息，缺少本任务所需的租户绑定与工具转发。已有 OpenCode 驱动对应独立后端，未替换 MiMo Code。
- 未改宿主 PATH、租户配置或预设，未发真实上游调用或 Rue.js 创建请求。补齐可稳定发现的固定版入口、经审查的 MiMo Code 请求级模型中继、原租户授权和强制写入边界后，再按原请求验收启用重读、会话绑定与文件创建。
- 两个自有网关和最终探针均退出 0，回环端口复查关闭。结束全局配置与本轮及首轮基线一致，Rue.js 6303 项普通文件 SHA-256 零差异，Git 状态一致，目标目录未创建。本轮无代码变更，沿用首轮 3 项定向单测证据，未重跑 Cargo、浏览器或页面构建。私有证据目录为 `/tmp/shiju-mimo-code-retry-20261118.eAGmwr/`。


## omp 验收（2026-11-18）

结论为已阻塞。系统安装器装入了 OMP 18.22.0，网关可以发现入口。入口缺少 Bun 无法启动，ShiJu 对指定的会话模型来源返回 `unsupported_cli`。

- 前序 MiMo Code 已记录阻塞与环境恢复。从 `e683f7394^` 核对来源任务 19。提交 `e683f7394` 和 `34e5a5595` 交付了 OMP 注册、安装白名单、内置预设和协议夹具。当前缺少任务 19 约定的脱敏接入证据。注册优先发现 `omp acp`，备用入口为 `npx -y @oh-my-pi/pi-coding-agent@18.22.0 acp`。当前 `llm-runtime` 的 `acp/model_source/drivers/mod.rs` 无 OMP 已验证版本及模型驱动。
- 复用 Harn 验收的 shiju 0.6.0 二进制，SHA-256 为 `245d9857ee0005fba1936d1a8bea31719406ef6c550bfe0cc53cb46115ce130e`。构建基线 `f0d2a6d56a43ea6d1ff2673ebea254db698dd2fa` 包含两笔 OMP 接入提交，构建含前序 Harn 实现。为自有网关进程的 PATH 加入 npm 可执行目录，宿主 PATH 无修改。
- 按 CLI 帮助使用现有开发配置，在 `127.0.0.1:60336` 启动自有网关。请求均携带平台 10001、公司 100010005 和 Rue.js 目录头，安装、预设、ACP 及全局配置基线 GET 均为 HTTP 200。系统安装 POST 返回 HTTP 200，curl 退出 0，耗时 34.055991 秒。该响应在白名单安装子进程成功退出后返回。网关报告安装状态从 `false` 转为 `true`，保留本次安装的包。
- 系统入口为 `/opt/homebrew/Cellar/node/26.13.0_2/bin/omp`。链接指向包内 `dist/cli.js`，其 SHA-256 为 `3829c41cecf990533d04467d5391bd5076abaa4a16cbffc14e336527895a3cd3`。本地包清单确认版本 18.22.0，并声明 Bun >=1.4.4。入口的解释器为 `/usr/bin/env bun`，实跑 `--version` 退出 127，报告找不到 Bun，未取得 CLI 自报版本。
- 将安装前 GET 读出的固定版 `npx` ACP 配置交给普通诊断。HTTP 200，curl 退出 0，耗时 2.000700 秒，结果为 `handshake_failed`。独立模型能力 GET 为 HTTP 200，curl 退出 0，耗时 0.001043 秒，报告 `session_model=unsupported`、`reason=unsupported_cli`、`cli_local=supported`、`connected=false`。
- 预设 GET 确认 `acp-omp` 的 `builtin=true`、`runtime.kind=acp`、`backend=omp`、`enabled=false`。因启动与模型能力均阻塞，未 PATCH 启用或验证保存重读，未提交指定 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 的创建请求。会话 SSE、summary、ACP 选项、权限和绑定无证据，编辑器文件未生成。
- 已确认 Rue.js 的 `packages/rue/src` 和 `@rue-js/rue` 包。排除 `.git`、`node_modules` 和 `target` 后，前后快照分别有 6308 和 6311 项普通文件，差异为 `acpdemo/stakpak/README.md`、`acpdemo/stakpak/index.html`、`acpdemo/stakpak/src/App.tsx` 及 `acpdemo/stakpak/src/style.css`。这些文件在验收窗口内由外部活动变动，本次未执行 OMP 聊天或写入，未处理这些文件。Git 短状态一致，`acpdemo/omp/` 始终不存在。
- 结束 GET 的全局配置与基线一致，本次无临时配置需恢复。自有网关正常退出，端口已关闭。仓库有其他 Rust 编译运行，复制槽位 1 已构建的 `config-acp-install` 和 `agent-config` 测试产物后运行过滤 `acp_onboarding_omp --test-threads=1`，分别 1 项和 2 项通过，均退出 0。旧 `.plans/references/rust.md` 缺失。未重新调用 Cargo，未运行协议夹具、Clippy、桌面编译、页面构建或浏览器检查。
- 原始响应、网关日志、源码与构建摘要、测试和快照存于本机 0700 目录 `/tmp/shiju-omp-accept-20261118.796KMW/`。复跑需使网关可发现受支持的 Bun，补齐经实证的 OMP 指定模型驱动与脱敏接入证据，并在验收窗口内协调 Rue.js 工作区写入。


### Stakpak 补齐接入与最终实机验收（2026-11-18）

结论为通过。补齐固定版本系统安装和指定会话模型驱动后，Stakpak 通过真实租户模型创建了 8 项完整编辑器文件。首次会话失败，预建空目标目录后原请求通过。

- 提交 `f6a2a91df` 固定官方 Stakpak 0.3.88 Darwin arm64 发行和归档、二进制、launcher 摘要，安装器仅接受固定 HTTPS 源和单普通文件归档。沿用 Cursor 的受审查运行时与请求级模型中继。驱动为原生选模生成仅含一个模型的私有目录，CLI 请求使用中继要求的模型 ID，ShiJu 保留原租户及 provider 绑定。请求凭据存于私有启动文件，launcher 使用隔离 HOME、回环网络与文件写入限制，屏蔽本机命令执行。模型仅收到查看、创建和替换工具，文件写入使用现有宿主审批及 ACP 回调。原生正向、取消审批、回调拒写和获批落盘各实测一项，使用合成回环回复。
- 复用当前共享工作区编译的 shiju 0.6.0，确认产物含 Stakpak 专用工具白名单及固定摘要运行时，二进制 SHA-256 为 `75501c6789b6166fdce7f2c9c0359b0c96f7020205eb171eae11fc45524ade15`。使用现有开发 SQLite 配置与凭据的 0700 隔离副本启动自有网关，保留平台 10001、公司 100010005 和 Rue.js 目录。系统安装 HTTP 200、curl 退出 0、21.000876 秒，状态从 false 到 true，管理入口 `stakpak --version` 为 0.3.88，模型能力重读支持 `session_model`。普通诊断 HTTP 200，结果分别为 `authentication_required` 与 `timed_out`。
- GET 预设确认 `builtin=true`、`backend=stakpak`。首次临时 PATCH 返回 HTTP 200 和 JSON `null`，验收脚本初版误判响应后恢复预设，未发聊天请求。修正后两轮均 PATCH 并重读 `enabled=true`。首轮真实请求 HTTP 200、35.551140 秒，Stakpak 将不存在的 `acpdemo/stakpak` 当作文件读取，宿主返回 ENOENT，原生 ACP 传播为 `chat.error`。无文件写入。预建空目录后重新发出原请求，HTTP 200、curl 退出 0、90.485968 秒，收到 `chat.done`，无 `chat.error`。
- 成功会话 `ses_1ec573161001WH9A4gEIhWdszR` 执行期间确认 `connected=true`，summary 的预设为 `acp-stakpak`、后端为 `stakpak`、`full_access=true`。执行期间及结束后 options 的 `selected_model` 均为 `source=session_model`、`provider_id=alibaba-token-plan-cn`、`model=alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。结束后连接释放，`connected=false`。
- Stakpak 完成 11 项编辑工具操作。实际产物为 `index.html`、`vite.config.mjs`、`tsconfig.json`、`src/main.tsx`、`src/App.tsx`、`src/env.d.ts`、`src/style.css`、`README.md`。静态确认 Vite 复用本仓库 Rue 源码别名，textarea 输入驱动 computed 实时统计，localStorage 同步恢复和防抖写入，离页补写未保存内容。README 提供根目录运行命令。未运行浏览器、页面构建或创建后页面回归。
- Rue.js 普通文件 SHA-256 快照由 6303 项变为 6311 项，差异仅上述 8 项目标文件。两轮实际聊天后均恢复并重读完整配置和有效预设，均等于各轮基线。首次安装阶段恢复了有效预设，隔离副本额外保留显式停用值，最终随副本清理。所有自有网关退出 0、回环端口已关闭，原开发配置和服务未改动。诊断产生的空 Stakpak 本地状态已按创建时间、文件布局和空会话库核验，移入私有证据。验收 SQLite 和凭据副本已清理。
- 槽位 1 获得安装白名单和运行时注册各 1 项预期失败。槽位 3 复测注册 1 项、安装契约 2 项通过，均退出 0。槽位 3 对 `config-acp-install`、`acp-agent-registry`、`acp-client` 和 `llm-runtime` 的库目标执行 `cargo clippy -j 1 --lib --no-deps`，退出 0。保留 `llm-runtime` 既有 101 项警告，新增 Stakpak 驱动无警告。已完成静态产物核对和目录边界检查。
- 私有证据 `/tmp/shiju-stakpak-patch-20261118/` 保存安装回执、构建摘要、原始响应、SSE、产物 SHA-256、基线与清理记录。原始配置和供应商二进制均未提交。


### omp 官方发行与原生 ACP 重试（2026-11-18）

结论仍为已阻塞。私有 Bun 恢复了系统安装入口，官方独立二进制也可启动。两种入口都已通过隔离 ACP 向回环发送完整指定模型 ID。原租户推理、ShiJu 会话绑定及 Rue.js 编辑器落盘仍待验收。

- 校验 OMP 18.22.0 官方 npm 归档的 SHA-512 与 registry integrity 一致，归档内 `dist/cli.js` 与系统安装入口 SHA-256 一致。Bun 1.4.4 Darwin arm64 归档 SHA-256 为 `c240d7f0ccb90c263a849fcdb09b7ee62630d921348ed6aafda86b9d577bc510`，与 GitHub 资产摘要一致。私有 PATH 加入解出的 Bun 后，系统入口自报 `omp/18.22.0`。官方独立 Darwin arm64 产物 SHA-256 为 `d185113d63eba23925e5d70ab661a424b827c06c98f8e766c3bf4ad62ddaff9a`，与 GitHub 资产摘要及官方校验清单一致，不依赖 PATH 中的 Bun 即可自报相同版本。[OMP 固定发行](https://github.com/can1357/oh-my-pi/releases/tag/v18.22.0) · [OMP npm 发行](https://registry.npmjs.org/@oh-my-pi%2fpi-coding-agent/18.22.0) · [Bun 固定发行](https://github.com/oven-sh/bun/releases/tag/bun-v1.4.4)
- 在私有 HOME、工作目录及限制文件写入和仅允许回环联网的 macOS 沙箱内，两个入口均回报 ACP v1、OMP 18.22.0。空配置未发布可用的指定模型，旧 `session/set_model` 被拒。私有 `models.yml` 注册原 provider，短模型条目虽在 ACP 显示完整模型名，实际 HTTP 仍只发送短 ID。以完整 ID 登记模型时，经 `session/set_config_option` 选择并重读带双 provider 前缀的原生选项 `alibaba-token-plan-cn/alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，两种入口均向回环 `POST /v1/chat/completions` 发送完整原模型 ID 和 11 项工具定义，合成响应终态为 `end_turn`。[固定版模型配置](https://github.com/can1357/oh-my-pi/blob/v18.22.0/docs/models.md)
- 在宣告宿主文件回调的受测写入路径上，OMP 直接发送 `fs/write_text_file`，未先请求 ACP 审批。拒绝宿主回调时工具报告失败且未落盘，允许受限回调时写出预期 15 字节。省略文件回调能力时，私有目录内的 15 字节由 OMP 原生工具直接写入，目录外测试写入被沙箱拦下。接入时需同时约束宿主回调和 CLI 原生写入。
- 用原租户、目录和现有开发配置复查。自有网关仅在自身 PATH 引入私有 Bun，读取的绝对 OMP 入口可发现，`installed=true`。使用该入口与私有诊断 HOME，普通诊断 HTTP 200，curl 退出 0，耗时 1.543001 秒，返回 `authentication_required`。诊断在 initialize 返回认证方法时即报告该状态。模型来源 GET 仍返回 `unsupported_cli`、`connected=false`。原模型在租户目录存在，网关启动环境缺少其 provider 声明的一个授权变量，其他凭据来源未实测。
- 当前 OMP 未接入已验证版本、协议分派或请求级模型驱动。原生私有模型目录与独立二进制均未改变 ShiJu 能力门控。公开 OpenAI 兼容端点用进程默认 provider 处理纯文本消息，无法作为原租户与工具协议的中继。复跑需持久可发现的固定版运行入口、经审查的请求级模型中继、原租户可用授权、宿主和原生工具写入边界，以及脱敏接入证据。届时按原请求验证启用保存、真实指定会话和文件创建。
- 两轮复查均无 PATCH、真实上游推理或 Rue.js 创建请求。第一次全局配置前后相同。第二次仅 `agents.acp-junie.enabled` 在复查窗口变化，首轮验收到本轮之间也只涉及该 Junie 条目。本任务未写入或回滚其他任务的配置。两轮 Rue.js 快照各 6311 项，零差异，Git 状态一致，OMP 目标目录未创建。两个自有网关和十个原生探针全部退出 0，所用回环端口关闭。私有 Bun 和独立 OMP 只保留在本机 0700 证据目录 `/tmp/shiju-omp-retry-20261118.yQhe7P/`，宿主 PATH 未改。本轮沿用首轮三项定向测试，未重跑 Cargo、页面构建或浏览器验证。


## aionrs 验收（2026-11-18）

结论为已阻塞。系统安装入口拒绝 aionrs，模型来源接口拒绝 `session_model`，编辑器尚未创建。

- 从提交 `eaf5d0832^` 核对来源任务 21。提交 `eaf5d0832`、`797bc79f3` 与脱敏接入笔记 `docs/dev/evidence/acp-onboarding-aionrs.md` 确认桥接及预设已交付。任务 41 已在 `568046033` 移除，缺少独立 `shiju-pi-cli` 验收及恢复回执，批次 17 的 Pi 记录对应后端 `pi`。本次检查使用独立的开发配置副本。
- 复用 Harn 验收的 shiju 0.6.0，SHA-256 为 `245d9857ee0005fba1936d1a8bea31719406ef6c550bfe0cc53cb46115ce130e`。构建源为 `f0d2a6d56a43ea6d1ff2673ebea254db698dd2fa` 附近共享工作区及 Harn 实现，基线包含上述两个 aionrs 提交。当前桥接与安装执行分支和基线一致，相关源码摘要保存在私有目录。
- 在自有回环网关 `127.0.0.1:58553` 使用开发 SQLite 配置与授权文件的隔离副本，所有请求保留平台 10001、公司 100010005 和 Rue.js 目录头。安装、预设、ACP 及全局配置基线 GET 均为 HTTP 200。系统安装 POST 返回 HTTP 400，curl 退出 0、耗时 0.001595 秒，响应为“该 ACP 暂无一键安装命令，请使用官方安装器”。`gateway-desktop` 在白名单检查处返回，`config-acp-install` 对 aionrs 返回 `None`。安装状态前后均为 false。合并配置解析到 `aionrs-acp`，无参数。PATH 中两个可执行入口均缺失，未取得 CLI 版本。
- 按读出的 ACP 配置调用普通诊断，HTTP 200、curl 退出 0、0.039814 秒，返回 `startup_failed`。模型能力 GET 为 HTTP 200、curl 退出 0、0.001021 秒，报告 `session_model=unsupported`、`reason=unsupported_cli`、`cli_local=supported`、`connected=false`。当前 `llm-runtime` 未登记 aionrs 的已验证版本或模型驱动。
- 预设 GET 确认 `acp-aionrs`、`builtin=true`、`runtime.kind=acp`、`backend=aionrs`、`enabled=false`。前置条件未满足，未 PATCH、未发送指定 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 的聊天请求，无 SSE、summary、ACP 选项、权限及选模绑定证据。
- 已确认 `packages/rue/src` 与 `@rue-js/rue`。排除 `.git`、`node_modules`、`target` 后，前后各 6311 项普通文件 SHA-256 零差异，Git 短状态一致，`acpdemo/aionrs/` 始终不存在。结束配置 GET 等于基线，无临时启用值需恢复。自有网关退出 0、端口已关闭，隔离配置副本已清理。
- 复用槽位 1 已复制的 `agent-config` 测试二进制，过滤 `acp_onboarding_aionrs --test-threads=1`，预设元数据和目录绑定共 2 项通过，退出 0。其他 Rust 编译正在运行，本次未重编译或执行桥接协议及注册集成测试。未运行 Clippy、桌面编译、页面构建或浏览器验证。
- 原始请求、响应、日志、快照与摘要存于本机 0700 目录 `/tmp/shiju-aionrs-accept-20261118.fxP6on/`。补齐双入口的受管安装、经实证的指定模型与权限链路，并确认任务 41 的环境恢复后，按任务 42 完整重跑。


### Poolside 补丁与真实模型验收（2026-11-18）

系统安装、内置预设和真实指定模型调用已通过。任务 36 仍为已阻塞，真实会话在两段 SSE 等待期间均未发起写入，未取得编辑器或成功终态。

- 提交 `15bfb9815`、`decc0cdd8` 将 Darwin arm64 的官方 Poolside 1.0.16 纳入 ShiJu 的固定版本安装。托管启动在会话工作区建立只读沙箱，使用当前请求自己的租户模型中继。原生沙箱暂存的文本变更逐文件交给宿主 ACP 写入接口，原生批量应用始终被拦截。启动要求可用的本地 Docker 引擎，使用固定摘要的容器镜像。原生定向探针验证了嵌套新文件、目录外拒绝及宿主拒绝后零残留。
- 使用槽位 1 构建 `shiju 0.6.0`，命令为 `cargo build -j 4 -p vw-cli --bin shiju`，退出 0。构建完成时工作区 HEAD 为 `5c05e7d46674ce0bb26309bc083619c7967d3db2`，包含 Poolside 补丁和同仓并行任务变更。二进制 SHA-256 为 `0b3e616b039ead3877dc087b0324452692585e2c5adc5c48981a68b5d9d3366d`。
- 独立网关使用现有开发 SQLite 配置及密钥的私有临时副本，保留原平台、公司和 Rue.js 目录头。系统安装 HTTP 200，curl 退出 0，98.150942 秒，安装状态由 false 变为 true。管理入口的原生版本为 1.0.16，原生二进制 SHA-256 为 `5dcb0162fd21a535e4f67117125e7d205e27a6881862aa223d0424082848f874`。普通诊断 HTTP 200、`connected`。模型能力报告 `session_model=supported`。内置 `acp-poolside` 默认停用，临时启用后 GET 重读确认生效。
- 使用任务原始请求提交一次。运行中和续传结束后的 summary/options 均确认会话 `ses_1ec6e3281001IsHjkwT4oKwXyq` 为 `acp-poolside`、后端 `poolside`、`full_access=true`、`connected=true`、`access_status.support=client_only`，`selected_model` 为 `session_model`、`provider_id=alibaba-token-plan-cn`、完整模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。真实模型流返回文本和源码读取工具结果。
- 首段 HTTP 200、curl 退出 28、300.005226 秒。查询确认会话仍在线，使用原 stream request ID 和 Last-Event-ID 续传。续传 HTTP 200、curl 退出 28、300.001471 秒。两段共 95 次读取工具调用，无写入调用、`chat.done` 或 `chat.error`。目标目录始终不存在。未重新提交相同创建请求。后续完整验收须在同一模型上取得实际文件写入和成功终态，尚无证据将本次长时间读取归因于模型中继或宿主写入失败。
- 启用配置只写入临时副本。结束时 PATCH 恢复停用，GET 确认有效预设与基线一致。副本配置多出一个显式停用覆写，退出后临时副本已删除，原开发配置未收到 PATCH。自有网关退出 0，回环端口关闭，无残留 Poolside 容器。Rue.js 前后各 6311 个普通文件摘要相同。
- 槽位 1 中安装契约 1 项、模型选项及工作目录 1 项、Poolside ACP 协议 3 项均通过。Python 暂存过滤测试 2 项和原生允许/拒绝探针通过。限定四个受影响库的 `cargo clippy --lib --no-deps` 退出 0，已有 lint 仍在。未运行桌面编译、页面构建或浏览器检查。原始请求、SSE、配置、构建与清理证据仅存 0700 私有目录 `/tmp/shiju-poolside-bridge.VUxOmR/`，原生实测另存 `/tmp/shiju-poolside-new-dir.dzL1Tg/`。


### aionrs 官方版本与原生 ACP 复验（2026-11-18）

结论仍为已阻塞。官方 CLI 可将完整指定模型 ID 和工具协议发送到回环端点。ShiJu 的系统安装、桥接建会和会话模型绑定仍有独立阻塞，Rue.js 编辑器未生成。

- 官方 v0.2.11 标签指向 `8e61a90329fa9f67c4fdf7e97fe02c24dba33f75`，发行页仅列出两个源码归档，发布 API 的二进制资产列表为空。从该提交归档在槽位 1 执行 `cargo build -j 1 --locked --manifest-path <源码>/Cargo.toml -p aion-cli --bin aionrs`。源码归档 SHA-256 为 `102f0682858a4f02b505b0413578a0de508a1837cd7b93e5626643d63dd06688`，Darwin arm64 本机产物自报 `aionrs 0.2.11`，SHA-256 为 `ab10d73f7a873544b689cb748051e0c8edc727b2aaba4d50b9a5f6e416885fe0`。官方 CLI 提供 JSON Stream，ACP 由 ShiJu 的独立 `aionrs-acp` 提供。AionCore 将 aionrs 作为 Rust 依赖，属于其他宿主的集成路径。 [官方发行](https://github.com/iOfficeAI/aionrs/releases/tag/v0.2.11) · [固定版协议](https://github.com/iOfficeAI/aionrs/blob/8e61a90329fa9f67c4fdf7e97fe02c24dba33f75/docs/json-stream-protocol.md) · [AionCore 依赖清单](https://github.com/iOfficeAI/AionCore/blob/main/Cargo.toml)
- 在私有 HOME、文件写入受限且只允许指定回环端口联网的沙箱中，使用 OpenAI 兼容启动配置。三个原生 CLI 探针均发送 `stream=true`、10 项工具定义和精确的 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，均取得 `stream_end` 并退出 0。批准一次生成预期的 15 字节测试文件，取消审批时文件未生成。批准私有工作目录外的测试写入时，外部沙箱拦截，工具报告错误且无文件。端点使用合成应答。
- 复用槽位 1 已构建的桥接副本，SHA-256 为 `caa297f11c2de9a784d282249e6d198e029852c488c5fa881bc72754b5205e5f`，二进制含 `acp-server::aionrs` 源路径。三次 ACP v1 `initialize` 成功，`session/new` 均返回 -32603，未到达选模和模型 HTTP。额外记录真实子进程首帧并原样转发，复现同一错误。子进程回报 `ready.version=0.2.11`、`tool_approval=true`、`current_mode=default`，而 `crates/libs/acp-server/src/aionrs/process.rs` 仅接受 `ready.version=0.2.0`。重跑需修正固定版握手契约，并核对初始化公布的会话恢复、MCP 和实际支持一致。
- 前两次自有网关在 SQLite 配置表初始化报 `database is locked`。配置目录变量之外，底层共享数据库路径还依赖 HOME。最终使用私有 HOME、SQLite 备份和私有配置目录，并由沙箱禁止写入原用户目录。网关复用 Poolside 章节记录的 shiju 0.6.0，构建基线 `5c05e7d46674ce0bb26309bc083619c7967d3db2`，SHA-256 `0b3e616b039ead3877dc087b0324452692585e2c5adc5c48981a68b5d9d3366d`。回环端口 51246，请求均携带原租户和 Rue.js 目录头。
- 私有网关 PATH 纳入两个已核验入口。安装状态前后为 true，说明入口已被发现。系统安装 POST 仍返回 HTTP 400、curl 退出 0、耗时 0.000643 秒。普通诊断 HTTP 200、退出 0、0.159164 秒，返回 `connected`。它只覆盖初始化，直接 ACP 探针仍无法创建会话。预设读回 `builtin=true`、`backend=aionrs`、`enabled=false`。模型来源 GET 为 HTTP 200、退出 0、0.000818 秒，仍为 `session_model=unsupported`、`reason=unsupported_cli`、`connected=false`。
- 原租户 `GET /v1/provider` 的目录含指定 provider 和模型。探针的启动环境缺少该 provider 声明的一项授权变量，其他凭据来源未实测。CLI 启动模型来自进程环境，宿主未为 aionrs 注册请求级驱动。现有 OpenAI 兼容入口按进程默认 provider 处理纯文本消息，无法保持原租户的模型和工具绑定。本轮未发真实推理或聊天请求，无 SSE、summary/options 或完整编辑器。
- 槽位 1 串行执行 `cargo test -j 1 --locked -p acp-server --lib aionrs -- --test-threads=1` 和 `cargo test -j 1 --locked -p vw-acp --test onboarding_aionrs acp_onboarding_aionrs_registry_contract -- --exact --test-threads=1`，桥接协议 2 项、注册与安装契约 1 项通过，均退出 0。沿用首轮 2 项预设测试证据。未运行 Clippy、桌面编译、页面构建或浏览器检查。
- 所有自有网关端口关闭，最终网关退出 0，私有配置和 HOME 副本已清理。本轮未 PATCH 预设，最终完整配置重读等于基线，对原租户只读复核未发现 aionrs 覆写。最终复验窗口前后 Rue.js 各 6319 项普通文件摘要一致，Git 短状态一致，目标目录始终不存在。其他窗口的快照数量变化发生于本轮各次取样之间，各自窗口内零差异。
- 私有证据存于 0700 目录 `/tmp/shiju-aionrs-retry-20261118.hfrQSP/`。完整重跑需确认任务 41 环境收尾，交付受管双入口安装、与锁定版本一致的桥接及宿主写入边界、原租户可用授权和请求级模型中继，再按任务 42 验证安装、预设保存恢复、会话绑定和真实文件创建。


### Junie 补丁与最终实机验收（2026-11-18）

结论：通过。Junie 用原指定租户和模型创建了完整的 Rue.js 编辑器源码及运行说明。

- 参考 Cursor 的固定发行与模型中继方式，以及 TRAE 的进程外 macOS 沙箱。系统白名单仅在 Darwin arm64 启用 Junie 26.11.2（4100.5），安装时核对官方压缩包摘要和应用签名。启动器绑定宿主工作目录、会话模型和回环中继，关闭默认配置发现及更新。沙箱限制写入范围、网络和子进程。ACP 在 prompt 前设置并确认 Safe 模式和模型，宿主审批继续生效。原生夹具证明批准后能在工作区创建文件和子目录，但写入工作区外未落盘。
- 修复了此前握手失败的两个启动差异。宿主没有把 ACP 进程当前目录设为项目，且中继地址没有尾斜杠。启动器从私有会话配置读取项目目录并规范化中继 URL。保留初次启动失败、下载中断及恢复证据。最终二进制 shiju 0.6.0 SHA-256 为 `73ef571b2b65c7a14b49bb318e0ab23dcb154a4c185eb341847cf6f68ff4733b`，以共享工作区构建，核对内嵌启动器和安装代码。
- 使用开发配置的私有 SQLite 快照和独立宿主目录启动回环网关。所有请求使用平台10001、公司100010005和Rue.js目录头。内置 `acp-junie` 确认为 builtin、backend=junie，临时启用后重读通过。普通诊断 HTTP200、结果 `handshake_failed`，因为它不携带受管会话模型配置。真正会话另行验收。最后从未安装状态调用系统安装接口，HTTP200、客户端退出0、57.609387秒，重读为已安装并核对入口及版本。
- 原请求会话 `ses_1ec80f1f2001eH7yU2gm0bPuU9` 返回 HTTP200，curl 退出0、137.962439秒，收到 `chat.done` 且无 `chat.error`。执行中 summary/options 验证 agent=acp-junie、backend=junie、full_access=true、connected=true。selected_model 为 source=session_model、provider_id=alibaba-token-plan-cn、model=`alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。会话结束释放连接，仍保留相同模型绑定。SSE 有8条完成的 edit 事件。
- 产物共8项，分别为 `index.html`、`vite.config.mjs`、`tsconfig.json`、`README.md`、`src/App.tsx`、`src/main.tsx`、`src/env.d.ts`、`src/style.css`。静态确认 Vite 别名指向本仓库 `@rue-js/rue` 源码，编辑器使用 textarea、computed 实时计数及 localStorage 保存与恢复。README 提供开发启动命令。未运行浏览器、构建或物理刷新验证。
- Rue.js 普通文件快照从6311项到6319项，差异仅 `acpdemo/junie/` 内上述8项。私有配置还原后 GET 与基线相等。确认早期尝试在共享数据库中仅遗留了与本任务记录一致的 disabled 覆写后，在事务内移除该覆写并重读确认，其他值保持一致。自有网关和原生进程退出、端口关闭，移除私有凭据及数据库副本。
- 两项安装策略测试及一项 ACP Safe 与模型控制测试通过。定向 CLI 构建通过。定向 Clippy 遇到 llm-runtime 已有101项警告，未指向本次 Junie 文件。未运行全仓测试或桌面构建。私有证据 `/tmp/shiju-junie-patch-20261118/`（0700）保存构建与安装指标、SSE、配置重读、快照及清理结果。


### Poolside 定向补全通过（2026-11-18）

Poolside 以原指定模型完成 Rue.js 编辑器落盘验收。保留前次请求和续传超时记录。本轮在前次网关停止并确认无产物后发起独立的定向补全会话，直接提供已核实的 Rue 用法、同层示例的 Vite 配置和六项文件清单，减少重复源码探索。

- 复用上节 shiju 0.6.0 构建与经摘要校验的 Poolside 1.0.16 管理安装。临时开发配置副本启动遇到 SQLite 锁，改用隔离 HOME 后就绪。一次重新下载安装失败，随后复用前轮已校验的原生二进制和管理入口，系统安装端点幂等验证返回 HTTP 200。诊断为 `connected`，能力为 `session_model=supported`，预设启用后 GET 重读确认保存。
- 使用原平台、公司和 Rue.js 目录。会话 `ses_1ec938c6c001utmldYyG1TOUb8` 返回 HTTP 200、curl 退出 0、耗时 36.128496 秒，恰好一个 `chat.done`，无 `chat.error`。运行中 summary/options 确认 `agent=acp-poolside`、`backend=poolside`、`full_access=true`、`connected=true`、`access_status.support=client_only`。选中模型来源为 `session_model`，provider 为 `alibaba-token-plan-cn`，完整模型为 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。结束后连接释放，模型绑定仍可重读。
- 六条完成的写入分别对应 `acpdemo/poolside/index.html`、`package.json`、`vite.config.ts`、`src/main.tsx`、`src/style.css`、`README.md`。静态核对确认 Vite alias 和编译入口均存在，页面从本仓库 `@rue-js/rue` 导入运行时，使用 textarea、computed 实时计数、localStorage 初始化恢复和逐次输入同步保存。README 的启动命令是 `pnpm --dir acpdemo/poolside dev`。未运行浏览器或页面构建。
- Rue.js 前后普通文件分别为 6319 和 6325 项，摘要差异恰好为上述六个文件。恢复预设后临时配置 GET 与基线完全相等，原开发配置未收到 PATCH，临时副本已删除。自有网关退出 0、端口关闭。一个本轮的已退出沙箱容器经时间和挂载核实后已移除。无代码变更，沿用上节的定向测试和 Clippy 证据。
- 本轮原始请求、SSE、配置重读、文件摘要、逐项核验和清理记录存于 0700 私有目录 `/tmp/shiju-poolside-complete.4yPggO/`，未提交运行产物或凭据。
