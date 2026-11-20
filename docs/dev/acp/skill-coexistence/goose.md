# Goose 会话技能与 MCP 共存验收

2026-11-20，Darwin arm64。固定 Goose v1.57.0，ACP 入口为 `goose acp`。`session_model` 继续经宿主私有 OpenAI Chat Completions 中继，并由现有模型锁约束。探针选择 `coexistence-fixture/deepseek-v4.1-flash-1017`，只替换供应商的模型响应。

## 受管制品

| 输入 | SHA-256 |
| --- | --- |
| 官方 v1.57.0 源码归档 | `36fcab524e25ca563830757bf2da8a68716ea2a5d7ca2f055dec4647b78548d9` |
| 历史 v1 补丁 | `3a47bbbdcf861d680ff051879d81d23ef872d035a01a0549f9cedecd65b54e06` |
| 共存 v2 补丁 | `3b75b96dc038b60f294de02354de34150a204e5e08f51a5bd37ff9095898ebcb` |

归档下载中断后，从官方 Git 标签 `30bba95105e59bc3e7df624cf6b4d433690cfe12` 重建归档并核对完整 SHA-256。受管安装修订号为 v2，选中技能需通过受审入口和 v2 补丁回执检查。[共存补丁](../../../shiju/acp-model-source/evidence/goose/v1.57.0-model-lock-skill-coexistence.patch)

## 发现与读取

旧补丁发现 `SHIJU_GOOSE_SKILL_ROOT` 时独占扫描结果，空值也会关闭原生目录和内置技能。新补丁保留项目、用户、配置、启用插件和内置技能的发现顺序，再追加受审会话视图。原生同名条目优先。宿主仍把空选集映射为空字符串，以屏蔽继承的会话根。会话视图按只读方式发现，条目的技能根由扫描器解析为规范路径。供应商附件读取仍拒绝跟随符号链接。授权与私有状态沿用原有路径。[Goose 原生发现](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/skills/mod.rs)

探针从真实模型请求的技能目录取名，经 `load_skill(name)` 获取正文，再用 `load_skill(name/references/value.txt)` 读取原生激活结果声明的附件。选中技能仅经会话视图进入目录，激活所得目录须等于对应技能的规范路径。未选中技能来自独立 ShiJu 条目。探针以调用 ID 匹配模型续轮，核对首次请求无随机正文、附件或 MCP 值。[Goose 原生激活](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/state_machine/ops_skills.rs)

## 回环

四个新会话分别检查基线、挂载、撤除和 MCP 拒绝。挂载和拒绝轮由宿主配置注入只读 stdio MCP。所有会话回读同一模型绑定和 `full_access=false`，只放行探针发出的确切读操作。历史步骤见 [MCP 验收](../mcp-acceptance/goose.md) 和 [技能挂载回执](../../../vibewindow/acp-skill-mounts/evidence/goose.json)。

旧版基线使用 Goose `d810a224d335490af294279bb89e5afb28e776f47a78a5eb16ab1293a60a6df2` 和网关 `a33ebf2cf7a35e986065bae9203472f539d2b0a96c32aa52e54d5874427f0f98`。真实会话 `ses_e076f8a01ffenzbsgFjhoR1anw` 的首个差异为 `unexpected native skill catalog`，退出 1。首次基线记录于 `target/acp-skill-coexistence/goose-red.json`。最终探针复跑得到相同首个差异，处理 4 次模型请求并退出 1。真实会话 `ses_e075aaf3cffevOg7iYLodIACc6` 有一个 `chat.error`、零次工具审批和零次 MCP 调用。复跑证据为 `target/acp-skill-coexistence/goose-red-final.json`。

首版共存制品已恢复原生目录，但挂载轮报告 MCP 工具目录缺失。原生 Goose 先探测 `server/discover`，旧夹具对未知方法返回空成功。夹具改为标准 `-32601` 后，实测 Goose 回退到 `initialize` 和 `tools/list`。这次协商复测还定位到附件读取拒绝会话视图的目录符号链接，终态为 `Not a directory`。v2 扫描改用规范目录后继续使用 Goose 的原生受限读取器。

最终 v2 Goose 入口 SHA-256 为 `8363c881ea296ddf1500d704903a3b93c1ca1ca09d9721079a181b04795f110b`。独立原生 ACP 对照运行四个新会话，全部返回 `end_turn`。原生正文和附件每轮均回传，挂载与拒绝轮也读到所选技能正文和附件。只读 stdio MCP 的实际调用计数为 0/1/0/0。批准和拒绝均作用于同一服务的同一工具。调用 ID 及决定见 `target/acp-skill-coexistence/goose-native.json`。

`llm-runtime` 定向过滤 `acp_skill_coexistence_goose` 运行 2 项且全部通过。Python 定向目录测试运行 1 项并通过。本轮网关构建使用 Cargo 槽位 10，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均指向该槽位的绝对路径。PlanMember.id 的两处私有字段访问使 `task-scheduler` 编译返回 `E0616`，网关构建退出 101。断点位于 `crates/libs/task-scheduler/src/store/postgres/task_plan.rs:247` 及第 249 行。本次 Goose 改动未涉及该包。

## 2026-11-21 最终网关回归

共享网关在 Cargo 槽位 6 构建并退出 0。构建 HEAD 为 `03e5fc747ed4571184fbb6a156bb48d093b9fee0`，网关 SHA-256 为 `c59878516f6a88bb8f60ce78955654b9859bca9b82af6711fc14c83d0a572eba`。构建前后源码一致，源码差异与双缓存路径见 `target/acp-skill-coexistence/final-regression-gateway.json`。复用此前通过的两项 Rust 定向测试。

四轮均绑定 Goose 与 `session_model`，选用 `coexistence-fixture/deepseek-v4.1-flash-1017`，回读确认完全访问关闭。网关探针退出 0。

| 场景 | 独立会话 | `chat.done` / `chat.error` | 模型请求 | MCP 调用 | MCP 审批 |
| --- | --- | --- | --- | --- | --- |
| 基线 | `ses_e05fed978ffeKysvfSWa2BEqyz` | 1 / 0 | 3 | 0 | 无 |
| 挂载 | `ses_e05feb5bcffeB0uaoe551rjIBg` | 1 / 0 | 6 | 1 | once |
| 撤除 | `ses_e05fe8bacffeUixfCGVhjvZdrB` | 1 / 0 | 3 | 0 | 无 |
| 拒绝 | `ses_e05fe6903ffeY9CRR6aY95zlKM` | 1 / 0 | 6 | 0 | reject |

挂载轮按原生正文、原生附件、所选正文、所选附件和只读 MCP 的顺序，回传调用 ID `call_ecbc6768b2830171628b3393`、`call_e5db4e1a1c556f072d7d05fd`、`call_cc7bb2708f14eb212b2604f2`、`call_77b4882adbdeb8688a62498a`、`call_ae8705d14f55d74461c758e2`。拒绝轮的 `call_94eca3c1a6b6a2d5cb78ba37` 回传原生拒绝，服务端执行零次。四轮的调用 ID、模型和权限回读见 `target/acp-skill-coexistence/goose.json`。
