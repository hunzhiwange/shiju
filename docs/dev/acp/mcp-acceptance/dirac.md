# Dirac 动态挂载 RustCodeGraph

2026-11-19，验收已阻塞。Dirac 0.5.24 的 ACP 建会话入口忽略 `mcpServers`。宿主现有门禁在发出 `session/new` 前明确拒绝非空列表。

## 环境与请求

- Darwin arm64，隔离回环网关复用已验收的 ShiJu 0.6.0，SHA-256 为 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`。本机 Dirac CLI 0.5.24 的入口 SHA-256 为 `c88e043f590a520da3345050e32bbe6930a68cefad60945ef4a7d14648217429`，与此前 Dirac 验收吻合。
- RueJS 使用 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 干净副本。RustCodeGraph 1.2.10 索引 3478 个文件，命中 `app/app.tsx:32`。挂载轮在副本配置 `ruejs_graph_probe`，使用原生 stdio 和 `--no-watch`。
- 三轮以同一个 curl 请求另建会话，沿用租户 `10001 / 100010005`、`session_model`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 与 `full_access=true`。预设为 `acp-dirac`，后端为 `dirac`。提示来自 `189d22dc1`，UTF-8 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。[复用请求](../ACP验收.md#可复用的验收请求)

## 真实模型三轮

| 轮次 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 终态 | 答复与工具 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1effcbcdf0015hMeKPoaa5fuon` | 200 / 0 / 5.572 | 一个 `chat.done` | 包含 `MCP_NOT_AVAILABLE`，另有开场文字。零 MCP 事件，原生技能列表与 `respond` 共 8 条工具状态事件。 |
| 挂载 | `ses_1effcd394001UdGcYrayHZ8RiM` | 200 / 0 / 1.525 | 一个 `chat.error` | 零工具事件。 |
| 撤除 | `ses_1effcdac6001zeiF3MvEX2aCVt` | 200 / 0 / 3.867 | 一个 `chat.done` | 可见答复为 `MCP_NOT_AVAILABLE`。零 MCP 事件，原生 `respond` 有 3 条状态事件。 |

挂载轮先收到 `chat.model_recovery`，`phase=stopped`，再收到 `chat.error`。两条事件均指向 `Dirac does not consume ACP MCP servers; session-scoped MCP is unavailable`。模型未收到工具声明，三种工具均无调用和输出，答复未能覆盖调用链、分支、快照影响和验证场景。两组对照均确认服务不可见。无挂载轮的额外文字和技能列表调用违反原提示，撤除轮也有 Dirac 原生完成事件。

三轮 UI summary 均确认 `acp-dirac / dirac` 与 `full_access=true`。两组对照运行中 ACP options 确认 `connected=true`、指定模型和 `support=client_only`，终态连接释放，模型绑定仍可回读。挂载轮 options 无有效连接、模型选择或权限状态。真实 MCP 审批缺少可达的工具调用。

## 原生断点与复跑条件

- 以独立状态目录、无上游凭据启动已安装的 `dirac --acp`。握手报告 ACP v1、Dirac 0.5.24、`loadSession=true`，未声明 MCP 能力。分别发送 stdio、HTTP 和 SSE 的 `session/new`，三次均返回会话。stdio 启动标记未生成，HTTP 和 SSE 的独立 MCP 回环监听器收到零连接。诊断模型中继捕获的三个首轮工具列表均只有 Dirac 原生工具，没有 RustCodeGraph 声明。该探针在捕获请求后返回固定诊断错误。
- 固定版发行包的 `DiracAgent.newSession` 使用 cwd 和会话选项创建运行时，未使用 `mcpServers`。直接检查 npm 0.6.1 发行包的同一入口，仍未消费该字段。0.6.1 仅做静态检查。[官方 CLI 文档](https://github.com/dirac-run/dirac/blob/master/cli/README.md) · [npm 0.6.1 发行](https://www.npmjs.com/package/dirac-cli/v/0.6.1)
- 独立 MCP 连接中，`rustcodegraph_query` 命中 `createRueSiteApp`（`app/app.tsx:32`），`rustcodegraph_callers` 返回 `mountRueSiteApp`（`app/app.tsx:39`），`rustcodegraph_node` 返回 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）转调 `_$createApp` 的源码。三项复核均成功。初次独立探针以 `query` 误传查询参数，按 MCP schema 改用 `search` 后成功。Dirac 没有读取这些独立结果。
- 保留 `validate_mcp_contract`、Dirac 驱动及既有审批策略。复跑需先取得支持会话级工具注册和宿主审批的 Dirac 入口，再验证真实工具闭环、撤除对照以及审批回归。

定向命令 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-acp --test onboarding_dirac acp_onboarding_dirac_rejects_ignored_mcp_servers` 退出 0，1 项通过、4 项被过滤。使用槽位 6，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为仓库的 `target/rust-slot-6`。审批实现和断言无改动，本轮未单独复跑权限用例。

私有网关、原生进程及回环监听器均已退出。私有授权副本和临时挂载文件已清理。原始证据留在本机 0700 验收目录，用户网关与 RueJS 原目录未写入。

## 共享工作区复验

使用当前共享工作区已有的较新 ShiJu 0.6.0 构建，SHA-256 为 `e755e458b4fce6ed045ef10a7c077a0d7dfb98d69637442d9c4449d0d50923ac`。重新使用独立网关和 RueJS 副本，网关实际启动的 Dirac 仍为 0.5.24。请求内容、租户、传输及模型沿用首轮。

| 轮次 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 终态与输出 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f0011541001UbjjdhXM6LMaUY` | 200 / 0 / 6.208 | `chat.done`，可见答复为 `MCP_NOT_AVAILABLE`。 |
| 挂载 | `ses_1f0012ed5001Fk7am1cEEPy2YK` | 200 / 0 / 1.625 | `chat.error`，原门禁错误及 `phase=stopped` 重现。 |
| 撤除 | `ses_1f0013617001av2JdbZFAMHIRF` | 200 / 0 / 4.588 | `chat.done`，可见答复为 `MCP_NOT_AVAILABLE`。 |

两个对照均有原生 `respond` 的开始、更新和完成事件。三轮目标工具事件为零，挂载轮所有工具事件为零。三轮 UI summary 均回读正确绑定和 `full_access=true`。对照轮在线确认指定模型和 `client_only`，完成后连接释放且模型可回读。挂载轮全程未建立有效连接，选模和审批均无结果。

- 在私有目录安装 npm 0.6.1，入口摘要为 `9d4b628a77082f661487c14f539cc463fa500dfdd61eb06b330a73519700be5e`。对最新版动态补测三种传输。ACP v1 三次均成功建会话，未声明 MCP 能力，stdio 启动标记缺失，独立 HTTP/SSE 监听器的 TCP 连接数为零。回环模型的三份工具表均仅含 17 个原生工具。此处验证了此前静态检查的实际后果。上游当前源码也仅在 resume 入口将 `mcpServers` 转传给 `loadSession`，后者未消费列表。[Dirac ACP 源码](https://github.com/dirac-run/dirac/blob/225d7cf86d1bd500352b0ef4875b86a1a17b6622/cli/src/agent/DiracAgent.ts)
- 0.5.24 和 0.6.1 的 `--acp --only-tools` 均在握手前退出 1，提示工具选择参数只用于普通 CLI 任务。0.6.1 的私有配置加载了一个无副作用的自定义标记工具。该工具同时出现在有挂载和无挂载的两个 ACP 会话的模型工具表中，表明工具目录与配置档绑定，无法由 `mcpServers` 注册或撤除。未修改用户工具、工作区或全局配置。[CLI 参数处理](https://github.com/dirac-run/dirac/blob/225d7cf86d1bd500352b0ef4875b86a1a17b6622/cli/src/index.ts)
- 核对仓库内的受管路径。Snow 补丁依赖 Snow 已有的 MCP 客户端、会话字段及调用循环。OpenClaw 转换依赖其私有 Gateway 原生 MCP，且原生调用不回调 ACP 宿主审批。Dirac 的私有中继只绑定模型和状态目录，缺少这两条路径所需的注册和回传接口。沿用私有自定义工具需要新增会话隔离、工具转换、逐次审批及撤销接线，超出任务卡要求先确认原生 Dirac 消费列表的边界。
- 当前分支及工作区中的 Dirac 驱动、协议门禁和聚焦目标相对前次提交无变化，复用此前 1 项聚焦测试通过证据。本次复验未再编译。私有网关与原生探针均退出，挂载文件及私有授权副本已清理，用户网关健康检查通过。

仍为已阻塞。解除条件为可安装的 Dirac ACP 入口实际消费会话 MCP 定义，并经宿主审批完成调用及撤除验证。若改用新建受管适配器，需先扩展任务卡范围，明确工具注册、宿主审批和会话撤销的交付要求。

## 0.5.24 补丁原型

在原版 `cli.mjs` 旁生成独立入口。补丁在 ACP 建会话时保存 stdio 定义，在提示阶段用 MCP SDK 发现工具，把定义和执行器注入 Dirac 同一份请求工具快照。图谱调用前向 ACP 宿主请求逐次审批，工具结果进入模型续轮。提示结束或取消会关闭连接。原型只接受使用绝对路径、无额外环境变量的 stdio 定义。带挂载的加载、恢复及派生请求保持明确拒绝。

从仓库根目录创建一个私有 npm 安装并复跑。最后一个参数使用已索引的 RueJS 副本。

```bash
umask 077
probe_dir=$(mktemp -d)
npm install --prefix "$probe_dir" --ignore-scripts --no-audit --no-fund \
  dirac-cli@0.5.24 @modelcontextprotocol/sdk@1.32.2
dist="$probe_dir/node_modules/dirac-cli/dist"
python3 patches/acp-model-source/dirac/apply.py "$dist"
python3 patches/acp-model-source/dirac/probe.py \
  "$dist/shiju-dirac.mjs" "$HOME/.rustcodegraph/bin/rustcodegraph" "$ruejs_copy"
```

补丁校验固定版上游入口摘要及全部替换锚点。两份生成脚本的 `node --check` 均通过。回环模型使用相同探针。原版挂载轮没有审批请求，断言失败，退出 1。干净安装的补丁版退出 0，覆盖五轮及两项重入检查。

| 场景 | 补丁版结果 |
| --- | --- |
| 无挂载和撤除后新建 | 两轮均以 `end_turn` 退出，目标工具目录、调用与审批均为空。 |
| 挂载并获批 | 三次宿主逐次审批通过，query 命中工厂，callers 命中挂载函数，node 读回挂载源码。三个成功事件和结果均进入模型续轮。 |
| 拒绝 | 三次拒绝均回传模型，目标工具成功事件为零。 |
| 审批期间取消 | 宿主发出取消，提示以 `cancelled` 退出，无工具结果。 |
| 重入 | 同一挂载会话以空列表加载、以非空列表恢复均被明确拒绝。 |

### 指定模型直连 ACP 三轮

首版原型使用同一 RueJS 提示和真实阿里模型，经私有回环中继运行。上游凭据只在中继中使用。该探针直连 ACP，未绕开或修改 ShiJu 的生产门禁。三轮均 `end_turn`。

| 场景 | ACP 会话 | 模型请求 | 结果 |
| --- | --- | --- | --- |
| 无挂载 | `4776847d-9564-4152-b04e-1d4bd638076e` | 1 | `MCP_NOT_AVAILABLE`，目标工具零次。 |
| 挂载 | `8a1709e4-d850-4d48-b109-58097fa4ab51` | 30 | query 成功 32 次，callers 成功 3 次，node 成功 24 次。 |
| 撤除 | `3d630439-4555-4e58-b5a1-f4e6dfd91795` | 1 | `MCP_NOT_AVAILABLE`，目标工具零次。 |

- 首组成功 ID 为 `9da1795c-a75e-4640-adce-aa55c34fed14`、`a22fd4f5-760a-4678-ac49-0b287c888da2`、`b256ae2e-600a-4847-9363-38bac65fa27f`，回执分别命中 `app/app.tsx:32`、`app/app.tsx:39` 及浏览器入口源码。最终答复引用了 `useApp → _$createApp`、挂载守卫、插件顺序与快照残留，也给出两项场景。第二项把运行时 `_$mountApp` 列作入口替代，直接调用它无法验证站点入口的清空逻辑。两项均未实际运行。
- 宿主拒绝了模型发起的 4 次其他 MCP 调用。5 次 node 请求因模型提交不存在的文件路径返回失败。两组对照均有 Dirac 原生 `respond` 完成事件。指定模型已读回三种真实工具，但严格验收仍存在额外调用、工具失败及答复偏差。
- 发布接线待完成。补丁原型、静态校验和原生探针均在私有环境运行。网关三轮、生产受管安装及原审批回归仍待验证，本轮未运行 Cargo。门禁保留。直连及探针进程均已退出，原始证据在本机私有目录。

### 限定工具与纯文本终态复验

使用相同指定模型及提示。私有 MCP 代理仅公开真实 RustCodeGraph 的 query、callers、node，补充说明 `projectPath` 默认使用索引根目录，`node` 的文件参数须来自已确认的路径。代理转发原查询和结果。Dirac 私有配置关闭原生工具，用户状态未改。第一组三轮保留 `respond`，挂载轮成功 66 次且无 MCP 失败，三轮仍各产生一次原生完成工具事件。

第二组沿用私有配置并显式关闭 `respond`。补丁根据每轮现有工具快照选择答复方式。缺少 `respond` 时，ACP 缓冲可见正文，在收到正常模型停机理由后发送完整文本一次。没有工具调用的截断响应仍按原请求循环继续。保留 `respond` 时继续使用原模式。

| 场景 | ACP 会话 | 模型请求 | ACP 终态与结果 |
| --- | --- | --- | --- |
| 无挂载 | `7970d560-5028-4b6c-aef6-0a2826c627df` | 1 | `end_turn`，完整 `MCP_NOT_AVAILABLE`，零工具事件。 |
| 挂载 | `c347f3a0-2ae0-4d7e-9fc2-3618274c9f23` | 35 | `end_turn`，query 25、callers 4、node 32，均获宿主逐次批准并成功完成。额外调用、失败和拒绝均为零。 |
| 撤除 | `c3935b9e-10f8-4f09-ae36-8a61c691a450` | 1 | `end_turn`，完整 `MCP_NOT_AVAILABLE`，零工具事件。 |

- 首组三种成功回执分别是 `aa3109ed-2544-49f8-a711-6e27b60f1750`、`3e75ef4c-d78b-4042-99ea-fc4250f29796`、`116ec79d-0183-465b-8f08-79eed6ea6884`。内容命中 `createRueSiteApp`（`app/app.tsx:32`）、`mountRueSiteApp`（`app/app.tsx:39`）及站点入口源码。
- 挂载轮只有一条 6937 字符的可见答复。它给出完整调用链、缺失容器错误、浏览器和 SSR 分支、路由和 i18n 安装、`/rust-layers` 布局绕过、快照重复推断与两个验证场景，并指出索引未报告模块级调用。场景均为建议，未运行浏览器。场景 B 的替代路线仍需要通过实际站点入口验证 `replaceChildren`。快照产物逐字节相同的说法也未经实测。
- 两个 Node 文件的语法检查退出 0。原生回环探针默认模式 5 轮通过，`probe.py --plain-text` 6 轮通过。纯文本探针让模型分两段返回文本，验证 ACP 只收到完整答复一次、正常停机和截断继续分开处理、原逐次审批及撤除断言保留。
- 本次达到原生三轮的工具和终态要求。尚未接入 ShiJu 受管安装和生产宿主门禁，因而没有网关 SSE、UI summary 或选模回读作为本轮证据。本次未编译 Rust。私有探针和监听器均退出，原始证据留在本机。
