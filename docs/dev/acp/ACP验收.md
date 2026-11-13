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
