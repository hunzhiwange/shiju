# Claude Code 模型来源证据

范围：macOS arm64，安装的 `@agentclientprotocol/claude-agent-acp` 0.26.0、`@anthropic-ai/claude-agent-sdk` 0.2.96、内嵌 Claude Code 2.1.96；不推断 Linux/Windows 支持。

## 配置契约

- `CLAUDE_CONFIG_DIR` 隔离用户设置、运行态和配置路径；SDK 的 Keychain 服务名也根据该目录摘要分域，不替换 HOME。
- 适配器 `dist/acp-agent.js::createSession` 接收 `_meta.claudeCode.options`，其中 SDK `settings` 属性为原生 flag settings 层，优先于项目/local 设置。不向 ACP 适配器追加原生 `--settings`。
- session_model 在私有 0700 目录生成 0600 settings.json；主模型、Sonnet/Opus/Haiku 别名、快模型及子 Agent 均绑定所选模型，认证只使用短期 relay token。SDK settings 文件只包含模型、环境与禁用原生 apiKeyHelper，不改变权限或工具策略。
- 托管 Claude 子进程清除继承环境，仅保留必要系统变量及显式启动环境，不合并账号凭据。OAuth、原生 API key、云后端、替代执行器和自定义认证头被显式清空/关闭；显式竞争配置拒绝启动。企业模型/认证/代理覆盖也明确拒绝，不绕过企业策略。
- cli_local 不带运行时 reference，不注入 SDK settings，沿用原始命令/参数/环境及账号合并行为。验证使用临时目录中的原生设置，不读取真实账号。

依据：已安装固定版本的 `dist/acp-agent.js`、`dist/settings.js`、SDK `sdk.d.ts::Options.settings` 和 `cli.js`。版本探针从适配器实际 Node 模块解析 SDK，并执行该 SDK 的 `cli.js --version`；不是包装器回显版本。

## 补充探测（不替代生产验收）

真实 ACP 适配器 initialize/new/prompt 已分别验证原生配置、SDK flag settings 加项目错误模型/认证/Bedrock 竞争两种情形。两次探测均收到 3 次认证通过的 `/v1/messages` 请求，模型为 `acp-fixture-primary`，tool_results 依次为 0、1、2，fixture 错误为 0。工具结果来自真实 CLI 文件读取，两份临时文件包含同一个挑战值，避免 Claude 的同文件读取缓存。

补充真实辅助模型探测：实际 Agent 工具启动 Haiku 辅助模型请求，fixture 观察到父/子 Agent 共 5 次认证 Messages 请求，子 Agent 的模型仍为 `acp-fixture-primary`，实际读取结果 0→1→2，错误为 0。生产验收同样要求独立观察到辅助请求。

Python fixture 合约 19 个用例通过，覆盖 Messages SSE 工具调用、两轮真实 HTTP 工具结果、认证拒绝、count_tokens 不计生成、原有 Chat fixture 和脚本安全约束。

原生文件入口另已单独验证：仅向进程传入 CLAUDE_CONFIG_DIR，模型/认证全部来自该目录 settings.json，父/子 Agent 共 5 次 Messages 请求、两次文件读取、错误 0；未通过进程环境预先注入模型或认证。

ACP 客户端聚焦验证：环境/元数据 2 个用例通过；独立的真实 JSON-RPC new/load/resume 边界用例 1 个通过。均位于槽位 10，退出码 0，不将协议 fixture 当作供应商 CLI 验收。

## 生产验收

双来源完整生产生命周期已通过；以下区分生产结果与补充探测。

## 已完成的生产与回归证据（2026-11-04）

- 真实 `session-model`：退出 0，有效 Rust 验收 1；11 次认证 `/v1/messages`，包括真实 Haiku helper（仍绑定所选 primary）、两次文件工具结果及切换后的 switched；fixture 错误 0。new、second_turn、cancel、restart、switch、isolated、handshake 全部 true。
- 真实 `cli-local`：退出 0，有效 Rust 验收 1；9 次认证 `/v1/messages`，模型全为原生配置 primary，真实 helper/文件工具与上述生命周期全部通过，fixture 错误 0。未注入托管 SDK settings。
- 请求级私有 settings 与长期 SDK 历史目录分开：历史按租户/项目/会话/Provider/模型摘要分域，目录 0700、校验属主并拒绝符号链接；凭据文件 0600，profile 销毁仍撤销 bearer 并移除文件。历史目录不承担凭据恢复。`VIBEWINDOW_CONFIG_DIR` 可将实际验收完全隔离到临时根目录。
- 修复 SDK 实际使用的精确 `/v1/messages?beta=true`；其它、重复或附加查询仍拒绝。修复原生环境持久化时 uppercase key 被误拒绝：只将 `agent_config.env` 当作环境变量字典，普通记录字段仍执行 snake_case 检查。
- 模型 transport/context/output/capability 从每次解析的当前 catalog 获取。缺失凭据/模型禁用不回退本地账号。Chat 上游要求工具与 temperature 能力且非 reasoning；原生 Messages 才保留 reasoning。Responses、Gemini 等未经转换的协议明确拒绝。
- 缓存提示只接受 ephemeral、可选 ttl=5m/1h、可选 scope=global，再在 Chat 转换中移除；未知字段/ttl/scope 仍拒绝。metadata.user_id 为原生分析标识，不转发给 Chat Provider；思考签名、未知上下文状态不伪造。

有效回归：Claude 10/10、Messages relay 4/4、ACP 客户端与环境字典 4/4、Python fixture 24/24，均退出 0。Python 24 包括其它串行任务新增的 Gemini 合同，不作为 Claude 供应商运行证据。

确切命令：

```sh
cargo fmt --all
cargo test -p vw-agent --lib --all-features acp_model_source_claude -- --nocapture
cargo test -p vw-agent --lib --all-features acp_model_source_relay_messages -- --nocapture
cargo test -p vw-acp --lib acp_model_source_claude -- --nocapture
python3 scripts/acp_model_source_fixture_tests.py
python3 scripts/verify_acp_model_source.py --agent claude --mode session-model --manifest docs/vibewindow/acp-model-source/cli-versions.json --output target/acp-model-source-evidence/claude/session-model --offline-fixture
python3 scripts/verify_acp_model_source.py --agent claude --mode cli-local --manifest docs/vibewindow/acp-model-source/cli-versions.json --output target/acp-model-source-evidence/claude/cli-local --offline-fixture
```

最终 Messages session-model 验收动态领取槽位 2（`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`）；cli-local、最终 Claude 10/10、relay 4/4、ACP 4/4 动态领取槽位 1（`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`）。历史绿灯 10/10 另在槽位 3（`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`）。所有 Cargo 与实际 CLI 子进程继承槽位，独占目录由原子 mkdir 获得，owner 包含 PID；持有进程待全部子进程停止后 finally 释放自身锁，保留构建缓存。

红灯记录：凭据 argv 用例曾实际进入并失败（错误接受含凭据的参数）；生产 Messages 初次因 beta 查询拒绝而失败，修复后首轮通过、再因历史目录被请求清理导致续聊失败；原生配置生产验收又揭示环境字典持久化错误。均按真实行为修复并重跑，未用编译成功/零测试冒充通过。早期 Rust 大测试二进制在失败断言时发生 unwind abort，报告保留退出 101，不伪称正常失败汇总。初轮仅版本调查和脚本 unavailable 的结果不计真实请求验收。

Clippy：`cargo clippy --all-targets --all-features -- -D warnings` 退出 0，动态槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。这是 lint/build 证据，不计行为测试数量。

## Chat 上游与最终状态

真实 `session-model` 的 Messages→Chat 路径同样退出 0、有效 Rust 验收 1；上游观察到 11 次认证 `/v1/chat/completions`，包含实际 helper 与切模，两次文件读取及全部生命周期 true，fixture 错误 0。使用公共 manifest 的临时副本，仅将 Claude session-model 配方 `upstream_protocol` 改为 `chat`；实际生产转换器与已安装供应商 CLI 均未被替代。

```sh
python3 scripts/verify_acp_model_source.py --agent claude --mode session-model --manifest target/claude-chat-manifest.json --output target/acp-model-source-evidence/claude/chat-upstream --offline-fixture
```

动态槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Chat 续聊曾实际失败：fixture 复用响应 ID 导致真实 SDK 合并历史；修复为每个生成响应唯一 ID、同流各 chunk 共用 ID，独立 HTTP 行为测试通过，保持转换器乱序历史拒绝逻辑。

可审查的脱敏原始报告：[Messages session-model](claude/session-model.json)、[原生 cli-local](claude/cli-local.json)、[Chat session-model](claude/chat-upstream.json)。报告仅包含版本、闭合诊断类别、请求路由/模型/认证布尔值/工具计数/生命周期，不包含令牌、挑战值、原始请求或供应商日志。

最终状态：本机 Darwin-arm64 通过；Windows 的私有目录 ACL 未验收，保持显式拒绝；Linux 仅有 Unix 代码路径，不宣称真实 CLI 通过。建议任务 33 按既定范围集成能力发布并统一回归，其它驱动不由本任务开启。
