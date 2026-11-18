# Qoder CLI 动态挂载 RustCodeGraph

2026-11-19，Qoder 1.1.88 的受管 ACP 已实际发现并调用动态挂载的三个目标工具，结果送回指定模型。统一三轮验收仍有失败，首个偏差出现在无挂载对照的模型行为。

## 环境与挂载链路

- Darwin arm64，私有网关复用 ShiJu 0.6.0，二进制 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。私有配置目录复用已安装的 `qoder-1.1.88-model-source-v1`。补丁后入口及绑定模块 SHA-256 分别为 `e812bfd63431758a96d7888fefc7e67a02735e0bf69edf6b5e531a261c7a7ed1` 和 `27753aa78105ebe8fe19a9ed09b34069c5ca6efaa9388da3c8e699a03169c53c`，与仓库受管清单一致。
- 使用租户 `10001 / 100010005`、`acp-qoder / qoder`、`session_model`、`full_access=true` 和 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。请求取自提交 `189d22dc1`，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。使用 RueJS `main` 的工作区副本，HEAD 为 `14fe287b13549f5183d42fbc45b28e1498ffae1c`，含既有未提交内容，索引 3687 个文件。[统一验收请求](../ACP验收.md#可复用的验收请求)
- 仅挂载轮的 `.vwacprc.json` 启用 `ruejs_graph_probe`，使用 RustCodeGraph 1.2.10 原生 stdio 和 `--no-watch`。固定版受管入口使用 `--acp` 加请求私有的 `--config-dir`、`--setting-sources user` 与绑定模型。Qoder 原生 `createSession` 将 `session/new.params.mcpServers` 写入会话状态。真实挂载轮记录了 RustCodeGraph 启动和带服务名的原生调用。
- 独立回环探针直接发出两次 ACP `session/new`，一次传空列表，一次传入真实 stdio 定义。协议 v1 握手报告 `qoder-cli 1.1.88`、`loadSession=true`、`http=true`、`sse=true`。两次模型请求均向回环 `/v1/chat/completions` 声明通用的 `mcp_list`、`mcp_get` 和 `mcp_call`。探针让模型调用 `mcp_list`，空列表会话续轮收到 0 项，挂载会话续轮收到 8 项，包含三个目标工具。两次均收到 `end_turn`。真实验收使用 stdio。
- 实际会话在连接期间报告 `access_status.support=partial`。独立 ACP 探针验证两种挂载状态均默认处于 `default`，并能通过 `session/set_mode` 切换为 `yolo` 再恢复 `default`。真实调用未出现 ACP 审批交互，受限模式下的实际工具审批矩阵未覆盖。固定二进制、参数、工作区和请求绑定门禁沿用现行实现。

## 指定模型三轮结果

| 轮次 | 新会话 ID | HTTP / curl / 耗时（秒） | SSE 终态 | 工具与答复 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efbf65530019A0H74dexMZzXQ` | 200 / 0 / 10.881 | 一个 `chat.done` | 目标工具 0 次。3 次 `mcp_list` 均返回空列表。答复含说明和 `MCP_NOT_AVAILABLE`。 |
| 挂载 | `ses_1efbf91720010sjqeNRxG9Vv3E` | 200 / 0 / 274.839 | 一个 `chat.done` | 目标工具实际返回源码和调用关系，同时出现 9 次文件读取失败以及额外工具调用。 |
| 撤除 | `ses_1efc3c474001vPNz4sGqYAOpPi` | 200 / 0 / 9.723 | 一个 `chat.done` | 目标工具 0 次。2 次 `mcp_list` 均返回空列表。答复含说明和 `MCP_NOT_AVAILABLE`。 |

三轮均无 `chat.error`，全程无需续传。运行中及终态的 UI summary 均确认预设、后端及 `full_access=true`。ACP options 均确认固定版本、选模来源、提供商及完整模型 ID，连接状态在运行中为 `true`、结束后为 `false`。

挂载轮在同一调用 ID 下记录工具开始和完成，以下结果来自原生 `mcp_call`。统计按实际调用标题区分 `mcp_call` 和用于读取描述的 `mcp_get`。

| 工具 | 成功 / 失败 | 返回证据 |
| --- | --- | --- |
| `rustcodegraph_query` | 25 / 0 | `createRueSiteApp` 命中 `app/app.tsx:32`（`call_542774e72ff6418f9742dd55`）。 |
| `rustcodegraph_callers` | 4 / 0 | `createRueSiteApp` 的调用者为 `mountRueSiteApp`，位于 `app/app.tsx:39`（`call_6c4acfbaed024b7cbe27dd66`）。 |
| `rustcodegraph_node` | 58 / 9 | `useApp`（`packages/runtime/src/hooks/useApp.ts:4`，`call_3594b642ae7b4671b4b4dce3`）转调 `_$createApp`（`packages/runtime/src/compiler-runtime/app.ts:57`，`call_54086cdbf06841869468b46a`）。还读回 `mountRueSiteApp` 和根组件源码。 |

挂载轮另执行了 19 次 `rustcodegraph_files` 和 3 次 `mcp_get`，没有 shell 或写入调用。最早的失败调用 `call_529cdb0c1599421f9ab577e0` 给 `rustcodegraph_node` 传入不存在的 `app/router.ts`，服务返回文件不存在。一次 `rustcodegraph_files` 调用传入 `pattern`，工具实际使用 `filter`，未过滤的列表超出 Qoder 的输出上限并被截断。两轮对照均确认目标工具不可见，但答复均多了一句说明，未满足仅返回 `MCP_NOT_AVAILABLE` 的合同。

## 答复核对与收尾

- 挂载答复引用了完整调用链及对应文件行号，正确定位客户端入口的 SSR 守卫、缺少 `#app` 时的异常、路由和 i18n 的安装顺序，以及 `/rust-layers` 绕过布局。答复从源码推断静态快照会和客户端 DOM 并存，并提出比较构建快照及在浏览器统计重复布局两种场景。此次未执行构建或浏览器验证。
- 答复把 `app/en22try-server.tsx` 称为当前静态 SSR 入口，副本的实际构建脚本仍指向已缺失的 `app/entry-server.tsx`。同一副本的 `app/app.tsx` 导入 `./router`，可读到的路由实现位于 `app/hooks/router/index.ts`。答复后文识别两处差异，但将备用文件的 SSR 行为归给当前入口缺少证据，所提静态构建场景也需要先恢复副本中缺失的入口和导入文件。
- 三轮定位到模型的工具选择、参数和答复偏差，受管入口的挂载、权限模式切换及结果回传均有实测。未发现需要修改 Qoder 驱动、受管入口或公共转发的缺口。未运行 Cargo 聚焦测试。严格验收复跑仍须使用原模型、同一提示与三个独立会话。
- 私有网关已退出，端口关闭，已清理私有租户配置、临时 MCP 配置及本次启动材料。源码副本与原工作区按验收排除项比较无差异。原始 SSE、回环探针和一次截断工具输出保留在本机 0700 验收目录。

## 共享构建复验

- 复用共享槽位 3 的 ShiJu 0.6.0 私有副本，SHA-256 为 `1a033a19a5c5e2c115e90abbd336f898c0df3ac101a9824ffe8b36f11b0bcc47`。受管 Qoder 1.1.88 两项摘要再次与清单一致。两个私有网关各使用同一 RueJS 提交的干净副本，原工作区缺失的 `app/router/index.ts` 和 `app/entry-server.tsx` 在提交中均存在。索引各覆盖 3478 个文件。租户、模型、提示、服务名和 `full_access` 沿用原合同。
- 两组各执行无挂载、挂载、撤除。第一组复用同批次的私有 stdio 转发器，只公布原名的 `query`、`callers`、`node`，补充文件路径和分页说明。协议对照确认三个调用的实际结果与 RustCodeGraph 直连完全一致，并验证额外工具和副本外读取被拒绝。第二组直接使用 RustCodeGraph 原生 stdio，检查清洁源码对缺失入口及工具选择的影响。两种路径均由原受管 Qoder 启动，仅挂载轮包含工作区 MCP 配置。

| 传输 | 轮次 | 新会话 ID | HTTP / curl / 耗时（秒） | SSE 终态及答复 |
| --- | --- | --- | --- | --- |
| 三工具转发 | 无挂载 | `ses_1f04ad49a0017CRU7q0CqH5wRm` | 200 / 0 / 9.240 | `chat.done`，说明后返回 `MCP_NOT_AVAILABLE` |
| 三工具转发 | 挂载 | `ses_1f04af9fc001ZukgKb4y86WZDx` | 200 / 0 / 221.186 | `chat.done`，三种工具均成功，另有一次节点读取失败 |
| 三工具转发 | 撤除 | `ses_1f04e5bc90019wjxgY07c6AMiN` | 200 / 0 / 8.522 | `chat.done`，仅 `MCP_NOT_AVAILABLE` |
| 原生 stdio | 无挂载 | `ses_1f04b7c86001GNlnq2mS63wbD5` | 200 / 0 / 9.688 | `chat.done`，说明后返回 `MCP_NOT_AVAILABLE` |
| 原生 stdio | 挂载 | `ses_1f04ba354001tDyl4QCszr7z2w` | 200 / 0 / 202.929 | `chat.done`，三个目标工具无失败，有额外调用 |
| 原生 stdio | 撤除 | `ses_1f04ebc49001pg3DXWUvUBvuXz` | 200 / 0 / 10.275 | `chat.done`，说明后返回 `MCP_NOT_AVAILABLE` |

- 六轮各收到一个 `chat.done`，无 `chat.error`，无需续传。四个对照会话各有两次空列表 `mcp_list`，目标工具均为零。运行中及结束后的 UI summary 和 ACP options 均确认原绑定、权限标志、提供商和选定模型，运行中权限支持为 `partial`，结束后连接均释放。
- 三工具转发轮的 query、callers、node 分别成功 42、5、31 次。代表调用 `call_5ff38b935cab4a1d81b525ee`、`call_b2746f7711db47c1bf2a1da1`、`call_60935e6de63846f0a50978be` 和 `call_56fb83853d654857b80a71bc` 依次返回之前核对的 `createRueSiteApp`、`mountRueSiteApp`、`useApp`、`_$createApp`。唯一失败由模型传入 `packages/router/src/index.ts` 引发（`call_725da8d9dcef4b10966c8173`），随后成功读到 `index.tsx`。该轮没有其他工具调用。
- 原生 stdio 轮 query、callers、node 分别成功 16、5、37 次，无失败。代表调用 `call_e86eed5bef8d4256bee4efb7`、`call_efb6edaa1ddb4cdf983524aa`、`call_ab0814a47a7248c284693171` 和 `call_1d50de15c3fc4940b2e49342` 返回相同四个关键符号和文件行号。另有 8 次 `files`、各一次 `explore` 和 `status`，以及 7 次 MCP 目录查询。两组均不含 shell 或写入调用。
- 两份挂载答复均给出完整链路、SSR 守卫与实际 SSR 入口、容器异常、路由和 i18n 安装以及布局分支。三工具轮还指出当前路由表缺少 `/rust-layers`，布局条件存在但在默认路由表下未找到匹配路由。两组均提出检查快照哨兵和检查重复布局与 SPA 导航的场景。三工具轮中的单哨兵夹具可直接检查旧节点是否保留，只有夹具含对应标题或导航时才能进一步断言该元素变成两份。这些推断未做浏览器实测。
- 两组首个偏差均为无挂载答复增加说明。干净副本排除了上次源码缺失的干扰，三工具转发也排除了额外图谱工具执行，但模型格式偏差与路径参数失败仍可单独复现。Qoder 的原生挂载、工具结果回传及撤除隔离已有真实证据，任务转为待回归。复跑继续沿用原模型和三轮原请求。只补充文档，未改受管驱动或运行 Cargo。
- 两个私有网关及子进程均已退出，端口关闭，私有配置、凭据副本、源码和临时挂载均已清理。每个副本中的 4108 个受版本跟踪的普通文件均与提交逐字节一致。原网关健康，原始证据留存于本机 0700 目录。
