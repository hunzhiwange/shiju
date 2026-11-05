# 动态工具运行链路

动态工具沿现有工具执行器运行，共用允许列表、审批、Hook 与 ToolEvent。稳定工具 ID 用于授权，结构化 identity 用于协议表达；注册规格不会绕过执行权限。模型请求持有规格和执行器的同一版本快照，调用期间替换目录不会改变已经发出的调用，下一次模型请求才使用新目录。

## 原生宿主接口

`vw-gateway-client::GatewayClient` 提供四个原生操作：

| 操作 | HTTP 路径（网关前缀 `/v1`） | 客户端方法 |
| --- | --- | --- |
| 注册宿主 | `POST /session/{id}/dynamic-tools/hosts` | `register_dynamic_tool_host` |
| 替换完整目录 | `PUT /session/{id}/dynamic-tools` | `replace_dynamic_tools` |
| 订阅调用 | `GET /session/{id}/dynamic-tools/calls` | `stream_dynamic_tool_calls` |
| 回应一次调用 | `POST /session/{id}/dynamic-tools/calls/{call}/response` | `respond_dynamic_tool_call` |

请求沿用网关认证、租户 header 和 directory 范围。服务器生成宿主 ID 和绑定秘密；绑定秘密仅放入 `x-vibewindow-tool-host` header，不放入 URL、日志或数据库。同一会话不允许抢占宿主或重复附着调用流。其他租户、目录或会话不能使用该绑定；分叉不能复用来源会话的绑定或调用 ID。

SSE 发送类型化 `dynamic_tool.call`，包括调用 ID、轮次 ID、工具身份和参数。回应支持文本、图片和音频内容。重复或取消后的回应失败。客户端不重连、不重放调用、不重试结果提交；结果提交遇到传输错误意味着结果未知，不能盲目重试。

关闭 SSE 连接会取消等待中的调用、移除宿主并使旧绑定失效。宿主需要重新注册，再提交当前规格。客户端取消执行 future 会移除该调用的等待状态，晚到的结果不能使它重新生效。

## 持久策略和恢复

默认拒绝 `persist: true`。请求体、模型参数和客户端自报能力不能开启持久化。只有受信 Rust 运行时在服务器装配时注入 `axum::Extension(PersistentToolPolicy::trusted_host())`，该路由接收的宿主才获得持久更新策略；普通启动路径不注入该策略。策略在注册时绑定到宿主，后续请求不能自行升级。

持久替换复用 `SessionStore::replace_dynamic_tools`：校验完整规格，在目录写锁内先提交数据库，再发布新快照。SQLite 和 PostgreSQL 使用同一作用域键和规格记录；持久记录只含版本及规格，不含执行器、授权、绑定秘密、连接或未完成回调。显式持久空目录也保存，不能将空目录当作从未注册。

原生会话可能先于 UI 投影存在。首次持久更新只在缺少投影索引时建立空会话头，不覆盖已有 UI 或历史数据。

恢复过程如下：

1. 验证租户、目录、项目和会话归属，加载保存的规格与版本。恢复历史中的 pending/running 动态调用会被标为中断，不重放副作用。
2. 宿主重新认证，取得新的随机绑定。目录保留保存的版本，但没有可执行工具。
3. 宿主通过替换接口提交匹配的当前规格，服务器建立新的 ClientTool 执行器。首次非持久重绑定必须匹配保存的规格；持久改变规格仍需要受信策略。执行仍受本轮允许列表、审批与 Hook 约束。
4. 分叉仅复制规格。分叉会话需要独立注册、重绑定及授权，来源会话的传输和秘密不会复制。

非持久替换只影响当前宿主；要让撤销在恢复后保留，应提交受信的持久空目录。

## 发现与兼容边界

延迟工具由 `tool_search` 搜索，命中的工具在下一模型请求出现。同一请求里的搜索不能给同批直接调用或 batch 内调用补授权。替换和撤销清除旧发现权限，工具结果不能恢复权限。普通 MCP 工具继续走原有缓存、通知刷新与执行通道；通知不授予新权限。文本工具协议沿现有实现处理。

这里实现的是识矩原生 HTTP/SSE 接口，不是完整 Codex app-server JSON-RPC，也不兼容其全部操作、事件或旧持久协议。Code Mode 及新增配置界面不属于此宿主接口；模型原生搜索协议的范围见下文。

## 验证范围

`session/dynamic_tool_acceptance_tests.rs` 使用真实回环网关、Rust 客户端、SQLite、目录和执行器；只在模型输出边界脚本化。覆盖持久替换后重新注册/重绑定、搜索后调用、调用中替换、取消、撤销及跨租户/会话/分叉拒绝。既有 `codex_discovery_` 与 MCP 安全测试继续保留。

本次执行平台为原生 macOS arm64。完整网关闭环使用 SQLite；PostgreSQL 通过专用本地临时服务验证持久化及原子索引补建，未覆盖完整网关闭环。其他发行平台、WASM 与完整 Codex 协议未验证。确切运行结果以任务交付时的测试与 clippy 日志为准。

## 模型原生搜索与普通函数发现

仅显式声明 `native_tool_search: true`、支持工具调用且实际使用 Responses 端点的模型启用原生路径。能力未知、关闭或端点不匹配时，`tool_search` 保持普通函数，完成同样的“搜索、下一请求加载、执行”任务。主循环与桌面处理器共用目录授权和执行器，协议解析不授予权限。

原生路径使用类型化 `tool_search_call`、`tool_search_output` 和 `namespace`，不把搜索结果伪装成普通函数结果。以下是离线测试保存的净化 fixture 形状；无真实凭据或业务数据：

```json
{"type":"tool_search_call","call_id":"search-call","execution":"client","status":"completed","arguments":{"query":"ledger"}}
{"type":"tool_search_output","call_id":"search-call","execution":"client","status":"completed","tools":[{"type":"namespace","name":"fixture","description":"","tools":[{"type":"function","name":"ledger","description":"Read ledger","parameters":{"type":"object","properties":{},"additionalProperties":false}}]}]}
{"type":"function_call","call_id":"read-call","namespace":"fixture","name":"ledger","arguments":"{}"}
```

搜索结果只来自当前授权目录；同批调用、未知身份、伪造搜索参数、服务端执行模式和未声明的原生搜索均不能扩大权限。搜索历史保留成对调用与专用结果及命名空间，用于回放和压缩；恢复历史不会恢复已加载工具权限。目录撤销后，旧 schema 仍是历史记录，不能进入新请求的工具声明或授权执行。

`session/llm/aisdk/native_search_acceptance_tests.rs` 使用真实回环 Responses HTTP/SSE、请求转换、临时文件和实际主循环/桌面执行器，覆盖两条发现路径、拒绝调用、历史恢复及撤销。仅模型输出由确定性 fixture 提供，不需要真实 API key。`native_tool_search_`、`dynamic_tools_`、`codex_discovery_` 与 SDK 原生协议过滤测试提供相邻回归证据。

协议依据本地 Codex 参考源码和已锁定 SDK 类型实现，不代表任意线上 provider 已支持该协议。此次验收只针对原生 macOS；未验证线上 provider、Linux、Windows、WASM 或完整 Codex app-server 兼容性。
