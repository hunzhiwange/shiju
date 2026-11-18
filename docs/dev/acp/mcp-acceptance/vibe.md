# Mistral Vibe 动态挂载 MCP 验收

2026-11-19，任务 37 已阻塞。Vibe 原生 stdio 挂载、三种目标工具的真实回传及撤除隔离均已验证。统一验收仍受两处行为阻塞。无挂载轮有额外答复和内置工具搜索，挂载轮有失败的源码路径查询。

## 环境和会话边界

- 使用 Darwin arm64 的仓库受管 `vibe-acp 3.0.4`，从已安装的 `vibe-3.0.4-model-source-v1` 复制到私有实例。原生进程独立握手退出 0，报告 ACP v1、`loadSession=true`。原生实现将 `session/new.params.mcpServers` 中的 stdio 声明转成会话级 MCP 配置。
- 网关复用仓库已有 `shiju 0.6.0`，SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。驱动将 `VIBE_HOME` 指向当前启动的私有目录，通过 `--session-model-config` 提供一次性 `session-model.toml`，模型凭证由网关中继持有。挂载轮观察到 ACP 进程连接回环中继并启动 RustCodeGraph 1.2.10 子进程，参数为 `serve --mcp --path <私有 RueJS 副本> --no-watch`。宿主只在副本的 `.vwacprc.json` 临时声明 `ruejs_graph_probe`。
- 原租户 `10001 / 100010005`，预设 `acp-vibe`，后端 `vibe`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，选模来源 `session_model`，`full_access=true`。请求来自 `189d22dc1` 的可复用 curl，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。
- 首次复制 RueJS 当前工作目录，索引 3,687 个文件。其相对入口引用的 `app/router/index.ts` 已在工作目录中删除，影响源码核对。第二次从同一分支 `main` 的提交 `14fe287b` 解出干净副本并重新索引 3,478 个文件。两次均命中 `createRueSiteApp` 于 `app/app.tsx:32`，均完整执行三轮。

## 干净副本结果

| 轮次 | 新会话 | HTTP / curl / 秒 | RustCodeGraph 结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1efc98791001cRibZRYQxVWOoh` | 200 / 0 / 7.932 | 0 次。答复包含 `MCP_NOT_AVAILABLE` 和额外说明，另有 3 条 Vibe 内置搜索事件 |
| 挂载 | `ses_1efc9a6d3001ACI22kAwTYdgI9` | 200 / 0 / 127.641 | 66 次成功、1 次失败，另有 2 条 Vibe 内置搜索事件 |
| 撤除 | `ses_1efcb9a6f001UQfdi6Jf8TbVYz` | 200 / 0 / 7.006 | 0 次。答复包含 `MCP_NOT_AVAILABLE` 和额外说明，另有 2 条 Vibe 内置搜索事件 |

每轮各有一个 `chat.done`，无 `chat.error`，无需续传。UI summary 回读确认预设、后端与 `full_access=true`。ACP options 在运行中显示 `connected=true` 和 `access_status.support=client_only`，终态连接释放，所选模型仍与请求一致。目标工具成功期间未见交互审批、文件或 shell 工具调用。`client_only` 表示宿主侧自动审批范围，Vibe 原生其他工具的审批行为未覆盖，未切换原生 `auto-approve` 模式。

同一调用 ID 的 SSE 开始和完成事件携带以下真实输出：

| 工具 | 调用 ID | 输出 |
| --- | --- | --- |
| `rustcodegraph_query` | `effect-a9124cfd-6658-51aa-ad91-6e37f50f99c6` | `createRueSiteApp app/app.tsx:32` |
| `rustcodegraph_callers` | `effect-16026898-80d1-58a6-81db-65f9a7e73ff6` | `mountRueSiteApp app/app.tsx:39` |
| `rustcodegraph_node` | `effect-44d9a2e1-7bbe-5554-8815-ed2151aea33f` | `packages/runtime/src/hooks/useApp.ts:4` 调用 `_$createApp(factory)` |
| `rustcodegraph_node` | `effect-de237984-a49c-5727-8450-37ad8e92a002` | `packages/runtime/src/compiler-runtime/app.ts:57` 的应用工厂及第 77–79 行的插件安装回调 |

答复与源码比对确认主调用链、SSR 入口、缺少 `#app` 时抛错、路由和 i18n 插件安装顺序，以及 `/rust-layers` 绕过布局。答复指出图中缺少浏览器模块顶层调用的边，提出静态快照重复及导航后旧内容残留两个检查场景。上述场景是静态推断，答复用“现状通过”描述预期，未取得浏览器运行证据。路由工厂实际在 `app/router/index.ts:787`，答复仅追到注册和上下文时序，未核对插件内部安装逻辑。

## 首个失败边界和收尾

无挂载与撤除轮的目标工具隔离成立。Vibe 的 Core 内置 `search_tool_functions` 生成 `Searching for relevant tools` 事件，模型在约定标记后附加文字。挂载轮的第一次实际工具失败是模型请求不存在的 `app/router.tsx`，RustCodeGraph 返回文件不存在，同会话继续完成其他查询。首份副本的挂载轮有 103 次成功、6 次缺失路径失败，无挂载与撤除也各有额外说明。两组共六轮确认 MCP 转发已抵达原生会话，随后发生的是工具搜索、模型输出和路径选择问题。

未修改驱动、审批策略或提示，未运行公共转发单测或 Cargo。两次验收脚本均退出 0，私有网关均停止，回查端口 curl 退出 7，子进程已退出，临时挂载与授权副本已清理。干净副本的 4,109 个归档文件在运行后逐项匹配，除索引外新增文件为零。原始 SSE、会话查询和日志留在本机 0700 私有目录。
