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

## 任务 2　部署身份与元数据匹配键

2026-11-08，macOS 27.0（26A428），arm64。批次 1 由 `964708504` 交付。本项复用其精确身份 DTO，并核对了任务 1 所列的主机授权、可取消操作、应用注册与定位、供应商修订事务入口。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现与下游 |
| --- | --- |
| `src/lib/modelMetadata.ts::endpointKey` | `crates/vw-agent-apps/src/models/identity.rs::MetadataEndpointKey::parse`。复用供应商端点校验。保留协议、非默认端口、路径大小写和内部斜杠，仅去除尾部版本与请求路径。OpenCode Zen 与 Go 各自只在规范 HTTPS 源上应用旧地址别名 |
| `src/lib/modelMetadata.ts::findModelInMap`、`src/lib/modelsDev.ts::normalizeModelsDevModelId` | 同模块的 `find_metadata_model` 和 `metadata_model_key`。只在已选元数据候选中查找，按精确 ID、大小写折叠、去命名空间和变体的顺序匹配，同级按来源顺序取首项。后续由任务 18、19 消费 |
| 任务 1 的共享身份及旧 Codex 保存入口 | `crates/vw-api-types/src/model_catalog.rs` 的 `ModelCatalogIdentity` 和 `ScopedModelCatalogIdentity` 继续用派生的精确比较和哈希。现有 `crates/vw-agent/src/model_catalog.rs::ModelCatalogStore` 使用原部署键。模型目录 API、SDK 和 Iced 接入由任务 36–43 交付，本项只验证业务层，无窗口验收项 |

### 验证

各阶段均动态领取共享槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。首次启动聚焦测试因仓库配置的共享中间目录等待锁，停止本项等待进程（退出 143）后将 `CARGO_BUILD_BUILD_DIR` 也指向同一槽位。两个 `cargo fmt --all` 阶段使用基于 `d42d23db3` 的源码快照，只回写本项三个 Rust 文件。聚焦测试在实际仓库根目录执行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_identity_ -- --nocapture`（红灯） | 101 | 10 条有效测试，3 通过、7 失败。套餐用例实际命中 Coding 套餐窗口 `202752`，普通套餐预期为 `200000` |
| `cargo test -p vw-agent-apps catalog_identity_ -- --nocapture`（绿灯） | 0 | 10 通过、0 失败。覆盖套餐、租户、执行主机、应用、供应商、模型 ID、地址变体及匹配优先级 |
| `cargo clippy --all-targets --all-features -- -D warnings`（实际工作区） | 101 | 无关并发改动在 `crates/vw-gateway-client/src/lib.rs:116` 导入了尚未从 `session` 导出的 `GatewaySessionForkTarget`（E0432） |
| `cargo clippy --all-targets --all-features -- -D warnings`（首次源码快照） | 101 | 槽位复用了实际工作区的 API 编译缓存，报告会话测试的 E0063、E0560。快照源码仍包含旧字段定义 |
| `cargo clippy --all-targets --all-features -- -D warnings`（最终源码快照） | 0 | 刷新快照文件时间戳，重新检查工作区全部目标及 features。快照基于 `d42d23db3`，仅叠加本任务三个 Rust 文件 |

差异审查和 `git diff --cached --check` 通过。各阶段均已释放本项持有的槽位锁并保留缓存。

## 任务 3　预设家族分组与搜索别名

2026-11-08，macOS 27.0（26A428），arm64。批次 2 由 `0e01c516e` 交付。沿用任务 1 所列的原生包、主机授权、可取消操作、应用定位和供应商修订事务契约。搜索接受单个应用的可见预设，并返回原始条目的引用。测试使用任务 1 的 `ScopedModelCatalogIdentity` 核对模型请求 ID、端点和原始对象。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | Rust 实现 |
| --- | --- |
| `src/config/presetFamilies.ts::PRESET_FAMILIES`、`PRESET_PLAN_KEYS`、`PRESET_REGION_KEYS`，以及 `presetGroups.ts::groupPresetRows`、`sortFamilyVersions` | `crates/vw-agent-apps/src/models/presets/families.json`、`models/preset_search.rs::group_presets`、`PresetPlan` 和 `PresetRegion`。单版本保留独立条目。未指定顺序的套餐按首次出现排序，同套餐按国内、海外排序 |
| `src/config/presetSearchAliases.ts::PRESET_SEARCH_ALIASES` 和 `presetGroups.ts::matchPresetRow` | `models/presets/search_aliases.json` 与 `models/preset_search.rs::search_presets`。厂商名、主域名与别名可命中整组，版本名、译文或专属域名可标出具体版本。搜索结果保留整组及其原始请求数据 |

逐条对照固定来源的 18 个家族、62 条别名、10 个套餐键和 2 个地区键。保留的来源许可见 `crates/vw-agent-apps/upstream/LICENSE`（MIT，Copyright (c) 2025 Jason Young）。本项交付业务层。目录 API 与 SDK 在任务 36 接入，Iced 搜索入口在任务 39 接入。本项无窗口验收项。

### 验证

红灯动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。绿灯动态领取槽位 **3**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。两阶段均将 Cargo 中间目录设为同一槽位。`cargo fmt --all` 在基于 `0e01c516e` 的源码快照运行，只回写本项三个 Rust 文件。聚焦测试和首轮 Clippy 在工作区根目录运行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_preset_search_ -- --nocapture`（红灯） | 101 | 5 条有效测试，全部按预期失败。首个用例搜索“智谱”实际返回 0 组，预期为 1 组 |
| `cargo test -p vw-agent-apps catalog_preset_search_ -- --nocapture`（绿灯） | 0 | 5 条通过。覆盖中文别名与精确请求身份、旧名与厂商名、地区和域名定位、单版本与源行顺序、套餐排序 |
| `cargo clippy --all-targets --all-features -- -D warnings`（共享工作区） | 101 | 并行改动在 `vw-agent/src/gateway/api/handlers/session/session_ops.rs:602` 导入不存在的 `crate::app::agent::worktree`（E0432），对应 `worktree.rs:21` 有未使用的导出。桌面并行改动在 `system_settings_ui_refresh_04_tests.rs:468` 的 `messages.try_into()` 缺少类型标注（E0282） |
| `cargo clippy --all-targets --all-features -- -D warnings`（隔离源码快照） | 0 | 工作区全部目标及 features 通过。快照基于 `0e01c516e`，仅叠加本项三个 Rust 文件和两个 JSON 文件。此阶段动态领取槽位 **3**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3` |

差异审查和 `git diff --cached --check` 通过。只提交本项源码、测试、数据与证据。各阶段已释放本项的槽位锁并保留编译缓存。
