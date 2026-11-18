# Factory Droid 动态挂载 RustCodeGraph

2026-11-19，已阻塞。Factory Droid 1.9.0 原生 ACP 能在会话内发现并调用 RustCodeGraph。统一验收首先在无挂载轮失败，模型为确认工具缺失而调用了 Droid 内置工具搜索。挂载轮成功运行三个必需工具，也运行了额外工具并产生一次工具失败。[复用请求](../ACP验收.md#可复用的验收请求)

## 环境

- Darwin arm64 使用受管 Factory Droid 1.9.0，制品 SHA-256 为 `b62ce8de0ed7ffaada1abb2a017fe182021003694e85fe7f3b1136ad43628a5d`，与受管清单一致。入口为 `exec --output-format acp`，保留 Airgap、私有 Factory 状态、回环中继和启动环境过滤。私有网关为已有 shiju 0.6.0，SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`，健康检查 HTTP 200。仅在私有实例启用预设与模型，复用已授权的认证来源。[Factory Airgap](https://docs.factory.com/enterprise/airgapped-deployment)
- 三轮使用 `10001 / 100010005`、`acp-droid / droid`、`session_model`、`full_access=true` 和 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。RueJS 使用 `main` 提交 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 的导出快照，索引覆盖 3478 个文件。挂载采用 `ruejs_graph_probe` stdio，直接运行原 RustCodeGraph，并传入私有副本路径与 `--no-watch`。

## 真实模型三轮

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 终态 | 答复与工具 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efa2fd1f001uCPxWyN65d3XU5` | 200 / 0 / 16.823 秒 | `chat.done` ×1 | `MCP_NOT_AVAILABLE`，图谱调用 0，`ToolSearch` ×1，返回 `Tool(s) not found` |
| 挂载 | `ses_1efa3412e001aaSVbsStmrNr4k` | 200 / 0 / 242.177 秒 | `chat.done` ×1 | 完成三部分调查，105 对工具事件，其中 104 项成功、1 项失败 |
| 撤除 | `ses_1efa6f5830019M8XPIZpIqAuSG` | 200 / 0 / 16.358 秒 | `chat.done` ×1 | `MCP_NOT_AVAILABLE`，图谱调用 0，`ToolSearch` ×1，返回 `Tool(s) not found` |

三轮均无 `chat.error`。UI summary 回读预设、后端和 `full_access=true`。ACP options 在运行中和结束后都回读固定模型和 `session_model`，运行时报告 `connected=true`、`access_status.support=client_only`，终态连接关闭。撤除后，服务及其工具从新会话消失，仍有内置工具搜索事件。两个对照轮的搜索失败均在 SSE 中标为 `failed`。

## 真实工具输出

以下调用均有同一 ID 的开始和成功完成事件，SSE 标题绑定 `ruejs_graph_probe`。

| 工具 | 调用 ID | 输出节录 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_23b916bbd5c19457_0` | `createRueSiteApp` 命中 `app/app.tsx:32` |
| `rustcodegraph_callers` | `call_b6871d9359658e4f_0` | 调用方为 `mountRueSiteApp`，位于 `app/app.tsx:39` |
| `rustcodegraph_node` | `call_9a4551d8d7d61289_1` | `packages/runtime/src/hooks/useApp.ts:4` 的 `useApp` 转发给 `_$createApp` |
| `rustcodegraph_query` | `call_36882c17af45220b_0` | `_$createApp` 命中 `packages/runtime/src/compiler-runtime/app.ts:57` |

挂载轮还调用了 `ToolSearch` 与图谱的 `status`、`files`、`explore`、`callees`、`impact`。调用 `call_2d86a46bfc4291a6_3` 读取不存在的 `packages/i18n/src/index.ts`，SSE 记为 `failed`。这些事件已使挂载轮不符合统一合同。

答复追踪了四段调用链。源码复核确认浏览器挂载及 SSR 跳过（`app/app.tsx:54`）、容器缺失抛错（`app/app.tsx:40–42`）、路由和 i18n 在根工厂前安装（`app/app.tsx:34`、`packages/runtime/src/compiler-runtime/app.ts:45–48`、`:77–79`）、`/rust-layers` 走无站点布局分支（`app/app.tsx:17–28`）。答复也指出顶层调用和同名 `.use` 的图谱差异。它分析了删除清空逻辑可能保留旧快照并追加新树，给出了布局节点计数、路由和语言切换两项验证场景。具体重复 ID、译文和整页跳转属于推断，未运行浏览器实验。答复中 `/jsx/basic-elements` 的路由定位应为 `app/router/index.ts:46`。

## 原生边界与权限

- 私有回环模型探针确认 `initialize` 为 ACP v1，代理身份 `@factory/cli` 1.9.0，声明 `loadSession`、`resume`、HTTP 和 SSE。分别向原生 `session/new` 提交空列表、真实 stdio 定义及 HTTP/SSE 合成定义，四轮均建会话，固定短提示均为 `end_turn`。初始模型工具列表均含 `ToolSearch`，图谱工具延迟发现。真实挂载轮经搜索成功调用图谱，说明 `FACTORY_DISABLE_DYNAMIC_CONFIG=true` 与 ACP 会话 MCP 可以同时启用。统一三轮采用原生 stdio。
- 同版 `--only-tools` 指定三个会话 MCP 选择器时，进程在 ACP 握手前以 `Unknown tool identifier(s)` 退出，空列表与拟挂载两种设置均复现。独立使用 `--remove-tools ToolSearch` 后，原生模型请求仍声明 `ToolSearch`。两个选项合用则被 CLI 判为无效。未将这些探针参数加入受管入口。[Factory CLI 参数](https://docs.factory.com/droid-cli/cli-reference#droid-cli-flags)
- 另发独立权限探针，将 `full_access` 设为 `false`。HTTP 200，curl 退出 0，5.929 秒，SSE 返回 `chat.error`，原始消息为 `droid approval mode cannot constrain native tools; a current full access grant is required`。未出现 `chat.done` 或工具执行。保留该门禁。

## 复跑条件与清理

原生挂载及撤除已经生效。严格验收仍需能在会话级限定工具集合、同时保留 Droid 权限边界的受审入口，再使用固定提示和模型跑三轮。当前生产驱动和公共 ACP 转发未改。未触及公共转发测试所覆盖的故障，未运行 Cargo、桌面构建或浏览器测试。

私有网关退出 0，端口关闭，未发现本次工具或代理子进程残留。撤除配置后与导出快照逐项比较，4109 项内容一致，额外文件仅为私有图谱索引。私有源码、运行目录及授权副本已清理。现有网关的 Droid 预设仍为停用。原始 SSE、回读和诊断证据保存在本机 0700 目录。
