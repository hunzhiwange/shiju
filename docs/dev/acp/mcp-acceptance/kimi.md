# Kimi Code CLI 动态挂载 RustCodeGraph

2026-11-19。修复受管模型配置生成。隔离验证确认 Kimi 能使用会话级 stdio MCP 完成模型工具调用，挂载轮超出统一合同规定的工具范围。

## 环境与首个失败边界

- Darwin arm64，私有安装官方 npm 包 `@moonshot-ai/kimi-code@2.5.0`，使用原生 `kimi acp`。入口模块 SHA-256 为 `acd6894e2e2db50d37c449eefe9d3549a0177bd2d12cf17a8e1da94c60eb359f`。ACP v1 握手读回 Kimi Code CLI 2.5.0，声明 HTTP 和 SSE，建会话实测使用 stdio。厂商文档说明 ACP 将 `session/new.mcpServers` 转换为 Kimi 服务配置。[Kimi ACP 文档](https://github.com/MoonshotAI/kimi-code/blob/main/docs/en/reference/kimi-acp.md#mcp-forwarding)
- 请求正文取自提交 `189d22dc1`，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。复用租户 `10001 / 100010005`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017`、`session_model` 和 `full_access=true`。RueJS 副本来自 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c`。RustCodeGraph 1.2.10 索引 3478 个文件，命中 `app/app.tsx:32`。[复用请求](../ACP验收.md#可复用的验收请求)
- 使用已有网关产物（SHA-256 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`）的无挂载、挂载、撤除三轮分别为 `ses_1efb46df3001KrRHOX0IunF1z4`、`ses_1efb47534001Ey0z3ho9nDzGGR`、`ses_1efb47b1c001Bluopj3x1Zyl7h`。三轮 HTTP 200、curl 0，均在 ACP `session/new` 返回 `Authentication required`，SSE 各一个 `chat.error`，零工具、零 `chat.done`。另一个已有网关产物（SHA-256 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`）原样复现三次。
- 首个失败发生在 Kimi 加载受管模型配置时。原驱动把 JSON 值序列化为 TOML，启用任意精度的 `serde_json` 将两项模型限额写成带 `$serde_json::private::Number` 的子表。两项字段直接以整数序列化后，Kimi 读回配置并进入同一个模型中继。生产修复只改 Kimi 的 TOML 类型。

## 同一请求的隔离对照

下列三轮使用第二个现有网关产物。私有探针只把启动时生成的两项 TOML 子表改为对应整数，并记录原生 ACP 轨迹。Kimi、MCP、模型和请求正文均沿用原样。

| 轮次 | ShiJu 会话 | HTTP / curl / 秒 | SSE 终态及可见答复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1efb68238001LS4zG7s4Y6J4ow` | 200 / 0 / 5.060542 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1efb69790001cPWv1yRl4VkwEm` | 200 / 0 / 257.928441 | `chat.done`，三种目标工具完成，另有四种图谱工具调用 |
| 撤除 | `ses_1efba875c0011quOTRtzZcQe4g` | 200 / 0 / 4.180166 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具 |

三轮均无 `chat.error` 或模型恢复事件。UI summary 回读 `acp-kimi / kimi`、`full_access=true`，ACP options 运行中为 `connected=true`、`access_status.support=partial`，结束后连接释放，模型绑定仍可回读。原生 ACP 新会话分别收到 0、1、0 项服务。三轮均确认已有的 `mode=yolo` 控制，`model=session-selected` 和 `thinking=on` 保持原值。实际工具调用均属所挂服务，未出现审批回调、Shell 或文件写入事件。本轮覆盖三个新会话。后台标题生成模型另有一次 HTTP 403，聊天选定模型完成三轮推理。

挂载轮共 861 条 SSE 工具更新。按原生调用 ID 汇总，147 次调用完成，其中 query 73 次、callers 11 次、node 51 次，另有 files 7 次、callees 3 次、status 1 次、explore 1 次。以下各调用均有原生开始、完成和结果。

| 目标工具 | 原生调用 ID | 实际输出摘录 |
| --- | --- | --- |
| `rustcodegraph_query` | `0:call_e5b2d2bfc35d428e9736e3cd` | `createRueSiteApp` 位于 `app/app.tsx:32` |
| `rustcodegraph_callers` | `0:call_c36f3f7805e84ee1967fc29a` | `createRueSiteApp` 的调用者为 `mountRueSiteApp`，位于 `app/app.tsx:39` |
| `rustcodegraph_node` | `0:call_bc5e611e22c743218a662ee1` | `app/app.tsx:33–34` 创建 router，并调用 `useApp(RootApp).use(router).use(i18n)` |
| `rustcodegraph_node` | `0:call_45010a3041b84336b38ac60c` | `packages/runtime/src/compiler-runtime/app.ts:57` 给出 `_$createApp` 实现 |

答复的主调用链、SSR 与浏览器分支、缺少 `#app` 的异常、路由和 i18n 安装、`/rust-layers` 绕过布局均与副本源码吻合。移除清空后可能保留旧 DOM 的分析及两项回归场景属于推断。答复将双份页头必然出现、`/rust-layers` 必然残留旧布局写得过于确定。这些结果取决于原始快照内容和客户端入口。静态快照执行客户端模块的实际调用点是 `packages/server-renderer/src/static.mjs:785`。`static.mjs:747` 是函数入口。

## 定向验证与后续

- 用槽位 3 缓存的原生依赖编译实际驱动、`AcpLaunchProfile` 及同一测试 `kimi_session_model_writes_integer_limits`。旧驱动 1 项失败，退出 101，context 字段解析为非整数。修复后 1 项通过，退出 0。轻量入口命令保存在私有证据目录，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为仓库下的 `target/rust-slot-3`。
- 工作区已有外部 Cargo 全目标检查在编译。需按槽位规则串行补跑 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib kimi_session_model_writes_integer_limits` 并构建修复后的 CLI 网关。现有公共转发路径在原生轨迹中已得到验证，本次没有改动该路径。私有网关及 MCP 子进程已退出，端口关闭，已撤除临时工作区挂载。
- 整体验收仍有两项缺口。需要用修复后的常规产物按相同合同复跑三轮，挂载轮需要仅调用约定的三种工具并准确限定静态快照推断。
