# 任务 23：组合应用 SDK 合同

2026-11-14，实现完成，未验证。任务 22 的 `client/project_profile_api.rs` 已提供维护 SDK；任务 21 的 `project_profiles_execution.rs` 已注册执行路由。沿用同一 SDK 所有者、ProjectProfileApiScope、版本前提及结构化错误，不新增权限、存储或操作注册表。

GatewayClient 方法与 `/v1/agent-apps/project-profiles` 下的路由：

| 方法 | HTTP / 后缀 | 返回 |
| --- | --- | --- |
| project_profile_preview | POST /{id}/preview | JSON，当前服务端 503 profile_observation_unavailable |
| project_profile_apply | POST /{id}/apply | JSON，当前服务端 503 profile_execution_unavailable |
| project_profile_progress | GET /operations/{operation_id} | JSON，当前服务端 404 profile_operation_not_found |
| project_profile_result | GET /current/result | ProjectProfileStoredResult |
| project_profile_cancel | POST /operations/{operation_id}/cancel | JSON，当前服务端 404 profile_operation_not_found |

preview/apply 的 body 为 projectId、targetHostId、scope、expectedVersion；取消 body 只有上下文；读取使用 query。租户与 endpoint 不一致、空主机和无效路径 ID 在请求前拒绝；ID 作为单独路径段编码。全部方法使用已有 native_transport，禁止重试与重定向，沿 endpoint 认证发送请求；传输、HTTP 状态/响应、解码和参数错误分别返回。

断线后由调用者重新建立连接并调用 result 或 progress，仅执行读取，不隐式调用 apply。apply 传输失败表示结果未知，不能推导“没有执行”；result 的 null 也不能作为安全重发依据。current/result 是当前项目/主机/组的最近持久结果，并非特定操作收据，调用者应核对 profileId/group；需要操作级恢复时，仍等待执行端注册表和成功契约交付。

持久结果复用 ProjectProfileApplyResult，完整保留 selected、整体 applied/partial/uncaptured/cancelled/failed 和各项状态，不将 HTTP 成功或 selected 映射成 applied。预览、启动、进度、取消尚无服务端成功 wire DTO，暂返回 serde_json::Value，不能声称已有完整异步操作 SDK；后续由执行 API 所有者冻结成功 DTO 后接入强类型解码，不在客户端猜测 operationId、取消确认或进度形状。当前 503/404 直接返回结构化错误，不伪造成功。

按用户本轮覆盖仅写代码及必需契约，未新增或运行测试，未执行任何 Cargo、构建、检查、实机测试或最终回归。未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置，无测试通过数或退出码。仅静态阅读了来源 profiles.ts、任务 21 路由、任务 22 SDK、共享结果 DTO 及 native_transport；不构成编译、行为或 macOS 验收通过证据。编号卡按本轮要求删除，plan.md 未修改。

恢复验证后补 profile_t23_ 本地 HTTP 场景：apply 连接中断后写请求恰一次、重连结果仅 GET、partial/uncaptured/cancelled 和成功项无损解码、404/503 保留、参数拒绝零请求与路径编码；使用仓库 cargo_slot.py 执行聚焦验证。生产闭环仍需执行 API 接入真实观察、领域应用及操作注册表。
