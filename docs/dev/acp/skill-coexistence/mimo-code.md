# MiMo Code 原生技能与会话挂载

2026-11-20，Darwin arm64，受管 MiMo Code 0.1.17，安装修订 `mimo-code-0.1.17-model-source-v1`。隔离探针核对可执行文件版本、构建收据以及制品 SHA-256 `c086c1bd5c4ada962880f7636de5b07fc442db19edccf3e383887f6f3c47f7ae`。源码和原 stdio 补丁摘要见 [受管合约](../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json)。使用 `acp --pure`、`coexistence-fixture/deepseek-v4.1-flash-1017` 的 `session_model` 绑定和只读 stdio MCP 的宿主同端口中继。[历史 MCP 验收](../mcp-acceptance/mimo-code.md) · [审批审计](../mcp-policy-audit/coexistence/mimo-code.md)

## 接线

固定版原生扫描器保留项目 `.agents/skills`，并在加载原生根之后扫描 `skills.paths`。宿主将当前请求验证后的技能视图追加到私有配置。Seatbelt 只增加该视图及其选中目标的文件读取规则。扫描器按固定版原生顺序处理同名条目。仅受管 Darwin arm64 的 `session_model` 且原生工具可用时接入挂载。

MiMo 的原生目录发布技能的文件 URI。探针按实际发布的地址调用原生 `read` 读取 `SKILL.md`，随后用同一工具读取附件。固定版 `skill` 工具取样附件时需要 ripgrep 子进程，因此受管工具名单沿用现有的原生文件工具及经审批的 MCP。原生禁用项由 MiMo 处理。选集撤除时，新请求仅使用原生目录。

## 修复前对照

旧网关 SHA-256 为 `8ee6357b62005376c74ef75d26aa03e9379af73f81e9e8b3a63f5552ec768c52`。严格探针在首个主模型请求发现原生技能目录缺失，退出 1。额外以已知路径调用旧版原生 `read`，无挂载会话 `ses_e06eb3a66ffeKez8y9SXAO2fTW` 读取正文和附件，有 1 个 `chat.done`、0 个 `chat.error`。尝试挂载后在宿主技能能力门禁拒绝，零主模型请求。两份对照报告位于 `target/acp-skill-coexistence/mimo-code-before.json` 和 `mimo-code-before-read-only.json`。

## 独立进程对照

同一受管制品在私有配置和 Seatbelt 策略下完成 ACP v1 启动，报告 OpenCode 0.1.17。无挂载原生正文与附件的两次调用均回到模型。挂载时模型在原生目录发现项目技能及选中技能，随后收到四次 `read` 结果。选中技能路径经过请求视图，绝对源目录位于工作区之外。两轮以 `end_turn` 结束。另在生产 MCP 沙箱策略内独立启动共用 stdio 夹具，完成初始化、工具目录和一次读取，写入一条实际调用审计。

独立 ACP 四轮使用实际 MiMo 进程、私有回环模型、同端口测试中继和 Seatbelt 内的只读 stdio 服务。首次运行的拒绝轮未发布 MCP，复跑四轮均为 `end_turn`。拒绝轮从 ACP 失败终态核对调用 ID，审计记录零次 `tools/call`。

| 场景 | ACP 会话 | 主模型请求 | MCP 审批 | 实际 MCP 调用 |
| --- | --- | ---: | --- | ---: |
| 无挂载 | `ses_ffe5e06e0c5e5ffezgBINxIPQE` | 3 | 无 | 0 |
| 挂载 | `ses_ffe5e06e0bfa4ffeDfgq5fiXU0` | 6 | 单次批准 | 1 |
| 撤除 | `ses_ffe5e06e0b92bffeVjtI4dLaD9` | 3 | 无 | 0 |
| 拒绝 | `ses_ffe5e06e0b339ffeYQ5NICH212` | 5 | 单次拒绝 | 0 |

挂载轮的原生正文与附件调用为 `call_74fe77104144976f8cf0fc81`、`call_f3f5b29acc37782476689915`，选中正文与附件为 `call_4275315f8b6b61f634b250fa`、`call_16ec47134ac50501d60d5f6f`，MCP 为 `call_bff0c19a334a1260fb365bd2`。拒绝调用 `call_00f2cf726aacda9c1ee73d00` 终态为失败。记录见 `target/acp-skill-coexistence/mimo-code-direct.json`。

额外的原生目录对照中，声明 `disable-model-invocation: true` 的项目技能未发布到模型目录。原生根和会话根出现同名技能时，固定版扫描器采用后注册的会话视图，模型收到该视图的正文和附件。记录见 `target/acp-skill-coexistence/mimo-code-direct-guards.json`。

## 正式网关验收

首次定向网关构建使用槽位 6，因 `task-scheduler` 在 `src/store/postgres/task_plan.rs:247,249` 访问私有字段 `PlanMember.id`，报 `E0616` 并退出 101。调度器修复后，复用已构建的槽位 10 网关，SHA-256 为 `bb626e51a5235a6db6037d93b0242e945e573eea8aeb0471fc17da5c49d4fdc2`。两次运行的 `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 分别指向各自槽位的绝对路径。

首次正式拒绝轮已产生原生失败调用和零下游执行。MiMo 在拒绝后结束提示，回环模型此前未发任何正文，网关报「模型未返回内容」。复跑时，回环模型在发出只读 MCP 调用的同一响应中说明正在检查夹具。拒绝后依然核对原生失败调用 ID 和零次实际执行。

最终探针退出 0。四个独立会话各有 1 个 `chat.done`、0 个 `chat.error`。均回读 `acp-coexistence / mimo-code`、所选模型、`session_model` 与 `full_access=false`。

| 场景 | 网关会话 | 主模型请求 | MCP 审批 | 实际 MCP 调用 |
| --- | --- | ---: | --- | ---: |
| 无挂载 | `ses_e06b7bf9effebW4L4uBqv3G004` | 3 | 无 | 0 |
| 挂载 | `ses_e06b7acbbffe1IoZHdWxXxl192` | 6 | 单次批准 | 1 |
| 撤除 | `ses_e06b7967bffegt9Zen2w3yMwNw` | 3 | 无 | 0 |
| 拒绝 | `ses_e06b783bbffeTukRUzD7VL1INQ` | 5 | 单次拒绝 | 0 |

挂载轮的原生正文与附件调用为 `call_4b26486d13845d1a9ec7c1b0`、`call_2fe11c0db3287b71be6f1044`，选中正文与附件为 `call_bd812e4e14d6d9301961e121`、`call_1c32ab1d277296f795fc1156`，MCP 为 `call_3251d69b5a42b67c00f846a0`。拒绝调用 `call_e0a481f129f4464e67af89e0` 的 ACP 失败终态和独立 stdio 审计均已核对。网关证据位于 `target/acp-skill-coexistence/mimo-code.json`。

复跑时将 `ACP_MIMO_REVIEWED_DIR` 指向固定 MiMo 可执行文件和构建收据所在目录。从仓库根调用 `scripts/cargo_slot.py`，在同一槽位构建 `cargo build --locked -j 1 -p vw-cli --bin shiju` 并以 `$CARGO_TARGET_DIR/debug/shiju` 运行 `scripts/verify_acp_skill_coexistence.py --agent mimo-code`。构建阶段使用共享 Cargo 锁。

槽位 6 的 `cargo test --locked -j 1 -p llm-runtime --lib acp_skill_coexistence_mimo_code -- --test-threads=1` 退出 0。实际运行 2 项，2 项通过，覆盖缺选时保留配置和只投影受信选集。两个 Cargo 缓存路径均为仓库下 `target/rust-slot-6` 的绝对路径。
