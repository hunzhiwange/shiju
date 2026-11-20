# TRAE 0.121.3 原生技能与 ShiJu 技能共存

2026-11-20，Darwin arm64。受管入口为 `traecli acp serve`，安装修订为 `trae-0.121.3-model-source-v1`。上游构建提交为 `72fe00ff52f56b0eb11a37bd561d4f10a4a44354`。沿用原归档、组合二进制及启动器，安装器与探针均核对制品摘要。[受管合约](../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json#L707) · [既有 MCP 回执](../mcp-acceptance/trae.md)

| 制品 | SHA-256 |
| --- | --- |
| 官方归档 | `30fd43892949dba6d1bff5fb464c8ec3a936b61cb97ed5a9d5af3a97b6e3f3f8` |
| `native/trae-cli` | `7648c66ffb535e83e4b1f5bbd820f0384d77234763cfb32700b1be3289358758` |
| `native/trae-local` | `f77806ca15efea1c33ffc4c3c08ab3f1c57111883dd3313822e673791484adde` |
| `traecli` | `de3bb2e987468cfc57023dbf64b8e990ef7228a4253b6f7f0e5e50201d3f1f1c` |

## 原生入口

受管启动器把原生用户目录和进程工作目录绑定到单次请求目录。固定版还扫描用户 HOME 下的 `.agents/skills`。宿主在请求目录的 `skills` 入口发布经校验的 `SessionSkillView`，由供应商枚举、激活和读取。独立同名对照中，TRAE 的私有用户技能先于 HOME 下的同名项，保留该供应商优先级。空选集保留原生发现。项目根下的 `.trae/skills` 在现有受管启动目录语义下未被发现，本次覆盖用户级原生发现。

模型使用 `session_model` 绑定 `coexistence-fixture/deepseek-v4.1-flash-1017`，只替换供应商的流式 Chat Completions 响应。探针从 `Skill` 的参数枚举读取目录，用 `Skill(skill)` 和 `Read(file_path)` 的响应按调用 ID 验证随机正文及附件。首次模型请求只检验目录与元数据，排除随机正文、附件和 MCP 返回值。MCP 服务由会话配置通过原生 stdio 注册，审批使用受限模式下的 `desktop-acp` 交互。

## 修复前

复用已有网关，SHA-256 为 `2b5ff908c8839a56a0eab76332a33d929b942a468f7b26c7858cfb7b122cd5a1`。使用同一探针，无挂载轮由两条工具调用 ID 验证原生正文与附件，三次模型请求、一次 `chat.done`、零 `chat.error` 和零 MCP。挂载轮触发宿主的 `skill directory mounting is unverified; adapter acceptance required`，零模型请求、零 MCP、一次 `chat.error`，退出 1。结构化证据为 `target/acp-skill-coexistence/trae-before.json`。

## 2026-11-20 初次验证

`llm-runtime` 已通过槽位 6 网关构建阶段的编译。网关构建退出 101，`task-scheduler` 的 PostgreSQL 存储在 `crates/libs/task-scheduler/src/store/postgres/task_plan.rs:247` 和 `:249` 访问了私有字段 `PlanMember.id`（E0616）。当时未生成修复后的四轮报告。构建的 `CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-6`。

定向单测复用槽位 3 的同源 Cargo 测试目标 `llm_runtime-85f23fc8970681f8`，持有槽位时按 `acp_skill_coexistence_trae --test-threads=1` 运行，退出 0，实际执行 2 项且全部通过。覆盖选集链接、加载正文和附件、拒绝失效视图，以及空选集保留绑定配置。两个缓存变量均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。

独立原生 ACP 使用同一受管版本验证了两类技能的正文及附件进入对应调用 ID 的模型续轮，stdio MCP 同意后执行一次。旧网关的 MCP 独立对照也验证了受限权限下单次批准和执行、拒绝和零下游调用。TRAE 拒绝后用原调用 ID 报告 `Tool call rejected by user`，直接结束该回合。旧网关对照覆盖原生 MCP 路径，完整的 ShiJu 选集网关回归见下节。

## 2026-11-21 最终网关回归

复用任务 5 在槽位 6 构建的网关。构建 HEAD 为 `03e5fc747ed4571184fbb6a156bb48d093b9fee0`，网关 SHA-256 为 `c59878516f6a88bb8f60ce78955654b9859bca9b82af6711fc14c83d0a572eba`。持有槽位时核对源码树、构建范围内的差异摘要、两项缓存路径和二进制摘要。构建差异仅调整其他适配器的模块与分派顺序。证据见 `target/acp-skill-coexistence/final-regression-gateway.json`。

TRAE 探针退出 0，四轮均回读 `session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017` 和 `full_access=false`。继续采用前次已通过的两项定向 Rust 单测。

| 场景 | 新会话 ID | 模型请求 | `chat.done` / `chat.error` | 下游 MCP | 审批 |
| --- | --- | ---: | --- | ---: | --- |
| 基线 | `ses_e05fa7d30ffedak8THqmEaUkwS` | 3 | 1 / 0 | 0 | 无 |
| 挂载 | `ses_e05fa5558fferDb0Bz2P2EWW53` | 6 | 1 / 0 | 1 | once |
| 撤除 | `ses_e05fa2a48ffewN4Bhc691ylktr` | 3 | 1 / 0 | 0 | 无 |
| 拒绝对照 | `ses_e05fa0341ffeHK1FVwCFe8J7fX` | 5 | 1 / 0 | 0 | reject |

挂载轮的对应调用为 原生激活 `call_f181a340960cad2581d777ef`、原生附件 `call_3249e7ac83d0806fb9daf9e6`、选中激活 `call_ce9a976b4120f1ca86c04e38`、选中附件 `call_15bfb6f95d0b86509ecb6f27`、MCP `call_6d9a7a92ddce0a82be237602`。
拒绝对照以 `call_eee7799349ed23dd630abf6a` 对应的供应商终态回报确认拒绝。完整调用 ID、权限和制品回读见 `target/acp-skill-coexistence/trae.json`。
