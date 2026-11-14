# macOS API 交付说明（未验证）

2026-11-14。本轮交付的是源码接口说明，不是 API 验收通过声明。按用户最新指令，构建、单元测试、实机测试、验收及最终回归均未执行，格式和链接验证也延期。任务卡删除不代表产品行为通过。

[运行契约](runtime-contract.json) 状态为 `prepared_unverified_not_executable`，`verified_entrypoints.api.value` 为 null。尚无绑定冻结产物的成功请求，不能从下列源码路径推定部署可用。记录见 [任务56持久记录](evidence/56.json)。仅 macOS 范围，ARM64/x64 均未验证。

## 接入与安全边界

下表采用既有文档中的 `/v1` 前缀；实际网关地址、部署前缀、认证配置和冻结包须由交付方提供，不提供猜测启动命令。Iced 经 SDK 调网关，不能据此推导任意 Tauri command 调用入口。

应用读取需要网关认证及应用主机授权两层边界。管理员登记的应用凭证经 `x-agent-apps-token` 传送，策略存摘要；平台、公司、主机 grant 必须匹配。租户上下文不等于主机管理权，读取权限也不等于写入或启动权限。项目组合另取 projectId、targetHostId、scope，授权返回的执行端身份用于操作，查询和取消也重新授权。部署说明参考[应用读取 API](../agent-apps/task-33.md)及[本机初始化](../agent-apps/local-setup.md)，其中历史测试没有在本轮复核，旧初始化描述应以 local-setup 的后续规则为准。

不得把真实凭证、授权文件、用户路径或供应商密钥复制到交付记录。下例仅为请求形状，不是可执行或已验收请求；省略认证头与服务地址，不包含凭据。

## 读取应用

| 方法与路径 | 输入及源码声明 |
| --- | --- |
| GET `/v1/agent-apps/catalog` | targetHostId；返回 AppCatalogResponse |
| GET `/v1/agent-apps/installations` | targetHostId、appId；返回 AppReadResponse |
| GET `/v1/agent-apps/diagnostics` | 同上 |
| GET `/v1/agent-apps/versions` | 同上 |

查询使用 camelCase 并拒绝未知字段。读取 handler 在获取授权后才调领域读取。源码入口：[read.rs](../../../crates/vw-agent/src/gateway/api/handlers/agent_apps/read.rs)，DTO：[agent_apps](../../../crates/vw-api-types/src/agent_apps/mod.rs)。以上均为静态接口参考，未取得本轮成功响应。

## 应用供应商

源码根路由 `/v1/app-providers/{app}/{id}` 注册 GET、PUT、POST、PATCH，body 上限 16 KiB；其他子路由由该模块 merge。它与模型供应商入口独立。本说明不列未逐项读取的子路由为已交付能力。

AppProviderWrite 使用 name、protocol、apiKey、baseUrl、model 及可选 requestOptions；protocol 为 anthropic、openai_responses 或 gemini。这是写入敏感字段的 schema，不应作为响应或日志内容。AppProviderSummary 使用 id、appId、name、configured。不要从保存接口推断已激活或已写入原生应用。

根模块错误返回安全 `{message: ...}`：非法输入 400、未找到 404、引用/版本冲突 409、存储不可用 503；不能宣称所有供应商子接口采用统一错误体。参考[handler](../../../crates/vw-agent/src/gateway/api/handlers/app_providers.rs)及[DTO](../../../crates/vw-api-types/src/app_providers.rs)。运行契约中 provider-contracts 的历史 GET/PUT 描述不覆盖现有全部注册方法，须后续核证。

## 项目组合：进度与取消的实际限制

统一前缀 `/v1/agent-apps/project-profiles`。输入 scope 限 claude、claude-desktop、codex。preview/apply 请求形状：

```json
{"projectId":"<已授权项目ID>","targetHostId":"<已登记主机ID>","scope":"claude","expectedVersion":1}
```

| 方法及后缀 | 当前源码分支（未经运行） |
| --- | --- |
| POST `/{id}/preview` | 完成权限、实体及版本检查后 503 profile_observation_unavailable |
| POST `/{id}/apply` | 完成维护权限等检查后 503 profile_execution_unavailable |
| GET `/current/result` | query 为 projectId、targetHostId、scope；读取 result，可为 null |
| GET `/operations/{operation_id}` | 同上 query；授权后 404 profile_operation_not_found |
| POST `/operations/{operation_id}/cancel` | body 为上述三个上下文字段；维护授权后 404 profile_operation_not_found |

这些 503/404 表示当前观察、执行与操作注册表缺口，不能展示为应用成功、进度完成或取消确认。错误体为 `{error:{code,message}}`，响应采用 no-store；非法请求 400、缺实体 404、版本冲突 409，其他授权或存储错误保留各自状态。current/result 只是当前槽的持久结果，null 不能证明写入未发生。不能以 selected 代替 applied；调用结果未知时不能自动重发 apply。

依据：[执行 handler](../../../crates/vw-agent/src/gateway/api/handlers/project_profiles_execution.rs)、[父路由与错误体](../../../crates/vw-agent/src/gateway/api/handlers/project_profiles.rs)、[schema](../../../crates/vw-api-types/src/project_profiles.rs)、[领域交付缺口](../agent-apps/profile-execution-api.md)、[SDK恢复限制](../agent-apps/profile-apply-sdk.md)。没有真实成功 wire DTO 的路径不编造 operationId 或取消响应。

## 后续统一验证与交付条件

母任务2及各领域所有者需提供绑定冻结版本的 API/SDK 成功契约、租户/项目/主机授权资源及真实响应；项目组合执行所有者需交付观察、领域 worker 和操作注册表。缺这些资源与成功路径，不代表上游不支持，也不以文档补齐实现。

以后统一核对本页源码与部署路由、schema、认证及逐领域错误；补充脱敏请求/响应、拒绝与撤权、进度恢复和安全取消的实际观察。每条产品行为须关联有效用例、退出码、提交、产物哈希、macOS版本及架构；历史记录需核对适用性。随后检查 JSON、链接和示例无凭据，才可改为已验收说明。本轮无运行进程、账号或临时策略需要清理。
