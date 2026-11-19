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
