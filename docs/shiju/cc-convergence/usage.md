# 用量与价格唯一入账路径

2026-11-14，任务10。本轮按用户指示不运行任何 Cargo、构建、单元测试、实机测试或最终回归。以下是静态链路核对，不是行为验证通过声明。

## 所有者与调用链

- 正式账本所有者为 `crates/vw-agent-apps/src/usage/store.rs`。`save_usage_record` 开启事务并调用 `save_usage_record_in_transaction`；扫描器通过同一事务入口保存记录与来源身份。`cursors.rs` 将游标和记录同事务提交，`snapshot_scan.rs` 也复用该入口。
- `crates/vw-agent/src/usage_bridge/{native,relay}.rs` 均调用该正式账本，连接来自 `usage_bridge/context.rs::database`，按执行主机 home 使用已有 GlobalDatabase。原生执行、路由日志、文件扫描是不同来源，不是三套独立账本，不能删除来源适配器。
- 账本键是结构化 `(ProjectScope, target_host)`；来源身份含 app、instance、文件身份、原生会话和请求标识。重复来源返回原记录，归档身份及 receipts 保留重放识别。`dedup.rs::find` 只在同授权作用域内以跨来源稳定请求标识与语义匹配关联，模糊匹配只标疑似；不把指纹相似当成直接丢账理由。
- 正式价格所有者为 `usage/pricing.rs`：`usage_model_prices` 按 `(ProjectScope, host)` 与 model_id 存储。`models_sync.rs`、`pricing_batch.rs` 复用价格合同，汇率由 `exchange_store.rs` 独立维护，查询余额缓存 `usage/cache.rs` 不拥有计费账本。
- 网关 `crates/vw-agent/src/gateway/api/handlers/usage/{read,prices,scan}.rs` 接受授权项目和目标主机；价格写操作额外核验维护权限。SDK 的 `usage_read_api.rs`、`usage_price_api.rs`、`usage_scan_api.rs` 供 Iced `crates/vw-desktop/src/app/views/usage/center/` 调用。桌面不直接写计费表。

## 删除边界

本域零运行代码删除。正式源码已经复用统一账本和价格存储，没有查到需要切换的同职责副本。来源 `crates/vw-agent-apps/upstream/src-tauri/src/services/{usage_stats,model_pricing,usage_cache}.rs` 及 DAO 是来源参考，不在正式包模块树；整体归档与来源树移除由任务21、22处理，不在此逐文件删除。平台推理统计、余额/套餐查询与正式应用账本的职责不同，不强合。

前置索引见 [准入与所有者](准入与所有者.md)，原有来源和合同见 `docs/vibewindow/usage/{coverage,dependencies}.md` 及 `docs/shiju/apps/usage/{coverage,contract}.md`。任务卡规划的旧入口目录未完整描述当前正式模块，以真实生产链路为准。

## 静态证据与后续验证

使用 RustCodeGraph status/explore 检查索引；查询主要命中旧来源，新增正式模块再以 rg 和源码读取核对。静态查询覆盖 `save_usage_record`、`save_usage_record_in_transaction`、价格入口及 SDK/Iced 调用。不存在需要新增生产后门或为清理制造测试的差异。

现有回归入口包含 `crates/vw-agent-apps/tests/usage_{claude_scan,gemini,kimi,logs,exchange}.rs` 。后续统一验证仍需确认同请求只记一次、路由与原生日志重叠、价格展示同源、跨租户隔离和失败无副作用。本轮未执行，未生成数据对照或恢复夹具，也不声称上述行为已通过。

恢复仅撤销本证据文件；没有生产切换、数据写入或迁移。保留当前工作区其他任务及桌面用量改动。删除任务卡遵循本轮用户只实现后删卡的覆盖指令，不代表完成运行验收。
