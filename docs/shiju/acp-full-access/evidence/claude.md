# Claude 完全访问：固定制品验收

日期：2026-11-06（Asia/Shanghai）。平台：macOS arm64。依赖批次 5 提交 `07c99fa92` 已满足；共享策略、协议确认、启动授权与恢复屏障均已存在。

## 身份和源码（E2）

隔离 npm 安装 `@agentclientprotocol/claude-agent-acp@0.26.0`，实际解析 SDK `@anthropic-ai/claude-agent-sdk@0.2.96`，该 SDK 的 `cli.js --version` 输出 `2.1.96 (Claude Code)`，Node 26.13.0。未升级、覆盖用户安装，未读取或打印真实凭据。

官方来源：[固定源码](https://github.com/agentclientprotocol/claude-agent-acp/blob/v0.26.0/src/acp-agent.ts)、[npm 适配器](https://registry.npmjs.org/@agentclientprotocol%2fclaude-agent-acp/0.26.0)、[SDK](https://registry.npmjs.org/@anthropic-ai%2fclaude-agent-sdk/0.2.96)。源码与实测分别记账：

- `createSession` 将 SDK 的 `permissionMode` 设为设置解析值，覆盖 `_meta.claudeCode.options.permissionMode`；同样管理 `allowDangerouslySkipPermissions`。不能靠 meta 注入 bypass。
- 公布的精确配置是 `configId=mode`，值为 `default / acceptEdits / plan / dontAsk / auto / bypassPermissions`。`setSessionConfigOption` 调用 `query.setPermissionMode`，成功后响应完整 `configOptions`，同时发送 `current_mode_update`。`dontAsk` 是未预授权即拒绝，不能冒充完全访问。
- `ExitPlanMode` 有独立 `requestPermission` 交互，先于普通 bypass 分支；它是流程确认。原生工具免审批不保证免除计划、问答或企业限制。
- 非 root 才公布 bypass；root 的限制不通过伪造 `IS_SANDBOX` 绕过。设置/策略拒绝由 SDK 保留。

实际文件 SHA256（生产匹配还包含两份 package.json、lib/settings/tools/utils，完整列表在 `access_policy/claude.rs`）：

| 文件 | SHA256 |
| --- | --- |
| 适配器 dist/index.js | `2859ccaa5ba91ed27b80614eae47399370015acbcbc7d01d1408c71f7964565d` |
| 适配器 dist/acp-agent.js | `00e66a056cc39ba087dffe63da3b62d27fcff6694e6eee5fe3b25961e6218ddd` |
| SDK sdk.mjs | `d4b7aa175a37db7835d354fbdd8a6a0c03ad6a11ab32842d5362531b20ccd35b` |
| SDK cli.js | `d4d06b148b05d3d21ad250a9900b7c8c47780088712299acd9892a8777f76ab4` |

## 真实进程和工具结果（E3）

使用隔离 HOME、CLAUDE_CONFIG_DIR、临时项目和 loopback Anthropic Messages SSE 上游；上游只发出固定 Bash 或 Write 工具调用与结束响应，工具仍由真实 SDK/CLI 执行。临时密钥只用于本地 fixture，不调用供应商模型。适配器和 SDK 未打补丁。没有修改全局设置。

| 原始 ACP JSON-RPC 路径 | default | bypassPermissions | default（撤销） |
| --- | --- | --- | --- |
| Bash 写临时文件，客户端取消审批 | 文件不存在；审批 1 | 写入成功；审批 0 | 文件不存在；审批 1 |
| Write 写临时文件，客户端取消审批 | 文件不存在；审批 1 | 写入成功；审批 0 | 文件不存在；审批 1 |
| 项目 permissions.deny=[Bash] | 拒绝；审批 0 | 仍拒绝；审批 0 | 拒绝；审批 0 |

真实 initialize 返回协议 1、适配器名称/版本、load/resume、image、embeddedContext、MCP HTTP/SSE。new 返回上述精确模式和 `configOptions`；即使 meta 请求 bypass，新建回读仍为 default。每次 set_config_option 响应确认当前值；直接 load/resume 均保留同一 sessionId，回读 default。模型始终为 `claude-sonnet-4-6`。

上述探针未公布客户端 fs/terminal 能力，普通工具由 CLI 自己执行，`fs/read_text_file`、`fs/write_text_file` 和 `terminal/create` 回调均为 0。因此这些结果只证明原生工具审批控制，不能宣称已验收客户端代执行或原生 OS 沙箱解除。项目 deny 不是企业托管 deny 实机证据；企业、网络、Linux/Windows、root、Bun 单文件制品均未验收。

仓库真实 `AcpClient` 另完成两种模型来源的跨进程验收（隔离 npm 入口；session_model 使用现有驱动同形的私有 SDK settings/launch reference，不重复验证 gateway 模型绑定）：

| 路径 | 新建 false | 同 ID load true | 同 ID resume false |
| --- | --- | --- | --- |
| cli_local | default；审批 1；Unavailable | bypassPermissions；审批 0；Native | default；审批 1；Unavailable |
| session_model 启动材料 | default；审批 1；Unavailable | bypassPermissions；审批 0；Native | default；审批 1；Unavailable |

两条路径各有 3 次真实 Bash 工具结果，均成功写入临时文件：关闭时由现有 ApproveAll 客户端应答审批，开启时原生免审批。没有替换逻辑会话、模型或用户提示词来实现权限切换。new/load/resume 均在首次 prompt 前确认当前配置。独立 ExitPlanMode 上游尝试未取得有效权限回调，不计入 E3；其确认语义为 E2 源码证据及确定性客户端回归。

## 生产接入与限制

- `access_policy/claude.rs` 核对真实 npm 入口、六个 JS 文件、包元数据和最近解析的 SDK/CLI 哈希；`client/protocol.rs` 还核对 macOS arm64、agentInfo 和执行器/Node 注入环境。仅该入口可报告 Native；npx、未知包装器、未知版本/平台保留 ClientOnly。自定义执行器和 Bun 单文件入口不能复用本证据。
- `sync_claude_access` 复用真实配置请求/响应与 prompt 屏障，在 new/load/resume 和每次 prompt 前应用当前授权：`full_access && ApproveAll` 才能设置 bypass；关闭或 ApproveReads 回到 default。配置缺失、拒绝、超时或错误回读时禁止继续，不回退 legacy mode。未知运行时恢复后若仍处于 bypass 而当前授权不足，明确拒绝继续，不猜测撤销控制。
- Native 还要求当前会话的成功确认与相同配置快照；仅公布值或伪造 UI 快照不产生 Native。退出、重新握手清除旧确认。legacy mode 和显式 config 同样不得绕过当前授权；DenyAll 在首次会话/prompt 前拒绝原生执行。
- 活动 turn 切换请求取消并返回明确错误，等待完成后重试，不承诺配置响应能撤销已开始的工具。逻辑 sessionId 保持不变。
- 当前 plan 在关闭权限时保留；开启完全访问不会隐式退出计划，而是报错并要求用户先显式离开计划。共享事件客户端没有结构化计划确认界面，因此 Claude switch_mode 回调明确返回需要独立交互的错误，不自动选择可提权的 bypass 选项。未把这类确认伪装成普通工具免审批。
- 关闭原生 bypass 恢复 default 审批语义，并不关闭既有客户端 ApproveAll 自动应答。生产验收以回调数量区分这两层；不会声称关闭后所有写入均应失败。

必要相邻变更限于客户端握手、状态、生命周期、会话/提示词屏障和事件回调；仅增加后端映射不能触达生产权限路径。没有修改安装、模型来源实现、认证、品牌或全局权限。

## 验证记录

所有 Cargo 阶段原子领取共享槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，并以相同路径设置 `CARGO_BUILD_BUILD_DIR`（仓库 build-dir 配置）。每阶段先运行 `cargo fmt --all`；最终行为复核前退出 0，只保留本任务差异。最后定向 Clippy 前的重复 fmt 因无关 Git 模块暂缺测试文件退出 1，见下文。测试使用默认 features；Clippy 使用全部 features。

| 确切命令 / 阶段 | 退出码 | 有效测试或结果 |
| --- | --- | --- |
| 红测：`cargo test -p vw-acp --lib acp_full_access_claude -- --nocapture` | 101 | 2 条行为断言失败：无当前授权 bypass、DenyAll 未拦截执行 |
| 计划红测：`cargo test -p vw-acp --lib acp_full_access_claude_plan -- --nocapture` | 101 | 2 条行为断言失败：隐式退出计划、计划确认自动批准 |
| 最终：`cargo test -p vw-acp --lib acp_full_access_claude -- --nocapture` | 0 | 14 条通过 |
| 相邻：`cargo test -p vw-acp --lib client::session_control_tests -- --nocapture` | 0 | 43 条通过，包含上述 14 条 |
| 相邻：`cargo test -p vw-acp --lib client::event_client_tests -- --nocapture` | 0 | 24 条通过 |
| 真实客户端：`cargo run -p vw-acp --example claude_full_access_probe --` | 0 | cli_local 的 new/load/resume，3 个真实工具结果 |
| 真实客户端：`cargo run -p vw-acp --example claude_full_access_probe -- model-source` | 0 | session_model 启动材料的 new/load/resume，3 个真实工具结果 |
| 首轮：`cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全仓无警告；SDK 查找顺序最终审查修正前 |
| 最终：`cargo clippy --all-targets --all-features -- -D warnings` | 101 | 并发无关变更导致失败，见下文 |
| 最终定向：`cargo clippy -p vw-acp --all-targets --all-features -- -D warnings` | 0 | 本 crate 无警告 |

最终全仓失败位于未修改的 `vw-agent-apps/src/apps/lifecycle/local_agy.rs:80`（SHA 输出不实现 LowerHex），以及 desktop Git 面板的私有函数重导出/访问、未使用导入和 `GitHistoryFileDto.status` 测试字段缺失。此前首轮全仓 Clippy 通过；最终复核期间这些文件由其他任务变更。遵守 AGENTS.md，不处理无关错误，不把首轮成功冒充当前全仓通过。本任务状态为 **已完成，待回归**，保留任务 6 文件供全仓修复后重跑。

最后的入口审查将 SDK 查找起点改为 `dist`，符合 Node 的最近模块解析顺序；随后完整重跑 14 条测试和两条真实客户端路径均通过。定向 Clippy 退出 0（`focused-clippy.log`）。该阶段前重复 fmt 退出 1，原因是其他任务的 `crates/vw-agent/src/gateway/api/handlers/git/repository.rs` 声明了尚未出现的 `repository_tests.rs`。此前最终行为复核阶段 fmt 已退出 0，此后无 Rust 修改；不修复其他任务的中间文件状态。

真实协议 harness 位于 `/tmp/shiju-full-access-claude6/`：`probe.py`、`write-probe.py`、`deny-probe.py` 分别执行原始 JSON-RPC，均退出 0，各有 3 个有效工具结果；对应 `probe.log`、`write-probe.log`、`deny-probe.log` 及随机私有目录下的 `trace.json`、`upstream.json` 保存本地证据。模型均为固定 `claude-sonnet-4-6`。临时 Rust example 在验收后删除，源码副本为 `claude_full_access_probe.rs`；`final-replay.log` 是最终 14 测试与双路径实测日志，`final-checks.log` 保存相邻回归和首轮 Clippy。外部进程均在各自脚本结束时收敛，不将无效 ExitPlanMode 探测计为通过。

未修改只读 plan.md、安装、模型来源实现、认证、品牌或其他任务源码。槽位锁在各阶段结束后释放，保留缓存。下一步修复其他任务的全仓编译问题后，按任务 6 文件回归 Clippy；通过后再删除任务文件。
