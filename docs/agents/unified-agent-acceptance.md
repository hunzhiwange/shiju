# 统一智能体运行链路验收

验收日期：2026-11-04。状态：已阻塞（实际窗口等待解锁）。范围为统一智能体计划任务 1–12，不涉及安装或分发全部第三方 CLI。

## 行为与证据

| 行为 | 当前验证 |
| --- | --- |
| 资料、推荐提示词、规则、启用状态、默认值落盘；内置项删除及改绑被拒绝，失败写入不改变配置 | `agent_profile_`：9 项通过（含本任务 2 项） |
| 22 个 ACP 基础智能体与原生 main、默认启用集合、委托角色资格、无 CLI 的干净环境 | `agent_catalog_`：4 项通过，其中干净环境通过子进程验证 |
| 原生技能、MCP 选择与真实工具注册、显式空集合、硬禁用、权限上限 | `agent_native_defaults_`：9 项通过 |
| ACP MCP 及规则请求、受控对端文件写入拒绝、模型与权限支持边界 | `agent_acp_defaults_`：3 项通过；真实 JSON-RPC 对端记录 `/tmp/shiju-unified-acp-6-requests.jsonl` |
| 固化身份、编辑后规则不变、租户作用域、敏感字段不落盘、权限撤销 | `agent_session_identity_`：7 项通过 |
| 统一聊天选择器、草稿、未安装提示、重启 UI 投影 | `agent_unified_selection_` 桌面库：11 项通过 |

新增测试位于 `crates/vw-agent/src/gateway/api/handlers/session/agent_profile_acceptance_tests.rs`，由 session 模块注册：

- `agent_profile_acceptance_runs_selected_profile_end_to_end`：使用真实配置存储、网关 SSE 入口、会话存储；编辑规则后再启动独立测试进程，验证从磁盘恢复的会话仍使用原规则；随后由网关选择 ACP 智能体运行受控 JSON-RPC 对端，验证规则、MCP 和固定权限真正到达运行路径，不回退到原生模型服务。
- `agent_profile_acceptance_rejects_revoked_capabilities`：先通过真实解析器固化已启用的 MCP，随后撤销该服务；同一会话的后续流请求必须拒绝，不启动不存在的命令。

原生模型由本地 HTTP OpenAI 兼容服务返回 SSE，检查实际三次请求（含新进程）中的模型和规则。模型服务、MCP 和 ACP 对端可使用受控 fixture，配置默认解析和能力决策不模拟。前序实现通过新增边界时不人为破坏生产代码。初次编译的 E0560 来自共享缓存中的旧 DTO；源码及 HEAD 均已有字段，清理本槽 `vw-api-types` 缓存后消除，不作为产品行为红测。

## 已安装 ACP 的实际启动

在 `/tmp/shiju-task12/acp` 中，以独立 `CLAUDE_CONFIG_DIR` 启动已安装的 `/opt/homebrew/bin/claude-agent-acp`。实际 `initialize` 返回协议 1、适配器 0.26.0，`session/new` 返回真实会话 ID、模型和模式。记录：`protocol.json`、`stderr.log`；适配器随后受控终止，退出 -15。未发送模型请求，不将此证据冒充模型回复或全链路运行。

## 验证执行

构建和测试使用共享槽位 2；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，`CARGO_BUILD_BUILD_DIR` 同路径。各阶段原子领取锁并在所属进程退出后释放，保留缓存。另一次仅格式化阶段动态取得槽位 4，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-4`，`CARGO_BUILD_BUILD_DIR` 同路径。

日志根目录：`/tmp/shiju-task12`。仓库外初始日志：`/tmp/shiju-task12-red.log`、`/tmp/shiju-task12-refresh.log`、`/tmp/shiju-task12-boundary.log`。

| 命令 | 退出码 / 有效数量 | 日志 |
| --- | --- | --- |
| `cargo fmt --all` | 0（后续缺失模块恢复后） | `fmt-http.log`、`fmt-recovery.log` |
| `cargo test -p vw-agent --lib agent_profile_acceptance_ -- --nocapture` 首轮 | 101 / 0，缓存 DTO E0560 | `../shiju-task12-red.log` |
| 同命令，刷新本槽依赖缓存 | 0 / 2 | `../shiju-task12-refresh.log` |
| 同命令，补跨进程及固化后撤销边界 | 0 / 2 | `../shiju-task12-boundary.log` |
| `cargo test -p vw-agent --lib agent_profile_ -- --nocapture` | 0 / 9 | `agent_profile_.log` |
| `cargo test -p vw-agent --lib agent_catalog_ -- --nocapture` | 0 / 4 | `agent_catalog_.log` |
| `cargo test -p vw-agent --lib agent_native_defaults_ -- --nocapture` | 0 / 9 | `agent_native_defaults_.log` |
| `cargo test -p vw-agent --lib agent_acp_defaults_ -- --nocapture` | 0 / 3 | `agent_acp_defaults_.log` |
| `cargo test -p vw-agent --lib agent_session_identity_ -- --nocapture` | 0 / 7 | `agent_session_identity_.log` |
| `cargo test -p vw-desktop --bin shi-ju agent_unified_selection_ -- --nocapture` 初次 | 101 / 0，未能编译 | `desktop-bin.log` |
| `cargo test -p vw-desktop --lib agent_unified_selection_ -- --nocapture` | 0 / 11 | `desktop-lib.log` |
| `cargo test -p vw-agent --lib agent_profile_acceptance_ -- --nocapture` 最终双路径 | 0 / 2（含 HTTP 三次请求、ACP 运行及固化后撤销） | `dual-peer.log` |
| `cargo clippy --all-targets --all-features -- -D warnings` 初次 | 101，其他任务新增测试 E0594 与 unused_mut | `clippy.log` |
| 同命令，最终双路径测试完成后 | 0 | `clippy-dual.log` |
| 原计划桌面 `--bin` 命令，外部编译恢复后 | 0 / 0，不作行为验收 | `desktop-bin-recovery.log` |
| `cargo build -p vw-desktop --bin shi-ju` | 0 | `desktop-build-window.log` |

## 验证中已恢复的问题

本次桌面命令在无关 `crates/vw-desktop/src/app/message/settings/general.rs:29` 报 E0004：`GeneralSettingsMessage` 新增的登录启动、关闭到托盘、托盘 tick/show 等消息未全部匹配。该文件和对应枚举是其他工作区改动，本任务不修改。

上述通用设置与通知测试后来由对应任务修复；本次桌面库的 11 项测试、普通桌面构建及一次完整 Clippy 已恢复退出 0。实际窗口证据另记，不使用前序截图或缓存二进制代替。

全仓 Clippy 的本次原始错误为 `crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs:2345` 的 E0594：`request.resume_history_only = history_only` 试图修改 `Arc<AgentRequest>`；2344 行同时出现 `unused_mut`。该新增通知测试不属于本任务。此前格式化也曾遇到其他任务声明但尚未写入的 `desktop_notification.rs`（`fmt-final.log`）；文件补齐后本次格式化已恢复退出 0。

HTTP 夹具首轮（`http-acceptance.log`）退出 101，2 项中 1 失败：测试构建默认走 scripted LLM，报 `missing scripted LLM stream`。后续使用既有 `VW_RECOVERY_WINDOW_HTTP` 测试开关，且用析构守卫在正常返回及 panic 时恢复原值；不改变任何生产配置或决策。

ACP 夹具也曾因未携带真实桌面请求的 `cwd` 而被插件作用域校验拒绝（`dual-path.log`），以及只写 `acp_saved` 未登记运行时 `acp` 映射而超时（`dual-cwd.log`，101 / 2 项中 1 失败）。核对真实桌面调用和 `lookup_acp_command` 后，夹具按真实形状登记对端并携带工作目录；新增启动前断言，确保解析出的命令和参数只能指向受控对端。最终 `dual-peer.log` 为 0 / 2 全通过；未改生产解析器或能力校验。

## 实际窗口与待回归项

`cargo run -p vw-desktop --bin shi-ju` 在槽位 2 执行两次，均退出 0；通过 Cargo runner 复制同一产物到独立 bundle 以便 CUA 定位。配置、项目及辅助网关数据均放在 `/tmp/shiju-task12/window`，没有使用用户真实配置作为验收输入。日志为 `desktop-light.log`、`desktop-dark.log`，调度及退出记录为 `../window-phases.log`。

本次有效截图均位于上述 window 目录：`light-home.png`、`light-catalog.png`、`light-profile.png`、`light-profile-scroll.png`、`light-cancel-restored.png`。实际观察到中文目录与表单渲染、滚动时标题和底部按钮固定、取消草稿后重新打开仍为原名称；应用日志未见字形整形或主线程 panic。截图中的背景透出保持原样，不把截图当作已修复其他 UI 改动的证据。

两个辅助网关分别触及其原有 180 秒时限，`gateway-light.log` 与 `gateway-dark.log` 均退出 101（1 项失败，`window driver must stop fixture`），属于验收夹具生命周期失败，不能计入产品通过证据。项目打开另出现既有索引 UNIQUE 约束错误，应用退回本地扫描。辅助脚本曾错误写入 `theme` 而非桌面配置字段 `app_theme`；已修正仓库外脚本，尚未重新取得暗色截图。

后续 CUA 操作多次返回 `The user changed ... Re-query the latest state`，按要求重新读取状态后仍再次中断。未绕过此保护。部分重新定位还可能在原窗口退出后触发 Launch Services 重启，因此仅把明确由 Cargo runner 启动且记录退出的窗口作为本次运行证据；已停止专用 bundle 的重启进程，并为仓库外 bundle 增加固定隔离目录的启动包装，避免后续重启丢失覆盖目录。所有本任务产物进程结束后释放自己的锁，未触碰其他活跃锁。

**待回归**：在可稳定独占操作的窗口环境中，重新启动有效期内的辅助网关，使用修正后的 `app_theme` 与项目 fixture，补齐暗色中文表单、未安装项发送前提示与草稿保留、切换引擎和发送失败/取消的实际截图；确认固定资料编辑在真正重启后恢复。核心双路径、重启和能力撤销自动验收已通过，缺少上述窗口证据时不删除编号任务文件、不声明完整窗口验收完成。

## 2026-11-05 继续验收

修复 `/agent` 忽略请求租户、读取默认租户目录的问题：处理器显式进入请求 `TenantScope`，保留原有敏感字段清理。新增真实 Axum 路由测试，在两个非默认租户及默认租户哨兵之间验证隔离。修复前真实 HTTP 目录返回“主代理”而非请求租户哨兵（`catalog-http-red.log`）；修复后独立网关的 `/agent?include_disabled=true` 返回“识矩验收助手”（`regression-window/catalog-before-ui.json`），配置脚本断言通过。

与已合入的租户全局智能体定义契约对齐，验收夹具改写全局定义，项目配置 API 测试明确拒绝项目级定义且原配置不变。手动窗口 fixture 生命周期由 180 秒调整为 900 秒，仍须停止文件正常退出。

本轮动态领取槽位 4，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-4`。以下命令全部退出 0：

| 命令 | 数量 / 日志（`/tmp/shiju-task12`） |
| --- | --- |
| `cargo fmt --all` | `global-fmt.log` |
| `cargo test -p vw-agent --lib agent_profile_ -- --nocapture` | 10 / `global-agent_profile_.log` |
| `cargo test -p vw-agent --lib agent_native_defaults_ -- --nocapture` | 9 / `global-agent_native_defaults_.log` |
| `cargo test -p vw-agent --lib agent_session_identity_ -- --nocapture` | 7 / `global-agent_session_identity_.log` |
| `cargo test -p vw-desktop --lib agent_unified_selection_ -- --nocapture` | 11 / `regression-selector-latest.log` |
| `cargo clippy --all-targets --all-features -- -D warnings` | `global-clippy.log` |
| `cargo build -p vw-desktop --bin shi-ju` | `global-desktop-build.log` |

实际窗口启动使用全新 `/tmp/shiju-task12/regression-window` 隔离配置、项目及数据库，先前截图保留在 `regression-window-initial`。HTTP 配置和目录断言通过后，CUA 返回“Mac is locked and automatic unlock could not unlock it”。因此本轮没有新的实际窗口截图，不能声明暗色表单、发送前提示、引擎切换、取消及真正窗口重启持久化通过。已终止本次专用桌面进程（退出 143，主动清理，不计为通过）；辅助网关经停止文件正常结束，槽位在所有所属进程退出后释放。

**恢复条件**：用户手动解锁 Mac 并保持桌面可用；重新动态领取空闲槽位，启动隔离网关与专用 bundle，完成上述窗口证据后再关闭任务。不得调整用户锁屏设置或使用旧截图冒充本轮结果。
