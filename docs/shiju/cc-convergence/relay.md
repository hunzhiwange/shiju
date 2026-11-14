# 原生代理与会话 relay 边界

2026-11-14，任务16。本轮按用户要求仅实现与静态审查，不运行任何 Cargo、构建、单元测试、实机或最终回归。本文不表示当前版本已通过行为验收。

## 唯一所有者及保留边界

|职责|真实入口与调用链|保留的权限边界|
|---|---|---|
|会话模型能力|`crates/vw-agent/src/session/llm/acp/model_source/profile.rs` → `RelayBinding::for_prompt`；`model_source/mod.rs` → `RelayServer::start`|绑定 tenant、project、session、provider、model、generation；独立回环监听与能力令牌，不使用管理API认证替代|
|会话令牌核验与撤销|`model_relay/binding.rs` 的 `issue/accepts/revoke`；`server.rs` 的 `handle/shutdown/Drop`|HMAC核验、完整身份相等、过期/取消检查；所有者退出撤销并取消在途请求，不从请求头重建身份|
|常驻应用路由|`app_routing/route_auth.rs` 的 `VerifiedRouteBinding/AuthorizedRoute` → `dispatch::forward_json`|绑定 ProjectScope、host、app、account及account_generation；模式事务提供证明，保留应用路由容量、出口、失效及账号权限上下文|
|共享协议适配|`app_routing/dispatch.rs` → `model_relay::{native_responses,native_gemini,messages_responses_request,messages_chat_request,messages_gemini_stream}` 等|转换复用同一生产模块；各入口独立校验路径、模型和引用作用域，不能复用某入口的已授权身份绕过另一个入口|
|共享官方账号传输|同一dispatch调用 `model_relay::{codex_oauth_transport,xai_oauth_transport,gemini_oauth_transport}`|传输适配复用，但账号选择与发布证明由当前应用路由所有者提供；普通请求不能借会话令牌扩权|

`RelayServer` 只监听 `127.0.0.1` 随机端口，每个能力拥有独立listener；`RelayRequest` 仅将协议路径与正文送到backend，不传入客户端URL、头或凭据。会话支持的路由、正文限额、速率和并发限制与常驻应用路由不同，不应为减少文件而合并server或binding。

常驻路径先核对provider、应用和支持的方法/路径，再处理正文与上游；例如不支持的 Messages count_tokens 明确返回未实现，Responses WebSocket握手明确返回426。保留这些失败行为，不能将会话入口当作通用代理回退。共享网络构造边界另见 [network.md](network.md)。

## 零删除结论

本域最小删除集合为空。正式常驻路由已经调用 `model_relay` 的协议/官方账号适配，而会话relay保留不同身份与生命周期。未发现同权限、同作用域的第二运行转换实现需要切换；不添加新的抽象、配置或兼容回退，也不制造字符串匹配业务测试。

来源 `crates/vw-agent-apps/upstream/src-tauri/src/services/proxy.rs`、`src/proxy/` 与 `upstream/src/lib/api/proxy.ts` 仍是参考快照。正式 `vw-agent-apps/src/lib.rs` 未将来源运行树作为模块接入，桌面为Iced SDK链路；来源完整归档与删除交给任务21、22，不能在本域先拆除许可或来源文件。

## 静态证据与后续验证

- `rustcodegraph status` 退出0，7988个索引文件；广泛explore有同名噪声，后以精确file node核对 `model_relay/server.rs`、`binding.rs`、`app_routing/dispatch.rs` 与 `route_auth.rs` 的实现，不将同名图边视作完整调用证明。
- `rg -n 'RelayServer::start|RelayBinding::(new|for_prompt)' crates/vw-agent/src --glob '*.rs' --glob '!**/*tests*'` 退出0，正式会话入口为上述ACP model_source；不把测试启动当生产调用者。
- `rg -n 'model_relay::' crates/vw-agent/src/app_routing/dispatch.rs` 退出0，确认共享请求转换、JSON/SSE与账号传输引用。
- 现有行为测试入口：`crates/vw-agent/src/model_relay/transport_tests.rs`、`crates/vw-agent/src/app_routing/route_auth_tests.rs`、`reference_scope_tests.rs`。本轮未执行，不宣称认证隔离、协议JSON/SSE、撤销或实机行为已通过。
- 未写用户配置、凭据或业务库；无双写、数据迁移、脱敏夹具前后对照结果。未领取构建槽位，没有有效测试数。后续统一验证应覆盖不同租户/项目/账号/代次令牌不能交叉使用、普通请求不能借会话路由扩权及取消后不再访问上游。

恢复基线沿用 [准入与所有者](准入与所有者.md)。本任务仅新增本文，无运行切换；恢复时只逆转本任务文档提交，不覆盖其他任务或原工作区改动。
