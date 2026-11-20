# Hermes ACP 技能共存验收

2026-11-20，Darwin-arm64，状态为已阻塞。

- 固定 Hermes 0.19.0，入口 SHA-256 为 `e66a196bceb0124acc9979a4187c8af49dbc48cb8c1cc67b482dfa32b9b33beb`。冷下载的 wheel 未通过合约摘要校验。改用已安装 v1 的私有副本，从缓存安装 MCP SDK 1.26.0，核对入口版本及 v2 合约的全部文件摘要。探针在新建的私有网关配置目录中使用该副本。
- 复用已通过 CodeBuddy 验收的网关构建，SHA-256 为 `92a1bcb04e026d7595e756afd2fba3f4f4b01929fad223d13d782cc59e8d10c3`。探针提交了 `full_access=false`、`session_model` 和 `coexistence-fixture/hermes-coexistence`。基线会话 `ses_e07ad1106ffeLHRDWufwZ8D5P4` 在子进程启动前收到 `requires_full_access`，退出 1。计数为 `chat.done=0`、`chat.error=1`、模型请求 0、MCP 调用 0。网关证据保存在 `target/acp-skill-coexistence/hermes.json`。
- [桌面原生权限门禁](../../../../crates/libs/acp-client/src/client/process.rs)拒绝受限 Hermes 会话，因为宿主审批无法约束后端的原生工具。历史 MCP 回执使用的全访问属于当时的会话。本次探针使用受限授权。
- 对同一受审 Python 副本运行供应商本地对照，原生 `HERMES_HOME/skills` 能发现技能元数据，并经 `skill_view` 读取随机正文及附件。设置空 `SHIJU_HERMES_SKILL_ROOT` 后，扫描与激活均被拒绝。既有 `skill_mount_tests.py` 的 5 项用例均通过，其中已选根仍会屏蔽原生目录和外部目录。模型来源驱动还会为受管会话创建私有 `HERMES_HOME`，因此需要一个按租户隔离的原生技能来源。
- 挂载、撤除及 MCP 拒绝对照均未进入。模型和权限回读、调用 ID、三轮终态及 `0/1/0` 仍待验。Cargo 槽位 6 仅等待编译锁，复用网关取得基线后退出排队。生产 Rust 和制品配置未改动，未运行 Rust 单测。

复验需要能约束 Hermes 原生工具的受审启动方式，或为隔离验收会话明确提供适用授权。还需确认原生技能的受信来源及逐次 MCP 拒绝如何传递到下游。当前仍沿用现有技能和权限门禁。

## 受限沙箱原型

Darwin-arm64 上对经过 v2 合约校验的私有副本应用 [候选补丁](../../../../scripts/acp_skill_coexistence/hermes_sandbox.patch)，并校验补丁涉及的全部输入和输出文件摘要。原型保留私有 `HERMES_HOME/skills` 的原生优先级，将受信会话视图追加为外部根。调用方发出原生 `skill_view` 请求，供应商照常发现、激活和读取技能目录内附件。MCP 在 Hermes 发出实际调用前通过 ACP 请求单次批准。沙箱只连接本次模型和 MCP 回环端口，MCP 回环中继将调用交给独立的真实 stdio 进程。

| 新会话 | ACP 终态 | 相关模型请求 | MCP 批准 | 下游调用 |
| --- | --- | ---: | --- | ---: |
| 基线 | `end_turn` | 3 | 无 | 0 |
| 挂载 | `end_turn` | 6 | 单次批准 | 1 |
| 撤除 | `end_turn` | 3 | 无 | 0 |
| 拒绝 | `end_turn` | 6 | 拒绝 | 0 |

四轮均验证原生正文及附件调用 ID，挂载和拒绝轮另验证选中技能调用 ID。挂载轮模型收到真实 MCP 附件，拒绝轮模型收到对应调用的拒绝结果。未选技能未进入目录，初始模型请求仅包含技能元数据。每轮额外检查沙箱拒绝同级私有文件读取、工作区写入、启动 shell 和连接非白名单端口。复验退出 0，独立会话 ID、调用 ID 和补丁摘要位于 `target/acp-skill-coexistence/hermes-sandbox.json`。

从仓库根运行，将 `HERMES_COEXISTENCE_RUNTIME` 指向已有 v2 合约副本：

```sh
PYTHONPATH="$PWD/scripts" python3 -m acp_skill_coexistence.hermes_sandbox \
  --output target/acp-skill-coexistence/hermes-sandbox.json
```

该证据覆盖直接 ACP 会话。正式网关仍需接入受信沙箱启动配置、按租户绑定的原生目录及消费单次 MCP 批准的生产中继。网关 `chat.done`、模型与权限回读、历史恢复及撤销仍待复验。
