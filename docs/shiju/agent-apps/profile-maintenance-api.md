# 项目组合维护 API（任务 20）

2026-11-14：仅实现、静态审阅，未经编译或行为验证。按本轮要求未运行 Cargo、构建、检查、单元测试、实机测试或最终回归；未领取 Cargo 槽位，CARGO_TARGET_DIR 未设置。

在任务 19 的 `/agent-apps/project-profiles` 路由上增加：

| 方法与路径后缀 | JSON 字段 | 结果 |
| --- | --- | --- |
| PATCH `/{id}` | projectId、targetHostId、scope、expectedVersion、name | 改名后从存储重新读取的组合 |
| DELETE `/{id}` | projectId、targetHostId、scope、expectedVersion | deleted；事务删除实体及所有引用标记 |
| POST `/{id}/resnapshot` | projectId、targetHostId、scope、expectedVersion | 当前明确返回 503 snapshot_unavailable，无写入 |
| POST `/current/clear` | projectId、targetHostId、scope | 重新读取的 profileId（null），不改原生配置 |

租户仅取鉴权扩展；每条路由均经既有主机/项目策略分派，并要求显式 `project_profiles_maintain: [projectId]` grant。读和创建权限不隐含维护权限。此授权允许修改租户共享组合，并允许删除时清理该组合在其他主机和项目的引用标记；应仅授予共享组合维护者。无配置、无匹配授权、remote 不支持等沿用既有拒绝行为。未知 JSON 字段拒绝，继承 16 KiB body 限制和 no-store 响应。实体维护要求目标存在且 expectedVersion 相符；跨项目清槽由存储拒绝。

直接依赖静态核对：rename.rs 提供任务 4 的版本化改名；profile_store.rs 提供任务 5 的事务删除、任务 6 的按项目清标记、任务 9 的版本化重拍；project_profiles.rs 提供任务 19 的生产路由和策略入口。快照读取器尚未在任务 19 接线，任务 20 同样拒绝重拍，不能用空集合或客户端载荷冒充当前状态。待真实读取适配和逐领域读授权交付后才能开放成功重拍；当前不声称完整维护闭环。

后续验证需覆盖各路由缺授权拒绝、创建权不可维护、租户隔离、版本冲突无写入、删除清引用、清槽跨项目拒绝和幂等、改名重读一致、重拍不可用零写入。未编写或运行测试；不得将静态审阅计作通过证据。
