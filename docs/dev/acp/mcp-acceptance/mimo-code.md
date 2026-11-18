# MiMo Code 动态挂载 RustCodeGraph

2026-11-19，验收阻塞于受管 MiMo Code 启动 stdio 服务。宿主成功连接 RustCodeGraph，原生 ACP 执行 `posix_spawn` 时收到 `EPERM`。

## 环境

- Darwin arm64，受管 MiMo Code 0.1.17。复用固定源码及 stdio 补丁构建的安装产物，二进制 SHA-256 为 `c086c1bd5c4ada962880f7636de5b07fc442db19edccf3e383887f6f3c47f7ae`。私有网关报告已安装。独立握手返回 ACP v1，代理自报 `OpenCode` 0.1.17，声明 HTTP、SSE 和会话 load、fork、list、resume。
- 私有回环网关复用 shiju 0.6.0，SHA-256 为 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`。仅在私有实例启用预设，使用租户 `10001 / 100010005`、模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 与 `session_model`。
- [复用请求](../ACP验收.md#可复用的验收请求) 取自提交 `189d22dc1`，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。RueJS 私有副本来自 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c`。RustCodeGraph 1.2.10 索引 3478 个文件并命中 `app/app.tsx:32` 的 `createRueSiteApp`。挂载使用 `ruejs_graph_probe` 原生 stdio，关闭文件监听。

## 真实模型三轮

| 轮次 | 新会话 | HTTP / curl / 秒 | 终态和答复 | 原生工具调用 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efb70598001a1yENBpMDVj4Nq` | 200 / 0 / 8.201515 | `chat.done`，标记及额外说明 | 2 次 `mcp_tool_search` |
| 挂载 | `ses_1efb72755001zrtE5s96hA0iDN` | 200 / 0 / 9.061355 | `chat.done`，标记及额外说明 | 2 次 `mcp_tool_search` |
| 撤除 | `ses_1efb74bed001zBsSMF3rCk45Id` | 200 / 0 / 6.937092 | `chat.done`，标记及前置说明 | 1 次 `mcp_tool_search` |

各轮一条 `chat.done`、零 `chat.error`。三轮均输出 `MCP_NOT_AVAILABLE`，去掉推理标签后仍有额外文字。按调用 ID 去重，SSE 分别记录 2、2、1 次原生工具调用，合计 16、16、8 条更新，完成标题均为 `No deferred MCP tools available`。三种目标工具 `rustcodegraph_query`、`rustcodegraph_callers`、`rustcodegraph_node` 在各轮均零调用、零结果。挂载轮未回答调用链、行号和静态快照验证，两个对照也未达成仅输出标记且零工具事件的要求。

三轮运行中及终态的 UI summary 回读均为 `acp-mimo-code / mimo-code`、`full_access=true`，ACP options 均回读指定提供商、模型及 `session_model`。运行中 `connected=true`、`access_status.support=client_only`，终态连接释放。此次只有目录检索，未触发写入或终端审批。标题生成另有不同模型的 HTTP 403。

## 失败边界与收尾

挂载轮宿主成功完成 stdio MCP 初始化。独立探针复用同一原生入口、`--pure`、私有 `MIMOCODE_CONFIG` 和受管沙箱的进程及网络约束，将同一服务定义传入 `session/new`。MiMo 日志记录该服务为 `local`，尝试启动 RustCodeGraph 时失败，随后仍返回会话 ID。用私有回环模型核对，挂载和空目录对照收到的主提示工具定义相同，均为 `read`、`edit`、`write`、`mcp_tool_search` 和 `mcp_tool_use`，两轮均以 `end_turn` 结束。回环探针只用于定位工具发现边界。

现有 `MIMOCODE_CONFIG` 继续使用私有模型配置，项目配置和外部兼容配置入口继续隔离。受管沙箱只允许执行 MiMo 入口，出站限定本次模型中继端口。MiMo 声明远程 MCP 能力，但已装 RustCodeGraph 只提供 stdio，仓库的 stdio 回环桥属于测试夹具，现有中继只支持模型协议。固定版补丁仅将 MiMo 内部 SDK 请求留在进程内。复跑需要可保留会话、租户、审批、撤销和进程生命周期的宿主兼容入口。

生产代码、安装契约和权限策略未修改。生命周期夹具未运行，原生探针已定位到真实进程启动失败。本次未运行 Cargo。私有网关和原生探针均退出，端口关闭，挂载已撤除，私有配置、凭据和数据库副本已清理。原 RueJS 工作区未写入，现有用户网关的 MiMo 预设仍停用。原始 SSE 和诊断保存在本机私有目录。
