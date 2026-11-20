# Poolside 原生技能与会话技能共存验收

2026-11-20，Poolside 1.0.16 v2 的受管技能路径验收阻塞。无挂载普通读取完成。挂载时，宿主在模型推理前返回 `poolside: no verified skill directory mount branch`。受管启动器的私有配置还显式禁用了 `skill`，无挂载请求中没有原生技能目录或激活工具。[宿主门禁](../../../../crates/libs/llm-runtime/src/acp/skill_mounts/mod.rs) · [受管启动器](../../../../patches/acp-model-source/poolside/launcher.py)

## 制品与入口

- Darwin arm64，Docker 29.9.2。复用缓存的受管目录，四项文件均按安装合约核对，`pool --version` 回读 1.0.16。原生程序 SHA-256 为 `5dcb0162fd21a535e4f67117125e7d205e27a6881862aa223d0424082848f874`，启动器为 `9f4a7ac15e4d7d5804ad90ed326183e100e071b8ba977f37603893c4bb29145c`。另核对固定发行归档 SHA-256 `0932af3eb2b57a863acacb664ec8b2b1d3a76c2570a788b086001608cc585f74`。[安装合约](../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json)
- 独立诊断用同摘要原版 `pool-native acp --sandbox required` 和私有设置。首个 Chat Completions 请求声明 `skill(name)`，列出了 `.agents/skills` 和 `.poolside/skills` 下的随机技能名，未包含正文或附件。调用 `diagnostic-skill` 后，模型续轮按原调用 ID 收到 `.agents/skills` 的正文，原生提示以 `end_turn` 结束。另将 `.poolside/skills` 放入受管工作区，启动器在 ACP 握手前以项目配置覆盖错误退出 1。[Poolside 1.0.16 的技能说明](https://github.com/poolsideai/pool/blob/v1.0.16/README.md#spec-support)

## 网关结果

探针启动实际网关、受管 ACP 和 Docker，供应商模型响应由回环夹具提供。通过 `defaults.skills` 和 `defaults.mcp` 选集，配置现有 stdio 服务，由受管启动器负责原有 HTTP 转接。普通读取使用私有技能夹具的附件路径，按调用 ID 校验模型续轮中的随机值。四轮都使用独立会话。无挂载与撤除回读 `poolside`、`coexistence-fixture/selected-model`、`session_model`、`full_access=false` 和 `access_status.requested=false`，访问状态的 `support` 为 `unavailable`。

| 轮次 | `chat.done` / `chat.error` | 模型请求 | 普通读取调用 ID | 下游 MCP |
| --- | --- | --- | --- | --- |
| 无挂载 | 1 / 0 | 2 | `call_b293506f5c594cde196ce85b` | 0 |
| 挂载 | 0 / 1 | 0 | 无 | 0 |
| 撤除 | 1 / 0 | 2 | `call_a8d3f8273ea63b1a01184830` | 0 |
| 拒绝对照 | 0 / 1 | 0 | 无 | 0 |

探针主入口在无挂载轮完成普通读取后，因受管原生技能目录和激活工具缺失退出 1。其余轮次逐次调用同一运行器独立诊断，均退出 1。拒绝对照停在技能门禁，未到 MCP 工具审批。未取得选中技能的发现或读取、已批准 MCP 调用和撤权验收证据。历史受管 MCP 验收覆盖 HTTP 桥及单次审批。[历史审批与撤权回执](../mcp-policy-audit/coexistence/poolside.md#审批恢复与入口边界)

约定的新构建进入槽位 3 后等待共享 Cargo 锁，确认证据指向受管原生入口后取消排队，入口退出 130，未开始本次编译。随后在重新领取的槽位 3 复用现有网关，两个 Cargo 缓存变量均指向 `target/rust-slot-3`，网关 SHA-256 为 `808b01432ed6f30664f849b0130a97a320b952d395aa34c6723e6ec19be2edf8`。本次没有 Rust 实现改动，未运行 Rust 单测或桌面构建。结构化证据位于 `target/acp-skill-coexistence/poolside.json` 和同目录的按阶段报告。探针退出时按私有工作区精确匹配清理沙箱容器。

继续接入需要经审查的原生技能注册入口，明确只读会话视图的扫描范围并验证激活和撤销，随后复跑三轮及 MCP 拒绝对照。

## 2026-11-21 受管 v3 实现与定向复验

固定原版仍为 1.0.16，原生二进制、归档、原有 MCP HTTP 桥和 stdio 模块摘要沿用 v2。受管启动器 v3 SHA-256 为 `7aacfe58ead97965dc8ccff880c22c59c16a0327ba2a973b3712bc9ba90516e6`，在 Darwin arm64 的实际 Docker 29.9.2 上验证。

Poolside 在宿主发现及激活技能，在 Docker 内读取附件。单独放入私有 `.agents/skills` 或 `.config/poolside/skills` 可以发现并激活，但附件读取失败。追加私有目录的只读 Docker 绑定后，原生 `read` 才能返回附件。原版扫描器可通过工作区技能符号链接读取路径规则拒绝的区外正文，而且该激活没有 ACP 审批回调。v3 给宿主原生进程加文件访问隔离，实测挡住技能目录和 `SKILL.md` 文件两种区外链接。相同技能名按供应商顺序优先使用工作区原生技能，诊断中目录和正文均来自工作区副本。

宿主仅为已核摘要的 session-model 启动器传递受信技能视图。启动器物化该视图中的普通文件至进程私有 `.agents/skills`，拒绝内嵌符号链接，并只读绑定至 Docker。缺选时保留工作区原生技能且不创建技能绑定。拒绝全部或禁用工具时保持原技能禁用，选择技能则拒绝请求。原有 MCP 单次审批及工作区暂存写入审查继续生效。[v3 启动器](../../../../patches/acp-model-source/poolside/launcher.py) · [宿主接线](../../../../crates/libs/llm-runtime/src/acp/skill_mounts/poolside.rs)

| 直连 v3 ACP | 原生激活 / 附件 | 选中激活 / 附件 | MCP 调用 ID | 下游调用 / 终态 |
| --- | --- | --- | --- | --- |
| 无挂载 | `call_0c0b36b06b898b6f3a14cd6d` / `call_7b844d033f02218a5e7aa26b` | 无 | 无 | 0 / `end_turn` |
| 挂载 | `call_6e0c6864f93bdf92b52c6aea` / `call_e30750910ff180833ca7f025` | `call_0e93a116a7c74fafec3a21de` / `call_668e4db6096af6b87d69d7ad` | `call_f20138f55a95494c1fb85e61` | 1 / `end_turn` |
| 撤除 | `call_3cab299f7866088ea4b11ab9` / `call_33c87a47bca64c7a745376dc` | 无 | 无 | 0 / `end_turn` |
| 拒绝对照 | `call_02a4ce7c49b62aa0e0977673` / `call_3c5806d57019c5622b079691` | `call_8d76cd96f2585bbcd7f869cc` / `call_77a1beda1cd82176fafd1b07` | `call_c7c59a0aae4d1c2e02c105b7` | 0 / `end_turn` |

四个独立 ACP 进程通过 `session/set_config_option` 选择回环模型，均退出 0。批准使用 `allow_once`，续轮按原 ID 收到随机 MCP 值。拒绝使用 `reject_once`，同 ID 的工具终态为 `failed`，原生返回 `Permission denied` 后结束回合。定向 Python 测试通过 5 项。

暂存写入补测分别使用 `call_27e9262388ef9f9577cd7c10` 和 `call_0b9da396f02451b70eea0d18`，两轮都在收到宿主写回调前确认目标文件不存在。批准后宿主写入私有工作区，原生为 `end_turn`。拒绝后文件保持不存在，启动器返回 `-32000`。定向 `llm-runtime` 测试在槽位 9 通过 1 项，退出 0。两个 Cargo 缓存变量均为 `target/rust-slot-9`。

以最终 v3 摘要再次直连挂载轮，在容器运行期间通过 Docker 检查确认投影目录对应的绑定只有一项且 `RW=false`。该轮完整返回双方技能及 MCP 结果，MCP 调用 ID 为 `call_6ef7af7bae607dbdf4e9c0d6`。

## 生产网关验收

槽位 6 的网关构建退出 0，两个 Cargo 缓存变量均为 `target/rust-slot-6`。
网关 SHA-256 为 `a3670c105e91e61ebbc9b310fa14b6fc8ef5f5cda368850645b1005b59c7f2f9`。复用同一构建运行最终探针，退出 0。
四轮仅以回环夹具替换模型响应，通过网关的 `defaults.skills` 与 `defaults.mcp` 选集。
均回读 `poolside`、`session_model`、`coexistence-fixture/selected-model` 与 `full_access=false`。
每轮有一个 `chat.done` 和零个 `chat.error`。

| 轮次 | 独立会话 | 原生激活 / 附件 | 选中激活 / 附件 | MCP 决策及调用 ID | 下游调用 |
| --- | --- | --- | --- | --- | --- |
| 无挂载 | `ses_e06123479ffeCQ81JjAyMYklrv` | `call_deb4b54defcd09bad86d25d3` / `call_c7b5fda8b63e75a611b0842a` | 无 | 无 | 0 |
| 挂载 | `ses_e0611f9d7ffePBQUmb7wvo9snm` | `call_c3b2222d3d21278f6112e385` / `call_8e9c80c997963568af04fe79` | `call_600ab4b2d2655c36c789faf1` / `call_f33ad4bedfca47d9b37497e9` | `once` / `call_e18c65ebaee26a0babfc506f` | 1 |
| 撤除 | `ses_e0611bd50ffeA3SZRQ14Rv9qdR` | `call_d308c84b88a8096ee779c800` / `call_e5784add0fc48ce64f35320e` | 无 | 无 | 0 |
| 拒绝对照 | `ses_e06118c04ffebo5C1gkr3RX4Rv` | `call_b8167f1e4913032d82410b38` / `call_8865ce7ee50cb952d3db24df` | `call_1199c2553b774c2f43384283` / `call_bcc8cacc6a3d74bb9edb72ad` | `reject` / `call_d4a67e6d6873759b3fa2585e` | 0 |

第一次网关运行中，Poolside 在拒绝后直接结束，夹具没有发出助手文本，网关将该轮判为空响应。
复验仅调整模型夹具，使其在请求 MCP 时先返回简短说明。拒绝的同 ID 工具终态仍为失败，下游调用为零。
网关退出后确认无属于本探针的 Docker 容器。结构化证据见 `target/acp-skill-coexistence/poolside.json`。
