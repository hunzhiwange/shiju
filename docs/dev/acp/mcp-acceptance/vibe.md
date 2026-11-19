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

## 共享构建与限定工具复验

复用共享槽位 6 的 `shiju 0.6.0` 构建副本，SHA-256 为 `6954a2bb76ca26aa39c3b0dd909786c68b85f336648526ef39651b1e3a31f718`。仍使用受审 Vibe 3.0.4、RustCodeGraph 1.2.10、固定 RueJS 提交和原验收请求。复用 [Goose 验收](goose.md#通过的三轮) 的私有 stdio 转发，目录只公布 query、callers、node。元数据加入分页字段、真实入口文件及 `projectPath` 可省略的根索引说明。协议对照确认原目录 8 项、限定目录 3 项，三种真实工具的参数和返回保持一致，分页读取命中 `app/router/index.ts:787`，额外工具和工作区外路径被拒绝。

| 轮次 | 新会话 | HTTP / curl / 秒 | 结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f0559b04001Q5HPQNszfp7u4H` | 200 / 0 / 6.533 | 零图谱工具，2 次内置搜索，标记后有额外文字 |
| 挂载 | `ses_1f055b5e6001i6fxvseP5NHTRq` | 200 / 0 / 112.783 | 图谱 224 次成功、1 次失败，另有 2 次内置搜索 |
| 撤除 | `ses_1f0576eb70013Iy1gkPRYmV3i7` | 200 / 0 / 7.816 | 零图谱工具，2 次内置搜索，标记后有额外文字 |

三轮各一条 `chat.done`、零 `chat.error`。UI summary 均确认 `acp-vibe / vibe` 和 `full_access=true`。ACP options 回读原租户所选模型和 `session_model`，运行中为 `client_only`，终态连接均已释放。挂载轮的 225 次图谱调用均有同 ID 的开始与完成事件。query、callers、node 分别成功 9、3、212 次，node 在模型猜测 `packages/server-renderer/src/static.ts` 时真实返回文件不存在。未看到 shell 或文件工具调用。

- query `effect-a6f9e73a-9dc4-563b-bf23-5081426f70ae` 返回 `createRueSiteApp app/app.tsx:32`，callers `effect-5fc9735e-fc35-587a-9f88-eb4b0c5b6309` 返回 `mountRueSiteApp app/app.tsx:39`。
- node `effect-2032cbe1-6298-55be-b54e-223fc7ea46e4` 返回 `useApp.ts:4` 对 `_$createApp` 的调用，`effect-12040cf1-5569-5ac2-b9b6-815f74aba78e` 返回 `compiler-runtime/app.ts:57` 的工厂。
- 答复覆盖主链、SSR、缺少容器、路由及 i18n 安装和布局条件，并给出预填模板残留和预渲染页重复 DOM 两项验证设想。两项均为源码推断，实际 DOM 数量未经浏览器验证。

独立原生 ACP 回环探针检查了 Vibe 送给模型的真实工具声明。默认设置、按名停用 `search_tool_functions` 及停用连接器三种空挂载会话均返回 `end_turn`，初始推理请求均包含 `search_tool_functions`。同一次按名过滤成功移除了对照工具 `unified_harness_scratchpad`，确认过滤设置已生效。该探针仅使用本地占位模型和令牌，真实验收仍使用原模型。内置搜索由 Vibe Core 提供，限定 MCP 工具目录后它仍可用。[Mistral 工具过滤配置](https://docs.mistral.ai/vibe/code/cli/configuration-reference)

严格验收仍为已阻塞。保持现有审查入口及原请求时，需要支持直接暴露会话 MCP 工具而不触发内置搜索的受审 Vibe 版本，并由原模型完成精确的无挂载答复。另一种复跑条件是验收方明确接受 Vibe 的原生搜索事件并修改对应答复合同。未改运行时代码、提示或审批，未跑 Cargo。私有网关及子进程已退出，端口关闭，临时挂载与授权副本已清理。RueJS 的 4,109 个归档文件与符号链接逐项一致。原始证据保留在本机 0700 目录。

## 原生直挂补丁实验

隔离补丁通过指定阿里模型的严格三轮。[Vibe 3.0.4 实验补丁](../../../shiju/acp-model-source/evidence/vibe/mistral-vibe-3.0.4-direct-mcp-experiment.patch)使用 Core 已有的 `direct` 暴露类型和 `programmatic.mode=disabled`。MCP 仍经原生 `session/new`、原生 MCP 执行器与原审批回调。Core 在该模式下会拒绝仅支持程序化调用的功能，因此实验同时关闭子代理和后台进程。只在私有受管副本中打补丁。补丁没有进入默认安装流程。[Mistral Vibe 3.0.4 Core 用例](https://github.com/mistralai/mistral-vibe/blob/v3.0.4/harness/core/src/tests/acceptance_scenarios/builtin_features.rs)

原生回环红绿证据来自同一固定发行的两份副本。原版即使按名过滤仍声明 `search_tool_functions`。补丁版在空、挂载、空三个新会话都不声明 `search_tool_functions` 或 `run_typescript`，仅挂载轮直接声明真实图谱工具。回环模型实际调用 `rustcodegraph_query` 时，原生 ACP 发出 `session/request_permission`。一次批准返回 `createRueSiteApp app/app.tsx:32` 并完成工具事件。单独拒绝轮返回工具失败，模型侧无图谱结果。两次探针均完成 `end_turn`。归档补丁的等价整理版重新通过上述两项探针和 `patch --dry-run`。

指定模型轮复用前节网关构建、租户、选模、提示摘要、RueJS 提交及三工具 stdio 转发。每轮均为新会话，三轮各有一个 `chat.done`，没有 `chat.error`。UI summary 回读 `acp-vibe / vibe`、`full_access=true`，ACP options 回读原模型和 `session_model`，在线阶段为 `client_only`，终态释放连接。

| 轮次 | 新会话 | HTTP / curl / 秒 | 结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f159af9b001eORAARl1z51Ktk` | 200 / 0 / 4.564 | 可见答复仅 `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1f159c25d001TEElYkJ72wEnKH` | 200 / 0 / 153.912 | 14 次 query、3 次 callers、43 次 node 均成功，零额外工具 |
| 撤除 | `ses_1f15c1c0a001WTpvkjgJTjEnxI` | 200 / 0 / 5.515 | 可见答复仅 `MCP_NOT_AVAILABLE`，零工具 |

60 次挂载调用均有匹配的开始及完成事件和原始结果。query `effect-81c80f99-1022-531f-90d7-6978bd3ac4ab` 命中 `app/app.tsx:32`，callers `effect-cf9f9c7e-4aac-574b-a34c-efa36d67db5f` 返回 `mountRueSiteApp`，node `effect-d8a69c95-070b-53c8-a731-0a00b698ea19`、`effect-8ce32cff-9429-5792-92f6-19048cc40001` 依次取回 `useApp` 和运行时工厂。答复覆盖调用链、SSR、容器缺失、路由及 i18n 安装、布局分支，并提出首屏唯一性和预填快照去污染两个验证场景。答复的“实际结果”是对修改后行为的源码推断，两项均未运行浏览器。

实验已突破内置搜索和严格三轮验收。未运行 Cargo。私有网关和其子进程已退出，端口关闭，临时挂载配置和授权副本已清理。

## 默认受管安装

macOS arm64 的 Vibe 受管安装现默认同时应用会话绑定和 MCP 直挂补丁。安装目录升至 `vibe-3.0.4-model-source-v2`。安装前校验固定 wheel 和补丁目标的原始 SHA-256，安装与启动校验均覆盖两份新增的补丁产物。旧 v1 目录需要重新执行受管安装。

受管 v2 保留原生 ACP 审批，关闭程序化工具、子代理和后台进程。直挂工具在模型目录中需要唯一的函数名。

私有目录通过仓库安装脚本冷安装，`vibe-acp --version` 返回 3.0.4。七项产物摘要均与契约匹配，v1 在两项新增摘要上不匹配。冷安装入口通过原生 ACP 回环批准、拒绝与撤除验证。批准轮返回真实图谱结果，拒绝轮返回工具失败。复用上节已通过的真实模型证据，未重跑 Cargo。
