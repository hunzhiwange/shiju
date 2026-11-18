# Pi 动态挂载 RustCodeGraph

2026-11-19，Pi 受管 v1 动态挂载验收复现缺口。指定模型在三轮均完成回复。挂载轮宿主已连接 RustCodeGraph，Pi 的模型工具表仍缺少 MCP 工具。[复用请求](../ACP验收.md#可复用的验收请求)

## 环境

- Darwin arm64，复用 ShiJu 0.6.0，二进制 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。在私有网关的配置目录使用受管 pi-acp 0.0.22 和原生 Pi 0.73.1。实际适配器及引擎 SHA-256 分别为 `5132b90b8244ef8ab8ddbda77a812b5e477501baafb25862d1c0c0d1623f9dc6` 和 `e959f463b06ddd15ed882783ac02f39ecaeef950ef967da86380f39dda6595fa`，与仓库固定值一致。
- 租户 `10001 / 100010005`，`acp-pi / pi`，`session_model`，`full_access=true`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。提示取自 `189d22dc1`，SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。
- RueJS 副本来自 `main` 的 `14fe287b` 及当时工作区，RustCodeGraph 1.2.10 索引了 3687 个文件。索引命中 `app/app.tsx:32` 的 `createRueSiteApp`、`app/app.tsx:39` 的 `mountRueSiteApp` 和 `packages/runtime/src/hooks/useApp.ts:4` 的 `useApp`。选择原生 stdio，挂载轮宿主日志记录握手成功，MCP 协议版本为 `2024-11-05`。

## 指定模型三轮结果

| 新会话 | HTTP / curl / 耗时 | SSE 终态 | 回复 | 工具事件 |
| --- | --- | --- | --- | --- |
| 无挂载 `ses_1efbc1c21001WEeomE7CBFW6pO` | 200 / 0 / 3.450 秒 | `chat.done` | `MCP_NOT_AVAILABLE` | 0 |
| 挂载 `ses_1efbc2a5f001Gl1XnDYqx3qU6H` | 200 / 0 / 2.452 秒 | `chat.done` | `MCP_NOT_AVAILABLE` | 0 |
| 撤除 `ses_1efbc3477001o4zOKGEgcZjQPJ` | 200 / 0 / 2.739 秒 | `chat.done` | `MCP_NOT_AVAILABLE` | 0 |

- 每轮恰有一个 `chat.done`，且 SSE 无 `chat.error`。两轮对照回复符合预期。挂载轮的 `rustcodegraph_query`、`rustcodegraph_callers`、`rustcodegraph_node` 均无开始、完成或输出，无法验证结果续轮、调用链答复准确性及查询审批。
- 三轮 UI summary 均回读 `acp-pi / pi` 和 `full_access=true`，终态 ACP options 均回读指定模型与 `session_model`。无挂载轮捕获在线状态及权限报告 `partial`。其余两轮只捕获连接释放后的 options。三轮均无工具审批事件。
- 每轮完成后，网关独立生成会话标题时向 `kimi-k2.7-code` 发起的请求返回 403。该请求属于 `session_title`，发生在 ACP 助手回合完成后。

## 首个失败边界

使用同一固定入口、原生引擎、独立状态目录和占位令牌，直接向 ACP 发送三次建会话及提示。ACP v1 报告 `loadSession=true`、`mcpCapabilities.http=false`、`sse=false`。非空轮明确传入原 RustCodeGraph 的 stdio 描述并获得新会话，三轮均以 `end_turn` 结束。回环模型只收到四个原生工具 `read`、`bash`、`edit`、`write`，部分辅助推理请求没有工具。所有请求均缺少 MCP 工具。真实网关轮保留了宿主握手日志和 SSE，原始 ACP 报文由独立探针复现。

固定版 `src/acp/session.ts` 在会话对象中保存 `mcpServers`，原生 RPC 启动未使用该字段。断点位于 Pi 工具注册之前，发生在结果回传或压缩之前。上游主分支仍记录同一限制。[pi-acp 上游限制](https://github.com/svkozak/pi-acp#limitations)

上游未合并的 #164 依赖较新 Pi 的 `registerMcpServer`，固定的 0.73.1 扩展 API 仅提供 `registerTool` 等接口。#145 的注册工具方案直接在扩展中执行 MCP 调用，缺少 ShiJu 宿主逐工具审批回接，并允许连接失败后继续建会话。普通 Pi MCP 扩展采用独立配置，受管启动以 `--no-extensions` 隔离自动发现。适配沿用受管扩展隔离和两个发行固定值。[#164 会话桥接](https://github.com/svkozak/pi-acp/pull/164) · [#145 扩展方案](https://github.com/svkozak/pi-acp/pull/145)

v1 私有网关已退出，端口已关闭，副本挂载条目及私有认证文件已清理。原始证据保留在本机私有目录。

## 受管 v2 适配与聚焦验证

- 保留 pi-acp 0.0.22、Pi 0.73.1 与原 Relay 绑定。v2 只加载 reviewed adapter 显式指定的扩展，将本会话选中的 stdio 工具注册到 Pi 0.73.1。执行请求须匹配当前提示中原生调用的 ID、工具名及参数，宿主通过 ACP 审批后才查询。取消或关闭会话会撤销在途调用，响应沿 Pi 原生工具 ID 返回。工作区规则在注册时过滤，撤除和重新加载时按新请求重建工具集。
- 调用复用受审 macOS stdio 沙箱和单操作 runner。MCP 使用独立临时 HOME、工作区只读视图，禁止网络和派生子进程。受管配置接受绝对路径、无额外环境变量的 stdio。私有验收副本保留 SQLite WAL 与 SHM，供只读沙箱打开原索引。
- 仓库安装脚本基于固定发行包完成 v2 安装，并校验适配器、扩展和两个 Python 文件的摘要。适配器 SHA-256 为 `579df420c0a8d19f72bf34343e592174ff50d40776fda6311c04f488a44dad60`。TypeScript 定向类型检查退出 0。
- 用同一个 `patches/acp-model-source/pi/probe.py` 测试原生 Pi 和真实 RustCodeGraph。v1 退出 1，挂载工具表缺三种工具。v2 退出 0，七个场景通过，覆盖无挂载、批准、拒绝、撤除、加载重绑及清除、等待审批时取消，以及宿主规则的拒绝优先。工具审批、结构化 ACP 完成事件和模型续轮以同一调用 ID 核对。真实输出含 `createRueSiteApp`、`mountRueSiteApp` 和 `useApp` 对 `_$createApp` 的调用。
- 宿主定向命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p session-engine --lib acp_mcp_mount_carries_resolved_tool_policy`。槽位 6 的 `CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为仓库的 `target/rust-slot-6`。退出 0，实际 1 项通过，同时断言 Pi 和 Cortex 的宿主工具规则覆盖请求元数据。

## 真实网关复验

- 同一槽位继续执行 `cargo build -j 1 --locked -p vw-cli --bin shiju`，退出 0，用时 15 分 52 秒。私有网关二进制 SHA-256 为 `7f8fb6ab97e9e6b00e6694ce48376e2d50a3ce4b2bff3fff93004b98db824916`。未编译桌面客户端。
- 首轮复验沿用带工作区改动的 RueJS 副本。三轮 HTTP 200、curl 0、均收到 `chat.done`。无挂载与撤除会话 `ses_1f06917c5001su50lzQ5G7SJjt`、`ses_1f06bc64a001h5zp457hzScbQX` 均回复 `MCP_NOT_AVAILABLE`，目标工具事件为零。
- 挂载会话 `ses_1f06923800016101sioznYHq1a` 以 172.486 秒完成。三个工具分别成功 65、8、30 次。模型另有 12 次节点读取和 2 次查询失败，涉及副本中不存在的路径和工作区外未建索引的父目录。实际失败由 RustCodeGraph 返回并被 ACP 标记为失败，故该轮未通过严格验收。随后改用同一提交的干净存档重新索引，保持请求提示、模型、租户、服务名与批准方式。

干净副本索引 3478 个文件。三轮 HTTP 200、curl 0、各一个 `chat.done`，没有 `chat.error`。UI summary 均为 `acp-pi / pi`、`full_access=true`，在线 ACP options 均回读 `partial` 及指定模型，结束后仍可重读选模。挂载轮使用原生 stdio，仅向模型登记该服务的三种目标 MCP 工具。SSE 实际调用均来自这三种工具。单独的标题生成仍出现上游 403。

| 轮次 | 新会话 | 耗时 | 工具或回复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f06bd8b4001r7QhWNnD1bvgqS` | 2.932 秒 | `MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1f06be48d001HfTPqL6mYVh4Of` | 208.494 秒 | 1105 条工具事件，128 次成功、7 次失败 |
| 撤除 | `ses_1f06f132a001UfsbWhsqLMUBE1` | 2.968 秒 | `MCP_NOT_AVAILABLE`，零工具事件 |

- `rustcodegraph_query` 的 `call_2216a55a4d844a3ba9035064` 返回 `createRueSiteApp`（`app/app.tsx:32`）。
- `rustcodegraph_callers` 的 `call_348abb999b42411b81609250` 返回 `mountRueSiteApp`（`app/app.tsx:39`）。
- `rustcodegraph_node` 的 `call_d2fe89c968474f7193407eb6` 返回 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）转调 `_$createApp` 的源码。三个 ID 均有真实开始和成功完成事件。

答复按源码追踪了完整调用链、SSR 与浏览器分支、缺少 `#app` 时的异常、路由与 i18n 的安装顺序、`/rust-layers` 的布局分支，指出索引遗漏，并给出静态快照冷启动及切换路由后卸载的两项可检验预期。6 次 `node` 失败来自模型猜测的不存在路径，1 次 `query` 失败来自把源文件传为索引项目目录。所有失败均在 SSE 中记录失败状态，参与答复的目标调用和续轮已获得真实证据。严格三轮验收待回归，复跑条件为原模型在挂载轮完成调查且未发起上述无效查询。两份私有挂载、认证文件及网关进程已清理，槽位已释放。
