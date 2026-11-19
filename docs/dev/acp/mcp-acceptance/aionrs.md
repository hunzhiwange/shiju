# aionrs 动态挂载 MCP 验收

2026-11-19，受管 v2 已按当前标准通过真实模型三轮验收。旧版的选模及建会话拒绝记录保留在下方。[复用请求](../ACP验收.md#可复用的验收请求)

## 环境

- 复用上一轮模型绑定验收的 shiju 0.6.0、受审 `aionrs-acp` 及原生 aionrs 0.2.11（Darwin arm64，固定源码 `8e61a90329fa9f67c4fdf7e97fe02c24dba33f75`）。网关、桥接和引擎 SHA-256 依次为 `fbe0b5f4c1ed7ec567c0e025a8601e901a5980f936ec1f0efe37bd5da5e0092d`、`3e542400ed7128d9aff1fa05a2764fb6ab54d249db5905b4b54113f4f4a75928`、`aae767bf2d5132d730a77526132fe3c689ff2cf60c81284d0b7a86f97ffa1662`。
- RueJS 取 `main` 提交 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 的源码副本。RustCodeGraph 索引 3478 个文件，查询命中 `createRueSiteApp`（`app/app.tsx:32`）。
- 请求保留租户 `10001 / 100010005`、模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`、`session_model` 和 `full_access=true`。提示取自 `189d22dc1`，SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。挂载轮使用项目配置中的 stdio 服务 `ruejs_graph_probe`。

## 网关三轮

从已有授权源复制配置和数据库到私有目录，启用隔离实例的 `acp-aionrs`。健康检查 HTTP 200，安装状态为 true，模型能力端点返回 `session_model=supported`。隔离实例的有效配置缺少指定模型。改用与既往成功验收接近的 HOME 布局后复现同样结果。

| 轮次 | 首次会话 | 复跑会话 | HTTP / curl | SSE 终态 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1ef87bfc9001vw1iXPwH1Xr4Tw` | `ses_1ef89338e001qNb0miMltH13yI` | 200 / 0 | `chat.error` |
| 挂载 | `ses_1ef87c282001ycuIcPwQQPi44P` | `ses_1ef893675001RoyyjCmMFwAFY1` | 200 / 0 | `chat.error` |
| 撤除 | `ses_1ef87c47b0014IZO9YV1SNeq9f` | `ses_1ef893874001M7Xt4EuylK3212` | 200 / 0 | `chat.error` |

六轮均报告「未找到模型」，`chat.done`、模型答复和工具事件均为零。summary 回读均为 `acp-aionrs / aionrs`、`full_access=true`，ACP options 均为 `connected=false`，无 `selected_model`。因此三种图谱工具的输出、答复准确性、撤除后的模型行为、实际工具审批均未取得证据，HTTP 轮次也未确认桥接收到 `session/new`。现有开发网关的 aionrs 预设处于停用状态，原租户配置未变更。

## ACP 独立探针与权限边界

- 固定桥接的 `initialize` 返回 ACP v1，`loadSession=false`、`mcpCapabilities.http=false`、`mcpCapabilities.sse=false`。随后实际发送新会话请求，其中 `mcpServers` 包含同一 stdio RustCodeGraph 服务。桥接返回 `-32602` 和 `MCP servers are not supported by this backend`，拒绝发生在引擎启动前。
- `aionrs_acp.rs` 使用 `AionrsBackend`。公共入站解析保留 `mcpServers`，由默认 `create_session_with_mcp` 拒绝非空列表。原生 0.2.11 有第一条消息前的 `AddMcpServer` 入口。原生 stdio 启动会继承引擎环境，受管沙箱同时禁止派生子进程。受管远程传输的网络权限仅限已绑定的模型中继端口，当前驱动没有会话级 MCP 代理。桥接工具审批限定工作区内的 `Read`、`Write`、`Edit`、`Glob`。
- 保留原有门禁。复验需恢复隔离环境下相同租户的有效模型绑定，并提供保持会话隔离、审批、撤销和进程生命周期的 MCP 转发。两台私有网关均退出 0，监听端口关闭，私有运行状态已清理。
- 本次提交仅含证据文档。未运行 Cargo 或桌面构建。任务所列 `vw-agent` 测试覆盖 ShiJu 原生会话工具与技能挂载，本次定位的拒绝在独立 aionrs 桥接。

## 共享工作区复验

复用 Snow 验收从共享工作区构建的 shiju 0.6.0，SHA-256 为 `e755e458b4fce6ed045ef10a7c077a0d7dfb98d69637442d9c4449d0d50923ac`，及上列同一受审桥接和引擎。通过本机开发网关读取已授权的模型定义，移除内嵌 API key 后写入私有网关。凭据从现有授权文件经标准输入送入私有网关的授权接口，私有预设单独通过全局配置接口启用。有效配置回读确认同一租户和模型。工作区、提示摘要和请求权限沿用上文。

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 终态 | 工具事件 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1eff957be001t2Yy7e5wTZeoLz` | 200 / 0 / 4.305 | `chat.done` ×1 | 0 |
| 挂载 | `ses_1eff968d5001XgrFzto3FLHwNl` | 200 / 0 / 1.568 | `chat.error` ×1 | 0 |
| 撤除 | `ses_1eff96f3d001paGM3esrE7L5C4` | 200 / 0 / 5.611 | `chat.done` ×1 | 0 |

两轮对照均输出 `MCP_NOT_AVAILABLE`，SSE 另带 `<think>` 段。三轮的 UI summary 均回读 `acp-aionrs / aionrs` 和 `full_access=true`，终态 ACP options 均断开。两轮对照回读精确的 `session_model` 绑定。挂载轮在建会话时报 `MCP servers are not supported by this backend`，无已选模型回读，未进入模型生成或工具审批。两轮对照均无 `chat.error`。私有网关退出 0，监听端口已关闭，网关状态和凭据副本已清理。

独立 ACP 探针分别提交 stdio、HTTP、SSE，均收到 `-32602`，相同拒绝理由。原生 JSON Stream 只有消息、审批、控制和挂载等命令，没有可把宿主执行结果注回原生推理的外部工具回调。现有模型中继在请求发送上游前还会过滤 aionrs 的工具表，只保留四个工作区内置工具；中继未注册 MCP HTTP 路由。Snow 的受管实现是该后端自有的工具运行层，未提供 aionrs 可调用的宿主转发。上游最新发布仍为固定版 0.2.11。[aionrs 发行页](https://github.com/iOfficeAI/aionrs/releases/tag/v0.2.11)

当前阻塞是挂载实现缺口。若保持引擎的派生进程和网络边界，需要受审的宿主 MCP 转发，以及中继按该会话已发现的工具名单放行；还需把 ACP 单次批准与实际执行绑定，验证撤销、进程回收和续轮结果。这涉及模型中继和独立桥接的共同改动，需要扩展当前限定的执行范围或提供已审入口。未改 Rust，未运行 Cargo。

## 同端口转发原型

原型从当前 aionrs 驱动读取原生引擎沙箱模板，保留派生进程禁令和单一回环端口。私有探针在同一端口分别处理 Chat Completions 和 Streamable HTTP MCP。受审原生引擎收到 `AddMcpServer` 后通过 HTTP 发现工具，RustCodeGraph 仍由宿主以 stdio 启动。探针使用临时令牌和本地模拟模型，图谱调用及续轮结果来自真实 RustCodeGraph。

| 场景 | 模型请求 | 原生审批 | 实际图谱调用 |
| --- | ---: | --- | --- |
| 无挂载 | 1 | 无 | 0 |
| 挂载并批准 | 4 | 单次批准 3 次 | query、callers、node 各 1 次 |
| 挂载并拒绝 | 2 | 拒绝 1 次 | 0 |
| 撤除后新会话 | 1 | 无 | 0 |

批准轮回传的结果依次命中 `createRueSiteApp`、调用者 `mountRueSiteApp` 和调用链源码。三个结果均出现在对应的模型续轮中。拒绝结果也进入续轮。同一令牌重放已消费的工具调用返回 HTTP 403，错误令牌返回 HTTP 403，无挂载会话的 MCP 路径返回 HTTP 404。四轮均收到原生 `stream_end`。

```bash
python3 patches/acp-model-source/aionrs/probe.py "$reviewed_engine" "$rustcodegraph" "$indexed_ruejs_copy"
```

这条路径可以复用原生引擎的 MCP 注册能力。正式接入仍需在 ShiJu 的桥接与中继之间绑定会话目录、已发现的工具和一次性审批，限制代理调用参数和进程归属，并在中继撤销时销毁 MCP 客户端。探针只验证固定工具及参数，未接入实际网关和指定阿里模型。未修改 Rust 或运行 Cargo。

## 受管 v2 补丁

`aionrs-acp` 新建会话时消费项目的 stdio MCP 声明，经模型中继的同一个回环端口向固定的 aionrs 0.2.11 提供 Streamable HTTP。引擎沙箱保留现有派生进程禁令和网络范围。桥接与模型中继之间使用私有 Unix socket 和独立能力令牌。模型中继保留原四个内置工具，仅额外放行已发现且通过项目工具策略的名称。

MCP 本地进程在清空环境后启动，网络和派生进程均被禁止。RustCodeGraph 使用无守护进程、无监视模式。已 checkpoint 的 SQLite 索引复制到运行时私有目录，源码在 MCP 进程中保持只读。模型调用先经过现有 ACP 单次审批，工具名和参数都相符的授权只消费一次。受管运行时目录升级为 `aionrs-0.2.11-model-source-v2`，原生引擎版本保持 0.2.11。

聚焦回归使用槽位 3，两个缓存路径均为仓库内 `target/rust-slot-3`。命令为 `python3 scripts/cargo_slot.py cargo test -j 1 --locked -p acp-server --lib aionrs -- --include-ignored --test-threads=1`。通过环境指向已索引 RueJS 副本和已安装的 RustCodeGraph。退出 0，共 5 项通过。真实工具测试覆盖项目允许与拒绝名单、query/callers/node 输出，以及逐个调用的无授权拒绝和授权已消费重放。

受管桥接的原生 ACP 补充探针覆盖无挂载、批准、拒绝和撤除四轮。使用固定的 aionrs 0.2.11、真实图谱和回环模型。通过 ACP 批准的三个调用分别产生成功工具终态，三个结果均进入模型续轮。拒绝轮消耗一次审批请求，图谱执行数为零，撤除轮的模型工具表无挂载工具。额外用指定模型直连此探针的挂载轮触发 155 秒探针上限，未用于判定网关验收。

正式构建在槽位 10 执行 `cargo build -j 1 --locked -p vw-acp --bin aionrs-acp -p vw-cli --bin shiju`，由仓库槽位脚本持有两个缓存目录至产物复制完成。两目录为仓库内 `target/rust-slot-10`。退出 0，用时 33 分 36 秒。网关 SHA-256 为 `b2a05ed113477712c36de9dd731517d27f151d4e51f7c8096c0e80ae2e27d276`，桥接为 `b2ab720e864645b1b0dc681fcf852873c501d3a8f19de7b9a1ffc2408c9a466a`。隔离实例复用前次受审引擎，配对该桥接并校验私有 v2 安装回执。

当前共享网关在 `2b35013ae` 移除了聊天请求的 `acp_agent` 字段。原字段导致三轮 HTTP 422，均在反序列化阶段结束，未发送模型请求。复验去掉该字段，由 `agent=acp-aionrs` 解析后端，提示、模型、租户、`session_model` 和权限保持相同。

### 指定模型正式网关三轮

按更新后的标准，官方工具可以沿用原权限共存，挂载工具必须实际执行并回传对应结果。本次项目 `allowedTools` 仅选择 `rustcodegraph_query`、`rustcodegraph_callers`、`rustcodegraph_node`。原生引擎仍可见原有工作区工具，本次模型只调用了挂载工具。

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 终态 | 图谱工具完成 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1f194b895001Wa5P9C76nXlVBD` | 200 / 0 / 4.796 | `chat.done` ×1 | 0 |
| 挂载 | `ses_1f194cbcb001cZAYuXpaJxQ7n7` | 200 / 0 / 201.171 | `chat.done` ×1 | 101 |
| 撤除 | `ses_1f197ddfb001lCU2h3bqJ9QEtE` | 200 / 0 / 4.734 | `chat.done` ×1 | 0 |

- 三轮均无 `chat.error`。UI summary 回读 `acp-aionrs / aionrs` 且 `full_access=true`。运行中 ACP options 为 `connected=true`、`access_status.support=client_only`，运行中与终态均确认指定模型的 `session_model` 绑定，终态断开。
- 挂载轮共有 70 个 query、8 个 callers 和 23 个 node 成功完成事件。每个完成事件都有同 ID 的开始事件和非空输出。有些查询正常返回无匹配。首组 query `call_2f59b5c71b3b4ed884b62708` 命中 `createRueSiteApp`（`app/app.tsx:32`），callers `call_804192c06b744592a9c2f5e8` 回传 `mountRueSiteApp`（`app/app.tsx:39`），node `call_6e01f945b7b14f57825c06be` 回传工厂源码。最终答复引用这些结果继续追到 `useApp` 和 `_$createApp`，没有其他工具事件。
- 按快照源码复核，答复准确覆盖 SSR 与浏览器入口、缺少 `#app` 的抛错、根工厂前安装路由与 i18n、`/rust-layers` 的布局绕过。答复把移除清空后残留 DOM 标为推断，给出首屏唯一性和卸载重挂两项验证。文中直接子节点恰好翻倍的表述取决于快照和新根结构，源码确认的是静态节点可能残留、新应用节点追加。本次未执行浏览器场景。
- 两轮对照均返回 `MCP_NOT_AVAILABLE`，无工具事件。无挂载轮附带解释，SSE 仍可见带标签的推理文本。网关另有一条后台标题模型 403，指定模型的三个聊天回合均正常完成。
- 私有网关退出 0、端口关闭，网关运行目录及其授权副本已清理，受管桥接、引擎及验收 MCP 进程均已退出。未运行桌面构建或全仓测试。新增的模型中继分片单测未单独执行，正式网关构建和三轮聊天已运行。
