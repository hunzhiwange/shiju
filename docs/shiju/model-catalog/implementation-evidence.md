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

## 任务 4　通用供应商模型预设

2026-11-08，macOS 27.0（26A428），arm64。批次 3 由 `897b2bbf7` 交付。沿用现有供应商 DTO 和事务边界。本项的目录测试使用原生 `providers/universal.rs::save` 和 `providers/universal_sync.rs::project` 做进程内验证，未连接实际账号或网关。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现 |
| --- | --- |
| `src/config/universalProviderPresets.ts::universalProviderPresets`、`findPresetByType`、`NEWAPI_DEFAULT_MODELS` | `crates/vw-agent-apps/src/models/presets/universal.json` 嵌入 Rust，由 `models/preset_catalog.rs::universal_presets` 按源顺序列出，`universal_preset` 按精确 key 查询。复用 `vw-api-types/src/app_providers/universal.rs` 的 `UniversalApps` 与 `UniversalModels` |
| `src/config/universalProviderPresets.ts::createUniversalProviderFromPreset` 的调用方提供 base URL，`src-tauri/src/provider.rs::UniversalProvider::to_{claude,codex,gemini}_provider` 定义投影 | `UniversalPreset::endpoint` 校验部署地址，并和 `providers/universal_sync.rs::project` 共用 `projected_endpoint`。Claude 用 Anthropic，Codex 用 OpenAI Responses，Gemini 用 Gemini。Codex 为纯 origin 补 `/v1`，保留自定义路径 |

逐条核对两个稳定 key（`newapi`、`custom_gateway`）、六组应用模型默认值、十二处模型字段（五个不同的模型 ID）和一个展示网址。源预设的部署 URL 均由用户提供，家族、限额、模态和能力均无已核实值。Codex 的 `high` 保留为默认配置，部署能力视为未知。JieKou 和 PPIO 属于各应用预设。保留的来源许可见 `crates/vw-agent-apps/upstream/LICENSE`（MIT，Copyright (c) 2025 Jason Young）。

现有配置入口是 `vw-agent/src/gateway/api/handlers/app_providers/universal.rs` 的 `/v1/universal-providers`，SDK 为 `vw-gateway-client/src/client/app_providers/universal.rs`，Iced 为 `vw-desktop/src/app/components/app_providers/universal.rs::overlay`。这些入口继续处理已保存配置。预设目录的 API 与 SDK 由任务 36 接入，Iced 搜索由任务 39 接入。本项无窗口验收项。

### 验证

红灯和绿灯均动态领取共享槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，中间目录设为同一槽位。`cargo fmt --all` 在基于 `897b2bbf7` 的源码快照中运行，只回写本项 Rust 文件。聚焦测试和 Clippy 在实际工作区根目录运行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_presets_universal_ -- --nocapture`（红灯） | 101 | 4 条有效测试，4 条预期失败。首个断言实际查到 `[]`，预期 `["newapi", "custom_gateway"]` |
| `cargo test -p vw-agent-apps catalog_presets_universal_ -- --nocapture`（绿灯） | 0 | 4 条有效测试全部通过。覆盖稳定键与默认模型、外部预设隔离、无效部署地址、自定义路径与原生投影一致性 |
| `cargo test -p vw-agent-apps cc04_universal -- --nocapture` | 0 | 5 条已有配置与同步回归全部通过 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 实际工作区全部目标和 features 通过 |

差异只包含目录、数据、独立测试、复用的端点规则和本证据。提交前检查差异和空白错误。各阶段已释放本项的槽位锁并保留编译缓存。

## 任务 5　Claude 双应用模型预设

2026-11-08，macOS 27.0（26A428），arm64。批次 4 由 `5b87c60fe` 交付。本项沿用其内嵌 JSON 读取入口，由 `models/preset_catalog.rs::claude_presets` 按 `AppId` 保留源顺序，`claude_preset` 在应用内按精确名称查询。隐藏预设仍可查询，并保留其隐藏标记。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现 |
| --- | --- |
| `src/config/claudeProviderPresets.ts::providerPresets` | `models/presets/claude.json` 中的 `claude` 行，解析为 `models/preset_claude.rs::ClaudePreset::Code`。复用 `ClaudeModels` 保存默认模型和 Haiku、Sonnet、Opus 角色，另外保留小模型、子代理模型、发现 URL 及显式配置的窗口值 |
| `src/config/claudeDesktopProviderPresets.ts::claudeDesktopProviderPresets`、`CLAUDE_DESKTOP_ROLE_ROUTE_IDS`、`passthroughRoutes`、`mappedRoutes`、`brandedRoutes` | 同一文件中的 `claude-desktop` 行，解析为 `ClaudePreset::Desktop`。展开源工厂的输出，保留路由顺序、直连或代理模式、路由 ID、上游模型、显示名称和 1M 标志。复用 `DesktopMappingMode` 和 `DesktopModelRoute` |
| `tests/config/claudeProviderPresets.test.ts`、`claudeDesktopProviderPresets.test.ts` | `models/preset_claude_tests.rs` 通过真实目录验证 Kimi For Coding 的角色和窗口、OpenCode Go 的同名部署隔离及 OpenCode Zen 的应用隔离。另验证 Desktop 路由顺序和 Haiku 窗口、官方空默认值及全量查询 |

从固定 Git 对象读取两份 TypeScript 数组，并独立逐行比对投影结果。共核对 268 条预设（Code 139、Desktop 129）、331 个 Code 模型字段、292 条 Desktop 路由、266 个部署地址或模板、266 个候选端点及 7 个发现地址。128 个同名项保留各自的应用身份，OpenCode Go、Microsoft Foundry 和 Cloudflare AI Gateway 各自的端点差异也保留。展示网址保留域名、路径和片段，移除推广或敏感查询参数。目录只保存模型与部署所需的配置提示，凭据值、通用环境开关及推广字段均未迁移。源 API 格式、OAuth 类型、端点模板、配置窗口与 1M 标记是目录数据，执行端仍需独立验证支持情况。来源许可见 `crates/vw-agent-apps/upstream/LICENSE`（MIT，Copyright (c) 2025 Jason Young）。

现有 Claude Code 表单经过网关 `app_providers/claude.rs` 的 `/v1/app-providers/claude/{id}/form`、SDK 的 `client/app_providers/claude.rs::claude_form_create` 和 `claude_form_update` 以及 Iced 的 `components/app_providers/claude.rs::fields`。Desktop 配置及映射分别经过网关 `app_providers/claude_desktop.rs` 的 `/{id}/config` 和 `app_providers/desktop_mapping.rs` 的 `/{id}/model-mapping`，对应 SDK 的 `claude_desktop_provider_save`、`claude_desktop_mapping_save` 和 Iced 的 `components/app_providers/desktop.rs::overlay`。这些入口继续处理已保存配置。预设目录 API 与 SDK 在任务 36 接入，Iced 搜索在任务 39 接入。本项验收原生离线目录，无窗口、账号或线上请求验收项。

### 验证

红灯、绿灯与全工作区 Clippy 均动态领取共享槽位 **3**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`，Cargo 中间目录使用同一槽位。`cargo fmt --all` 在基于 `5b87c60fe` 的源码快照中运行，只回写本项四个 Rust 文件。聚焦测试和全工作区 Clippy 在实际工作区根目录运行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_presets_claude_ -- --nocapture`（红灯） | 101 | 6 条有效测试，6 条预期失败。同名 OpenCode Go 查询实际为 `(None, None)`，预期为 `(Some(Claude), Some(ClaudeDesktop))` |
| `cargo test -p vw-agent-apps catalog_presets_claude_ -- --nocapture`（绿灯） | 0 | 6 条有效测试全部通过 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 实际工作区全部目标及 features 通过 |

审查范围仅包含目录入口、类型、数据、独立测试与本证据。各阶段已释放本项的槽位锁并保留编译缓存。

## 任务 6　Gemini 与 Grok Build 模型预设

2026-11-08，macOS 27.0（26A428），arm64。批次 5 由 `1eba39833` 交付。本项沿用 `models/preset_catalog.rs` 的内嵌 JSON 读取方式。`gemini_grok_presets` 按 `AppId` 保留源顺序并将官方条目置首，`gemini_grok_preset` 在应用内按精确名称查询。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现 |
| --- | --- |
| `src/config/geminiProviderPresets.ts::geminiProviderPresets` | `models/presets/gemini_grok.json` 的 `gemini` 行解析为 `models/preset_gemini_grok.rs::GeminiGrokPreset`。保留 `GEMINI_MODEL`、`GOOGLE_GEMINI_BASE_URL`、候选地址和 TeamoRouter 的 Bearer 配置提示，使用共享的 `ProviderProtocol::Gemini` |
| `src/config/grokBuildProviderPresets.ts::grokBuildProviderPresets`、`grokPresetConfig` 和 `src/utils/grokBuildConfig.ts::GROK_BUILD_DEFAULT_MODEL` | 同一 JSON 的 `grokbuild` 行保留源 TOML 的 `model`、`base_url`、候选地址及分组，按 `wire_api = "responses"` 和显式格式使用 `ProviderProtocol::OpenaiResponses`。模型命名空间原样保留 |
| Gemini 的 Google Official、`grokBuildOfficialPreset` 和 `src-tauri/src/database/dao/providers_seed.rs::OFFICIAL_SEEDS` | 保留 `gemini-official`、`grokbuild-official` 及 Google 的 `google-official` 认证识别标记。官方行保留空部署配置。Grok 的 xAI API Key 预设是独立行 |
| `src/config/grokBuildProviderPresets.test.ts`、`tests/components/GeminiFormFields.test.tsx` 和 `tests/components/ProviderForm.addDraftProjection.test.tsx` | `models/preset_gemini_grok_tests.rs` 验证真实目录的官方空配置、逐应用枚举、排除名单、模型 ID 和协议、候选地址和同名部署隔离 |

执行固定来源的两份数组后，逐行核对顺序与所投影字段。Gemini 50 条包含 49 个部署地址、49 个模型值（6 个不同 ID）、73 个候选端点。Grok Build 55 条包含 54 个部署地址、54 个模型值（3 个不同 ID）、79 个候选端点。展示网址移除查询参数。推广链接、凭据占位、E-FlowCode 的客户端通用设置以及 Grok 表单共用的上下文窗口默认值均在本项模型目录投影范围之外。来源许可见 `crates/vw-agent-apps/upstream/LICENSE`（MIT，Copyright (c) 2025 Jason Young）。

现有配置入口由网关 `app_providers/gemini.rs` 和 `app_providers/grok.rs` 分别提供 `/v1/app-providers/gemini/{id}/form` 和 `/v1/app-providers/grokbuild/{id}/form`。SDK 对应 `client/app_providers/gemini.rs::gemini_form_create`、`gemini_form_update` 及 `client/app_providers/grok.rs::grok_form_create`、`grok_form_update`。Iced 配置字段由 `components/app_providers/gemini.rs::fields` 和 `components/app_providers/grok.rs::fields` 渲染。预设目录 API 与 SDK 在任务 36 接入，Iced 搜索在任务 39 接入。本项验收原生离线目录，无窗口、账号或线上请求验收项。

### 验证

各阶段动态领取共享槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，Cargo 中间目录使用同一槽位。`cargo fmt --all` 在基于 `1eba39833` 的源码快照中运行，只回写本项四个 Rust 文件。聚焦测试和全工作区 Clippy 在实际工作区根目录运行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、首轮复验、最终绿灯） | 0、0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_presets_gemini_grok_ -- --nocapture`（红灯） | 101 | 5 条有效测试，5 条预期失败。首个精确测试对 Gemini OpenModel 查到 `None`，预期 `Some((Some(Gemini), Some("gemini-4-argon")))` |
| 同一聚焦命令（首轮复验） | 101 | 4 通过、1 失败。源清单两边都有 OpenModel，隔离用例改用仅 Gemini 收录的 OspreyAI，并断言其在 Gemini 可查 |
| 同一聚焦命令（最终绿灯） | 0 | 5 条有效测试全部通过 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 实际工作区全部目标及 features 通过 |

审查范围仅包含目录入口、类型、数据、独立测试与本证据。各阶段已释放本项的槽位锁并保留编译缓存。

## 任务 7　Codex 模型预设

2026-11-08，macOS 27.0（26A428），arm64。批次 6 由 `f08af768d` 交付。本项复用任务 4 的 `crates/vw-agent-apps/src/models/preset_catalog.rs` 内嵌读取入口，按应用和源名称查询。模型查询限定在已选预设的主端点或候选端点内，并保留原始模型 ID。共享候选端点的套餐也分别持有模型清单。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现 |
| --- | --- |
| `src/config/codexProviderPresets.ts::codexProviderPresets`、`generateThirdPartyConfig` | `models/presets/codex.json` 由 `models/preset_catalog.rs::codex_presets` 和 `codex_preset` 读取。保留源顺序、套餐、地区、默认模型、推理默认值、主端点与候选地址。显式 `apiFormat` 优先，其余有配置的条目采用源 TOML 的 Responses 格式 |
| 同源 `modelCatalog`、`codexProviderPresets.tokenPlanTextOnly.test.ts`、`codexProviderPresets.zhipuFlashVision.test.ts` | `models/preset_codex.rs::CodexPresetModel` 保留每行的窗口、推理档位与默认档位、图片声明、并行工具调用和模型指令。独立测试覆盖第三方 Token Plan 纯文本、DeepSeek 官方图片声明、智谱国内与国际版 Flash 图片声明和同模型不同部署 |
| `src/config/codexProviderPresets.test.ts` | `models/preset_codex_tests.rs` 核对 Copilot 的默认模型和目录、两条托管 OAuth 的回退标记均为 `false`，以及普通 API Key 预设的显式 `true`。目录只记录认证类型及回退标记 |

执行固定来源数组并逐行核对源名称、模型字段、候选端点和 TOML 投影。137 条预设包含 136 个主端点、174 个候选端点和 213 个模型条目（118 个不同 ID）。展示网址移除查询参数和片段。凭据、推广字段、完整 TOML 模板及 Chat 协议转换参数在本项目录范围之外。来源许可见 `crates/vw-agent-apps/upstream/LICENSE`（MIT，Copyright (c) 2025 Jason Young）。

已有配置入口为网关 `app_providers/codex_catalog.rs` 的 `/v1/app-providers/codex/{id}/form`，SDK 为 `client/app_providers/codex_catalog.rs::codex_form_create` 和 `codex_form_update`，Iced 由 `components/app_providers/codex.rs::fields` 渲染现有配置字段。预设目录 API 与 SDK 由任务 36 接入，Iced 搜索由任务 39 接入。本项验收原生离线查询，无窗口、账号或线上请求验收项。

### 验证

红灯动态领取共享槽位 **5**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-5`。绿灯与 Clippy 分别动态领取共享槽位 **3**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。Cargo 中间目录也使用对应槽位。`cargo fmt --all` 在基于 `f08af768d` 的源码快照执行，仅回写本项四个 Rust 文件。聚焦测试和 Clippy 在实际仓库根目录执行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_presets_codex_ -- --nocapture`（红灯） | 101 | 4 条有效测试均按预期失败。首条 `catalog_presets_codex_keeps_plan_vision_declaration` 查询千帆 Token Plan 的 `deepseek-v4-flash`，纯文本判断实际为 `None`，预期为 `Some(true)` |
| `cargo test -p vw-agent-apps catalog_presets_codex_ -- --nocapture`（绿灯） | 0 | 4 条有效测试全部通过，531 条过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 实际工作区全部目标及 features 通过 |

差异审查与 `git diff --cached --check` 通过。各阶段的槽位锁已释放，编译缓存已保留。

## 任务 8　OpenCode 与 OpenClaw 模型预设

2026-11-08，macOS 27.0（26A428），arm64。批次 7 由 `1a031a7f3` 交付。本项沿用任务 4 的内嵌 JSON 读取方式。目录按应用及源名称选择预设，模型查询要求该行的精确部署地址和原始模型 ID。同域名的不同路径保留为独立部署，未声明的窗口、输出限制或模态保留为空。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现 |
| --- | --- |
| `src/config/opencodeProviderPresets.ts::opencodeProviderPresets` | `crates/vw-agent-apps/src/models/presets/opencode_openclaw.json` 的 `opencode` 行保存 `settingsConfig.options.baseURL` 和原生 `npm` 标识。按模型键保留 `limit.context`、`limit.output`、输入与输出模态、推理标记和模型级选项 |
| `src/config/openclawProviderPresets.ts::openclawProviderPresets` | 同一 JSON 的 `openclaw` 行保存 `settingsConfig.baseUrl` 和原生 `api` 标识。按源数组顺序保留 `contextWindow`、`maxTokens`、输入模态、推理与兼容性声明，以及建议的默认模型、回退和别名 |
| `tests/config/opencodeProviderPresets.test.ts`、`therouterOpenCodeOpenClawPresets.test.ts`、`presetModelMetadata.test.ts` | `models/preset_catalog.rs::opencode_openclaw_presets` 和 `opencode_openclaw_preset` 提供离线入口，`models/preset_opencode_openclaw.rs::OpenCodeOpenClawPreset::model_at_endpoint` 限定部署。独立测试覆盖 MiMo 与 Qwen 模态、TheRouter 命名空间和应用隔离、TokenFactory 与模力方舟窗口差异，以及同域名的火山套餐路径隔离 |

从固定 Git 对象运行两份 TypeScript 数组，独立逐行核对源预设顺序、模型 ID、部署及投影字段。OpenCode 110 条预设、107 个主端点、338 个模型，195 个模型声明上下文及输出限制，185 个声明双向模态。OpenClaw 122 条预设、122 个主端点、383 个模型，全部声明上下文，331 个声明输出限制，335 个声明输入模态，119 条预设含建议默认值。源未声明 OpenClaw 输出模态。展示网址移除查询参数与片段。模型目录范围为两个供应商预设数组，通用 OpenCode 适配器候选库、价格、凭据和运行时请求设置在本项范围之外。来源许可见 `crates/vw-agent-apps/upstream/LICENSE`（MIT，Copyright (c) 2025 Jason Young）。

现有配置入口是 `vw-agent/src/gateway/api/handlers/app_providers/{opencode,openclaw}.rs` 的 `/v1/app-providers/{opencode,openclaw}/{id}/form`，SDK 对应 `vw-gateway-client/src/client/app_providers/{opencode,openclaw}.rs` 的 `*_form_{read,create,update}`，Iced 对应 `vw-desktop/src/app/components/app_providers/{opencode,openclaw}.rs::fields`。这些路径处理已保存配置。预设目录 API 与 SDK 由任务 36 接入，Iced 搜索由任务 39 接入。本项验收原生离线查询。

### 验证

红灯动态领取共享槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。绿灯与 Clippy 动态领取共享槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Cargo 中间目录使用对应槽位。格式化在源码快照执行，仅回写本项四个 Rust 文件。聚焦测试和首轮 Clippy 在实际仓库根目录执行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_presets_opencode_openclaw_ -- --nocapture`（红灯） | 101 | 5 条有效测试均按预期失败。首个指定用例实际窗口为 `(None, None)`，预期为 `(Some(1048576), Some(1000000))` |
| `cargo test -p vw-agent-apps catalog_presets_opencode_openclaw_ -- --nocapture`（绿灯） | 0 | 5 条有效测试全部通过，535 条过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 实际工作区全部目标与 features 通过 |

差异仅包含原生目录入口、类型、数据、独立测试和本证据。槽位锁均已释放，编译缓存已保留。

## 任务 9　Hermes 与 Mcode 模型预设

2026-11-08，macOS 27.0（26A428），arm64。批次 8 由 `e97f73930` 交付。本项沿用任务 4 的内嵌 JSON 读取方式。`models/preset_catalog.rs::hermes_mcode_presets` 按应用枚举源顺序，`hermes_mcode_preset` 按应用和源名称精确选择。`HermesMcodePreset::model_at_endpoint` 仅在该预设的精确端点查询原始模型 ID。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现 |
| --- | --- |
| `src/config/hermesProviderPresets.ts::hermesProviderPresets` | `crates/vw-agent-apps/src/models/presets/hermes_mcode.json` 的 Hermes 行保留原生 API mode、端点、供应商键、建议默认模型，以及模型级 `context_length` 与 `supports_vision`。未声明的窗口、输出上限与模态保持未知 |
| `src/config/mcodeProviderPresets.ts::mcodeProviderPresets` | 同一 JSON 的 Mcode 行取自源模块执行过滤后的数组，包含仅 Mcode 使用的 OpenAI API 条目。保留原生 API 格式、端点、`limit.context`、`limit.output`、双向模态、推理、兼容性与自适应推理档位。无需 Pi 目录在运行时提供数据 |
| `tests/config/hermesMinimaxProviderKey.test.ts`、`src/config/mcodeProviderPresets.test.ts` | `models/preset_hermes_mcode_tests.rs` 验证 MiniMax 的原生供应商键与图片标记、Kimi 兼容性过滤、PackyCode 的推理档位和火山套餐隔离。两个应用的窗口分别取自所选预设，即使模型 ID 和端点相同也不互补 |

从固定 Git 对象执行两个源模块，独立逐行核对目录顺序、部署、模型 ID 及全部投影字段。Hermes 有 121 条预设、400 条模型记录，其中 309 条声明窗口、2 条声明图片支持。Mcode 有 115 条预设、406 条模型记录，全部声明输入及输出模态和窗口限制；109 条含兼容性声明，64 条含推理档位。展示网址移除查询参数和片段。凭据、推广、请求头、标题生成和配置处理器在本项范围之外。来源许可见 `crates/vw-agent-apps/upstream/LICENSE`（MIT，Copyright (c) 2025 Jason Young）。

现有配置入口为网关 `vw-agent/src/gateway/api/handlers/app_providers/{hermes,mcode}.rs` 的 `/v1/app-providers/{hermes,mcode}/{id}/form`，对应 SDK `vw-gateway-client/src/client/app_providers/{hermes,mcode}.rs` 的 `*_form_{read,create,update}`，Iced 由 `vw-desktop/src/app/components/app_providers/{hermes,mcode}.rs::fields` 渲染。预设目录 API 和 SDK 由任务 36 接入，Iced 搜索由任务 39 接入。本项只验收原生离线查询，不涉及窗口、账号或网络请求。

### 验证

红灯动态领取共享槽位 **4**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-4`。绿灯和首轮 Clippy 各自动态领取共享槽位 **5**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-5`。Clippy 复验使用共享槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。Cargo 中间目录使用对应槽位。`cargo fmt --all` 在基于 `e97f73930` 的源码快照执行，只回写本任务的四个 Rust 文件；聚焦测试和 Clippy 在实际仓库根目录执行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_presets_hermes_mcode_ -- --nocapture`（红灯） | 101 | 5 条有效测试均按预期失败。首个指定用例实际窗口为 `(None, None)`，预期为 `(Some(1048576), Some(1000000))` |
| `cargo test -p vw-agent-apps catalog_presets_hermes_mcode_ -- --nocapture`（绿灯） | 0 | 5 条有效测试全部通过，540 条过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings`（首轮） | 101 | 同期桌面变更使 `crates/vw-desktop/src/app/message/chat/mod.rs:24` 的 `agent_message` 导入未被使用；该导入已由其变更移除，本任务未编辑此文件 |
| `cargo clippy --all-targets --all-features -- -D warnings`（复验） | 0 | 实际工作区全部目标与 features 通过 |

差异只包含原生目录入口、类型、数据、独立测试和本证据。`git diff --cached --check` 通过。各阶段的槽位锁均已释放，编译缓存已保留。

## 任务 10　Kimi 与 DSH 模型预设

2026-11-08，macOS 27.0（26A428），arm64。批次 9 由 `61a467630` 交付。本项沿用任务 4 的内嵌 JSON 读取方式。`models/preset_catalog.rs::kimi_dsh_presets` 按应用枚举，官方账号独立列在首位。`kimi_dsh_preset` 按应用及来源供应商键精确选择预设。两个应用的 `model_at_endpoint` 都要求所选预设的精确部署地址。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现 |
| --- | --- |
| `src/config/kimiProviderPresets.ts::kimiProviderPresets`、`kimiAccountPreset` | `crates/vw-agent-apps/src/models/presets/kimi_dsh.json` 的 Kimi 行保留按 `供应商键/模型ID` 索引的原生 `models`、`max_context_size`、能力和推理字段。保留 Kimi Coding 的 `/v1` 端点与官方账号的空模型映射 |
| `src/config/dshProviderPresets.ts::dshProviderPresets`、`dshAccountPreset` | 同一 JSON 的 DSH 行取自来源执行后的派生结果，保留三种受支持协议、原始模型 ID、双向模态、窗口与输出上限、按协议筛选的兼容字段和原生推理档位。保留 `off: null`、禁用后省略的 `off` 和非推理的 `false`。官方账号的模型映射为空 |
| `src/config/{kimi,dsh}ProviderPresets.test.ts` | `models/preset_kimi_dsh_tests.rs` 验证两个账号独立、Kimi 原生能力及 Anthropic 输出上限、DSH 协议和推理档位、精确部署隔离及完整条数 |

执行固定 Git 对象中的两个 TypeScript 模块，对嵌入数据逐条核对行顺序、标识、端点、请求头及全部模型字段，并与 Pi 来源数组交叉验证窗口和模态。两个应用各有 114 条供应商预设及 1 条官方账号，共 410 条模型记录，各有 310 条支持图片输入、399 条支持推理。DSH 有 115 条模型记录含兼容字段。展示网址移除查询参数和片段。离线目录仅保留预设声明，密钥字段（含本地占位符）、推广元数据与账号流程由各自配置或账号层处理。来源许可见 `crates/vw-agent-apps/upstream/LICENSE`（MIT，Copyright (c) 2025 Jason Young）。

现有供应商配置由 `vw-agent/src/gateway/api/handlers/app_providers/{kimi,dsh}.rs` 的 `/v1/app-providers/{kimi,dsh}/{id}/form` 与 `/form/read` 提供，SDK 对应 `vw-gateway-client/src/client/app_providers/{kimi,dsh}.rs` 的 `*_form_{read,create,update}`，Iced 对应 `vw-desktop/src/app/components/app_providers/{kimi,dsh}.rs::fields`。目录 API 和 SDK 由任务 36 接入，Iced 搜索由任务 39 接入。本项验证原生离线查询，未涉及窗口渲染或账号登录。

### 验证

红灯、绿灯和首轮 Clippy 分别动态领取槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。全仓复验和 crate 检查每次均动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Cargo 中间目录使用对应槽位。格式化在基于 `7739bc1ca` 的源码快照执行，只回写本任务的四个 Rust 文件。聚焦测试和 Clippy 在实际仓库根目录执行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_presets_kimi_dsh_ -- --nocapture`（红灯） | 101 | 5 条有效测试按预期失败。首个指定用例中两个官方账号的模型计数实际为 `[None, None]`，预期为 `[Some(0), Some(0)]` |
| `cargo test -p vw-agent-apps catalog_presets_kimi_dsh_ -- --nocapture`（绿灯） | 0 | 5 条有效测试全部通过，545 条过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings`（首轮） | 101 | 同期会话重置变更触发 E0063，`vw-desktop/src/app/message/chat/session.rs:1821` 和 `vw-agent/src/gateway/api/handlers/session/session_ops_tests.rs:699` 的构造器缺少 `revert_code` |
| `cargo clippy --all-targets --all-features -- -D warnings`（复验） | 101 | 会话重置错误消失，同期 `vw-agent/src/gateway/api/handlers/session.rs:120` 声明的 `fork_acceptance_tests.rs` 尚未写入 |
| `cargo clippy -p vw-agent-apps --all-targets --all-features -- -D warnings` | 0 | 本任务所属 crate 全目标及 features 通过 |
| `cargo clippy --all-targets --all-features -- -D warnings`（第二次复验） | 101 | 同期会话测试文件已写入，但其第 233 行通过错误的模块路径调用 `load_fork_history`，触发 E0433 |
| `cargo clippy --all-targets --all-features -- -D warnings`（最终复验） | 0 | 同期会话模块引用已修正，实际工作区全部目标及 features 通过 |

`git diff --cached --check` 通过。差异包含原生目录入口、类型、数据、独立测试和本证据。各阶段的槽位锁已释放，编译缓存已保留。

## 任务 11　Pi 基础模型目录

2026-11-08，macOS 27.0（26A428），arm64。批次 10 由 `4a8bf34c4` 交付。Pi 预设沿用任务 4 的内嵌 JSON 读取入口，按应用和供应商键查找。选中供应商预设后，模型查询要求端点、Pi 原生协议和模型 ID 精确匹配。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现 |
| --- | --- |
| `src/config/piModelCatalog.ts::piModelCatalog` | `crates/vw-agent-apps/src/models/presets/pi.json` 的 `baseModels` 收录 96 条基础能力。通过 `models/preset_catalog.rs::pi_model_capabilities` 以来源键离线查询 |
| `src/config/piModelCatalog.ts::piModel`、`src/config/piProviderPresets.ts::piProviderPresetDefinitions` | 在迁移时运行固定来源的构造逻辑，将 115 个供应商预设的 416 条部署模型展开成原生字段。通过 `models/preset_catalog.rs::{pi_presets,pi_preset}` 按来源顺序枚举并按键选择。部署查询由 `models/preset_pi.rs::PiPresetConfig::model_at_deployment` 提供 |
| `tests/config/piProviderPresets.test.ts` | `models/preset_pi_tests.rs` 验证 TokenFactory 覆盖值、端点与协议隔离、Volcengine 套餐端点、Pi 原生协议、Kimi 兼容字段、DeepSeek 图片能力、管理键及全量条数 |

逐条对照固定来源执行结果，核对 96 条基础能力和 416 条部署模型的顺序、ID、名称、推理标记、模态、窗口及输出上限。55 条模型存在基础能力字段的本地覆盖，51 条模型包含预设直接声明的兼容字段，1 个供应商预设含静态 User-Agent。JSON 只保存展开后的模型字段。API 密钥空占位、展示链接的查询参数和片段、推广及主题字段在导入时移除。推理档位及其派生兼容字段由任务 13 处理。来源许可见 `crates/vw-agent-apps/upstream/LICENSE`（MIT，Copyright (c) 2025 Jason Young）。

Pi 供应商表单的现有网关入口为 `vw-agent/src/gateway/api/handlers/app_providers/pi.rs` 中的 `/v1/app-providers/pi/{id}/form` 和 `/form/read`，SDK 为 `vw-gateway-client/src/client/app_providers/pi.rs` 中的 `pi_form_{read,create,update}`，Iced 表单为 `vw-desktop/src/app/components/app_providers/pi.rs::fields`。离线目录 API 和 SDK 由任务 36 接入，Iced 搜索由任务 39 接入。本项验证 Rust 离线目录。

### 验证

各阶段动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。在 Git 源码快照执行格式化，仅回写本项四个 Rust 文件。测试和 Clippy 在实际工作区执行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_presets_pi_ -- --nocapture`（红灯） | 101 | 5 条有效测试失败，550 条过滤。首个指定用例的 TokenFactory 查询返回 `None`，预期为含图片输入、1,048,576 上下文及 384,000 输出上限的部署模型 |
| `cargo test -p vw-agent-apps catalog_presets_pi_ -- --nocapture`（绿灯） | 0 | 5 条有效测试通过，550 条过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 workspace、全目标及 features 通过 |

`git diff --cached --check` 通过。差异仅包含 Pi 目录的入口、类型、数据、独立测试和本证据。各阶段的槽位锁均已释放，编译缓存已保留。

## 任务 12　OMP 派生模型目录

2026-11-08，macOS 27.0（26A428），arm64。批次 11 由 `576be8c3a` 交付。OMP 目录通过任务 4 的 `models/preset_catalog.rs` 入口，从已嵌入的 `presets/pi.json` 延迟派生。方案省去了计划所列的 `omp.json`，共享不可变的来源标识、端点、协议、请求头与模型能力，兼容性转换只作用于 OMP 的映射。目录查找限定应用，模型查找限定供应商预设、端点、协议和原始模型 ID。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现 |
| --- | --- |
| `src/config/ompProviderPresets.ts::ompProviderPresets`、`toOmpPreset` | `models/preset_catalog.rs::{omp_presets,omp_preset}` 按来源顺序枚举并按键选择。 `models/preset_omp.rs::OmpPreset::from_pi` 生成不含 Pi 配置名称的 OMP 声明 |
| 同文件的 `toOmpModel`、`toOmpCompat` | `models/preset_omp.rs::OmpPresetModel::from_pi` 与 `project_compat` 排除 Pi 专用字段，把 DeepSeek 推理格式转成 `zai`，并在来源显式要求回放时映射为四项 OMP 控制。模型查询由 `OmpPresetConfig::model_at_deployment` 提供 |
| `tests/config/ompProviderPresets.test.ts` | `models/preset_omp_tests.rs` 验证 PPIO 模型和 Pi 原值、Kimi 与腾讯的不同回放行为、部署隔离，并用原生 OMP 验证器检查全部声明 |

从固定 Git 对象执行来源 TypeScript，并与 Rust 查询结果逐条对比。115 个预设、416 条模型记录的顺序、标识、部署、模型字段及 51 份模型兼容映射一致，其中 25 条采用 `zai`，13 条声明推理回放。比较沿用 Pi 导入规则，移除空密钥占位、推广与主题字段，以及展示 URL 的查询参数和片段。来源没有供应商级兼容映射。来源许可见 `crates/vw-agent-apps/upstream/LICENSE`（MIT，Copyright (c) 2025 Jason Young）。

现有配置入口由 `vw-agent/src/gateway/api/handlers/app_providers/omp.rs` 提供，路由为 `/v1/app-providers/omp/{id}/form` 和 `/form/read`。SDK 对应 `vw-gateway-client/src/client/app_providers/omp.rs::omp_form_{read,create,update}`，Iced 表单对应 `vw-desktop/src/app/components/app_providers/omp.rs::fields`。这些路径操作带授权主机和修订的供应商配置。目录 API 和 SDK 由任务 36 接入，Iced 搜索由任务 39 接入。本项验收原生离线查询，未涉及桌面窗口。

### 验证

各阶段均动态领取共享槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。格式化在 Git 源码快照执行，只回写本任务的 Rust 文件。Cargo 构建、测试和 Clippy 在实际工作区执行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_presets_omp_ -- --nocapture`（红灯） | 101 | 4 条有效测试按预期失败，555 条过滤。首个指定用例查询 PPIO 时返回 `None`，预期为含 1,048,576 上下文、393,216 输出上限及 OMP 回放控制的模型 |
| `cargo test -p vw-agent-apps catalog_presets_omp_ -- --nocapture`（绿灯） | 0 | 4 条有效测试全部通过，555 条过滤 |
| `cargo build -p vw-agent-apps --message-format=json`（逐条核对） | 0 | 以本槽位的 Rust 库导出目录。首轮辅助比较脚本因本机 Python 不接受 `zip(strict=True)` 退出 1，改为先断言两侧长度再遍历，复验退出 0 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 workspace、全目标及 features 通过 |

`git diff --cached --check` 通过。差异限于 OMP 离线目录入口、派生类型、Pi 兼容字段的读取、独立测试及本证据。各阶段的槽位锁均已释放，编译缓存已保留。

## 任务 13　Pi 推理档位绑定

2026-11-08，macOS 27.0（26A428），arm64。批次 12 由 `9754a8983` 交付。Pi 预设加载时解析模型目录键和原生协议。模型局部档位随所选预设、端点、协议和原始模型 ID 查询。Anthropic 通用绑定自动带上 `forceAdaptiveThinking: true`。模型级兼容设置优先于供应商级设置，有效设置显式冲突时省略该绑定。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现 |
| --- | --- |
| `src/config/piThinkingProfiles.ts::PI_THINKING_LEVELS`、`piThinkingProfiles`、`piThinkingBindings` | `models/presets/pi_thinking.json` 保留 16 组非空映射和 30 条显式绑定。`models/pi_thinking.rs::PiThinkingLevel` 覆盖 7 个档位，包括来源的 `max`。映射区分 `null` 和缺失键，保留原生字符串及大小写 |
| `src/config/piThinkingProfiles.ts::resolvePiThinkingProfile`、`clonePiThinkingLevelMap` | `models/pi_thinking.rs::resolve` 按目录键、协议和自适应兼容设置筛选。`models/preset_pi.rs::PiPresetModel::thinking_level_map` 返回副本 |
| `src/config/piModelCatalog.ts::getPiModelCatalogReference`、`src/config/piProviderPresets.ts::materializeVerifiedThinkingProfiles` | `models/presets/pi.json` 记录 159 条候选模型的内部引用。`models/preset_catalog.rs::{pi_presets,pi_preset}` 提供展开后的模型，序列化仅输出选中的映射和兼容字段。按本项验收规则，无绑定的模型省略字段，来源的空映射回退不进入 Rust 预设 |
| `tests/config/piThinkingProfiles.test.ts` | `models/pi_thinking_tests.rs` 验证协议、目录键、兼容冲突、部署覆盖、档位空值、映射副本和 OMP 隔离 |

执行固定 Git 对象中的 TypeScript，通过当前槽位构建 Rust 探针，对照真实目录查询返回的全部 115 个预设、416 个模型。132 个模型拥有映射，其中 48 个由预设局部指定，84 个来自通用绑定。64 个模型带派生的自适应兼容字段，284 个模型省略映射。来源中 271 个空映射按上述规则省略，其余投影字段逐条一致。来源许可见 `crates/vw-agent-apps/upstream/LICENSE`（MIT，Copyright (c) 2025 Jason Young）。

现有配置入口为 `vw-agent/src/gateway/api/handlers/app_providers/pi.rs` 的 `/v1/app-providers/pi/{id}/form` 和 `/form/read`。SDK 为 `vw-gateway-client/src/client/app_providers/pi.rs::pi_form_{read,create,update}`，Iced 由 `vw-desktop/src/app/components/app_providers/pi.rs::fields` 渲染。目录 API 和 SDK 由任务 36 接入，Iced 搜索由任务 39 接入。本项验收 Rust 离线查询，无窗口、账号或网络调用。

### 验证

各阶段动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Cargo 中间目录使用同一槽位。格式化在 Git 源码快照执行，只回写本项 Rust 文件。聚焦测试、目录测试、探针对照和 Clippy 在实际工作区执行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_pi_thinking_ -- --nocapture`（红灯） | 101 | 5 条有效测试按预期失败，559 条过滤。首个指定用例中 Completions 和 Responses 的 GPT-6.1 Sol 查询均省略档位，预期只有 Responses 返回 7 档映射 |
| `cargo test -p vw-agent-apps catalog_pi_thinking_ -- --nocapture`（绿灯） | 0 | 8 条有效测试通过，559 条过滤 |
| `cargo test -p vw-agent-apps models:: -- --nocapture` | 0 | 66 条有效测试通过，501 条过滤，包括 Pi 和 OMP 既有用例 |
| `cargo build -p vw-agent-apps --message-format=json` | 0 | 构建当前 Rust 库，随后通过 `rustc` 链接临时探针，固定来源对照逐条通过 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 workspace、全目标及 features 通过 |

差异限于 Pi 目录入口、类型、数据、独立测试和本证据。`git diff --cached --check` 通过。各阶段的槽位锁均已释放，编译缓存已保留。

## 任务 14　Codex 自定义模板

2026-11-08，macOS 27.0（26A428），arm64。批次 13 由 `b16af1bde` 交付。此次迁移了 1 份新建草稿。每次读取都返回独立的 TOML 字符串与只含空 `OPENAI_API_KEY` 占位的映射。读取无需账号、网络或文件。草稿不预设端点或 Goal 模式。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现 |
| --- | --- |
| `src/config/codexTemplates.ts::getCodexCustomTemplate` | `crates/vw-agent-apps/src/models/codex_template.rs::codex_custom_template` 返回 `CodexTemplate`。逐字节对照确认 TOML 一致，默认模型为 `gpt-5.6-sol`，推理档位为 `high`，使用 Responses，禁用响应存储，保留 `requires_openai_auth = true` |
| `tests/config/codexTemplates.test.ts` | `models/codex_template_tests.rs` 解析真实 TOML 并验证 Goal 模式缺省、空密钥和副本隔离。通过 `providers/codex_catalog/form.rs::prepare` 验证该路由要求端点及密钥，补齐测试值后保留模型和协议 |

模板读取 API 与 SDK 归任务 36。现有保存路径为 `vw-agent/src/gateway/api/handlers/app_providers/codex_catalog.rs` 的 `POST /v1/app-providers/codex/{id}/form`，经授权主机上下文、表单校验及供应商事务。SDK 为 `vw-gateway-client/src/client/app_providers/codex_catalog.rs::codex_form_create`。Iced 新建草稿目前由 `vw-desktop/src/app/state/app_providers/codex.rs::Draft::new` 构造，表单保存在 `vw-desktop/src/app/message/app_providers/codex.rs::save`。本项仅验收原生读取与表单校验，未修改网关、SDK 或界面。来源许可见 `crates/vw-agent-apps/upstream/LICENSE`（MIT，Copyright (c) 2025 Jason Young）。

### 验证

各阶段动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Cargo 中间目录使用同一槽位。格式化在 Git 源码快照执行，只回写本项三个 Rust 文件。聚焦测试和 Clippy 在实际工作区执行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯及两次复验） | 0、0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_codex_template_ -- --nocapture`（红灯） | 101 | 3 条有效测试按预期失败，567 条过滤。首个指定用例的认证映射实际为 `{}`，预期为 `{"OPENAI_API_KEY": ""}` |
| `cargo test -p vw-agent-apps catalog_codex_template_ -- --nocapture`（首次复验） | 101 | 2 条通过，1 条失败。首次表单用例错误预期启用 OpenAI 认证的第三方路由可以不填密钥。修正用例以验证现有拒绝规则及补齐后通过 |
| `cargo test -p vw-agent-apps catalog_codex_template_ -- --nocapture`（绿灯） | 0 | 3 条有效测试通过，567 条过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 workspace、全目标及 features 通过 |

`git diff --cached --check` 通过。提交范围为模板、模块注册、独立测试和本记录。各阶段的槽位锁均已释放，编译缓存已保留。

## 任务 15　用户代理预设

2026-11-08，macOS 27.0（26A428），arm64。批次 14 由 `8d7d29d56` 交付。已与固定来源逐字节核对 5 条预设的值和顺序。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现 |
| --- | --- |
| `src/config/userAgentPresets.ts::USER_AGENT_PRESETS` | `crates/vw-agent-apps/src/models/user_agent_presets.rs::USER_AGENT_PRESETS`，按来源顺序公开静态列表 |
| `src/lib/userAgent.ts::isValidUserAgentHeader`、`src-tauri/src/provider.rs::parse_custom_user_agent` | `models/user_agent_presets.rs::custom_user_agent` 将显式选择的预设或自定义值转成可选 HTTP 头值。未指定或为空白时返回 `None`。沿用母任务 4 的 `providers/validation.rs::header_value` 字节校验及 8192 字节上限。裁剪空白时保留非法控制字符交由校验拒绝，覆盖值边缘的 CR/LF |
| `tests/lib/userAgent.test.ts` | `models/user_agent_presets_tests.rs` 验证无选择、全部预设显式可选、自定义 UTF-8 与内部制表符、控制字符和超限拒绝 |

当前验收边界为原生目录读取及可选头值解析。发现 HTTP 请求由任务 21 接线，目录读取 API 与 SDK 由任务 36 接线。现有供应商请求选项路径为 `vw-agent/src/gateway/api/handlers/app_providers/request_options.rs`、`vw-gateway-client/src/client/app_providers/request_options.rs` 及 `vw-desktop/src/app/message/app_providers/endpoints.rs`。这条路径复用同一供应商校验，预设选择界面及网关入口尚未接入。本项无 HTTP 请求、账号访问或窗口验收。来源许可见 `crates/vw-agent-apps/upstream/LICENSE`（MIT，Copyright (c) 2025 Jason Young）。

### 验证

红灯和绿灯阶段均动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Cargo 中间目录使用同一槽位。格式化在 Git 源码快照执行，只回写本项 Rust 文件。聚焦测试和 Clippy 在实际工作区执行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_user_agent_ -- --nocapture`（红灯） | 101 | 4 条有效测试按预期失败，570 条过滤。首个指定用例中未选择时实际返回 `Some("")`，预期为 `None` |
| `cargo test -p vw-agent-apps catalog_user_agent_ -- --nocapture`（绿灯） | 0 | 4 条有效测试通过，0 失败，570 条过滤 |
| `cargo test -p vw-agent-apps cc04_validation_ -- --nocapture` | 0 | 6 条供应商校验回归通过，0 失败，568 条过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 workspace、全目标及 features 通过 |

差异限于预设、独立测试、模块注册、复用供应商请求头值校验和本记录。各阶段槽位锁已释放，编译缓存已保留。
