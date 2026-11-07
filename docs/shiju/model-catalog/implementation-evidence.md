# 模型目录实施证据

## 任务 1　共享契约

2026-11-08，macOS 27.0（26A428），arm64。

本项前置是正式包、租户与目标主机授权、可取消操作、应用注册和可信 CLI 定位，以及供应商配置 DTO 与带修订事务。母任务 4 的 14 应用实机矩阵属于该母任务的整体验收。此次核对了已注册的 `vw-agent-apps`、`TenantScope`、网关 `agent_apps/access.rs` 的 `authorize_provider_config` 和 `authorize_provider_write`、`apps/lifecycle/operations.rs::Operation`、`apps/catalog.rs`、`apps/discovery/posix.rs` 及 `apps/launch/request.rs::validate_launch_request`。供应商前置由 `vw-api-types/src/app_providers/opencode.rs` 的 revision、`providers/opencode_form.rs` 的修订检查和 `providers/live/commit.rs::ProviderTransactions::commit_checked` 提供。提交 `a717c6424` 已验证 Claude 和 Codex 经生产 Gateway 与 SDK 的隔离事务。

### 来源与接口

固定上游提交为 `ca58c0313360eae7c851fbd7553c1614c7af03e7`。

| 来源符号 | 共享 API 路径 | 执行端、SDK 与桌面 |
| --- | --- | --- |
| `src/lib/modelMetadata.ts::KnownModelMetadata`、`ResolvedModelMetadata`、`resolveModelMetadata` | `crates/vw-api-types/src/model_catalog.rs` 的 `ModelMetadataDto`、`ModelCapabilityState`、字段来源及获取时间、补全预览和带修订提交类型 | 本项验证传输形状。元数据解析和补空执行由后续任务接入，目录 SDK 和 Iced 入口随对应任务交付 |
| `src-tauri/src/services/model_fetch.rs::FetchedModel`、`fetch_models` | 同文件的 `ScopedModelCatalogIdentity`、`ModelDiscoveryRequest`、`ModelDiscoveryStatus`。模型 ID 保留原样，展示别名独立。发现请求只引用已有供应商和可选账户身份，公开失败状态使用固定分类 | 本项覆盖 Serde 边界，发现执行、路由、SDK 和 Iced 后续接入。网关届时须独立核对身份作用域与授权，执行端复用现有供应商端点校验 |
| 已交付 Codex 保存目录 | 复用原 `ModelCatalogIdentity` 作为部署身份，新增 `ScopedModelCatalogIdentity` 组合部署、租户和模型 ID | `crates/vw-agent/src/model_catalog.rs::ModelCatalogStore` 按租户分区，部署和修订序列化为既有存储键。保存入口为 `gateway/api/handlers/app_providers/codex_catalog.rs`，SDK 为 `client/app_providers/codex_catalog.rs`，桌面为 `components/app_providers/codex.rs`。本项测试旧键兼容 |

### 验证

红灯和首轮绿灯使用共享槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。最终回归动态领取槽位 **3**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。为隔离共享工作区中的其他未提交编辑，`cargo fmt --all` 在同一提交的临时源码快照中运行，只复制回本项三个 Rust 文件。聚焦测试和全工作区 Clippy 在仓库根目录运行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯阶段） | 0 | 格式化完成 |
| `cargo test -p vw-api-types catalog_contract_ -- --nocapture`（红灯） | 101 | 本项 1 条预期失败。省略能力后实际为 `Unsupported`，预期为 `Unknown`。过滤器另命中 1 条既有 Codex 用例且通过 |
| `cargo fmt --all`（首轮绿灯） | 0 | 格式化完成 |
| `cargo test -p vw-api-types catalog_contract_ -- --nocapture`（首轮绿灯） | 0 | 8 条通过，其中 7 条为本项用例，1 条为既有 Codex 用例 |
| `cargo clippy --all-targets --all-features -- -D warnings`（首轮） | 101 | 报告 `ModelCatalogEdit` 的 `large_enum_variant`。已装箱 Upsert 的元数据，序列化形状保持一致 |
| `cargo fmt --all`（最终） | 0 | 格式化完成 |
| `cargo test -p vw-api-types catalog_contract_ -- --nocapture`（最终） | 0 | 8 条通过、0 条失败。其中 7 条本项用例覆盖未知能力和空值、精确身份、来源时间、带修订提交、凭据引用与脱敏状态、旧 Codex 键以及带默认切换的批量编辑 |
| `cargo clippy --all-targets --all-features -- -D warnings`（最终） | 0 | 工作区全部目标和 features 通过 |

差异审查只纳入共享契约、独立测试和本证据。母计划和子计划的 `plan.md` 均保持只读。各阶段完成后已释放本项持有的槽位锁并保留缓存。本项只涉及共享 DTO，无 macOS 窗口验收项。
