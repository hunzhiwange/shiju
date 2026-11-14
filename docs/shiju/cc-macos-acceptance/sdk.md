# macOS Rust SDK 使用说明

状态：说明已编写，未验证（2026-11-14）。本轮按指令不构建、不运行测试、不做验收或回归。下列用法来自源码阅读，不能作为已通过的运行故事。范围限现有 Rust SDK，不覆盖浏览器或其它平台。

## 前置条件与证据边界

先取得授权的网关端点、租户上下文、执行主机引用及按能力授予的凭据。[运行契约](runtime-contract.json) 的 `verified_entrypoints.sdk.value` 当前为 null，状态 unverified；未交付绑定冻结产品树的成功示例、构建哈希及隔离主机。[任务记录](evidence/57.json) 保存缺口与延期事项。前置[任务 1 记录](evidence/01.json)属于准备材料，不能替代 SDK 行为证据。

本说明不新增 Rust 示例或编译命令。任务要求复用前置任务已编译、已验收的示例，但目前没有此类输入；待所有者交付后再补充可复现示例与对应命令。

## 构造客户端

使用现有 `GatewayEndpoint` 作为 `GatewayClient::new(endpoint)` 的输入，处理 `Result<Self, String>` 后复用客户端。构造失败应向调用方返回错误；不要继续发请求。端点必须来自宿主已授权的连接配置，不在示例中硬编码密码、令牌或个人路径。

[构造入口](../../../crates/vw-gateway-client/src/client/mod.rs) 中，原生请求客户端设置超时；执行主机传输另行禁用重定向及重试。本轮未验证实际网络行为，不能据此推导所有 SDK 方法使用同一传输策略。

## 认证上下文

偏好请求在所选端点下发送，并携带 `targetHostId` 查询参数，经 `apply_request_context` 应用端点上下文。使用 `agent_apps_preferences_authorized` / `agent_apps_preferences_update_authorized` 时，额外传入独立授予的应用凭据，SDK 放入 `x-agent-apps-token`。普通版本不会补充该凭据，不能靠切换方法绕过服务端授权。[源码入口](../../../crates/vw-gateway-client/src/client/agent_apps_settings_api.rs)。

授权管理接口使用另一组明确参数：`AgentAppsAuthCredentials` 包含 `username`、`password`、`principal_token`。两项 Basic 字段同时为空时不附加 Basic；主体令牌单独放入应用凭据头。该路径克隆端点并清除常规网关认证，保留端点租户上下文，再应用该组认证信息。不要将普通网关令牌直接当成可信主体令牌；权限由网关判定。[凭据定义](../../../crates/vw-gateway-client/src/client/agent_apps_auth_read.rs)、[授权请求实现](../../../crates/vw-gateway-client/src/client/agent_apps_auth_flow.rs)。

## 一次读取与一次写入

以下是调用顺序说明，尚无当前运行证据：

1. 读取：在已授权的执行主机上，等待 `agent_apps_preferences_authorized(host, token)`，取得 `AppPreferencesResult`。保留响应作为 UI 当前值，不从 SDK 所在机器的文件系统推测远端偏好。
2. 写入：按实际需要构造 `AppPreferencesPatch`，等待 `agent_apps_preferences_update_authorized(host, &request, token)`，成功后采用返回的 `AppPreferencesResult`。该入口发送 PATCH 到 `/v1/agent-apps/preferences`，携带同一目标主机。不要因连接断开而自动重放写入；结果不明时先重新读取。

方法签名、HTTP 方法和传输逻辑见[偏好 SDK](../../../crates/vw-gateway-client/src/client/agent_apps_settings_api.rs)。DTO 按该接口实际导入的 `vw_api_types::agent_apps` 类型构造，不在此猜测尚未验收的字段组合。

## 错误与取消

偏好方法返回 `AgentAppsSettingsError`：`Transport` 表示传输错误，`Http { status, body }` 保留网关失败响应，`Decode` 表示解码失败。调用方应显式匹配并显示脱敏反馈，避免直接记录完整响应体、端点认证信息或请求载荷。不能把所有 SDK 错误描述为统一稳定错误码。[定义](../../../crates/vw-gateway-client/src/client/agent_apps_settings_api.rs)。

授权流程由 `agent_apps_auth_start` 建立；保留网关返回的流程 ID，使用 `agent_apps_auth_poll` 读取，并按返回的 `retry_after_seconds` 安排下次轮询。需要取消时等待 `agent_apps_auth_cancel(host_id, service, &credentials, &flow_id)`：实现发送 DELETE，并仅在收到 204 后返回成功。停止本地等待不能等同于远端取消完成；该取消接口也不是偏好 PATCH 的撤销接口。授权错误 `AgentAppsAuthError` 区分输入、令牌、传输、HTTP 和解码错误，HTTP 分支可能带业务码与关联 ID。[授权流程实现](../../../crates/vw-gateway-client/src/client/agent_apps_auth_flow.rs)。

## 后续统一验证

先由 SDK/领域所有者补齐已验证入口及隔离资源，再绑定冻结产品树、独立构建产物哈希、macOS 架构与应用版本，复用其已验收示例。随后验证构造、上下文隔离、读取、写入、错误与取消的实际行为，补充有效用例数和观察记录；检查文档链接、凭据脱敏、JSON 格式及差异。本轮均延期，零行为用例不能计为通过。缺生产能力回交对应领域所有者，说明文档不替代能力实现。
