# OpenClaw 2026.10.10 技能共存验收

2026-11-20 在 Darwin arm64 验证受管修订 `openclaw-2026.10.10-model-source-v2`，入口版本为 `OpenClaw 2026.10.10 (7dc3350)`。本机只有 v1，故在 0700 私有目录复制该制品，校验并还原扫描器的上游字节，再应用仓库现行 v2 补丁。探针校验包名、版本、npm 入口及合约的三个 SHA-256。

| 文件 | SHA-256 |
| --- | --- |
| `openclaw.mjs` | `469e60c9f60b615ff71c985a54238ee532a8f949866d76aebe7d6e4bc7219902` |
| `dist/workspace-skill-sources-DqAcMM_z.mjs` | `db073f36c9ac2d94a9a51b5076bbe6e3ae780c67ec820e81ef000bef1bb83459` |
| `dist/mcp-stdio-transport-Dlawzoy1.mjs` | `5bc81023917314bfba0ab85f6987909845a44ae43cfbde2e60038a71bf100b35` |

## 私有 ShiJu 网关

从槽位 3 复制已构建的 `shiju`，SHA-256 为 `403bb2827df94239bc831633f3cd3a1d10dd832594fe9d8471cc1c8bde2ebacf`。探针持有槽位 9，构建与目标路径均为仓库内的 `target/rust-slot-9`。请求指定 `session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017` 和 `full_access=false`。挂载和拒绝对照通过 `defaults.skills` 及工作区 MCP 配置选取夹具，使用受审的只读 stdio 传输。

| 轮次 | 会话 | `chat.done` | `chat.error` | 模型请求 | 下游 MCP |
| --- | --- | ---: | ---: | ---: | ---: |
| 基线 | `ses_e079b1435ffep4x6rc1bQ7kCQ3` | 0 | 1 | 0 | 0 |
| 挂载 | `ses_e079a6730ffeSujIOnqttA18pK` | 0 | 1 | 0 | 0 |
| 撤除 | `ses_e079a0c25ffer55BKdpquhotWY` | 0 | 1 | 0 | 0 |
| 拒绝对照 | `ses_e0799c4b0ffeDWmeh25yQA7Pjn` | 0 | 1 | 0 | 0 |

标准验收在基线退出 1。其余三种配置各使用一个新网关和会话单跑，均退出 1。首个断点均为 `requires_full_access`，由 `ensure_desktop_native_access` 在供应商启动前返回。拒绝对照也止于启动门禁，未到达逐次审批。模型绑定、权限状态回读和调用 ID 均未取得。结果保存在 `target/acp-skill-coexistence/openclaw.json` 及同目录的 `openclaw-{mounted,removed,rejected}.json`。

## 供应商加载器对照

在同一私有 v2 副本中，用原生异步运行时加载器单独扫描临时工作区。未设会话根时发现工作区原生技能，空根时目录为空，投影会话根时只发现选中技能。非选中技能始终不可见。直接调用同步目录读取接口时，投影软链接未被接收。异步路径会传递补丁设置的目标白名单，加载结果见 `target/acp-skill-coexistence/openclaw-scanner.json`。这组诊断验证了目录差异，网关中的正文和附件回读仍无实测结果。

恢复本项需要兼容现有审批约束的原生工具控制及 MCP 拒绝路径，以及可与会话视图合并的受信原生根。当前私有 Gateway 的 MCP 投影依赖完全访问，ACP 桥没有逐次 MCP 审批。现有生产配置、制品合约和 `cli_local` 门禁保留。 [OpenClaw 权限与传输审计](../mcp-policy-audit/3.md#openclaw)

Python 共享验收判定的 5 项定向测试通过。复用已有网关，未重新构建或运行 Rust 单测。

## 续查受审替代路径

在当前共享代码中复核了两处宿主权限门禁。请求绑定桌面权限后、技能视图准备前检查当前完全访问授权，ACP 启动参数生成时还会检查实际审批模式。OpenClaw 驱动随后独立校验 MCP 投影权限。共享工作区后续变更接通了 OMP、siGit 等适配器的受限启动，OpenClaw 的这三处检查和固定补丁均未变化。先前四轮结果由此可以定位为权限断点，复用已有报告。 [请求入口](../../../../crates/libs/llm-runtime/src/acp/request.rs#L185) · [ACP 进程入口](../../../../crates/libs/acp-client/src/client/process.rs#L383) · [MCP 投影](../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/openclaw.rs#L172)

用固定版原生异步加载器分别观测目录扫描和提示资格。在 0700 临时树放置工作区原生技能、显式禁用技能、其它项目根、私有管理根及链接投影。设置 `plugins.enabled=false`、`skills.allowBundled=[]` 和显式禁用项，结果如下。结构化证据保存在 `target/acp-skill-coexistence/openclaw-native-config.json`。

| 配置 | 进入提示的夹具技能 |
| --- | --- |
| 当前空会话根 | 无 |
| 当前投影会话根 | 选中技能 |
| 原生 `skills.load.extraDirs` 追加会话视图 | 原生技能、选中技能、其它项目根和私有管理根 |
| 原生 `workspaceOnly` 加同一 `extraDirs` | 原生技能 |

显式禁用和内置技能过滤在提示资格阶段生效。前一追加路径仍纳入两个未单独授权的根，后一条丢失会话视图。生产接入需要按来源限制原生根，并仅为经宿主校验的会话视图登记链接目标。

固定制品 `dist/server-D2T0Amp5.mjs` 的 `assertSupportedSessionSetup` 拒绝非空 MCP 列表。桥的 Gateway 事件处理只将 `exec.approval.requested` 转成 ACP 权限请求，命令执行的批准绑定到活动会话和命令调用。制品里的 Codex harness 审批及 MCP App 审批属于独立入口。历史同版私有 Gateway 的普通模型工具实测已显示 MCP 结果进入续轮，ACP 审批回调为零。OMP 的受审宿主审批绑定其自有沙箱启动器与 MCP 代理，可用来参考调用约束，但接在 OpenClaw 的 stdio 之后也无法代替对独占 Gateway 原生文件、命令和网络能力的约束。 [固定版回执](../mcp-acceptance/openclaw.md#私有-openclaw-网关探针) · [OMP 受限审批](../mcp-acceptance/omp.md)

三轮和逐次否决仍需要新的受审 OpenClaw 启动路径或固定制品。该路径须同时约束独占 Gateway 的原生 I/O，在普通模型 MCP 调用之前把同一会话、工具和参数交给宿主审批，并证明拒绝和取消均不会触达下游。单独授予完全访问只能进入现有投影，无法验证这项拒绝合同。

## 固定扫描器与受限 ACP 候选验证

2026-11-21，在 Darwin arm64 将候选补丁应用到固定 2026.10.10 的私有副本。原扫描器 SHA-256 为 `db073f36c9ac2d94a9a51b5076bbe6e3ae780c67ec820e81ef000bef1bb83459`，候选扫描器为 `d2320bcfae0b999e541b642155d510ade08d464bec5dc233d7ef5e4c785bd671`。补丁核对唯一匹配点和前后摘要，仅在工作区与宿主绑定一致时追加该工作区的物理 `skills` 目录。空选集保留这个原生根。挂载轮同时保留会话视图，原生过滤和优先级沿用供应商实现。 [固定候选补丁](../../../../patches/acp-model-source/openclaw/coexistence-candidate.json)

在同一运行配置下，Darwin 沙箱限制 Gateway 和 ACP 桥的读写、执行及网络范围。Gateway 启动依赖同一个 Node 入口的 SQLite 只读检查子进程，禁止启动其它程序。独立权限对照观测到私有目录读取、项目写入、非授权执行及非授权环回端口连接均被拒绝。工具同时使用精确的读取和当轮 MCP 目录，以及原生 `tools.fs.workspaceOnly`。四轮均通过真实 native `read` 调用验证 Gateway 私有状态中的测试文件未泄露，同时能读回已注册会话技能的外部附件。沙箱对照见 `target/acp-skill-coexistence/openclaw-candidate-sandbox.json`。

挂载轮经 OpenClaw 原生 Streamable HTTP 进入现有 OMP 宿主审批中继，再由 Cortex 受限 stdio 执行器调用只读服务。独立对照由测试客户端回答中继真实生成的、绑定到 ACP 会话及精确参数的权限请求。拒绝决定进入模型续轮，下游计数为零。

| 轮次 | ACP 终态 | 模型请求 | 逐次决定 | 下游 MCP |
| --- | --- | ---: | --- | ---: |
| 基线 | `end_turn` | 4 | 无 | 0 |
| 挂载 | `end_turn` | 7 | `allow_once` | 1 |
| 撤除 | `end_turn` | 4 | 无 | 0 |
| 拒绝 | `end_turn` | 7 | `reject_once` | 0 |

四个 ACP 会话、原生正文和附件、选中正文和附件、越界拒绝、MCP 回复均按调用 ID 校验。运行退出 0，Python 共享验收判定的 5 项定向测试通过。结构化结果在 `target/acp-skill-coexistence/openclaw-candidate-contained.json`。

```sh
PYTHONPATH=scripts python3 scripts/acp_skill_coexistence/openclaw_candidate.py --with-mcp \
  --output target/acp-skill-coexistence/openclaw-candidate-contained.json
```

候选诊断直接连到私有 OpenClaw ACP 桥。生产安装仍指向 v2，ShiJu 网关的权限回读、`chat.done` 和 `chat.error` 尚未取得新证据。本次未构建 Rust。剩余生产接线需要让 ShiJu 持有 OpenClaw Gateway、ACP 桥和 MCP 中继的共同生命周期，在配置 Gateway 前绑定只读服务，并把权限请求交到真实桌面审批通道。只有固定入口、隔离及审批接线联验通过后才可允许受限桌面请求走该路径。旧的完全访问与 CLI 本地门禁继续生效。

## 受管 v3 生产接线

2026-11-21，登记固定 v3 扫描器与启动器。Darwin arm64 的受限桌面请求必须使用受管 `session_model`、交互式 `ApproveReads`、当前桌面工作区和核验通过的制品。宿主为请求创建私有目录及模型中继。启动器共管独占 Gateway、原生 ACP 桥和一次性 MCP 会话，继续使用 v2 stdio 回收补丁的制品合约。较早版本仍沿原方式启动，受限分支要求 v3。

MCP 只接受会话选中的 stdio 服务，沿用宿主 allow/deny 过滤与已有 Cortex 受限执行器。宿主在 ACP 发起 `session/new` 前登记只读投影，网关只看到随机私有地址和允许的工具名。调用通过 ACP `session/request_permission` 返回绑定当前会话的一次性决定。取消撤销待审批调用，请求结束使桥、网关与下游连接退出。原生工具限定为读，工作区文件读取承袭供应商路径守卫及当前会话技能的附件范围。受限路径每个请求分配私有 Gateway 与原生会话，使用宿主提示重放；原有授权和 `cli_local` 门禁沿用。

直接运行生产启动器的真实 ACP 诊断退出 0，四轮下游调用数 0、1、0、0，模型请求数 4、7、4、7。批准及拒绝均经过 ACP JSON-RPC 交互，四轮均拒绝读取 Gateway 私有状态。另在待审批时取消，得到 `cancelled`、下游零调用、进程退出 0、Gateway 端口关闭。诊断结果在 `target/acp-skill-coexistence/openclaw-production-launcher.json` 和 `target/acp-skill-coexistence/openclaw-production-cancel.json`。共享 MCP 中继的 4 项定向测试与共存判定的 5 项定向测试均通过。此阶段的诊断用 ACP 客户端代替桌面交互，ShiJu 完整网关回归证据单独记录。

额外用真实生产启动器验证会话边界。初次请求给出其它工作区时返回错误，当前工作区可正常建会话，关闭后再次建会话被拒绝。结果见 `target/acp-skill-coexistence/openclaw-production-isolation.json`。同一启动器只能绑定一次原生会话，MCP 审批中继的生命周期与该会话一致。

### ShiJu 网关四轮

用共享槽位 10 已构建的网关副本运行标准验收，探针持有槽位 6，目标和构建目录同为该槽位。网关 SHA-256 为 `ff5da09342e645242942bc3208c4e579e417114c66e63b893e7fd256d00b79ca`。四次新会话各产生一个 `chat.done`、零个 `chat.error`。对照次数为 0、1、0、0，分别观测到挂载的 `once` 与拒绝对照的 `reject`。模型请求数为 3、6、3、6。每轮回读 OpenClaw 2026.10.10、`session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017`、`full_access=false`。报告包含四个会话与原生、选中技能及 MCP 的相关调用 ID，标准验收退出 0。结果见 `target/acp-skill-coexistence/openclaw-shared-build.json`。

共享槽位构建与本项末次单会话加固时间交叠。该加固另用生产启动器通过跨工作区与二次绑定的拒绝测试。定向 Rust 测试复用最新已链接的同一 lib-test 目标，结果如下。

私有目录内由仓库正式安装器从固定 npm 制品重新安装 v3，退出 0，入口报告 `OpenClaw 2026.10.10 (7dc3350)`，三个合约摘要一致。在同一共享网关副本上用该安装结果重跑标准探针，四轮再次通过，审批与下游计数一致。安装与验收报告分别为 `target/acp-skill-coexistence/openclaw-install-v3.json` 及 `target/acp-skill-coexistence/openclaw-official-install.json`。

包含单会话加固的槽位 3 网关已编译。持有槽位 3 使用其独立快照和正式安装产物完成最新标准网关复验，退出 0。目标与构建目录均为 `target/rust-slot-3`，网关 SHA-256 为 `54089fcae85172aeca2459440eae5fbb8c041c774d64ff9b045b669219044e19`。四轮终态与 MCP 计数保持 1/0 和 0/1/0/0，模型、权限及调用 ID 详见 `target/acp-skill-coexistence/openclaw.json`。

在同一个已绑定桌面会话中追加两轮，复用原模型及固定技能选集。首轮批准、次轮拒绝，均有 `chat.done`，MCP 调用计数为 1、0，确认批准不跨轮复用。结果见 `target/acp-skill-coexistence/openclaw-same-binding.json`。另行改变全局默认值时，既有会话仍沿用首次请求固定的服务端选集。标准撤除对照依计划使用新会话。

### 定向 Rust 测试

复用统一编译锁下槽位 10 产出的 `llm-runtime --lib` 测试二进制 `llm_runtime-85f23fc8970681f8`，构建与目标路径均为仓库内的 `target/rust-slot-10`。复制产物并使用 `acp_skill_coexistence_openclaw --test-threads=1` 运行，退出 0，1 项通过，549 项被过滤。结果保存在 `target/acp-skill-coexistence/openclaw-focused-reused.json`。同目标的槽位 9 重复编译随后取消，退出 130，槽位由原命令释放。
