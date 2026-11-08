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

## 任务 16　保留 models.dev 原厂指向字段

2026-11-08，macOS 27.0（26A428），arm64。前置批次 15 由 `977ecd849` 交付。批次 16 复用 `vw-provider-resolver` 的内置目录和磁盘缓存解析入口。随仓库提供的 `assets/model/api.json` 中已有 7580 个带指向的条目，未迁移或改写数据文件。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现 |
| --- | --- |
| `src/lib/modelsDev.ts::ModelsDevModel` | `vw-shared/src/provider/models.rs::Model` 反序列化并原样保留可选 `canonical_model_id`，以 `Option` 区分推理缺省与显式关闭、输入和输出价格缺省与显式零价。复用档位、模态、窗口和价格结构 |
| `src/lib/modelMetadata.ts::indexModelsDev` 的供应商归属 | `vw-provider-resolver/src/models.rs::parse_models_text` 保留供应商映射键、模型映射键和原始 `id`。原厂指向独立于部署身份。实际查找和回退策略归任务 18、19 |
| 运行时转换和下游 | `vw-shared/src/provider/state.rs::from_models_dev_model` 保留现有运行时默认值。现有路径经 `vw-agent/src/provider/provider.rs` → `gateway/api/handlers/provider.rs::provider_list` → `vw-gateway-client/src/client/provider_api.rs::provider_list` 提供 `GET /v1/provider`。Iced 的 `vw-desktop/src/app/message/settings/providers/helpers.rs::build_catalog_from_sources` 合并本地原始目录和供应商响应。本次验收原始目录层，目录元数据的 API、SDK 和界面接入由后续任务交付 |

### 验证

红灯、绿灯及基线对照均动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，中间构建目录使用同一槽位。格式化在源码快照中执行，只回写本次涉及的 Rust 文件。各阶段已释放锁并保留缓存。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成，审查差异无无关格式化 |
| `cargo test -p vw-provider-resolver catalog_modelsdev_parse_ -- --nocapture`（红灯） | 101 | 3 条有效测试因指向丢失而失败，119 条过滤。首个指定用例解析结果为 `Null`，预期为 `"vendor/Flagship/2026-11"` |
| `cargo test -p vw-shared catalog_modelsdev_parse_ -- --nocapture`（红灯） | 101 | 2 条有效测试失败，408 条过滤。缺省推理为 `false`，缺省输出价格为 `0.0`，预期均为 `Null` |
| `cargo test -p vw-provider-resolver catalog_modelsdev_parse_ -- --nocapture`（绿灯） | 0 | 3 条有效测试通过，119 条过滤。覆盖直接映射、两种包装、同名部署隔离、内置目录及识矩叠加 |
| `cargo test -p vw-shared catalog_modelsdev_parse_ -- --nocapture`（绿灯） | 0 | 2 条有效测试通过，408 条过滤 |
| `cargo test -p vw-shared provider::state:: -- --nocapture` | 0 | 26 条运行时转换相关测试通过，384 条过滤 |
| `cargo test -p vw-provider-resolver bundled_ -- --nocapture` | 101 | 6 条通过，3 条既有失败，113 条过滤。既有两条目录审计分别遇到 `scaleway/whisper-large-v3` 缺上下文和 `novita-ai/baai/bge-m3` 缺输出限制。Qwen3.7 Plus 输入价格实际为 `0.276`，断言预期 `0.5` |
| `cargo test -p vw-provider-resolver models::models_tests::bundled_ -- --nocapture`（批次 15 源码快照） | 101 | 3 条通过，同 3 条既有用例失败，113 条过滤。两项审计按映射遍历顺序命中 `openrouter/respan/span-01` 和 `aihubmix/gpt-image-1.5` 的缺省限制，Qwen 价格差异一致 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区、全目标和全 features 通过 |

`git diff --cached --check` 通过。提交范围为原始模型字段、兼容转换、独立测试和本记录。额外审计的既有失败保持在本批次范围外。

## 任务 17　加固模型缓存刷新与离线读取

2026-11-08，macOS 27.0（26A428），arm64。批次 16 由 `3c03ca93b` 交付。已核对原生 `vw-agent-apps` 的应用注册与主机命令、网关 `TenantScope`、`ProviderOperation` 的取消与发布阶段，以及 `ProviderTransactions::commit_checked`。本次处理公开基线快照，沿用 resolver 的本地缓存所有权。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现 |
| --- | --- |
| `src/lib/modelsDev.ts::fetchModelsDev`、`modelsDevQueryOptions` | `vw-provider-resolver/src/models.rs::refresh` 返回刷新状态、`fetched_at` 和 `last_error`。同一进程及配置源的并发刷新共享结果，成功结果在 1 小时内复用。失败后允许重试 |
| `src-tauri/src/commands/usage.rs::fetch_models_dev`、`services/model_pricing.rs::fetch_models_dev` | `vw-provider-resolver/src/models_cache.rs::download` 保留原 resolver 的 10 秒超时，限制响应声明和流式读取至 32 MiB。同源重定向有跳数限制，拒绝跨源及带凭据的重定向目标，不发送 Referer。错误信息省略 URL 和响应内容 |
| 源体积测试及原 resolver 磁盘格式 | `models_cache.rs::install_snapshot` 在 UTF-8、结构及模型条目检查后，同目录暂存并原子替换 `models.json`。已有缓存的获取时间从同一文件句柄的修改时间读取。离线读取合并内置 Shiju 目录，缺少可用磁盘缓存时使用内置基线，避免回写覆盖正在发布的快照。自定义路径继续优先读取，显式下载仍写入托管缓存 |
| 原生调用方及后续接点 | `models::snapshot` 为目录与价格消费方提供同一原生快照。现有模型读取经 `vw-agent/src/provider/models.rs`、`gateway/api/handlers/provider.rs::provider_list` 和 `vw-gateway-client/src/client/provider_api.rs::provider_list`。Iced 现有同步入口 `vw-desktop/src/app/message/settings/providers/tasks.rs::sync_remote_task` 继续调用 resolver。刷新状态的服务及 SDK 投影由后续任务接入。当前范围没有界面变更或价格自动同步 |

### 验证

红灯和绿灯均动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，Cargo 中间目录使用同一槽位。格式化在 Git 源码快照执行，只回写本项 Rust 文件。测试使用临时目录与回环 HTTP，无生产请求。初次红灯编译命中旧 `vw-shared` 产物，退出 101。更新原字段文件的 mtime 触发依赖重编译后取得行为红灯，文件内容保持一致。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成，审查差异仅涉及本项 |
| `cargo test -p vw-provider-resolver catalog_modelsdev_cache_ -- --nocapture`（红灯） | 101 | 1 条通过，5 条失败，122 条过滤。首个指定用例重开后实际读到 `{`，预期读到旧有效目录 |
| `cargo test -p vw-provider-resolver catalog_modelsdev_cache_ -- --nocapture`（绿灯） | 0 | 18 条通过，114 条过滤。覆盖快照及时间保留、离线重开、写入失败、声明与流式超限、超时、凭据与重定向、并发合并、失败重试和环境变量优先级 |
| `cargo test -p vw-provider-resolver catalog_modelsdev_parse_ -- --nocapture` | 0 | 3 条通过，129 条过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区、全目标及 features 通过 |

`git diff --cached --check` 通过。各阶段的占用记录与锁已清理，编译缓存已保留。

## 任务 18　实现同端点元数据优先级

2026-11-08，macOS 27.0（26A428），arm64。批次 17 由 `62b98d10a` 交付。本项为只读原生查询，使用既有端点键与模型 ID 匹配规则。保留部署的协议、端口和路径区分。MiniMax 与 Umans 的按量和订阅条目可共用端点及模型 ID，缺少套餐标识时跳过有歧义的 models.dev 候选，保留已匹配的预设字段。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现 |
| --- | --- |
| `src/config/presetModelMetadata.ts` 的七种预设源 | `vw-agent-apps/src/models/metadata.rs::preset_metadata` 复用现有预设。按「预设数 / 模型行数」统计，Codex 137 / 213、OpenCode 110 / 338、OpenClaw 122 / 383、Hermes 121 / 400、Mcode 115 / 406、Pi 115 / 416、Kimi 115 / 410。OpenClaw 的 214 行价格按源预设顺序、名称、端点和模型 ID 逐行核对，原目录其他字段一致。Pi 的原生档位随原预设端点、协议和模型兼容性声明返回 |
| `src/lib/modelMetadata.ts::resolveModelMetadata` 同端点阶段 | `metadata.rs::resolve_model_metadata` 接收 `vw-provider-resolver/src/models.rs::ModelsSnapshot`，在 `vw-api-types/src/model_catalog.rs::ModelMetadataDto` 中按字段登记实际贡献来源和快照获取时间。价格作为一个字段取自匹配部署，保留零价。缺省工具能力与显式关闭分开，既有运行时投影继续将缺省工具能力视为关闭。识矩内置叠加项属于本地数据，不标为 models.dev |
| 现有下游路径 | 公共基线经 `vw-agent/src/provider/models.rs`、`vw-agent/src/gateway/api/handlers/provider.rs::provider_list`、`vw-gateway-client/src/client/provider_api.rs::provider_list` 暴露给现有供应商目录。Iced 在 `vw-desktop/src/app/message/settings/providers/helpers.rs::build_catalog_from_sources` 合并现有来源。后续目录补全服务使用 `ModelCatalogFillPreviewRequest` 和 `ModelCatalogFillPreviewResponse` 接入本项 DTO，本项验收原生查询 |

### 验证

各阶段动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，Cargo 中间目录使用同一槽位。格式化在 Git 源码快照执行，只回写本项 Rust 文件。测试和 Clippy 在实际工作区执行，测试只读本机固定数据。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all` | 0 | 各阶段格式化完成 |
| `cargo test -p vw-agent-apps catalog_metadata_endpoint_ -- --nocapture`（红灯） | 101 | 1 条通过、9 条失败，574 条过滤。首个指定用例的实际值为 `(None, None)`，预期为 `(Some(500000), Some(98304))` |
| `cargo test -p vw-agent-apps catalog_metadata_endpoint_ -- --nocapture`（歧义回归红灯） | 101 | 10 条通过、1 条失败，574 条过滤。两个套餐共享地址时实际返回一项，预期无匹配 |
| `cargo test -p vw-agent-apps catalog_metadata_endpoint_ -- --nocapture`（绿灯） | 0 | 11 条通过，574 条过滤。覆盖同端点优先、来源及时间、空值、端点别名、套餐与端口隔离、七应用投影及 Pi 档位绑定 |
| `cargo test -p vw-shared provider::state:: -- --nocapture` | 0 | 26 条通过，384 条过滤 |
| `cargo test -p vw-provider-resolver catalog_modelsdev_ -- --nocapture` | 0 | 21 条通过，111 条过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区、全目标及 features 通过 |

首轮绿灯尝试中，应用隔离用例误将同时出现在 Codex 与 OpenCode 的模型当作只存在于 Codex。更换为同端点同模型在 Codex 和 Mcode 各自声明不同窗口的真实预设后通过。

审查确认差异限于本项原生解析、七应用投影、OpenClaw 价格、原始工具能力缺省值、模块连线、独立测试及本记录。`git diff --cached --check` 通过。各阶段已释放占用锁并保留构建缓存。

## 任务 19　原厂兜底与 GPT 窗口上限

2026-11-08，macOS 27.0（26A428），arm64。前置批次 18 由 `a854674af` 交付。已核对 `TenantScope`、应用注册 `apps::catalog::catalog`、可信主机命令 `apps::launch::host::command`、可取消的 `ProviderOperation` 及修订发布 `ProviderTransactions::commit_checked`。本项沿用只读快照解析入口。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际实现 |
| --- | --- |
| `src/lib/modelMetadata.ts::findByVendor` | `crates/vw-agent-apps/src/models/vendor_fallback.rs::find` 按归一模型 ID 查找，优先使用精确的原厂指向。多条记录指向同一目标只算一个候选。没有有效指向时，要求唯一自有条目。悬空的跨厂指向不能使托管条目成为自有条目。识矩本地叠加项不参与 |
| `src/lib/modelMetadata.ts::capGptContextWindow`、`resolveModelMetadata` | `models/metadata.rs::resolve_model_metadata` 按预设、同端点 models.dev、原厂兜底的顺序补空。借用的 GPT 上下文上限为 272,000，预设和同端点已知窗口沿用原值。用户值由调用方持有，当前查询无用户值输入或写入。价格及其来源记录仅取自同部署，供母任务 17 消费 |
| `tests/lib/modelMetadata.test.ts` | `models/vendor_fallback_tests.rs` 经真实解析入口覆盖 12 个场景。原同端点端口及路径隔离测试同时断言兜底值可用且价格仍隔离 |
| 下游接点 | 原生输出使用 `crates/vw-api-types/src/model_catalog.rs::ModelMetadataDto`，目录补全的 API 接点为同文件的 `ModelCatalogFillPreviewResponse::proposed`。现有公开基线的网关路径为 `vw-agent/src/gateway/api/handlers/provider.rs::provider_list`，SDK 为 `vw-gateway-client/src/client/provider_api.rs::provider_list`，Iced 为 `vw-desktop/src/app/message/settings/providers/helpers.rs::build_catalog_from_sources`。本项的兜底由原生入口验收，补全 API、SDK 和 Iced 由后续批次接线 |

### 验证

红灯和绿灯均动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Cargo 中间目录使用同一槽位。格式化在 Git 源码快照执行，只回写本项 Rust 文件。测试及 Clippy 在实际工作区执行。两阶段占用锁已释放，编译缓存已保留。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_metadata_vendor_ -- --nocapture`（红灯） | 101 | 12 条有效用例，4 条通过、8 条失败，585 条过滤。首个指定用例的价格和窗口实际为 `(None, None)`，预期为 `(None, Some(272000))` |
| `cargo test -p vw-agent-apps catalog_metadata_vendor_ -- --nocapture`（绿灯） | 0 | 12 条通过，0 失败，585 条过滤 |
| `cargo test -p vw-agent-apps catalog_metadata_endpoint_ -- --nocapture` | 0 | 11 条通过，0 失败，586 条过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区、全目标及 features 通过 |

审查后差异限于原厂查询、元数据连线、独立测试和本记录。`git diff --cached --check` 通过。本项使用固定本地快照，未迁移预设或公共目录数据，未涉及生产请求、配置写入或界面变更。

## 任务 20　迁移模型发现 URL 规则

2026-11-08，macOS 27.0（26A428），arm64。批次 19 由 `5735258f3` 交付。已核对租户作用域 `TenantScope`、`apps::catalog::catalog`、可信主机命令 `apps::launch::host::command`、`ProviderOperation` 以及修订发布 `ProviderTransactions::commit_checked`。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际路径 |
| --- | --- |
| `src-tauri/src/services/model_fetch.rs::build_models_url_candidates` | `crates/vw-agent-apps/src/models/discovery_url.rs::build_models_url_candidates` 使用现有 `AppProviderEndpointKind` 和 `ProviderProtocol`。显式覆写独占结果。完整 URL 按首个 `/v1/` 或末段的父路径派生。普通 URL 优先使用已有版本段，再按源顺序添加兼容回退并去重。保留智谱 / Z.ai 版本路径及百炼 / 千问 Token Plan 的 `/compatible-mode/v1/models` 等站点规则 |
| `src-tauri/src/services/model_fetch.rs::fetch_models` 中的 Google URL 分支 | 同一原生入口对 Gemini 基础 URL 使用 `/v1beta/models`，同时保留显式覆写和完整 URL 的优先级。空白覆写统一视为缺省 |
| 端点授权 | 基础 URL 由调用方从已授权供应商解析。派生地址只修改 URL 路径。跨源覆写要求与调用方已授权的完整端点精确匹配。入口复用 `providers::validation::endpoint`，拒绝凭据、查询串、片段和非回环 HTTP。地址簿由 `AppProviderStore::list_endpoints` 在租户范围内读取，条目本身不授予网络访问。元数据匹配键仅供元数据解析 |
| 下游接点 | URL 候选由本项原生入口提供给后续受限 HTTP 发现。已存在的公开 DTO 位于 `crates/vw-api-types/src/model_catalog.rs`，包括 `ModelDiscoveryRequest` 和 `ModelDiscoveryStatus`。已有地址簿网关和 SDK 位于各自的 `app_providers/endpoints.rs`。发现网关、SDK 与 Iced 由后续批次接线。本项验收为纯 URL 派生，使用固定测试输入，无网络请求或界面操作 |

### 验证

红灯和绿灯均动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Cargo 中间目录使用同一槽位。格式化在 Git 源码快照运行，只回写本项 Rust 文件。测试和 Clippy 在实际工作区运行。两阶段占用锁已释放，编译缓存已保留。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 两阶段均完成格式化 |
| `cargo test -p vw-agent-apps catalog_discovery_url_ -- --nocapture`（红灯） | 101 | 16 条有效用例，2 条通过，14 条失败，597 条过滤。首个指定用例实际得到 `/apps/anthropic/v1/models`，预期唯一地址为 `/exact/models/` |
| `cargo test -p vw-agent-apps catalog_discovery_url_ -- --nocapture`（绿灯） | 0 | 16 条通过，0 条失败，597 条过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区、全目标及 features 通过 |

差异审查限于 URL 规则、独立测试、模块注册及本记录。`git diff --cached --check` 通过。

## 任务 21　实现受限 HTTP 模型发现

2026-11-08，macOS 27.0（26A428），arm64。批次 20 由 `36c6baaee` 交付。已核对 `TenantScope`、`apps::launch::host::command`、`ProviderOperation` 以及 `ProviderTransactions::commit_checked`。本次读取已授权供应商的 HTTP 目录，无配置写入或 CLI 调用。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际路径 |
| --- | --- |
| `src-tauri/src/services/model_fetch.rs::fetch_models`、`build_model_fetch_headers` | `crates/vw-agent-apps/src/models/discovery_http.rs::discover` 接收上层授权解析后的供应商部署和取消 future。调用批次 20 的 `discovery_url::build_models_url_candidates`，同源覆写来自已授权的供应商配置，跨源覆写必须精确匹配该供应商获授权的地址。按协议发送 Bearer、`x-api-key` 或 `x-goog-api-key`。UA 由独立配置显式提供，自定义头限制数量、名称、值和总大小，拒绝保护头及大小写重复。取消前不发请求，取消中丢弃请求或响应体读取 |
| `fetch_models` 的 HTTP 限制与错误分支 | `crates/vw-provider-resolver/src/remote_models.rs::discover_catalog_models` 与既有连接供应商发现共用 `fetch_at` 和 `read_body`。新入口直连，每候选限时 15 秒，声明长度和流式读取均限制 1 MiB，禁止重定向。仅 404/405 切到下一候选，401/403 立即返回 `unauthorized`。错误只含稳定类别，舍弃 URL、响应正文、解析细节和底层网络错误内容 |
| `ModelsResponse`、`catalog_model_ids` | 同一 resolver 模块的 `parse_models` 优先使用 `data[].id` 并保留可用的 `owned_by`。没有 `data` 时依次使用 `models[].slug`、`id`、带 `models/` 前缀的 `name`。去除空 ID，按 ID 排序、去重。旧 `discover_openai_compatible_models` 仍读取 `data`，沿用 3 秒超时及错误类别 |
| 调用者及后续接点 | 既有连接供应商加载 `crates/vw-agent/src/provider/provider.rs` 与 Iced 自定义供应商 `crates/vw-desktop/src/app/message/settings/providers/models.rs::discover_custom_provider_models` 继续使用原 resolver 入口。应用目录的 DTO 已在 `crates/vw-api-types/src/model_catalog.rs` 定义，发现 API、SDK 及 Iced 接线由后续批次实施。本项的行为验收使用 macOS 回环 HTTP 服务 |

### 验证

红灯和绿灯均动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Cargo 中间目录使用同一槽位。格式化在 Git 源码快照执行，只回写本项 Rust 文件。测试和 Clippy 在实际工作区运行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_discovery_http_ -- --nocapture`（红灯） | 101 | 14 个有效用例，4 项通过、10 项失败，613 项过滤。首个指定用例在 401 后只发出一次请求且无泄露，但实际错误为 `http_status`，预期为 `unauthorized` |
| `cargo test -p vw-agent-apps catalog_discovery_http_ -- --nocapture`（绿灯） | 0 | 14 项通过，0 失败，613 项过滤。断言了 401/403 停止、404/405 按序回退、授权和凭据隔离、三种认证、响应形状、声明及流式上限、脱敏与取消时连接关闭 |
| `cargo test -p vw-provider-resolver remote_models -- --nocapture` | 0 | 12 项通过，0 失败，120 项过滤，包括共用 HTTP 读取超时、大小限制和旧调用者行为 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区、全目标及 features 通过 |

审查后差异限于原生 HTTP 发现、共用传输与解析、头部保护、独立测试、Tokio feature 声明和本记录。`git diff --cached --check` 通过。两个阶段的占用锁已释放，缓存已保留。

## 任务 22　迁移 OpenCode 运行时模型发现

2026-11-08，macOS 27.0（26A428），arm64。批次 21 由 `511859e82` 交付。已核对 `TenantScope`、`LocalReadEnvironment`、`Operation`、`ProviderOperation`、供应商路径与 `ProviderTransactions::commit_checked`。本入口只读取运行时目录。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际路径 |
| --- | --- |
| `src-tauri/src/commands/model_fetch.rs::get_opencode_models` | `crates/vw-agent-apps/src/models/discovery_opencode.rs::discover` 接收调用方授权后的主机环境、已保存的目录覆写和任务取消信号。母任务 3 的 `apps::discovery::read` 与新定位入口共用应用注册和有序可信路径扫描。模型发现直接执行首个路径默认的可执行文件，无版本探测子进程。复用 `apps::launch::host::command` 隔离环境，复用 `providers::paths` 解析配置目录。固定参数为 `models`，设置 `OPENCODE_CONFIG_DIR` 和 `OPENCODE_DISABLE_PROJECT_CONFIG=true`，配置目录缺失时以授权主机主目录为工作目录 |
| `get_opencode_models` 的超时与错误处理 | 原生入口限时 20 秒，stdout 最多 1 MiB。预取消时不启动进程，运行中取消、超时或读取失败时终止子进程组并等待主进程退出。stderr 直接丢弃，错误仅返回稳定类别 |
| `parse_opencode_models` | 同模块 `parse` 在第一个斜杠处分隔，按供应商和完整模型 ID 排序去重，过滤空字段、控制字符和格式不合法的行 |
| 调用者及后续接点 | 现有网关 `crates/vw-agent/src/gateway/api/handlers/agent_apps/read.rs` 提供授权主机环境的构建范例，`app_providers/paths.rs` 提供目录覆写读取范例。下游 DTO 为 `crates/vw-api-types/src/model_catalog.rs::ModelDiscoveryMethod::OpenCode`。目录发现的网关、SDK 和 Iced 接线属于后续批次 |

### 验证

各阶段均动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Cargo 中间目录使用同一槽位。格式化在 Git 源码快照中运行，只回写本项文件。验证在实际工作区执行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_discovery_opencode_ -- --nocapture`（红灯） | 101 | 11 条有效用例，2 项通过，9 项失败，627 项过滤。首个指定用例实际返回 `Err("unavailable")`，预期唯一模型为 `provider` / `vendor/model` |
| `cargo test -p vw-agent-apps catalog_discovery_opencode_ -- --nocapture`（绿灯） | 0 | 11 项通过，627 项过滤。覆盖固定参数及目标配置、控制字符、排序去重、错误脱敏、坏编码及体积限制、预取消、运行中杀进程组和实际 20 秒超时 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区、全目标及 features 通过 |
| `cargo test -p vw-agent-apps apps::discovery:: -- --nocapture` | 0 | 27 项相邻发现测试通过，611 项过滤 |
| `cargo test -p vw-agent-apps --test task22_real_opencode -- --nocapture` | 0 | 1 项通过。完整 OpenCode 1.19.5 Darwin arm64 包通过 npm SHA-512 完整性和 Mach-O 签名校验。临时集成测试以 `TASK22_OPENCODE_BIN` 指定可执行文件，在禁网沙箱和临时 HOME 中调用原生入口，用仅启用 `fixture` 供应商的配置得到唯一的 `fixture` / `vendor/nested-model` |

审查后差异限于 OpenCode 原生发现、共用 CLI 路径扫描、模块注册、独立测试、macOS Tokio feature 声明和本记录。`git diff --cached --check` 通过。临时真机测试入口已清理，各阶段占用锁已释放，编译缓存已保留。

## 任务 23　迁移 AGY 运行时模型发现

2026-11-08，macOS 27.0（26A428），arm64。批次 22 由 `cefd22229` 交付。已核对授权主机、可信 CLI 定位、任务取消和租户供应商档案契约。本入口仅读取运行时目录。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际路径 |
| --- | --- |
| `src-tauri/src/commands/model_fetch.rs::get_agy_models` | `crates/vw-agent-apps/src/models/discovery_agy.rs::discover` 接收调用方已授权的 `LocalReadEnvironment` 和可选的已保存 `AgyProfile`。用 `apps::discovery::read::locate_cli` 选择可信路径中的 AGY。无 key 时在目标主机 HOME 下运行固定 `models`，已有原生 settings 须可安全读取且为有效对象，缺文件时仍使用目标主机登录态 |
| `get_agy_models` 的 key 模式和进程清理 | 复用 `providers::agy::validate` 校验供应商档案。key 模式使用独立 HOME 和 `--gemini_dir`，只写 `antigravity-cli/settings.json` 的 `modelProvider`。`apps::launch::host::command` 清空继承环境，只有已保存的 key 和可选地址进入认证环境。AGY 与 OpenCode 共用 `discovery_process::output` 的 20 秒限时、1 MiB stdout 上限、错误脱敏、取消时终止进程组并等待 CLI 退出。临时目录在进程退出后移除 |
| `parse_agy_models` | 同模块 `parse` 从制表符分列输出中提取原生显示名，过滤非法别名、空名称、控制字符和不符合原生长度的值，按显示名排序去重。CLI 别名仅用于验证行格式 |
| 调用者及后续接点 | 现有授权接点为 `crates/vw-agent/src/gateway/api/handlers/agent_apps/read.rs`。租户凭据沿用 `crates/vw-agent/src/app_providers/agy.rs` 的 `AppProviderStore::agy_record` / `profile_from_record`。发现 DTO 为 `crates/vw-api-types/src/model_catalog.rs::ModelDiscoveryMethod::Agy`。发现网关、SDK 及 Iced 由后续批次接线。本项使用 macOS 受控 CLI 进程验证，无真实 AGY 登录或生产账号验证 |

### 验证

红灯和绿灯均动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Cargo 中间目录使用同一槽位。格式化在 Git 源码快照执行，只回写本项 Rust 文件。验证在实际工作区执行，绿灯阶段向父进程注入了测试用认证环境值以检查继承隔离。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 格式化完成 |
| `cargo test -p vw-agent-apps catalog_discovery_agy_ -- --nocapture`（红灯） | 101 | 11 条有效用例，1 项通过、10 项失败、638 项过滤。首个指定用例实际返回 `Err("unavailable")`，预期为 `Ok(["Gemini 3.8 Flash (High)"])` |
| `cargo test -p vw-agent-apps catalog_discovery_agy_ -- --nocapture`（绿灯） | 0 | 11 项通过，638 项过滤。覆盖显示名去重、目标主机登录态、key 模式隔离、坏配置、缺 CLI、输出边界、取消和 20 秒超时后清理 |
| `cargo test -p vw-agent-apps catalog_discovery_opencode_ -- --nocapture` | 0 | 共用执行逻辑的 11 项 OpenCode 回归通过，638 项过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区、全目标及 features 通过 |

审查后差异限于 AGY 原生发现、独立测试、模块注册、共用的 macOS 进程执行逻辑和本记录。`git diff --cached --check` 通过。两个阶段的占用锁已释放，编译缓存已保留。

## 任务 24　迁移 Codex 补空投影

2026-11-08，macOS 27.0（26A428），arm64。批次 23 由 `6c8a20c0c` 交付。已核对 `TenantScope`、授权主机环境 `LocalReadEnvironment`、应用注册和可信 CLI 定位、`ProviderOperation` 及 `ProviderTransactions::commit_checked`。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际路径 |
| --- | --- |
| `src/components/providers/forms/modelMetadataFill.ts::fillCodexCatalogModel` 和 `CodexFormFields.tsx::CODEX_REASONING_LEVELS` | `crates/vw-agent-apps/src/models/fill_codex.rs::fill_codex_catalog_model` 接收母任务 4 的 `CodexNativeModel`、当前目录元数据和批次 18–19 已按应用、端点与模型解析的 `ModelMetadataDto`。现有数值字段用 `None` 表示空窗口，补空差异中用十进制字符串表示建议值。档位按 Codex 顺序去重并过滤未知值。模态字段未设且确认支持图片时建议 `["text", "image"]`，文字输入能力不会产生关闭图片的建议。已设的模态列表，包括显式空列表，保持原样 |
| `tests/components/modelMetadataFill.test.ts` 中的 Codex 行为 | `crates/vw-agent-apps/src/models/fill_codex_tests.rs` 验证用户窗口、档位和模态、纯文字结果、不可用窗口、仅补允许字段，以及逐字段保留实际来源与抓取时间。返回值是内存中的建议差异 |
| 下游接点 | 补全预览 DTO 已定义于 `crates/vw-api-types/src/model_catalog.rs`，预览和修订提交由任务 32 组装，网关和 SDK 接线由任务 38 处理，Iced 来源展示由任务 41 接线。现有发布路径 `providers/codex_catalog.rs::CodexFiles::apply` 经 `models::codex_projection` 序列化已保存行。现有 Codex 档案网关、SDK 和桌面表单分别位于 `crates/vw-agent/src/gateway/api/handlers/app_providers/codex_catalog.rs`、`crates/vw-gateway-client/src/client/app_providers/codex_catalog.rs` 和 `crates/vw-desktop/src/app/components/app_providers/codex.rs`。本项在 macOS 验证原生纯投影 |

### 验证

两个阶段均动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Cargo 中间目录使用同一槽位。格式化在 Git 源码快照中执行，只回写本项 Rust 文件。测试和 Clippy 在实际工作区执行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 两阶段格式化完成 |
| `cargo test -p vw-agent-apps catalog_fill_codex_ -- --nocapture`（红灯） | 101 | 6 条有效用例，2 项通过、4 项失败、649 项过滤。指定用例保留了用户窗口，但档位和档位来源为空，预期档位为 `minimal, low, high, max, ultra`，来源为带抓取时间的 models.dev |
| `cargo test -p vw-agent-apps catalog_fill_codex_ -- --nocapture`（绿灯） | 0 | 6 项通过，649 项过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区、全目标及 features 通过 |

审查后差异限于补空投影、独立测试、模块注册及本记录。`git diff --cached --check` 通过。两个阶段的占用锁已释放，编译缓存已保留。

## 任务 25　迁移 OpenClaw 补空投影

2026-11-08，macOS 27.0（26A428），arm64。批次 24 由 `636d7a15e` 交付。已核对租户作用域、授权主机环境、应用注册与可信 CLI 定位、可取消供应商操作和 `ProviderTransactions::commit_checked` 契约。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际路径 |
| --- | --- |
| `src/components/providers/forms/modelMetadataFill.ts::fillOpenClawModel` | `crates/vw-agent-apps/src/models/fill_openclaw.rs::fill_openclaw_model` 接收母任务 4 的 `OpenClawProfile::declaration.models` JSON 对象行及按应用、端点和模型解析的 `ModelMetadataDto`。投影补齐空的 `contextWindow`、`maxTokens` 和 `cost`，保留已有数值和整个价格对象。确认支持推理时允许升级为 `true`，确认不支持时只为缺失字段补 `false`。空输入列表按 OpenClaw 规则补文字或文字和图片，已有列表只追加缺失的图片能力。返回原生字段差异及贡献字段的来源和时间，整个过程在内存完成 |
| `tests/components/modelMetadataFill.test.ts` 的 OpenClaw 场景 | `crates/vw-agent-apps/src/models/fill_openclaw_tests.rs` 验证已有窗口及价格、空字段、能力升级和防降级、原生字段名、来源筛选和部署价格边界。部署用例通过真实 `models::metadata::resolve_model_metadata` 解析受控快照，验证原厂回退贡献窗口时也只允许匹配端点贡献价格 |
| `OpenClawFormFields.tsx` 的补全调用者 | 现有原生配置读取和准备路径为 `providers/openclaw_form.rs::view` 和 `prepare`，发布由 `providers/openclaw.rs::OpenClawFiles` 处理。现有档案网关、SDK、Iced 表单分别为 `crates/vw-agent/src/gateway/api/handlers/app_providers/openclaw.rs`、`crates/vw-gateway-client/src/client/app_providers/openclaw.rs` 和 `crates/vw-desktop/src/app/components/app_providers/openclaw.rs`。本批次验证原生内存投影。目录补全预览和修订提交由任务 32 组装，网关和 SDK 由任务 38 接线，Iced 来源展示由任务 41 接线 |

### 验证

两个阶段均动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Cargo 中间目录使用同一槽位。格式化在 Git 源码快照执行，仅回写本项 Rust 文件。测试和 Clippy 在实际工作区执行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 两阶段格式化完成 |
| `cargo test -p vw-agent-apps catalog_fill_openclaw_ -- --nocapture`（红灯） | 101 | 7 条有效用例，2 项通过、5 项失败、655 项过滤。指定用例实际返回空差异，预期只建议 `maxTokens: 32768`、`reasoning: true` 和 `input: ["text", "image"]` |
| `cargo test -p vw-agent-apps catalog_fill_openclaw_ -- --nocapture`（绿灯） | 0 | 7 项通过，655 项过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区、全目标及 features 通过 |

审查后差异限于 OpenClaw 投影、独立测试、模块注册和本记录。`git diff --cached --check` 通过。两个阶段的锁已释放，编译缓存已保留。

## 任务 26　迁移 Hermes 补空投影

2026-11-08，macOS 27.0（26A428），arm64。批次 25 由 `e1014e153` 交付。已核对 `TenantScope`、`LocalReadEnvironment`、应用注册与可信 CLI 定位、`ProviderOperation` 和 `ProviderTransactions::commit_checked` 的现有契约。

| 固定来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` | 实际路径 |
| --- | --- |
| `src/components/providers/forms/modelMetadataFill.ts::fillHermesModel` | `crates/vw-agent-apps/src/models/fill_hermes.rs::fill_hermes_model` 接收 `HermesProfile::declaration.models` 的原生对象行和已按应用、端点及模型解析的 `ModelMetadataDto`。缺失、为 null 或仅含空白的 `context_length` 可补入正数窗口。只有模型行缺少 `supports_vision` 且元数据确认支持图片输入时才建议 `true`。用户显式设定的 `false` 和 `true` 均保留。返回内存差异与贡献字段的来源和抓取时间 |
| `tests/components/modelMetadataFill.test.ts` 的 Hermes 场景 | `crates/vw-agent-apps/src/models/fill_hermes_tests.rs` 验证空窗口、用户设置、文字和其他模态、无效窗口、来源筛选，以及用 `providers::hermes::normalize_declaration` 取得原生模型行后合并差异时保留用户自定义参数 |
| `HermesFormFields.tsx` 的补全调用者 | 现有原生表单读取和写入准备由 `providers/hermes/form.rs::view` 和 `prepare` 负责，发布由 `providers/hermes.rs::HermesFiles` 处理。现有 Hermes 表单网关、SDK 和 Iced 分别位于 `crates/vw-agent/src/gateway/api/handlers/app_providers/hermes.rs`、`crates/vw-gateway-client/src/client/app_providers/hermes.rs` 和 `crates/vw-desktop/src/app/components/app_providers/hermes.rs`。本批次验证原生纯投影。目录补全预览及修订提交由任务 32 组装，网关和 SDK 由任务 38 接线，Iced 来源展示由任务 41 接线 |

### 验证

红灯与绿灯均动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。Cargo 中间目录使用同一槽位。格式化在 Git 源码快照中执行，仅回写本项 Rust 文件。测试和 Clippy 在实际工作区执行。

| 命令 | 退出码 | 证据 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、绿灯） | 0、0 | 两阶段格式化完成 |
| `cargo test -p vw-agent-apps catalog_fill_hermes_ -- --nocapture`（红灯） | 101 | 5 条有效用例，1 项通过、4 项失败、662 项过滤。指定用例实际返回空差异 `{}`，预期为 `{"context_length":262144}` |
| `cargo test -p vw-agent-apps catalog_fill_hermes_ -- --nocapture`（绿灯） | 0 | 5 项通过、662 项过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区、全目标及 features 通过 |

审查后差异限于 Hermes 投影、独立测试、模块注册和本记录。`git diff --cached --check` 通过。两个阶段的锁已释放，编译缓存已保留。
