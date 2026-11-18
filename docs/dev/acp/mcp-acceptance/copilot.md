# GitHub Copilot 动态挂载 RustCodeGraph 验收

日期为 2026-11-19。使用已有 `shiju 0.6.0` 网关构建在私有回环实例上验收，二进制 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。实例只复制了指定阿里提供商的配置及宿主授权，并启用了内置 `acp-copilot`。Copilot 使用私有配置目录内的受管发行版 1.0.105，启动参数为 `--acp --stdio --no-auto-update`。核对了现有受管补丁的 `cli-main.js` SHA-256 `1f8978f33a7a54105087ab9e66313a4b973dd852e198ab85b39c3de44959b502`。会话使用独立的 `COPILOT_HOME`，模型经宿主回环中继走 OpenAI Chat Completions，阿里上游凭据只供网关使用。

RueJS 副本取自 `main` 的已提交源码 `14fe287b13549f5183d42fbc45b28e1498ffae1c`。RustCodeGraph 1.2.10 索引了 3478 个文件，包含 `app/app.tsx:32` 的 `createRueSiteApp`、`app/app.tsx:39` 的 `mountRueSiteApp` 及 `packages/runtime/src/compiler-runtime/app.ts:57` 的 `_$createApp`。

## 请求与会话

所有真实轮次沿用 [可复用的验收请求](../ACP验收.md#可复用的验收请求)。提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。每轮执行 `curl -sS -N --max-time 300` 并用同一请求体创建新会话。租户为 `10001 / 100010005`，使用 `acp-copilot / copilot`、`session_model`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 及 `full_access=true`。

| 轮次 | 会话 ID | HTTP / 秒 | 终态与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef7ec8c4001HTGLznumHhlRxJ` | 200 / 7.887 | `chat.done`，`MCP_NOT_AVAILABLE`，0 个工具事件 |
| stdio 挂载 | `ses_1ef7f4a40001Nln4mYGElPH4wc` | 200 / 7.761 | `chat.done`，`MCP_NOT_AVAILABLE`，0 个工具事件 |
| 移除 stdio | `ses_1ef7f68d7001aDQCGOp1ofNEde` | 200 / 7.015 | `chat.done`，`MCP_NOT_AVAILABLE`，0 个工具事件 |

最初的环境探针在启动 Copilot 前触发受管发行校验。将已安装的受审发行复制到私有实例的受管运行目录后通过该校验。表中的三轮均成功启动 Copilot。各轮 `ui/summary` 与 `acp-options` 返回 HTTP 200，确认绑定、完全访问请求、指定选模和发行版本。完成后连接释放。

## stdio 拒绝与 HTTP 路径

stdio 挂载轮的宿主日志确认 RustCodeGraph 初始化成功。使用同一受管入口、隔离的 `COPILOT_HOME`、仅含占位令牌的回环模型和同一 RustCodeGraph 服务重放原生 ACP 握手。ACP v1 宣布 `mcpCapabilities.http=true` 和 `sse=true`。携带 stdio 服务的 `session/new` 返回会话，但原生日志记录 `Rejecting non-http/sse MCP server "ruejs_graph_probe" from client`。随后的 `session/prompt` 以 `end_turn` 结束，回环收到的模型工具表只有 Copilot 内置工具。断点位于原生 ACP 服务接纳，发生在模型工具声明之前。[GitHub Copilot CLI ACP 文档](https://docs.github.com/en/enterprise-cloud@latest/copilot/reference/copilot-cli-reference/acp-server)

HTTP 轮在副本的 `.vwacprc.json` 使用 `type=http` 和一次性 Bearer 请求头，目标为只绑定 `127.0.0.1` 的 Supergateway 4.2.2 Streamable HTTP 端点。桥接进程在私有目录本地安装和启动，通过 stateful 模式把请求交给同一个 RustCodeGraph stdio 程序，只公开提示指定的三项只读工具。未鉴权的探针返回 HTTP 401。模型绑定、提示与访问参数沿用 stdio 轮，Copilot 的原生工具权限保持原设定。[Supergateway 配置文档](https://github.com/supercorp-ai/supergateway/blob/main/README.md)

首次 HTTP 挂载轮 `ses_1ef829101001TDNGVRHPzbs1tP` 返回 HTTP 200、curl 0、203.418 秒，收到一个 `chat.done)，无 `chat.error`。开始与完成事件按调用 ID 匹配，共 92 次调用，均属于 `ruejs_graph_probe`。其中 query 成功 53 次、callers 成功 8 次、node 成功 28 次及失败 3 次。以下为真实完成回执中的代表结果。

| 工具 | 调用 ID | 核对结果 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_07025645f95f4dcbbfe4b0bd` | `createRueSiteApp` 位于 `app/app.tsx:32` |
| `rustcodegraph_callers` | `call_3344cfdccb884f1a8ca5444a` | 调用者为 `mountRueSiteApp`，位于 `app/app.tsx:39` |
| `rustcodegraph_node` | `call_7edc254f306d4344b5a79b23` | `packages/runtime/src/hooks/useApp.ts:4` 显示 `useApp` 将根工厂传给 `_$createApp` |

三次失败来自猜测的 `app/i18n.ts`、`app/router.ts`、`app/router.tsx`。后续成功读取真实的 `app/i18n/index.ts` 和 `app/router/index.ts`。答复追到了运行时工厂，正确说明 SSR 跳过浏览器挂载、缺少 `#app` 会抛错、路由与 i18n 在执行根组件前安装，以及 `/rust-layers` 绕过站点布局。快照残留分析源于源码推断。其中第二个回归场景直接调用 `createRueSiteApp(...).app.mount(...)`，绕过了 `mountRueSiteApp` 的清空步骤，因而不能检验所声称的有无清空差异。

HTTP 挂载期间的 `acp-options` 读回 `connected=true`、`requested=true`、`support=client_only`，完整模型绑定与请求相同。真实轮次未调用原生文件或 shell 工具，工具审批的原生免审批行为未验证。撤除配置后保持桥接进程在线，使用新会话 `ses_1ef85ac25001M4xa1HSoS3jFlk`，HTTP 200、curl 0、5.859 秒，收到一个 `chat.done`，回复 `MCP_NOT_AVAILABLE`，工具事件为零。两轮的 summary/options HTTP 200。

HTTP 挂载复跑 `ses_1ef85fbd8001fI186hiuSJmQVo` 返回 HTTP 200、curl 0、191.023 秒，收到一个 `chat.done`，无 `chat.error`。成功数为 query 46、callers 4、node 26。另有 9 个 node 失败和 1 个 query 被原生工具目录保护拒绝，报告 MCP 服务器指令在调用前发生变化。复跑仍通过三种工具取得源码，提出静态首屏与切路由两种验证场景。桥接日志分别记录宿主 `rmcp` 和 `copilot-cli 1.0.105` 初始化各自的 MCP 会话。再次撤除后的 `ses_1ef88e694001kKAz29aXDY0NWO` 返回 HTTP 200、curl 0、7.655 秒，收到一个 `chat.done`，回复 `MCP_NOT_AVAILABLE`，零工具事件。复跑两轮 summary/options 均 HTTP 200，绑定与权限读回一致。

## 结论

Copilot 1.0.105 的动态挂载可走原生 HTTP MCP，三种真实工具均完成并返回结果，撤除后新会话恢复无工具。两次挂载均包含工具失败，按计划的零失败合同保留为已阻塞。保留 Copilot 的 stdio 拒绝与现有审批，采用已验证的 HTTP 路径复验时应同时核对工具目录稳定性及两项回归场景。生产侧无改动，未执行 Cargo 或定向单元测试。

私有网关和桥接进程均退出 0，监听端口已关闭，已清理授权副本与临时挂载，并复核原网关仍健康。原始响应、日志和 Copilot 会话记录保存在本机 0700 私有证据目录。
