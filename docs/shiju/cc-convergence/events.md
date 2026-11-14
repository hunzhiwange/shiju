# 事件发布边界

2026-11-14，任务14。按用户要求仅实现与静态审查，不运行任何 Cargo、构建、单元测试、实机或最终回归；本文不构成运行验收通过声明。

## 正式链路与唯一所有者

- 用量持久账本由 `crates/vw-agent-apps/src/usage/store.rs` 的 `save_usage_record_in_transaction` 在同一事务推进统计revision，`save_usage_record` 成功commit后返回。`crates/vw-agent/src/usage_bridge/events.rs::revisions` 只读已提交SQLite快照，不通过数据库写入hook发布未提交数据。
- 实时配额缓存由 `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_refresh.rs` 在重新授权、取消检查和cache.record后调用唯一的 `usage_bridge::events::cache_updated`。后者向容量256的进程内channel发送带ProjectScope、principal、host、app和provider的缓存失效通知；没有第二个等价发布点。
- `crates/vw-agent/src/gateway/api/handlers/usage/events.rs` 的 `/usage/events` 订阅每轮重新授权并核对本机host，统计revision只返回同ProjectScope/host，缓存通知还核对principal。初连与channel滞后设置recover，客户端应重新查询；这不是可靠逐条业务事件日志。
- `crates/vw-gateway-client/src/client/usage_scan_api.rs::stream_usage_revisions` 通过 `stream_json_sse` 消费，并核对type、project、tenant和host；无效事件停止消费。`crates/vw-gateway-client/src/stream.rs` 回调返回false即结束读取，不发起业务写入。重连建立新的恢复快照，不重放业务mutation。
- Iced当前 `crates/vw-desktop/src/app/views/usage/center/mod.rs::update` 的Refresh通过SDK重读summary、logs、charts与groups。静态搜索未找到桌面调用stream_usage_revisions，本文不声称桌面已消费该SSE或验证了实时刷新；本轮不补做母20的界面接线。

## 保留边界与零删除

`crates/vw-agent/src/bus/` 是通用进程事件分发，GatewayState.event_tx是网关通用广播；领域任务事件/查询负责任务进度与恢复。它们不等价于带应用、执行主机和租户权限的用量失效通知，不能强合成一个全局发布器。用量统计revision与配额缓存revision也不能互换：前者来自持久事务，后者对应已授权缓存结果。

旧 `crates/vw-agent-apps/upstream/src-tauri/src/database/mod.rs::register_db_change_hook`、`upstream/src/hooks/useUsageEventBridge.ts` 与 `useTauriEvent.ts` 是来源参考。正式 `crates/vw-agent-apps/src/lib.rs` 未包含upstream模块，正式 `crates/vw-agent-apps/src` 与 `crates/vw-agent/src` 搜索register_db_change_hook无匹配。本域没有需切换后删除的等价运行副本，最小删除集合为空；来源快照整体归档/移除由任务21、22负责。

## 静态证据与待验证项

- rustcodegraph status退出0，7988个文件。explore存在同名publish噪声，精确node提示usage/events.rs未索引，改用rg与文件读取，不将图中同名边当真实调用证明。
- `rg -n cache_updated crates/vw-agent/src` 退出0，仅定义及usage_refresh正式调用；`rg -n register_db_change_hook crates/vw-agent-apps/src crates/vw-agent/src` 退出1，无正式调用。
- 已读取事件源、SSE、SDK、store事务和Iced刷新路径，未操作数据库、真实账号、凭据或用户目录。没有生成脱敏夹具或前后数据哈希，不声称提交失败/重连/取消/租户隔离已通过行为测试。
- 现有相关测试：`crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_refresh_tests.rs`、`usage_read_tests.rs`、`crates/vw-gateway-client/src/stream_tests.rs`；本轮未运行，未新增字符串匹配测试。一次发布、断线重读、取消资源释放及跨租户隔离留待统一验证。

无运行代码改动、无数据切换或双写。恢复基线见[准入与所有者](准入与所有者.md)，逆转本任务证据文档提交即可；不覆盖其他任务变更。
