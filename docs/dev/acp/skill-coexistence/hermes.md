# Hermes ACP 技能共存验收

Darwin arm64。生产网关四轮和两项定向 Rust 单测通过。

## 修复前

- 固定 Hermes 0.19.0，入口 SHA-256 为 `e66a196bceb0124acc9979a4187c8af49dbc48cb8c1cc67b482dfa32b9b33beb`。冷下载的 wheel 未通过合约摘要校验。改用已安装 v1 的私有副本，从缓存安装 MCP SDK 1.26.0，核对入口版本及 v2 合约的全部文件摘要。探针在新建的私有网关配置目录中使用该副本。
- 复用已通过 CodeBuddy 验收的网关构建，SHA-256 为 `92a1bcb04e026d7595e756afd2fba3f4f4b01929fad223d13d782cc59e8d10c3`。探针提交了 `full_access=false`、`session_model` 和 `coexistence-fixture/hermes-coexistence`。基线会话 `ses_e07ad1106ffeLHRDWufwZ8D5P4` 在子进程启动前收到 `requires_full_access`，退出 1。计数为 `chat.done=0`、`chat.error=1`、模型请求 0、MCP 调用 0。修复前的网关证据保存在 `target/acp-skill-coexistence/hermes-baseline.json`。
- [桌面原生权限门禁](../../../../crates/libs/acp-client/src/client/process.rs)拒绝受限 Hermes 会话，因为宿主审批无法约束后端的原生工具。历史 MCP 回执使用的全访问属于当时的会话。本次探针使用受限授权。
- 对同一受审 Python 副本运行供应商本地对照，原生 `HERMES_HOME/skills` 能发现技能元数据，并经 `skill_view` 读取随机正文及附件。设置空 `SHIJU_HERMES_SKILL_ROOT` 后，扫描与激活均被拒绝。既有 `skill_mount_tests.py` 的 5 项用例均通过，其中已选根仍会屏蔽原生目录和外部目录。模型来源驱动还会为受管会话创建私有 `HERMES_HOME`，因此需要一个按租户隔离的原生技能来源。
- 挂载、撤除及 MCP 拒绝对照均未进入。模型和权限回读、调用 ID、三轮终态及 `0/1/0` 仍待验。Cargo 槽位 6 仅等待编译锁，复用网关取得基线后退出排队。生产 Rust 和制品配置未改动，未运行 Rust 单测。


## 受限沙箱原型

Darwin-arm64 上对经过 v2 合约校验的私有副本应用 [候选补丁](../../../../patches/acp-model-source/hermes/coexistence.patch)，并校验补丁涉及的全部输入和输出文件摘要。原型保留私有 `HERMES_HOME/skills` 的原生优先级，将受信会话视图追加为外部根。调用方发出原生 `skill_view` 请求，供应商照常发现、激活和读取技能目录内附件。MCP 在 Hermes 发出实际调用前通过 ACP 请求单次批准。沙箱只连接本次模型和 MCP 回环端口，MCP 回环中继将调用交给独立的真实 stdio 进程。

| 新会话 | ACP 终态 | 相关模型请求 | MCP 批准 | 下游调用 |
| --- | --- | ---: | --- | ---: |
| 基线 | `end_turn` | 3 | 无 | 0 |
| 挂载 | `end_turn` | 6 | 单次批准 | 1 |
| 撤除 | `end_turn` | 3 | 无 | 0 |
| 拒绝 | `end_turn` | 6 | 拒绝 | 0 |

四轮均验证原生正文及附件调用 ID，挂载和拒绝轮另验证选中技能调用 ID。挂载轮模型收到真实 MCP 附件，拒绝轮模型收到对应调用的拒绝结果。未选技能未进入目录，初始模型请求仅包含技能元数据。每轮额外检查沙箱拒绝同级私有文件及工作区无关文件读取、直接及经符号链接写入工作区、启动 shell 和连接非白名单端口。基线在新进程加载原会话，确认已回放绑定历史。复验退出 0，独立会话 ID、调用 ID 和补丁摘要位于 `target/acp-skill-coexistence/hermes-sandbox.json`。

从仓库根运行，将 `HERMES_COEXISTENCE_RUNTIME` 指向已有 v2 合约副本：

```sh
PYTHONPATH="$PWD/scripts" python3 -m acp_skill_coexistence.hermes_sandbox \
  --output target/acp-skill-coexistence/hermes-sandbox.json
```

该证据覆盖直接 ACP 会话和跨进程历史恢复。


## v3 正式接线

受限启动限定 Darwin arm64、`session_model` 和宿主的读取许可模式。宿主将授权工作区的 `.hermes/skills` 绑定到私有 Hermes 原生目录，并独立挂载已校验的会话选集。系统沙箱限制文件、进程和网络，ACP 客户端同时关闭文件及终端回调。MCP 由宿主投影已选 stdio 服务，转发前消费绑定到工具名及参数的单次批准。

首个生产网关构建完成，探针在 Hermes 握手前退出。对照得到 Python 导入依赖调用 `getcwd()` 时被文件规则拒绝。更新后沙箱只准读取工作目录项和各获准的子树。更新后的网关复验亦通过。


## 正式网关复验

2026-11-21，槽位 3。构建及探针退出 0。网关 SHA-256 为 `678f33a3799c31376b6f3f215214d3474507bcb3bb86bdd34a06678dd46885d2`。v3 入口 SHA-256 为 `e66a196bceb0124acc9979a4187c8af49dbc48cb8c1cc67b482dfa32b9b33beb`。两个 Cargo 缓存路径使用同一槽位，精确路径及 v3 合约文件摘要记录在 `target/acp-skill-coexistence/hermes.json`。

| 回合 | 新会话 ID | `chat.done` / `chat.error` | 模型请求数 | MCP 审批 / 下游次数 |
| --- | --- | ---: | ---: | --- |
| 基线 | `ses_e0648868fffeKnMF6dMb8OI9x4` | 1 / 0 | 3 | 无 / 0 |
| 挂载 | `ses_e064879d5ffeyNL9cDPB3P24Nj` | 1 / 0 | 6 | 单次批准 / 1 |
| 撤除 | `ses_e06486b46ffea6w5JIieQL3Ipp` | 1 / 0 | 3 | 无 / 0 |
| 拒绝 | `ses_e06485f48ffejz3ndIaeWAWsOD` | 1 / 0 | 6 | 拒绝 / 0 |

四轮回读均绑定 `hermes` 0.19.0、`session_model` 和 `coexistence-fixture/hermes-coexistence`，`full_access=false`。挂载轮原生正文、附件调用分别为 `call_7848e761fce0ac8f6e7e38c9`、`call_f8462b7c705423f87e16545b`，选中技能正文、附件为 `call_0d1e1842b8161820853cf486`、`call_413ca3adf13436a2541f100c`。MCP 调用 `call_5b963ff5e6d9e02c0545e2f5` 返回真实下游值，拒绝调用 `call_136e13a73fbe1dea788aeade` 返回拒绝结果。其余调用 ID 和模型回读记录在结构化回执中。

补充恢复对照复用了同一网关构建。重新打开已挂载会话 `ses_e0640095fffejoL2FXXM4fhd0E` 后，模型收到原对话，并看到该会话绑定的技能及 MCP。网关发出一次 `chat.done`，持久化权限仍为 `full_access=false`。修改代理默认配置只影响新会话，原会话保留服务端固化的默认值。撤除轮用新会话验证，目录与工具均已撤回。补充证据位于 `target/acp-skill-coexistence/hermes-restore.json`。


定向 Rust 检查先因并行 DimCode 变更中的两处 `E0283` 中止。共享修复后复用槽位 9 新编译的同一 `llm-runtime --lib` 测试目标，按 `acp_skill_coexistence_hermes --test-threads=1` 运行。两个用例实跑通过，退出 0。测试可执行文件 SHA-256 为 `333ad5b390bafe16c48793d98ff13873c191958d6c070867044ba6786c9fba33`。输出位于 `target/acp-skill-coexistence/hermes-tests-reused.log`。
