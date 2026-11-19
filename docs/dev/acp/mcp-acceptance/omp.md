# OMP 动态挂载 RustCodeGraph

2026-11-19，指定模型三轮完成。挂载轮阻塞在受管 OMP 启动 stdio 服务，macOS 拒绝子进程创建。

## 环境

- Darwin arm64，复用固定发行的 OMP 18.22.0 独立入口 `omp acp`，SHA-256 为 `d185113d63eba23925e5d70ab661a424b827c06c98f8e766c3bf4ad62ddaff9a`，与受管清单一致。网关回读确认此次会话的受管版本。
- 私有回环网关复用 shiju 0.6.0，SHA-256 为 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`。仅在该实例启用 `acp-omp`。三轮固定租户 `10001 / 100010005`、`session_model` 和 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。
- [固定验收请求](../ACP验收.md#可复用的验收请求) 取自提交 `189d22dc1`，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。RueJS 私有副本来自 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c`。RustCodeGraph 1.2.10 索引 3478 个文件，命中 `app/app.tsx:32` 的 `createRueSiteApp`。挂载名称为 `ruejs_graph_probe`，使用原生 stdio 并关闭文件监听。

## 指定模型三轮

| 轮次 | 新会话 | HTTP / curl / 秒 | 终态与答复 | 工具调用 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efbaaa54001OOnwLp5J4V4RWO` | 200 / 0 / 9.065747 | `chat.done`，最终正文 `MCP_NOT_AVAILABLE` | 0 |
| 挂载 | `ses_1efbacee7001TbzMS9mNrf108Q` | 200 / 0 / 9.105412 | `chat.done`，先输出服务启动失败，再输出 `MCP_NOT_AVAILABLE` | 0 |
| 撤除 | `ses_1efbaf34c001bHHkxfyVe72D0a` | 200 / 0 / 8.798236 | `chat.done`，最终正文 `MCP_NOT_AVAILABLE` | 0 |

各轮一条 `chat.done`、零 `chat.error`。两个对照的标记以移除模型的 `<think>` 区块后的正文为准。三轮均无 `rustcodegraph_query`、`rustcodegraph_callers` 或 `rustcodegraph_node` 的调用或输出。挂载轮没有调查答复，因此无法核对调用链、行号及试验场景。挂载错误通过普通 `chat.delta` 发送，终态不能作为挂载成功证据。

各轮运行中及终态的 UI summary 确认 `acp-omp / omp` 和 `full_access=true`，ACP options 回读指定选模来源、提供商及完整模型 ID。运行中 `connected=true`、`access_status.support=client_only`，结束后 `connected=false`。本次请求未触发文件或终端操作及权限回调。独立标题请求出现另一模型的 HTTP 403。

## 首个失败边界

挂载轮宿主成功完成 RustCodeGraph 的 stdio MCP 初始化。OMP 收到服务定义后尝试启动同一入口，在 `posix_spawn` 处返回 `EPERM: operation not permitted`。受管 OMP 沙箱只允许执行固定 OMP 入口并访问本次模型中继端口，请求的 `full_access=true` 不扩展沙箱权限。

独立协议探针复用受管 OMP、对应沙箱约束和独立的 `PI_CODING_AGENT_DIR`，使用占位凭据和私有回环模型。该目录中的 `models.yml` 只含探针模型。OMP 自报 18.22.0、ACP v1、`loadSession=true`、支持 HTTP 和 SSE MCP。三次独立 `session/new` 分别传入空、stdio 服务、空，均返回不同原生会话，三次短提示均以 `end_turn` 结束。回环模型三次都只收到 `read`、`write`、`edit`，确认在模型工具声明前已失去挂载。这些证据来自 OMP 独立入口及私有目录。

已装 RustCodeGraph 仅提供 stdio。仓库可定位的 stdio 到回环桥属于测试夹具，现有 OMP 回环只承载模型协议。复跑需要可复用的宿主兼容入口，并保留会话、租户、权限、撤销和进程生命周期约束。

本次提交仅包含验收记录。依据真实三轮及独立探针定位边界，未运行 Cargo 生命周期夹具。私有网关和探针已退出，网关端口关闭，挂载已撤除，私有配置与租户数据库已清理。原始证据保存在本机私有目录。

## 共享工作区复验

较新的共享构建 shiju 0.6.0（SHA-256 `6954a2bb76ca26aa39c3b0dd909786c68b85f336648526ef39651b1e3a31f718`）复用了前轮固定 OMP、RueJS 提交、提示及指定模型。仍以私有网关发起三次新会话。

| 轮次 | 新会话 | HTTP / curl / 秒 | 最终正文与工具 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f044c153001ImNMlnlmUqX2ZR` | 200 / 0 / 7.748094 | `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1f044e0cd001YOPxTzF7c0TnmG` | 200 / 0 / 7.087581 | `posix_spawn` 返回 `EPERM`，随后输出 `MCP_NOT_AVAILABLE`，零工具 |
| 撤除 | `ses_1f044fd8e001fZETcs0dnG4nYm` | 200 / 0 / 7.972795 | `MCP_NOT_AVAILABLE`，零工具 |

三轮均有一个 `chat.done`，没有 `chat.error`、`chat.model_recovery` 或目标工具事件。剥离推理标签后，对照正文均准确。运行中 UI summary 与 ACP options 再次确认 `acp-omp / omp`、`full_access=true`、`client_only` 和指定模型绑定。终态连接释放。挂载轮没有调查答复。

### 同端口传输和权限诊断

保留生产进程及出站沙箱，使用同一原生 OMP、私有 `PI_CODING_AGENT_DIR` 和占位密钥。在模型端口提供带独立令牌的私有 Streamable HTTP 诊断路由，转给宿主的真实 RustCodeGraph，仅发布三种目标工具。空目录与不同端口对照没有 MCP 请求抵达。与模型同端口时，OMP 完成 `initialize`、`notifications/initialized` 和 `tools/list`。三个工具出现在系统提示的 `xd://` 目录，直接函数声明仍只有 `read`、`write`、`edit`。

仅在诊断私有配置设 `tools.xdev=false` 后，三种工具以 `mcp__ruejs_graph_probe_` 前缀直接声明。诊断模型依次调用三项，收到真实 RustCodeGraph 结果并续轮至 `end_turn`。探针的 ACP 客户端对权限回调选择取消，但 OMP 没有为这三次调用发出任何 `session/request_permission`。因此生产路径还需要宿主逐次执行原工具审批和权限校验。

| 目标工具 | 诊断调用 ID | 真实结果 |
| --- | --- | --- |
| `rustcodegraph_query` | `omp_probe_0` | `createRueSiteApp` 位于 `app/app.tsx:32` |
| `rustcodegraph_callers` | `omp_probe_1` | `mountRueSiteApp` 位于 `app/app.tsx:39` |
| `rustcodegraph_node` | `omp_probe_2` | `packages/runtime/src/hooks/useApp.ts:4` 调用 `_$createApp(factory)` |

已核对 Cortex Code、DeepAgents 和 OpenClaw 新增的受管入口，分别依赖各自的客户端工具扩展或适配协议。现有模型中继路由仍缺少会话级 MCP 入口。上游 18.22.2 与 18.22.0 的 ACP 代理源码相同，升级不能提供所需的宿主中继或审批。下一步需要公共接线归属确认，将宿主侧会话 MCP 转接接入模型中继端口，并在转接内按会话授权每次工具调用及处理撤销。确认后才能为 OMP 绑定私有目录并用原模型复验。 [OMP 18.22.0 ACP 源码](https://raw.githubusercontent.com/can1357/oh-my-pi/v18.22.0/packages/coding-agent/src/modes/acp/acp-agent.ts) · [OMP 18.22.2 ACP 源码](https://raw.githubusercontent.com/can1357/oh-my-pi/v18.22.2/packages/coding-agent/src/modes/acp/acp-agent.ts)

这次只更新验收记录，未运行 Cargo。诊断和网关的进程与监听已退出，私有配置、模型凭据及租户数据库副本已清理，项目挂载已撤除。原始 SSE 和诊断保存在本机私有目录。

## 宿主审批补丁原型

在同端口诊断转接层加入逐次审批。OMP 原生二进制和进程、网络沙箱沿用原约束。探针在宿主侧绑定当前原生会话的提示回合，转接只发布三种只读工具。每次 `tools/call` 先交独立宿主夹具决定，获批后发往真实 RustCodeGraph 前再次检查回合仍有效。模型夹具与审批夹具独立，转接使用单独的诊断令牌。

| 场景 | 宿主夹具决策 | RustCodeGraph 实际调用 | 原生终态 |
| --- | --- | --- | --- |
| 三项逐次批准 | 3 次允许 | 3 次成功，查询、反查调用方及源码结果均回到模型续轮 | `end_turn` |
| 三项逐次拒绝 | 3 次拒绝 | 0 次 | `end_turn` |
| 新会话撤除挂载 | 0 次 | 0 次，仅声明原生工具 | `end_turn` |
| 首项批准后、执行前撤销回合 | 1 次允许后失效 | 0 次 | `end_turn` |

四组均为原生 OMP 18.22.0 退出 0，诊断监听已关闭。撤销组测试的是转接层的执行前检查，没有覆盖生产 `session/cancel` 或在途工具终止。原型使用本机私有回环模型和审批夹具，既有生产入口未接入该原型。生产接线需要将转接绑定到真实 ACP 会话、审批解析及关闭信号，再复验之前的指定模型三轮。未运行 Cargo。

## 受管入口接线

受管 OMP 有显式挂载时，ShiJu 将原生 stdio 描述交给受审启动器。转接器在同一个新建回环端口提供固定模型路由及每个 MCP 挂载的独立能力路径。原生沙箱只允许这个端口，原生程序仍为固定发行物。MCP 子进程复用 Cortex 的只读工作区沙箱，使用私有 HOME，禁止网络与派生进程。会话选中的工具先按宿主允许和拒绝规则过滤，再提供给 OMP；每次调用通过当前会话 ACP `session/request_permission` 请求一次授权。关闭、替换挂载或取消会撤销审批并清理工具进程。无挂载时保留原启动流程。

运行 `PYTHONDONTWRITEBYTECODE=1 python3 patches/acp-model-source/omp/mcp_bridge_tests.py`，四项通过。用固定 OMP、真实 RustCodeGraph 1.2.10 及回环模型在私有 RueJS 副本复验，保持只读沙箱并由宿主打开该副本索引以保留 SQLite WAL/SHM。旧入口零个目标工具，更新入口获得以下结果。库内 Python 用例和原生探针均已实际运行。

| 原生探针 | 宿主审批 RPC | 工具结果 | 终态 |
| --- | --- | --- | --- |
| 逐次批准 | 3 | 查询、反查和节点各完成一次 | `end_turn` |
| 逐次拒绝 | 3 | 三次拒绝，无完成 | `end_turn` |
| 撤除挂载 | 0 | 零工具事件 | `end_turn` |
| 取消后送达旧审批 | 1 | 调用取消，无完成 | `cancelled` |
| 拒绝优先 | 2 | 反查不声明，其他两项完成 | `end_turn` |

Rust 定向测试和新版网关指定模型三轮仍待复验。

同一原生会话在批准轮后以空 `mcpServers` 执行 `session/resume`。第二个提示以 `end_turn` 结束，模型只收到 `read`、`write`、`edit`，没有新增审批或图谱结果。受管挂载模式拒绝 fork，避免复制现有会话的工具能力。

### 受管入口与指定真实模型

使用固定 OMP、上述受管启动器、RustCodeGraph 1.2.11（SHA-256 `5fab2f2dbf506ddf71d842ef1a7cd6e744c5ec90ff3f5f043ea020e1b3696312`）和同一提示进行原生三轮。私有 RueJS 索引覆盖 3478 个文件。模型转接固定同一阿里模型，审批由独立原生探针宿主逐次回应。

| 轮次 | 原生会话 | 上游 HTTP | 原生终态 | 工具 |
| --- | --- | --- | --- | --- |
| 无挂载 | `01a1f170-cbf2-768f-aa9d-9d582783d8cf` | 200 × 1 | `end_turn` | 零调用，精确输出 `MCP_NOT_AVAILABLE` |
| 挂载 | `01a1f170-ddb9-71f9-8990-6f56709061a2` | 200 × 36 | `max_tokens` | 89 次完成，两次拟调用在参数被截断时未执行 |
| 撤除 | `01a1f174-d505-72f0-9e45-264ee5d3bd6d` | 200 × 1 | `end_turn` | 零调用，精确输出 `MCP_NOT_AVAILABLE` |

挂载轮逐次批准了 45 次 query、5 次 callers、39 次 node，三种工具均把真实结果交回模型续轮。答复追踪了调用链、浏览器守卫、容器和插件，达到输出上限时仍缺快照验证场景，因此严格合同未通过。这组原生请求没有 ShiJu HTTP SSE、UI summary 或 ACP options，仍需在新网关复验。

补充原生组将 `thinking` 通过 ACP 显式设置（原生回读为 `low`），沿用同一模型与提示。三轮均为 `end_turn`。挂载轮 `01a1f17b-02c1-72d1-bd4c-3ccab49cd6c2` 在 33 次上游 HTTP 200 中完成 44 次 query、8 次 callers、44 次 node，零失败。答复补齐完整调用链、分支和两项快照验证设想，并指出布局绕过路径的注册位置仍无证据。浏览器和 jsdom 场景均为设想，未实际运行。其中卸载场景需为测试取得 `site` 实例。两个对照均零工具，无挂载为准确标记，撤除为标记及额外说明。低档补测只用了私有会话，生产默认档位沿用。

网关回归状态为待回归。已有共享网关二进制缺少 OMP 启动器及 MCP 策略标识。同批槽位 6 的共享构建因并行 MiMo 宿主 trait 签名不一致而以 E0050 退出 101。后续共享重型 Rust 构建持续运行，串行等待没有获得执行窗口。此次没有启动 OMP Cargo 或新版隔离网关，未占用 Cargo 槽位。 [共享构建记录](poolside.md#受管启动器补丁)

复验先运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-acp --test onboarding_omp acp_onboarding_omp_session_lifecycle`，再使用包含补丁的新网关按原合同跑三轮。跟随 `2b35013ae` 仅去除已弃用的请求字段 `acp_agent`，通过 `agent=acp-omp` 解析原后端。原生探针与其监听均已退出，私有模型配置已清理。

## 正式网关验收通过

按更新后的标准，OMP 原生工具沿用既有权限，验收以真实挂载工具执行、结果回传及撤除隔离判定。使用共享槽位 6 构建的 shiju 0.6.0，SHA-256 `109066ea0b79ce6edfc4399624715f0dafa80d397adabe7a1ae46158407e7cba`，确认产物包含 OMP 启动器、私有目录校验及宿主工具策略。固定 OMP 18.22.0。RustCodeGraph 1.2.11 自同一 RueJS 提交索引 3478 个文件，使用等价 stdio 描述和现有 `allowedTools` 配置选择三项目标工具。受管入口将其转接为原生 OMP 支持的私有 HTTP MCP。沿用指定租户、模型和提示，按新 API 移除旧 `acp_agent` 请求字段，从 `agent=acp-omp` 解析同一后端。

| 轮次 | 新会话 | HTTP / curl / 秒 | 终态与工具 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f192019e001z2P5krGNMtr4oH` | 200 / 0 / 10.740795 | `chat.done`，目标工具零调用，原生 `read` 探查 `mcp://` 返回服务未配置 |
| 挂载 | `ses_1f1922de8001EIKcQAElglOzs1` | 200 / 0 / 195.622117 | `chat.done`，80 个成功完成的目标工具调用 |
| 撤除 | `ses_1f1952baa001R4iZwWpjKCYg6r` | 200 / 0 / 7.973995 | `chat.done`，零工具调用 |

挂载轮的 160 条工具事件可按同一调用 ID 配对。接口用模型提供的标题显示 OMP 工具，以下按结构化输入及真实结果确认工具身份。

| 工具 | 成功次数 | 代表性完成事件 |
| --- | --- | --- |
| `rustcodegraph_query` | 36 | `call_0e3e09c686024cf5a71ec978` 输出 `createRueSiteApp`（`app/app.tsx:32`） |
| `rustcodegraph_callers` | 1 | `call_1f2866643b484b4d9cd3a627` 输出 `mountRueSiteApp`（`app/app.tsx:39`） |
| `rustcodegraph_node` | 43 | `call_b6abbff6fc94467e97b86975` 读回 `packages/runtime/src/hooks/useApp.ts:4` 调用 `_$createApp` 的源码 |

两个对照的最终正文均为 `MCP_NOT_AVAILABLE`。挂载答复引用了回传源码，覆盖入口、SSR、容器缺失、插件注册及布局分支，并说明如何验证移除清空操作后静态快照与客户端 DOM 共存。第二个验证场景依赖 `/rust-layers` 可导航，本次源码只确认根组件的路径判断，路由表未直接注册该路径。两个场景均未执行。

三轮均以模型 `stop` 结束，无聊天错误或模型恢复。运行中及终态均回读 `acp-omp / omp`、`full_access=true`、指定模型和 `session_model`，运行中为 `client_only`，终态连接已释放。保留原生工具权限与宿主审批，辅助标题模型错误独立于主请求。网关退出后确认端口关闭、挂载和私有租户配置已清理，RueJS 归档 4109 项逐个比对无差异。四项 OMP 转接 Python 用例复跑通过。Rust 生命周期夹具覆盖通用模拟会话，串行编译队列未启动该夹具，本次以更新后的正式网关及实际 OMP 探针验证受管接线。
