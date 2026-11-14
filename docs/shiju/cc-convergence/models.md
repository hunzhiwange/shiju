# 模型目录解析收敛

2026-11-14。任务3按用户要求仅作静态实现审查，未运行 Cargo、构建、单元测试或实机测试。以下不是当前版本行为验收通过声明。

## 当前权威链路

- SDK `crates/vw-gateway-client/src/client/model_catalog_api.rs` 的 `model_catalog`、`model_catalog_capabilities` 只访问网关；不维护第二套可变目录。
- 网关 `crates/vw-agent/src/gateway/api/handlers/model_catalog.rs::read` 先校验执行主机权限，再核对已注册项目及租户。`read_view` 经 `AppProviderStore::read_model_catalog` 读取指定部署；能力视图要求目录中存在精确模型 ID，否则返回 `NOT_FOUND`。
- `crates/vw-agent/src/provider/model_catalog.rs::read` 读取同租户的供应商记录，经 `models::edits::read` 投影目录；只允许相同已配置端点和受支持的运行适配器。`project_model` 与网关能力视图都调用 `vw-agent-apps::models::capabilities::resolve_model_capabilities`，均禁止无声明时借用确认注册表。
- `crates/vw-agent-apps/src/models/metadata.rs::resolve_model_metadata` 是原生补空元数据入口，消费 `vw-provider-resolver` 的快照。先匹配应用预设，再匹配相同端点元数据；供应商回退不借用价格。身份归一化只用于元数据匹配，不改运行请求 ID。
- resolver 的 `provider.rs` 直接重导出 `vw_shared::provider::state` 的状态解析。普通推理模型基线、已保存部署目录和只读补空预设职责不同，不能把它们强合为一个可变存储。
- Iced 读取目录、发现及补空的实际接点由 `docs/shiju/model-catalog/implementation-evidence.md` 任务40–43索引；桌面消费 SDK，不调用来源 Tauri 命令。

## 零删除与保留边界

本域未发现需要切换的等价运行副本，因此生产代码删除集合为空，不制造行为变化或字符串匹配测试。RustCodeGraph `explore` 显示来源 `resolveModelMetadata` 的四个调用者全部位于 `upstream` 的组件、hook或测试；来源 `get_model_catalog`、`set_model_catalog` 调用者也在来源树内部。正式 `crates/vw-agent-apps/src/lib.rs` 明确该快照不属于模块树。来源整体归档/移除由任务21、22处理，本任务不局部破坏快照。

保留原生应用目录与普通运行供应商基线、发现任务与显式保存、租户和执行主机授权边界。未知模型在目录能力入口返回明确错误；补空入口无元数据时返回 `None`，不能以推理模型基线兜底伪造部署能力。

## 静态证据与后续验证

执行了 RustCodeGraph `status`、`explore` 和对上述生产文件的 `node --file` 阅读，均正常返回。索引未用于证明运行验收。已有行为覆盖见 `models/metadata_tests.rs`、`models/capabilities_tests.rs` 以及网关 `model_catalog_acceptance_tests.rs`；本轮未执行、未新增重复测试。

没有数据写入或调用者切换，未生成业务夹具，故无数据前后哈希或迁移回退操作。恢复点为本任务文档提交的父提交；仅回退本文即可，不触及其他任务修改。以后统一验证时需覆盖 SDK/运行投影能力一致、未知模型拒绝及跨租户/主机拒绝；历史模型目录证据不能替代当前版本验证。
