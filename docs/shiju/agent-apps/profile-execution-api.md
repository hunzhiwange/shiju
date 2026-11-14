# 项目组合预览、应用与结果 API（任务 21）

2026-11-14：仅代码和契约，未经编译或行为验证。直接依赖静态存在：任务 10 preview.rs 的纯差异计算、任务 20 维护路由及策略、任务 18 ProfileApplyOperation 的领域进度与安全取消句柄。原生观察、逐领域授权及完整执行 worker 尚未交付给生产 API；本次仅落地安全边界，完整预览、启动与操作进度/取消闭环尚未完成。

路径均位于 `/v1/agent-apps/project-profiles`，沿生产 router 注册，使用既有大小限制和 no-store 响应：

| 方法及后缀 | 输入 | 当前行为 |
| --- | --- | --- |
| POST `/{id}/preview` | projectId、targetHostId、scope、expectedVersion | 项目/执行端授权、槽归属、实体和版本检查后返回 503 profile_observation_unavailable；不合成空观察 |
| POST `/{id}/apply` | 同上 | 维护权限检查及上述检查后返回 503 profile_execution_unavailable；不改 selected，不创建虚假操作 |
| GET `/current/result` | projectId、targetHostId、scope 查询参数 | 返回 `{result: null}` 或存储的 ProjectProfileApplyResult；不把 selected 当 applied |
| GET `/operations/{operation_id}` | 同上查询参数 | 每次授权后 404 profile_operation_not_found |
| POST `/operations/{operation_id}/cancel` | projectId、targetHostId、scope | 每次维护授权后 404 profile_operation_not_found；不伪造取消确认 |

请求拒绝未知字段，客户端不能上传 payload、完成状态、结果或执行端目录。租户只取鉴权扩展，主机只取策略返回的执行端身份；策略沿现有所有者重新读取，权限撤销后不得靠旧 ID 继续访问。目标不存在返回 404，版本冲突 409，存储或执行器缺失 503，领域/槽状态异常返回脱敏的 409。结果读取在 with_operation 内复用 current/application_result 的项目槽占用核对，不提供跨项目查询。

后续开放成功路径的前提：由领域所有者提供同项目/主机/组的真实有效状态读取与逐项授权；应用在同一个 with_operation worker 内重新校验版本，完成 prepare_switch、四类领域调用和最终持久化。操作注册表必须绑定租户、项目、可信执行端及组，存储每次操作自己的最终结果，查询和取消重新授权；不得把最近组结果冒充历史操作结果。取消仅发送句柄信号，等待安全边界与持久化完成后才进入终态。当前未接注册表，因此所有操作 ID 均视为不存在。

按本轮要求未编写或执行测试，未执行 Cargo、构建、检查、实机测试或最终回归；没有通过证据。未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。以后补充 profile_t21_ 的只读零写入、授权与撤权、伪造字段拒绝、版本冲突、租户/项目/主机隔离及真实最终结果与取消场景，再做定向验证。编号卡按用户要求删除，不代表上述闭环已验证或实现。
