# Hermes ACP 技能共存验收

2026-11-20，Darwin-arm64，状态为已阻塞。

- 固定 Hermes 0.19.0，入口 SHA-256 为 `e66a196bceb0124acc9979a4187c8af49dbc48cb8c1cc67b482dfa32b9b33beb`。冷下载的 wheel 未通过合约摘要校验。改用已安装 v1 的私有副本，从缓存安装 MCP SDK 1.26.0，核对入口版本及 v2 合约的全部文件摘要。探针在新建的私有网关配置目录中使用该副本。
- 复用已通过 CodeBuddy 验收的网关构建，SHA-256 为 `92a1bcb04e026d7595e756afd2fba3f4f4b01929fad223d13d782cc59e8d10c3`。探针提交了 `full_access=false`、`session_model` 和 `coexistence-fixture/hermes-coexistence`。基线会话 `ses_e07ad1106ffeLHRDWufwZ8D5P4` 在子进程启动前收到 `requires_full_access`，退出 1。计数为 `chat.done=0`、`chat.error=1`、模型请求 0、MCP 调用 0。网关证据保存在 `target/acp-skill-coexistence/hermes.json`。
- [桌面原生权限门禁](../../../../crates/libs/acp-client/src/client/process.rs)拒绝受限 Hermes 会话，因为宿主审批无法约束后端的原生工具。历史 MCP 回执使用的全访问属于当时的会话。本次探针使用受限授权。
- 对同一受审 Python 副本运行供应商本地对照，原生 `HERMES_HOME/skills` 能发现技能元数据，并经 `skill_view` 读取随机正文及附件。设置空 `SHIJU_HERMES_SKILL_ROOT` 后，扫描与激活均被拒绝。既有 `skill_mount_tests.py` 的 5 项用例均通过，其中已选根仍会屏蔽原生目录和外部目录。模型来源驱动还会为受管会话创建私有 `HERMES_HOME`，因此需要一个按租户隔离的原生技能来源。
- 挂载、撤除及 MCP 拒绝对照均未进入。模型和权限回读、调用 ID、三轮终态及 `0/1/0` 仍待验。Cargo 槽位 6 仅等待编译锁，复用网关取得基线后退出排队。生产 Rust 和制品配置未改动，未运行 Rust 单测。

复验需要能约束 Hermes 原生工具的受审启动方式，或为隔离验收会话明确提供适用授权。还需确认原生技能的受信来源及逐次 MCP 拒绝如何传递到下游。当前仍沿用现有技能和权限门禁。
