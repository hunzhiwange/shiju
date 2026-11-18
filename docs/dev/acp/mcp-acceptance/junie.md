# Junie 动态挂载 RustCodeGraph

2026-11-19 验收阻塞。Junie 收到会话级 stdio 定义并识别服务名，受管环境内启动 RustCodeGraph 失败。

## 环境

- Darwin arm64，受管 Junie 26.11.2（4100.5）。原生入口 SHA-256 为 `4be1f51971aa638ebd9c3caa93e63d2a235e3a4bd133011476f6cbbbcfcc565e`，启动器 SHA-256 为 `db3c895af5065c401f7798385d27619e0aa4ef5c227b048b84e0b090c71f2fad`，均与仓库契约一致。原启动器通过签名检查。独立握手返回 ACP v1、`@jetbrains/junie` 26.11.2（4100.5）、`http=true` 和 `sse=true`。
- 私有回环网关复用 `shiju 0.6.0`，SHA-256 为 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`。仅在私有实例启用预设，使用租户 `10001 / 100010005`、模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 及 `session_model`。
- 请求取自提交 `189d22dc1`，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。RueJS 从 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出私有副本。RustCodeGraph 1.2.10 索引 3478 个文件，命中 `app/app.tsx:32` 的 `createRueSiteApp`。挂载选用原生 stdio。 [复用请求](../ACP验收.md#可复用的验收请求)

## 三轮真实请求

| 轮次 | 新会话 | HTTP / curl / 秒 | 终态与可见答复 | 工具事件 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efb082c0001OSlx7UueleK4qk` | 200 / 0 / 14.237481 | `chat.done`，`MCP_NOT_AVAILABLE` | 0 |
| 挂载 | `ses_1efb0bc070017eUNfft1Vx1Pj2` | 200 / 0 / 13.300183 | `chat.done`，`MCP_NOT_AVAILABLE` | 1 条原生服务启动失败事件 |
| 撤除 | `ses_1efb0f0a300124c3V500RLZlGT` | 200 / 0 / 13.985039 | `chat.done`，`MCP_NOT_AVAILABLE` | 0 |

三轮均无 `chat.error`。运行中和终态的 UI summary 均确认 `acp-junie / junie` 与 `full_access=true`。ACP options 在运行中为 `connected=true`，权限状态为 `client_only`，终态连接释放。两阶段均读回指定模型、提供商和 `session_model` 来源。受管会话进入 prompt 前须确认 `brave_mode=off` 和模型，三轮均进入推理，未出现原生审批事件。

挂载轮宿主的 RustCodeGraph 客户端已完成初始化。Junie 随后为同名服务发出工具事件 `dea786f3-4f30-4e69-9a85-ba15ab1f4248`，标题为 `ruejs_graph_probe failed`。输出报告 `state: FAILED`、无法执行 RustCodeGraph 和 `posix_spawn failed`。事件外层状态为 `completed`，对应服务失败通知已完成。三种目标工具均无调用或结果，答复没有调用链、文件行号或快照分析。挂载轮未达到验收标准。

## 首个失败边界

- 受管启动器禁用默认 MCP 发现。JetBrains 支持 ACP 会话传入的服务及显式 `--mcp-location`，两条路径的 stdio 服务都需要启动子进程。现有 Seatbelt 策略只允许执行固定的 Junie 原生入口，并将出站连接限定为会话模型中继端口。 [JetBrains MCP 配置](https://junie.jetbrains.com/docs/junie-cli-mcp-configuration.html)
- 原生 HTTP 和 SSE 能力已通过握手核实。当前 RustCodeGraph `serve --help` 仅提供 stdio，现有模型中继端口只承载模型协议。仓库中的测试桥接夹具转发原始 stdio 字节。当前缺少能在受管权限与会话范围内转发 RustCodeGraph 的可用 HTTP 或 SSE 入口。
- 未修改生产代码或审批策略。编号任务给出的生命周期回归未运行，此次失败发生在原生子进程启动边界。私有网关已退出，端口关闭，撤除轮前已删除挂载配置。后续在具备会话级兼容入口后复跑相同三轮。
