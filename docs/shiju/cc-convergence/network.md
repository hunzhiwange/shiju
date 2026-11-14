# 网络客户端构造边界

2026-11-14，任务12。本轮按用户要求仅实现与静态审查，不运行任何 Cargo、构建、单元测试、实机或最终回归。本文不是运行验收通过声明。

## 真实所有者与保留边界

|职责|实际入口与调用者|保留理由|
|---|---|---|
|Iced到网关管理请求|`crates/vw-desktop/src/app/config_gateway.rs` → `GatewayClient::new`（`crates/vw-gateway-client/src/client/mod.rs`）→各业务SDK|端点、认证和租户请求上下文属于SDK，不与上游模型凭据混用|
|受控原生请求|同一 `GatewayClient::new` 构造 `dynamic_host_client`，`native_transport` 返回该客户端|共用默认超时与回环no_proxy规则，但强制不重定向、不重试；不能被普通管理客户端替代|
|模型relay上游|`crates/vw-agent/src/model_relay/transport.rs` 的 `ProviderRelayBackend::new` → `upstream::upstream_http_client`|独立模型授权目的地、上游凭据与取消语义，不挂管理API|
|常驻应用路由上游|`crates/vw-agent/src/app_routing/shared_transport.rs` 的 `SharedTransport` → 同一 `model_relay/upstream.rs` builder|系统/显式代理、直连、编码保留及探测属于已授权路由的不同出口；不能强并成一个全局客户端|
|会话LLM provider|`crates/vw-agent/src/providers/session_llm.rs` → `vw_gateway_client` 会话流|该目录未另建等价上游reqwest客户端，沿用网关会话协议|
|长流与阻塞请求|`client/task_pool_api.rs` 的流式构造、`client/request_helpers.rs` 的blocking构造|流式连接/请求超时和blocking执行模型不同，不用普通异步客户端替换|

`model_relay/upstream.rs` 统一禁止重定向、10秒连接超时，普通请求另加120秒总超时；流式/探测使用无总超时builder再施加本场景策略。`execute` 核对授权目的地、拒绝覆写Host，并响应取消；`response_body` 也观察取消。TLS沿用现有reqwest默认校验，本轮不修改证书、代理、授权或超时策略。

`SharedTransport::build_custom_proxy` 从公共builder派生后明确no_proxy、设置指定代理、关闭重试；macOS保留DirectClients及系统代理匹配，探测防止出口回指监听器。普通客户端与encoded客户端负责不同响应解码合同，不能仅因构造相似删除。

## 零删除依据

本域最小删除集合为空。正式模型relay、应用路由与诊断已经复用 `upstream_http_client_builder`；SDK普通与受控客户端也在同一工厂构造。没有发现同权限、同超时/代理/取消职责的第二运行副本需要切换。未新增配置、工厂抽象、兼容层或行为测试。

来源 `crates/vw-agent-apps/upstream/src-tauri/src/services/proxy.rs` 与 `upstream/src/lib/api/globalProxy.ts` 是来源快照，未接入正式Rust模块与Iced运行链；本任务不逐项删除来源，完整归档/移除由任务21、22负责。包是否存在快照与是否存在重复运行入口分开判断。

## 静态审查与后续验证

- `rustcodegraph status`：退出0，有7988个已索引文件；以精确文件node核对SDK工厂、relay backend、upstream及SharedTransport。宽泛explore有同名噪声，未把同名边当完整调用证明。
- `rg -n 'upstream_http_client' crates/vw-agent/src --glob '*.rs' --glob '!**/*tests*'`：退出0，生产调用包括relay、SharedTransport、voice与诊断执行器，均派生公共builder。
- `rg -n 'GatewayClient::new' crates/vw-desktop/src/app --glob '*.rs'`：退出0，桌面配置网关与会话加载消费SDK工厂。
- 现有聚焦测试位置：`crates/vw-agent/src/app_routing/shared_transport_tests.rs`（授权目的地、Host校验、取消和截断流）、`crates/vw-gateway-client/src/client/tests.rs`、`crates/vw-agent/src/model_relay/transport_tests.rs`。本轮均未执行，亦未新增字符串匹配测试。
- 未操作用户配置、凭据、业务库或脱敏夹具；无切换、双写与数据对照结果。macOS真实代理/TLS/取消行为留待统一验证；没有测试数或行为通过结论。

恢复基线沿用[准入与所有者](准入与所有者.md)。本任务只有本文，逆转本任务文档提交即可；运行代码、公共接口及数据没有变化，不覆盖其他任务或工作区已有修改。
