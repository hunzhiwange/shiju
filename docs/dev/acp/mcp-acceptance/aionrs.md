# aionrs 动态挂载 MCP 验收

2026-11-19，已阻塞。隔离网关在选模阶段停止。独立 ACP 探针另行确认受管桥接拒绝带 stdio MCP 的 `session/new`。[复用请求](../ACP验收.md#可复用的验收请求)

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
