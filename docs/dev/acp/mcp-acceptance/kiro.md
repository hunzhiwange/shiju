# Kiro 动态挂载 RustCodeGraph

2026-11-19，Kiro 3.0.0 V3 的原生 ACP 能在新会话发现动态挂载的 RustCodeGraph，执行指定工具并把结果送回模型。共享构建复验通过。私有 stdio 转发器限定工具目录后，三轮均取得正常终态。[复用请求](../ACP验收.md#可复用的验收请求)

## 环境与会话边界

- Darwin arm64，私有回环网关复用 ShiJu 0.6.0，构建 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。私有配置目录内复用既有受管 Kiro `3.0.0-model-source-v1`。启动器、原生二进制、Node 和补丁后 V3 入口共四项摘要与仓库清单一致。V3 入口 SHA-256 为 `26907b1f2895cf28a850d7720b340cb206965d8ffbc17cb62dd78276f2dbd02d`。
- 每轮使用新会话，租户为 `10001 / 100010005`，预设和后端为 `acp-kiro / kiro`，模型为 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，选模来源为 `session_model`，请求带 `full_access=true`。统一提示取自 `189d22dc1`，SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。
- 挂载轮使用 RustCodeGraph 1.2.10 原生 stdio 和 `--no-watch`。宿主源码通过 `session/new.params.mcpServers` 传入配置。固定 V3 入口将这项配置交给会话 MCP 注册。实际原生日志记录 `ruejs_graph_probe` 经 stdio 连接、列出工具及执行调用。SSE 记录工具参数、调用 ID 与输出。本次网关未抓取原始 ACP 建会话请求体。
- 无凭据且断网的独立探针用同一启动器完成 ACP v1 `initialize`，返回 Kiro CLI Agent 3.0.0。该探针未启用已绑定模型，报告 `loadSession=false`、`http=false`、`sse=false`。真实三轮使用已验证的 stdio 路径。
- 暂时挂载来自副本的 `.vwacprc.json`。受管驱动仍使用请求绑定的回环中继与专用会话目录，并在启动前拒绝 `ASBX_KIRO_MANDATORY_MCPS` 外部覆盖。固定发行版的 mandatory 规则按显式标签附加工具，与本次会话注册分属两处逻辑。真实目标调用的原生策略结果为 `allow`、`capability=all`，没有交互审批事件。会话选项报告 `partial`，本次没有验证其他工具的审批矩阵。[Kiro 工具审批文档](https://kiro.dev/docs/mcp/usage/)

## 两组 RueJS 副本

首组复用 RueJS `main` 上 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 的工作区副本，索引 3687 个文件。原工作区有未提交变更，`app/app.tsx` 引用的 `app/router/index.ts` 已在工作区删除。挂载轮首个超出指定图谱工具范围的调用为 `rustcodegraph_files`（`call_1b05623cd802468ba0061fe2`），随后对该缺失路径的 `rustcodegraph_node` 调用实际失败（`call_85a4a0308a1f4cafb99dff4c`）。

第二组从同一提交导出干净副本，索引 3478 个文件，路由文件存在，`createAppRouter` 定义在 `app/router/index.ts:787`。该组未挂载轮先偏离合同。模型调用原生 `Kiro Powers` 的 `list` 操作，返回未安装 Power，随后输出约定标记及额外说明。该组挂载轮先加载服务的工具声明，首个额外图谱调用为 `rustcodegraph_status`（`call_6df119cf71954040944d0b40`）。另一次 `rustcodegraph_node` 尝试读取不存在的 `app/router.ts` 并失败（`call_b07b7c42ae0c47b4839b26cf`）。

两组挂载轮均从同一服务取得以下真实结果，指定工具的开始与完成可用 SSE 调用 ID 对应：

| 工具 | 核对到的结果 |
| --- | --- |
| `rustcodegraph_query` | 定位 `createRueSiteApp` 到 `app/app.tsx:32` |
| `rustcodegraph_callers` | 找到 `mountRueSiteApp`，位于 `app/app.tsx:39` |
| `rustcodegraph_node` | 回读 `app/app.tsx` 的启动分支、`useApp`（`packages/runtime/src/hooks/useApp.ts:4`）和 `_$createApp`（`packages/runtime/src/compiler-runtime/app.ts:57`） |

`Tool Load` 是 Kiro 读取工具声明的内部调用，单独记录在 SSE。它提供工具的输入参数结构。只有后续带 `@ruejs_graph_probe/` 前缀并在原生 MCP 日志中执行的调用计入图谱调用。两组均用当前受管入口直接运行，没有修改驱动、受管补丁或共享转发。未运行 Cargo。

## 工作区副本三轮

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 终态及答复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1efb62f94001GQFdaPOd8BqMR3` | 200 / 0 / 37.019 秒 | `chat.done`，只返回 `MCP_NOT_AVAILABLE`，工具事件为零 |
| 挂载 | `ses_1efb6c0e50011cOFB7dwuuGmh3` | 200 / 28 / 300.011 秒，续传 200 / 28 / 300.007 秒、200 / 0 / 243.084 秒 | 同一流和会话最终取得 `chat.done`，包含调查答复 |
| 撤除 | `ses_1efc3a0e8001evSEvuebJ9HlRT` | 200 / 0 / 30.652 秒 | `chat.done`，只返回 `MCP_NOT_AVAILABLE`，工具事件为零 |

三轮各一个 `chat.done`，无 `chat.error`。UI summary 回读 `acp-kiro / kiro` 与 `full_access=true`。运行中 ACP options 均为 `connected=true`、`access_status.support=partial`，回读所选模型与请求一致。无挂载和撤除的最终回读都显示连接已释放。挂载轮终态的即时回读仍为 `connected=true`，私有网关随后退出。

挂载轮共有 488 条工具状态事件。按调用 ID 计，`query` 成功 52 次，`callers` 成功 4 次，`node` 成功 72 次、失败 8 次。额外图谱调用 26 次，含 `files` 17 次、`explore` 6 次，及 `callees`、`status`、`impact` 各 1 次。另有两次 Kiro `Tool Load`。未观察到 shell、写入或其他 MCP 服务调用。

答复给出了目标调用链、SSR 守卫和缺失 `#app` 的异常，解释了路由与 i18n 的插件安装及 `/rust-layers` 绕过布局条件。快照残留与活应用 DOM 追加是基于 `replaceChildren` 和运行时插入语义的推断。答复给出的两个检验场景分别检查首屏 DOM 的唯一性与重复生成快照的幂等性，均未执行。答复对缺失相对导入的判断需要修正。实际工作区删除了被引用的文件，图谱返回的 `app/hooks/router/index.ts` 是磁盘上的另一份源码。答复还把快照链接无客户端事件绑定推定为 URL 不变，但原生链接可能触发页面导航。私有网关的独立自动标题请求另有上游 HTTP 403，未作为指定模型调查的结果。

## 干净副本三轮

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 终态及答复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1efbbe76b001rG8PkD1BNtsDfG` | 200 / 0 / 33.613 秒 | `chat.done`，标记后有额外说明，另有一次 Power 列表调用 |
| 挂载 | `ses_1efbc6bdf001Tw067loLkJgmEU` | 200 / 28 / 300.007 秒，续传 200 / 28 / 300.008 秒、200 / 28 / 300.000 秒 | 同一流和会话继续产生工具事件，未收到 `chat.done` 或 `chat.error` |
| 撤除 | `ses_1efca58bd001rgsQPgg62kCf7K` | 200 / 0 / 31.813 秒 | `chat.done`，只返回 `MCP_NOT_AVAILABLE`，工具事件为零 |

挂载轮在最后游标 783 前有 763 条工具状态事件。指定的 `query`、`callers`、`node` 分别完成 110、12、46 次。额外图谱工具完成 84 次，另有一次 `Tool Load`。一次 `node` 调用读取错误路径，一次 `explore` 因缺少 `query` 参数失败。只收到阶段性进展文本，无法评估该轮最终调查答复。

三轮运行中的 UI summary 和 ACP options 分别确认绑定、`full_access=true`、指定选模及 `partial`。两轮对照随后均回读连接已释放。两次续传使用相同会话和请求 ID，未重复发起挂载推理。续传超时后关闭私有网关，撤除配置并在同一副本的独立网关发出撤除对照。

## 首轮结论和清理

原生动态发现和撤除隔离已实测。严格合同首个行为偏差分别是工作区挂载轮的额外图谱调用，以及干净副本无挂载轮的 Power 列表调用。两组都存在额外图谱调用和工具失败。下方复验沿用原提示和阿里模型。原生转发已有开始、完成和结果证据，本任务未找到需要修复的 Kiro 转发断点。

配置受管目录前的启动尝试返回 `Reviewed ACP executable is unavailable`。把已审核安装置于私有网关实际配置目录后，进入原生 V3 和指定模型会话。私有网关和工具进程均已退出，端口已关闭，租户配置、授权副本及临时源码已清理。两个 RueJS 副本在比较窗口内分别有 6365、4107 个普通文件摘要一致，比较排除私有索引和测试挂载配置。原始 SSE、回读和原生 MCP 日志留在本机 0700 目录。

## 共享构建三轮通过

- 复用共享槽位 3 的 ShiJu 0.6.0 私有副本，SHA-256 为 `f46699dd46d8520d8a7cfc3324132ccd5a2e05857e89fde160052fdcf75add3c`。受管 Kiro 3.0.0 V3 的四项摘要重新核对一致。保留原请求、租户、模型、服务名及权限，用同一 RueJS 干净提交和 RustCodeGraph 1.2.10，索引 3478 个文件。
- 复用同批次的私有 stdio 转发器。工具目录由八项限定为原名的 `query`、`callers` 和 `node`，补充文件路径、源码分页和索引根说明。独立协议探针对照确认三种调用的实际返回与直连一致，额外的 `files` 调用和副本外读取均被拒绝。转发器仅用于私有挂载轮，它启动原 RustCodeGraph 并继承最小环境。

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 终态及答复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f040ed360013sZx82lx3RjcsZ` | 200 / 0 / 27.634 秒 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1f04159ae001tw3WYDlURBBlT9` | 200 / 28 / 300.006 秒，续传 200 / 0 / 122.526 秒 | 同一流取得 `chat.done`，三种指定工具均成功 |
| 撤除 | `ses_1f047cd4e001aA0XArbnmP0pwx` | 200 / 0 / 24.611 秒 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |

- 三轮各一个 `chat.done`，没有 `chat.error` 或模型恢复。挂载轮 313 条工具事件，包括 43 次 query、7 次 callers、54 次 node，共 104 次原生 MCP 执行，全部成功且有相同 ID 的开始、完成、输出和原生成功日志。Kiro 另外记录一次 `Tool Load`，输入和输出仅含这三个工具的 ID 与声明。这是 Kiro 延迟披露目录的步骤，未执行 MCP 工具。无 shell、Power、写入或其他工具执行。
- `call_e4653253399c45ab9b563a1d` 命中 `createRueSiteApp`（`app/app.tsx:32`），`call_e1afb9183ee241798dc2f884` 返回 `mountRueSiteApp`（同文件第 39 行）。`call_65204b9d41ce4aaebeab9a07` 回读 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`），`call_74144c17b6ba4015b0ad56d9` 读回 `_$createApp`（`packages/runtime/src/compiler-runtime/app.ts:57`）。
- 答复的调用链、SSR 守卫、缺少 `#app` 抛错、路由与 i18n 安装以及 `/rust-layers` 布局分支均与源码对应。旧快照残留是对追加挂载语义的推断。两项验证分别检查预渲染页面旧节点是否移除，以及预填容器中根组件抛错后旧内容是否保留。第二项需显式比较调用站点清空逻辑和省略该逻辑两组，直接调用 `app.mount` 不包含清空。重复 DOM 的精确数量和失败状态下的节点行为未做浏览器或单元验证。
- UI summary 三轮都回读 `acp-kiro / kiro` 和 `full_access=true`。ACP options 确认指定提供商和模型，连接期间为 `partial`，挂载即时回读仍连接，随后随私有网关关闭。两个对照终态均已断开。受管入口、mandatory MCP 和原有权限规则沿用。
- 私有网关及其子进程已退出，端口关闭，私有配置、源码和授权副本已清理。副本在索引目录外的 4107 个普通文件摘要一致，索引目录中随版本跟踪的 `.gitignore` 也单独核对一致。原网关健康。只更新验收记录，未运行 Cargo。
