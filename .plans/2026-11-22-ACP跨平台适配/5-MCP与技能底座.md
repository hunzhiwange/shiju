# 任务 5 适配 MCP 和技能的会话生命周期

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 1】 依赖：无

状态：待回归

来源任务：无

目的：复用现有挂载设施，提供原生与外挂共存、隔离和撤除。

## 文件与范围

修改 `crates/libs/agent-app-services/src/mcp/acp_mount.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/mod.rs` 的共用视图部分及相邻独立测试。逐适配器分派由任务 48 修改。

## 上下文

ACP 按会话提供服务与技能目录。视图及撤权应沿用现有生命周期。

## 实施

1. 核对 stdio、HTTP/SSE 各适配器已用传输的路径及权限。复用当前会话名单和撤权逻辑。保持原生目录，按源处理同名冲突和显式拒绝。
2. 核对 skill 视图在 Linux 的目录创建、中文路径、同名冲突、非法链接和清理。无法证明隔离的挂载失败关闭。
3. 测试先读原生技能和外挂技能的独立标记，验证新建、恢复、支持时的分叉及撤除。MCP 验证独立执行回执及结果续轮，撤除后零调用。

## 验证

- 先写独立回归测试，使用 `acp_platform_mount_lifecycle` 作为测试名或前缀。旧行为预期失败点为恢复沿用了旧挂载或外挂遮蔽原生条目。
- 后续在 Linux Docker 容器内使用同一红绿命令 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_mount_lifecycle`。仅列为后续命令，本轮不执行。绿灯要求命中本任务测试且零失败。
- 本轮静态逐项检查测试断言、调用者接线及本任务差异。记录受影响目标和待运行项。

MCP 目录及撤销专属红绿测试另用 `python3 scripts/cargo_slot.py cargo test -j 1 -p agent-app-services --lib acp_platform_mcp_revoke`。测试撤权时并发定位旧服务的行为，绿灯须命中用例且零失败。

## 静态交付记录

- MCP 沿用目录范围与固定版本，发布传输候选和首次执行前重新校验。stdio 回归在真实 stdio 外部进程初始化后撤销目录版本，验证撤销后旧服务零工具调用、原生 MCP 还能续轮以及恢复过滤旧版本。HTTP/SSE 用同名的不同目录来源验证冲突和撤销隔离，暂未跑传输端到端测试。项目原生运行时选择的撤销仍独立于 ACP 会话目录版本。
- 技能视图记录私有目录的文件系统身份。恢复入口重验实际缓存客户端持有的视图，相同指纹下也会退役失效实例。测试源码检查中文路径、原生与外挂标记、重建及分叉视图身份、入站同名冲突、非法链接和租约释放。适配器恢复及分叉协议行为、原生扫描器优先级、模型消费 MCP 回执的续轮仍需逐适配器实测。
- 任务 48 沿用现有技能分派接口和平台门控。仅在对应适配器证据满足时启用 Linux，未证实隔离的分派保持拒绝。
- 静态检查覆盖本任务差异、Rust 源码解析和测试入口引用。受影响测试目标为 `llm-runtime --lib` 和 `agent-app-services --lib`，覆盖依赖 `agent-skills` 和 `plugin-mcp`。上述两条 Linux Docker 红绿命令均未执行，暂无退出码或命中数。

## 最终静态复核

- 复用任务 49 的 `df77249c6`。入站更新和恢复仍经 `resolve_mounts` 与会话连接池。出站请求在逻辑会话锁下、选择缓存客户端前复核实际持有的技能视图。后续接线未改变共用视图入口。任务 48 的 Linux Pi 技能拒绝保留。
- `992af074c7` 已把移入启动任务的 MCP 配置副本与启动后复核借用分开。stdio 并发用例在真实子进程初始化后推进目录版本并释放启动等待，断言旧服务收不到 `tools/call`。HTTP/SSE 用例仅覆盖来源、冲突和恢复过滤。模型续轮和原生、外挂标记的端到端断言沿任务 49 的 Linux 宿主入口。
- 两个定向测试入口静态可见，分别有 3 个技能和 2 个 MCP 用例。沿用上方精确命令，在非 root GNU Linux 容器内提供 Cargo 槽位、SQLite 构建依赖及 stdio 用例所需的 `python3`。本轮未执行编译、测试、安装或容器，继续待回归。

## GNU 固定 stdio 目录接线

- 登记使用现有 `PUT /agent-apps/mcp/{id}` 更新入口。请求携带 `targetHostId: "desktop-local"`、`projectId`、`expectedRevision` 和 `linuxStdioSelection: {"action":"select","sha256":"<独立审定的 64 位小写十六进制摘要>","allowedTools":["<原始工具名>"]}`。同一请求不能编辑定义。`{"action":"disable"}` 撤销选择。宿主通过配置的私有 `SHIJU_AGENT_APPS_ACCESS_FILE` 校验 `X-Agent-Apps-Token`，同一授予项须包含 `provider_config_write` 和匹配的 `mcp_catalog_write_projects`，以及本机、平台和公司身份。
- pin 只授予明确选择的项目目录修订，规范 ELF 路径和精确 argv 来自该修订；GNU x86_64/aarch64 验证 ELF 头及可执行位，启动仍须用审定摘要复核实际 ELF。旧数据没有 pin 时保持原有行为。受审选择不会进入原生未隔离的运行时扫描，且禁用或再次选择会使旧投影失效。
- 任务 48 使用 `crate::agent_apps::mcp::acp_mount::reviewed_project_stdio(launch.clone(), selected)` 获取项目来源的 pin 和失败关闭的续用检查。由宿主 RelayBinding 确定项目及用户。接入前校验续用检查，发现完成后、审批签发及每次 dispatch 继续复核。供应商 `_meta` 不参与授权。会话外挂来源使用下方共用入口。iFlow/GLM 的逐适配器接线归任务 48。
- 新增两条待授权命令 `python3 scripts/cargo_slot.py cargo test -j 1 -p agent-app-services --lib acp_platform_mcp_review` 和 `python3 scripts/cargo_slot.py cargo test -j 1 -p gateway-mcp --lib acp_platform_mcp_review`。各应命中一个用例，容器须提供 `/usr/bin/true` 与 SQLite 构建依赖。本轮只做源码解析和差异检查。

## GNU 会话外挂接线

- 复用同一宿主项目登记。现有 `_shiju/session/set_mounts` 用 `mcpCatalog` 显式选择精确目录引用，提交会话挂载修订。受审引用进入提交记录与响应，普通 MCP 连接池不会启动它。进程池复核可撤销登记前缓存的普通执行器。供应商声明与已选来源同名即拒绝。
- 任务 48 调用 `crate::agent_apps::mcp::acp_mount::reviewed_linux_stdio(launch.clone(), cwd, project_selected).await`，其中 `project_selected` 只传本轮明确的项目来源候选。即使候选为空也须调用，以便读取会话来源。结果包含 `source`（`Project` 或 `Mounted`）、宿主服务声明、固定身份和 `current` 检查。使用 `guard(existing_approval)` 合并单次审批有效性，绑定到实际执行入口；发现前后及每次执行前均复核 `current`。会话来源只从已验证的当前工具上下文及持久化挂载记录读取。
- 定向回归入口 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent --lib acp_platform_mcp_review_mount_revokes_already_issued_approval`。子用例在隔离状态目录走真实宿主会话挂载更新、私有目录选择和生产单次审批消费。它检查同名冲突、原池零工具、移除、重挂及目录修订后旧批准失效。GNU 两目标须提供 `/usr/bin/true` 和 SQLite 构建依赖。本轮静态核对，尚未运行。
