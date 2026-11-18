# AutoHand Code 动态挂载 RustCodeGraph

## 环境与请求

2026-11-19，使用 [固定验收请求](../ACP验收.md#可复用的验收请求) 在 `189d22dc1` 的版本。提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。租户为 `10001 / 100010005`，会话使用 `acp-autohand / autohand`、`session_model`、`full_access=true` 和 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。借用已有受审构建启动私有回环网关，只在私有实例启用预设并写入租户认证。网关 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。

Darwin arm64 上已安装的 AutoHand CLI 为 `0.9.11 (46a28b16)`，以 `autohand --acp` 运行，入口 SHA-256 为 `339bdb0c8f4f8b7f9076108434d910b2cd5e14f2593b48c0e5858e04e91950eb`。独立握手返回 ACP v1 并声明 HTTP、SSE MCP 和会话加载、恢复能力。本次使用 `session/new.params.mcpServers` 中的原生 stdio，命令为 RustCodeGraph 的 `serve --mcp --path <私有副本> --no-watch`，服务名 `ruejs_graph_probe`。

RueJS 的 `main` 基准提交为 `14fe287b13549f5183d42fbc45b28e1498ffae1c`。私有副本保留工作区已有源码修改，索引 3687 个文件，`createRueSiteApp` 命中 `app/app.tsx:32`。副本中 `app/app.tsx:10` 仍导入 `./router`，旧路由入口已删除，`app/hooks/router/index.ts:787` 有同名工厂。该工作区差异影响答复中的路径判断。

## 真实工具轨迹

挂载期间观测到 RustCodeGraph stdio 子进程。三个指定工具均有同一调用 ID 的开始和成功完成事件。

| 工具 | 实际输入 | 成功调用 ID | 输出节选 |
| --- | --- | --- | --- |
| `rustcodegraph_query` | `search=createRueSiteApp` | `call_6b4b528d1f7549a293d798ab:1` | `createRueSiteApp`，`app/app.tsx:32` |
| `rustcodegraph_callers` | `symbol=createRueSiteApp` | `call_6bc97c548ecb4e969ae5fed1:0` | `mountRueSiteApp`，`app/app.tsx:39` |
| `rustcodegraph_node` | `name=useApp`，`file=packages/runtime/src/hooks/useApp.ts` | `call_e2f96229baeb4af0a26d3dee:0` | 第 4 行调用 `_$createApp(factory)`，从 `../compiler-runtime/app` 导入 |

## 配置边界

AutoHand 发行版按显式配置参数、`AUTOHAND_CONFIG`、`AUTOHAND_HOME` 下的默认配置文件依次选取路径。ShiJu 受管驱动将 `AUTOHAND_CONFIG` 指向私有 `AUTOHAND_HOME` 下的 `config.json`。回环探针在两个位置配置冲突的模型和端点，ACP 实际选择显式配置的模型并访问其回环端点。原生 `newSession` 在创建会话 agent 后连接收到的 `mcpServers`。独立进程的无挂载和撤除探针均没有向模型声明该 MCP 工具。

发行版默认开启 `features.codeMode`。在该模式下，模型先收到 `tool_search`、`run_tool_script` 等入口，真实 MCP 调用可作为脚本的子调用发生。私有探针将 `features.codeMode` 置为 false 后，模型侧仍可见内置 `tool_search`。再尝试仅允许三个限定名称的原生 `--allowed-tools`，挂载轮的模型请求也没有收到目标 MCP 工具。该参数作用于进程运行范围，当前受管入口只接受已验证的启动参数。复用它需要先证明按会话隔离、三个选中的工具仍可调用，以及原权限策略继续生效。

## 三轮结果

| 轮次 | 新会话 ID | HTTP / curl / 耗时 | SSE 与工具对照 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef8d004c001tEz3NLaNn8vje1` | 200 / 0 / 10.857 秒 | 一个 `chat.done`，目标工具为零。执行三次 `tool_search`，在 `MCP_NOT_AVAILABLE` 后追加说明及回顾 |
| 挂载 | `ses_1ef8d2ba50013tEsP15UFq2e2R` | 200 / 28 / 300.006 秒，续传 200 / 0 / 43.807 秒 | 使用原请求标识和末事件 ID 续传，同一会话最终收到一个 `chat.done`。指定三种工具均有成功调用，另有额外工具和失败事件 |
| 撤除 | `ses_1ef926c79001JjiMuNWrUXNJKB` | 200 / 0 / 16.951 秒 | 一个 `chat.done`，答复为 `MCP_NOT_AVAILABLE`，目标工具为零。仍执行两次 `tool_search`、三次 `run_tool_script` 和两次 `tools_registry` |

三轮无 `chat.error`。UI summary 均回读 `acp-autohand / autohand` 和 `full_access=true`。ACP options 在执行中和终态都回读到指定 `session_model` 及完整模型 ID。执行中 `connected=true` 且权限为 `client_only`，终态连接已释放。没有人工批准动作，结束时待审批列表为空。另用关闭 Code Mode 的原生回环探针调用只读 `rustcodegraph_explore`，确认真实输出进入下一次模型请求。探针未收到该调用的 `session/request_permission`，因此无法实测宿主拒绝该 MCP 调用的行为。宿主权限策略和原生会话模式沿用现有设置。

挂载轮共有 76 次 `query`、6 次 `callers` 和 49 次 `node` 成功完成，另有 5 次 `node` 失败。首次工具失败为读取不存在的 `app/router.ts`。同时完成了 37 次 `run_tool_script`、一次 `tool_search`，以及 `status`、`files`、`explore`、`callees`、`impact` 等额外 MCP 调用。还有一次脚本执行失败。第一处合同偏差出现在无挂载轮的第一条工具事件，模型调用了内置 `tool_search`。

答复的 `mountRueSiteApp → createRueSiteApp → useApp → _$createApp` 链路、浏览器的非 SSR 守卫、缺少 `#app` 时抛错、路由及 i18n 注册、`/rust-layers` 绕过布局均有源码证据。答复把保留快照与客户端节点重复、卸载后遗留快照写成待验证推断，并给出 DOM 计数和卸载验证两项场景。答复指出 `./router` 导入缺失，但把 `app/en22try-server.tsx` 称为实际启用的服务端入口。快照脚本仍引用已删除的 `app/entry-server.tsx`，现有证据只能确认站点入口的 SSR 守卫和副本中的候选模块。没有执行 RueJS 构建或浏览器场景。

## 结论

原生 stdio 的挂载、真实调用和撤除对照已有证据。任务保留为已阻塞，原因是原生工具发现与脚本行为超出提示的限定，且当前已验证的配置入口无法按会话限定模型仅使用指定工具。CLI 局部开关和进程级工具范围的回环探针没有恢复该合同。需要原生会话级工具范围支持后，按同一请求复跑三轮。

仅更新本验收记录。未触及公共 ACP 转发或生命周期边界，未执行 Cargo 定向测试或构建。原始 SSE 和回读保存在本机私有目录。临时配置已撤除，私有网关与 MCP 子进程已退出，回环端口已关闭。
