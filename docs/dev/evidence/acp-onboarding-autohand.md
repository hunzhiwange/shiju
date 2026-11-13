# Autohand Code 接入证据

验收日期：2026-11-13。交付 `autohand` ACP 注册、固定版本安装与本地发现，以及由生产目录派生的 `acp-autohand` 内置预设。用户最终明确要求“不用实机，直接完成”，因此免除桌面实机验收；不宣称完成暗亮主题或桌面交互观察。

## 官方来源及启动契约

- [旧适配器归档说明](https://github.com/autohandai/autohand-acp/blob/main/README.md)：旧桥接包 0.2.1 已归档，官方推荐 >=0.9.8 的原生 `autohand --acp`，保留 autohand ID。
- [原生 v0.9.11](https://github.com/autohandai/code-cli/releases/tag/v0.9.11)，commit `46a28b16c05b1984761ca2b0daa700b6fc21ef70`。
- 官方 npm 元数据：`npm view autohand-cli version engines --registry=https://registry.npmjs.org --json` 退出 0，版本 0.9.11，Node >=22，提供 autohand bin。注册与安装固定 `autohand-cli@0.9.11`。
- 优先发现本地 autohand 并使用 --acp；无本地 CLI 时执行 `npx -y autohand-cli@0.9.11 --acp`。没有加入别名或第二份桌面目录。
- 默认 npm postinstall 的可选 ahtraces 下载超时（退出 1），官方 registry 同样复现。固定版本官方脚本提供 `AUTOHAND_SKIP_AHTRACES_INSTALL=1` 和 `AUTOHAND_SKIP_COMPUTER_CONTROL_INSTALL=1`。注册与安装固定跳过这两个 ACP 不需要的附加应用，不关闭通用 npm scripts 或权限检查。设置这两个变量后，`npm exec --yes --foreground-scripts --registry=https://registry.npmjs.org --package=autohand-cli@0.9.11 -- autohand --version` 退出 0，报告 0.9.11 (46a28b16)。
- macOS arm64 官方原生 tar.gz SHA256 与官方同名校验文件一致：`d98de36979a1a4b6442a5b0ff746a2a5f5a5408e493c93c380aa4b3ef312fe24`，--version 退出 0。Unix/Windows 安装映射均通过数据测试；其他 OS/架构未冒充实际运行验收。

## 真实 CLI

在临时 HOME/AUTOHAND_HOME、独立 workspace 及受控环回 OpenAI 响应服务中运行官方固定发行物。未读取用户凭据或借用账号，配置只包含本地测试令牌。

脱敏协议形状：

```json
{"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{}}}
{"agentInfo":{"name":"autohand-cli","version":"0.9.11"}}
{"method":"session/new","params":{"cwd":"<temporary-workspace>","mcpServers":[]}}
{"method":"session/prompt","params":{"sessionId":"<same-session>","prompt":[{"type":"text","text":"<omitted>"}]}}
{"id":3,"result":{"stopReason":"end_turn"}}
{"id":4,"result":{"stopReason":"end_turn"}}
```

结果：

- 无配置时真实首轮报 auth_failed / No cookie auth credentials found，ACP error=-32603。前期据此判断必须外部账号，后续实测已纠正：独立 openai provider、本地测试令牌、环回 baseUrl 可以工作，不需要额外账号。
- 同一会话两轮 end_turn，实际 `/v1/chat/completions` 请求始终使用 fixture-model。随后 session/load 成功；取消活动轮次返回 cancelled，再次 prompt 返回 end_turn，恢复后的模型未改变。
- 从实际模型请求发现 tool_search / run_tool_script，通过真实 tool_search 获得 write_file 的 path/contents schema。受控模型发起临时文件写入，CLI 实际请求 session/request_permission。allow_once 后写入内容准确；reject_once 和 cancelled 均返回 Tool execution skipped by user，拒绝目标文件不存在。
- 固定 npm 0.9.11 的 session/set_config_option(configId=model,value=不存在的模型) 返回 -32602，服务收到 0 次请求，无默认模型回退。旧式 session/set_model 与 _unstable/session/set_model 返回 -32601，不能仅凭模型列表推断旧式方法可用。
- 仅传入 selected MCP stdio 服务，实际捕获 initialize、notifications/initialized、tools/list，后续两轮正常完成。HTTP/SSE 只有握手声明，未作为实测结论。
- 上述探测脚本退出 0，表示成功记录协议结果；CLI 探测结束主动终止，不将 SIGTERM 当正常退出验收。临时进程已清理。

## 内置预设及能力边界

生产 merged_agent_catalog 自动派生 builtin=true、runtime=Acp/backend=autohand、名称 Autohand Code、说明与推荐提示词。默认停用、禁止子代理，模型/权限/技能/MCP 默认项沿用既有继承结构。

生产配置与网关流测试实际验证保存重读、启用配置、正确 ACP 进程路由、会话绑定、选中 MCP 与固定拒绝权限，以及停用后在执行前拒绝。只替代外部 stdio 对端，未 mock 目录、配置存储或绑定实现。

CLI 本地模型已验证。ShiJu 会话模型中继与技能目录隔离尚无完整验收证据，保留现有明确拒绝门控，不宣称上游不支持，也不发布未经验证的分派。原生 default/plan/yolo/automode 名称不等于已审核完全访问，保持既有宿主权限边界。

桌面构建通过；尝试实机期间窗口多次外部变化，未完成官方目录操作。用户随后明确免除实机。读回检查确认本次测试配置仅在临时数据库，用户现有数据库没有本次临时配置。

## Rust 验证

全部 Cargo 命令通过 scripts/cargo_slot.py 原子领取共享槽位，设置绝对 CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR。最终阶段保持一个前台 shell，槽位 1，两个变量均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。

|命令|退出码|有效结果|
|---|---:|---|
|cargo fmt --all|0|格式化完成，差异按本任务范围审查|
|cargo test -p vw-acp --test onboarding_autohand -- --test-threads=1|0|3 通过：注册、两轮及取消后续聊、宿主拒绝写入|
|cargo test -p vw-config-types --lib acp_onboarding_autohand -- --test-threads=1|0|1 通过：Unix/Windows 固定安装映射|
|cargo test -p vw-agent --lib acp_onboarding_autohand -- --test-threads=1|0|3 通过：内置元数据、配置绑定、生产网关持久化与执行/停用拒绝|
|VIBEWINDOW_CONFIG_DIR=临时目录 cargo test -p vw-acp --lib agent_registry_tests -- --test-threads=1|0|24 通过，保留 Codex/Claude 等既有注册|
|cargo test -p vw-agent --lib agent::catalog_tests -- --test-threads=1|0|6 通过，包括无 CLI 时仍列出 23 个 ACP 后端|
|cargo build -p vw-cli --bin shiju|0|CLI 构建通过|
|cargo build -p vw-desktop --bin shi-ju|0|桌面构建通过|

`cargo clippy --all-targets --all-features -- -D warnings` 最终退出 0（2m 26s），有效完成整仓规定检查。未运行全量单元测试、fmt check、wasm check 或 Makefile。

前置失败：槽位 5 的规定三个聚焦命令均退出 101，ACP 1、配置 1、agent 3 个失败用例。注册/安装/目录缺失符合预期；网关首轮另有测试传输开关遗漏，已修正，不将该设置错误作为业务失败证明。第一轮通过验证还修正了测试对端 DenyAll 和取消步骤；最终 ACP 3/3 通过。

历史资源故障：槽位 3 的 vw-agent 代码生成曾因 No space left on device (os error 28) 触发 rustc panic，后续注册/目录验证亦未运行有效测试。磁盘恢复后，以上有效通过结果取代该阻塞。槽位 1 早期完整 clippy 曾退出 0；最终收尾重新运行规定 clippy。

历史绝对 CARGO_TARGET_DIR：`/Users/xiangmin/worker/code/shiju/target/rust-slot-5`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。各阶段结束释放自己的锁，保留共享缓存。
