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
