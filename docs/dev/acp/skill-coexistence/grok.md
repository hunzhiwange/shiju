# Grok Build ACP 技能共存验收

验收制品为 Darwin arm64 Grok Build `1.0.77 (50ca579fa996)`，原生入口为 `grok agent stdio`，SHA-256 为 `197c66a63e5ac46e07450b0ca21d141ce665050a412180f185dadfae1c7a5b2a`。探针在私有目录复制并复核该入口，只替换 Chat Completions 模型响应。会话绑定 `session_model` 与 `coexistence-fixture/deepseek-v4.1-flash-1017`，使用工作区权限和宿主逐次批准。[现有 MCP 验收](../mcp-acceptance/grok.md)

## 原生路径与接线

- 原生扫描器通过 `[skills].paths` 追加目录，模型优先收到名称、描述及绝对路径。固定版 ACP 工具清单提供原生 `read_file`，按目录声明读取完整的 `SKILL.md`，随后单独读取附件。直接原生 ACP 对照验证了临时选集视图中的符号链接能被扫描。等名的 Grok 用户技能和附加路径技能在原生目录中只出现一次，采用了用户技能。
- 测试原生技能经隔离 HOME 的 Claude 兼容扫描器发现，实际文件在私有项目中。探针从模型目录核对真实文件路径后读正文，保留项目文件夹信任门禁与宿主工作区读取边界。位于工作区外的用户技能仍受宿主文件读取政策约束。
- 会话只向版本、平台与摘要均经确认的原生入口追加选集视图。追加写入请求私有配置，保留原生配置项。宿主仅在已验证的租约与会话模型绑定同时存在时，把选集来源提供给现有只读文件处理器。
- stdio MCP 继续沿受管直接声明及原生分派。独立原生 `use_tool` 对照中，`reject_once` 产生同 ID 的失败通知和 ACP `cancelled`，下游调用为零。模型夹具在提交 MCP 请求时先输出一条进度消息，因此拒绝后可正常结束，无需伪造工具结果或模型续轮。

## 修复前

复用槽位 10 的网关，SHA-256 为 `b2a05ed113477712c36de9dd731517d27f151d4e51f7c8096c0e80ae2e27d276`。Cargo 双缓存路径均为仓库的 `target/rust-slot-10`。

- 无挂载轮有一个 `chat.done`、零个 `chat.error`，三次有效模型请求，MCP 调用及审批均为零。原生正文和附件分别通过 `call_grok_bdf3f89ffd89f5b9b6068c05` 和 `call_grok_5bfbd9efcfd8212409c94d48` 回传，绑定和权限回读符合预期。
- 挂载轮在模型请求前被 `grok: no verified skill directory mount branch` 拒绝，零 `chat.done`、一次 `chat.error`，无 MCP 审批和调用。探针退出 1。证据位于 `target/acp-skill-coexistence/grok-before.json`。

## 修复后及初次构建

槽位 3 的 Cargo 双缓存路径均为仓库的 `target/rust-slot-3`。定向 `cargo test --locked -j 1 -p llm-runtime --lib acp_skill_coexistence_grok -- --test-threads=1` 退出 0，实际运行 2 项并全部通过。Python 探针语法检查通过。

独立的固定版 ACP 进程复用仓库的 Grok 直出启动器，分别读取原生技能和私有视图内选中技能的正文及附件。挂载轮的随机 MCP 值在同一调用 ID 的模型续轮中回传。拒绝轮收到同 ID 的 `toolNotRun=rejected`。四个原生会话各自使用不同的隔离目录，选中技能仅在挂载及拒绝轮出现在模型目录。

| 轮次 | 原生终态 | 审批 | MCP 调用次数 |
| --- | --- | --- | --- |
| 基线 | `end_turn` | 无 | 0 |
| 挂载 | `end_turn` | `allow_once` | 1 |
| 撤除 | `end_turn` | 无 | 0 |
| 拒绝对照 | `cancelled` | `reject_once` | 0 |

直连挂载轮的原生正文和附件调用为 `call_grok_592c44a5a4995e5287bbfa55` 与 `call_grok_3b0be5297f59b570e76e275b`，选中正文和附件调用为 `call_grok_b3b203464e03028c30814ebb` 与 `call_grok_7083ddf2c617f1d4a84eeb5a`。获批与被拒的 MCP 调用分别为 `call_grok_b19024f8fd34de3cca4ccaa3` 与 `call_grok_19748e9ded35477b354b7925`。

直接 ACP 证据位于 `target/acp-skill-coexistence/grok-direct.json`。旧网关独立检查了原生技能与只读 MCP，取得四轮各一个 `chat.done` 和零 `chat.error`，审批与调用计数相同。证据位于 `target/acp-skill-coexistence/grok-mcp-control.json`。

正式 `cargo build --locked -j 1 -p vw-cli --bin shiju` 因既有的 `task-scheduler` 编译错误退出 101。PostgreSQL 工作计划的 `store/postgres/task_plan.rs:247,249` 从相邻模块访问私有字段 `PlanMember.id`，产生两处 E0616。该错误随共享网关构建恢复。支持范围限定为 Darwin arm64 上摘要相符的原生入口。

## 正式网关回归

复用 Cargo 槽位 6 的共享网关。两个 Cargo 缓存路径均指向仓库的 `target/rust-slot-6`。构建源码版本为 `03e5fc747ed4571184fbb6a156bb48d093b9fee0`，网关 SHA-256 为 `c59878516f6a88bb8f60ce78955654b9859bca9b82af6711fc14c83d0a572eba`。构建收据与当前源码的差异仅涉及模块声明及匹配顺序、Kimi 只读分派及其测试。Grok 主机逻辑一致。二进制内的 Grok 投影脚本与该源码版本逐字节一致，脚本 SHA-256 为 `7fee75938f5b3f13f7d9da935150337aa6c3743411d8b9c02036fbf18b03bfc4`。探针重新核对原生 Grok 入口的版本与摘要。

`verify_acp_skill_coexistence.py --agent grok` 退出 0。四轮各有一个 `chat.done` 和零个 `chat.error`，并回读 `grok`、`session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017` 与 `full_access=false`。拒绝对照由原生终止通知关联到被拒的调用 ID。

| 轮次 | 新会话 | 有效模型请求 | MCP 审批 | 下游调用 |
| --- | --- | --- | --- | --- |
| 基线 | `ses_e05f7640bffeTl2CeVc6Vz6C9k` | 3 | 无 | 0 |
| 挂载 | `ses_e05f6e721ffevRlKZBpYeaa3QR` | 6 | once | 1 |
| 撤除 | `ses_e05f6bcb0ffeYYLXGx4dO5SHQS` | 3 | 无 | 0 |
| 拒绝 | `ses_e05f684faffesGAX5blEhBM7O3` | 5 | reject | 0 |

挂载轮按调用 ID 回传原生技能正文 `call_grok_b26e63cb80f36484db8abb39`、附件 `call_grok_b5eeb4aad1cc28daac25d473`、选中技能正文 `call_grok_a49e2706b7f846626cc209f1`、附件 `call_grok_5802907017210ead3f698420` 和 MCP 结果 `call_grok_201d3f30d8b56f5744e3ab59`。拒绝调用 `call_grok_f92e455a5c91738892df3929` 的下游调用为零。基线和撤除轮均未发现选中技能。结构化证据位于 `target/acp-skill-coexistence/grok.json`，共享构建证据位于 `target/acp-skill-coexistence/final-regression-gateway.json`。两项 Rust 定向单测沿用此前通过的结果。
