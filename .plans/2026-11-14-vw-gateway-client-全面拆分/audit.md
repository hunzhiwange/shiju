# 源码与依赖审计

本轮仅生成计划；源码实现未拆分，动态行为全部未验证。完整284个源文件（含测试）逐项见 source-map.md，78张实施卡覆盖全部职责，不整体搬迁大包。

## 审计入口与现有依赖

包内只有 Cargo.toml 和 src/**/*.rs，没有 build.rs 或其他包内资源；无 include_str!/include_bytes! 引用。Cargo.toml 无 features 表。普通依赖：serde(derive)、serde_json、tracing、futures-util、vw-api-types、vw-shared；原生 reqwest 0.13 禁默认特性，启用 blocking/multipart/json/query/stream/rustls，tokio rt/time，tokio-tungstenite 0.29 rustls-tls-webpki-roots；wasm reqwest json/query/stream。测试 tokio macros/rt-multi-thread。清单条件不能在拆包时提升为无条件依赖。

直接消费者由 Cargo.toml 确认只有 vw-provider-resolver、vw-desktop、vw-agent、vw-cli（根 workspace 是注册者）；具体导入位置见 consumer-refs.md。下表消费者按当前源码符号出现筛选，为直接调用定位入口，迁移时进一步沿实例调用搜索，不将注释/同名匹配当作精确调用图。

## 状态与依赖方向

GatewayClient 当前字段只有 endpoint: GatewayEndpoint、client: reqwest::Client、原生 dynamic_host_client: reqwest::Client；没有应用级 Mutex/Arc 共享可变状态，连接池状态由 reqwest 内部持有。Clone 应保留连接池复用。测试捕获日志使用的 Arc/Mutex 不下沉到生产契约。VoiceStream、GatewayChatWebSocket、PtyConnection、ExternalSessionStream 各自拥有连接/流的可变状态，不能搬进通用传输。所有 impl GatewayClient 的领域扩展迁为显式领域结构体，禁止让新库依赖旧聚合客户端。

拟新增 GatewayTransport 只负责已有 HTTP 行为；端点库不依赖它。纯 session 类型继续使用现有 vw-api-types::session；stream 中 typed usage/tool/recovery 等客户端规整类型留聊天领域，不重复创建 API DTO。

```text
vw-api-types::id → gateway-endpoint → gateway-http
                                     ├→ 叶子领域客户端
                                     └→ gateway-chat-stream（SSE解析与订阅）
领域客户端 → vw-gateway-client（公开兼容门面）
领域客户端 → 直接消费者（内部调用改直连职责库）
```

图中箭头从基础指向使用者；下表方法边使用“调用者 → 被调用领域”。迁移后不允许领域回依赖门面。当前原模块以同一 impl 聚合，不存在 Rust 模块循环依赖约束，但形成私有方法访问耦合；消除宿主耦合后，下表跨域方法边均向前，无需用动态分派解环。

## 依赖深度与迁移顺序依据

0层：端点契约，只直接使用 vw-api-types（1个外部依赖）。1层：HTTP传输，使用端点、serde/serde_json、tracing、reqwest 与 API 租户头（比端点更多外依赖）。2层：仅使用传输与 DTO 的叶子领域；其中纯 DTO/配置/项目先于原生 WebSocket/SSE 的 futures-util/tokio/tungstenite 重依赖路径。3层：项目→文件、配置→桌面设置、应用读取→外部会话/MCP读取、应用生命周期与提示目录→提示资源、账号敏感请求→识矩业务。最后门面及全消费者收口。与该偏序无关的同层模块按已有目录邻接安排，全部串行；不宣称按行数证明编译收益。

## 现有库复用与前序计划依赖

- 实际存在的 vw-api-types/src/lib.rs 导出 id/session/chat/project/file/workflow 等模块；vw-shared::provider::types::Info 与 question/session 模块继续按源码引用。前序 `.plans/2026-11-14-vw-api-types-全面拆分`、`vw-shared-全面拆分`、`vw-provider-resolver-全面拆分` 下的目标仍是计划，不能当作现有接口。
- gateway-pairing 实际只导出 pairing（PairingGuard、hash_skey 等服务端配对鉴权）；GatewayAuth 是客户端 Bearer 参数，职责不同，不合并也不另造配对库。
- app-config-paths 导出 bounded/read/execution_environment 等本地路径能力，app-icons 导出 name/validation/library，app-provider-validation 导出 validate_identity/profile/endpoints/request_options/validation。当前客户端对应的是远端 HTTP 线协议，不能以同名路径/图标/校验为由搬入这些服务端库；沿用已有 API DTO，不复制它们的本地实现。
- 没有现成 gateway HTTP/SSE 客户端领域库。tui-stream 的终端展示职责也不等同于网关传输协议，保持单一实现所有者。

## 候选库、源路径、消费者及跨域契约

源路径全部相对于 crates/vw-gateway-client/src；每行目标路径为 crates/libs/<目标>（门面除外）。直接外部依赖按实际源码限定符统计，不包含 std 及库内引用；传输访问器使用的间接依赖见公共规则。真实函数列表在 source-map.md，避免在这里虚构未来接口。

| 任务/职责 | 目标 | 来源文件 | 直接外部依赖 | 消费者符号命中包 | 领域调用契约 |
| --- | --- | --- | --- | --- | --- |
| 1 端点契约 | gateway-endpoint | `endpoint.rs` | vw_api_types (1) | vw-provider-resolver, vw-desktop, vw-agent, vw-cli | 仅传输/领域内调用 |
| 2 HTTP传输 | gateway-http | `http.rs`, `client/request_helpers.rs` | reqwest, serde, serde_json, tracing, vw_api_types (5) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 3 聊天事件与SSE | gateway-chat-stream | `stream.rs` | futures_util, reqwest, serde, serde_json, tokio, tracing, vw_api_types (7) | vw-desktop, vw-agent, vw-cli | 仅传输/领域内调用 |
| 4 配置访问 | gateway-config | `client/config_api.rs` | serde, serde_json, vw_api_types (3) | vw-provider-resolver, vw-desktop, vw-agent, vw-cli | patch_json → gateway-http; put_json → gateway-http; endpoint → gateway-http; get_json → gateway-http; post_json → gateway-http |
| 5 项目与工作树 | gateway-project | `client/project_api.rs`, `client/worktree_management_api.rs` | serde_json, vw_api_types (2) | vw-desktop, vw-agent, vw-cli | patch_json → gateway-http; endpoint → gateway-http; delete_json → gateway-http; get_json → gateway-http; post_json → gateway-http |
| 6 文件访问 | gateway-files | `client/file_api.rs` | serde, serde_json, vw_api_types (3) | vw-desktop, vw-agent, vw-cli | get_json → gateway-http; endpoint → gateway-http; project_get → gateway-project; post_json → gateway-http |
| 7 项目档案 | gateway-project-profile | `client/project_profile_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop | endpoint → gateway-http; native_transport → gateway-http |
| 8 会话访问 | gateway-session | `client/session_api.rs` | serde, serde_json, vw_api_types, vw_shared (4) | vw-desktop, vw-agent, vw-cli | put_json → gateway-http; delete_json → gateway-http; patch_json → gateway-http; delete_empty → gateway-http; get_json → gateway-http; get_json_with_404 → gateway-http; post_json → gateway-http |
| 9 模型提供商 | gateway-model-provider | `client/provider_api.rs` | serde, vw_shared (2) | vw-provider-resolver, vw-desktop, vw-agent, vw-cli | get_json → gateway-http; post_json → gateway-http |
| 10 模型目录 | gateway-model-catalog | `client/model_catalog_api.rs`, `client/model_catalog_edits_api.rs`, `client/model_discovery_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent, vw-cli | 仅传输/领域内调用 |
| 11 应用读取 | gateway-app-read | `client/agent_apps_read_api.rs`, `client/agent_apps_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | get_json → gateway-http; patch_json → gateway-http; delete_empty → gateway-http; post_json → gateway-http |
| 12 应用生命周期 | gateway-app-lifecycle | `client/agent_apps_lifecycle_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 13 应用授权 | gateway-app-auth | `client/agent_apps_auth_read.rs`, `client/agent_apps_auth_flow.rs`, `client/agent_apps_auth_accounts.rs` | reqwest, serde, vw_api_types (3) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 14 应用设置 | gateway-app-settings | `client/agent_apps_settings_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 15 应用高可用 | gateway-app-failover | `client/agent_apps_high_availability_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | native_transport → gateway-http |
| 16 应用诊断 | gateway-app-diagnostics | `client/agent_apps_diagnostics_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 17 应用供应商记录 | gateway-app-provider | `client/app_providers_api.rs`, `client/app_providers/basic.rs`, `client/app_providers/catalog.rs`, `client/app_providers/create.rs`, `client/app_providers/update.rs`, `client/app_providers/delete.rs`, `client/app_providers/duplicate.rs`, `client/app_providers/presentation.rs`, `client/app_providers/endpoints.rs`, `client/app_providers/icons.rs`, `client/app_providers/universal.rs`, `client/app_providers/universal_sync.rs`, `client/app_providers/paths.rs`, `client/app_providers/preview.rs`, `client/app_providers/editor_merge.rs`, `client/app_providers/snippets.rs`, `client/app_providers/import.rs`, `client/app_providers/mode_state.rs`, `client/app_providers/mode_apply.rs`, `client/app_providers/env_diagnose.rs`, `client/app_providers/env_clean.rs`, `client/app_providers/env_restore.rs`, `client/app_providers/operations.rs`, `client/app_providers/request_options.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-provider-resolver, vw-desktop, vw-agent, vw-cli | get_json → gateway-http |
| 18 供应商适配-claude | gateway-app-provider | `client/app_providers/claude.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 19 供应商适配-codex_catalog | gateway-app-provider | `client/app_providers/codex_catalog.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 20 供应商适配-gemini | gateway-app-provider | `client/app_providers/gemini.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 21 供应商适配-claude_desktop | gateway-app-provider | `client/app_providers/claude_desktop.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 22 供应商适配-grok | gateway-app-provider | `client/app_providers/grok.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 23 供应商适配-desktop_mapping | gateway-app-provider | `client/app_providers/desktop_mapping.rs` | vw_api_types (1) | vw-desktop, vw-agent | claude_desktop_request → gateway-app-provider |
| 24 供应商适配-opencode | gateway-app-provider | `client/app_providers/opencode.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 25 供应商适配-hermes | gateway-app-provider | `client/app_providers/hermes.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 26 供应商适配-openclaw | gateway-app-provider | `client/app_providers/openclaw.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 27 供应商适配-pi | gateway-app-provider | `client/app_providers/pi.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 28 供应商适配-omp | gateway-app-provider | `client/app_providers/omp.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 29 供应商适配-mcode | gateway-app-provider | `client/app_providers/mcode.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 30 供应商适配-kimi | gateway-app-provider | `client/app_providers/kimi.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 31 供应商适配-dsh | gateway-app-provider | `client/app_providers/dsh.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 32 供应商适配-agy | gateway-app-provider | `client/app_providers/agy.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 共享错误/路径契约 → gateway-app-read |
| 33 应用路由 | gateway-app-routing | `client/app_routing/client_host.rs`, `client/app_routing/config_api.rs`, `client/app_routing/lifecycle_api.rs`, `client/app_routing/mod.rs`, `client/app_routing/mode_attach.rs`, `client/app_routing/outbound_api.rs`, `client/app_routing/outbound_probe.rs`, `client/app_routing/outbound_scan.rs`, `client/app_routing/request_status.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 34 应用数据迁移备份 | gateway-app-data | `client/agent_apps_data/backup_sdk.rs`, `client/agent_apps_data/migration_sdk.rs`, `client/agent_apps_data/mod.rs`, `client/backup_inventory_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | endpoint → gateway-http; native_transport → gateway-http |
| 35 应用技能分发 | gateway-app-skills | `client/agent_apps_skills/backups_api.rs`, `client/agent_apps_skills/bindings_api.rs`, `client/agent_apps_skills/claude_api.rs`, `client/agent_apps_skills/discovery_api.rs`, `client/agent_apps_skills/import_api.rs`, `client/agent_apps_skills/install_api.rs`, `client/agent_apps_skills/inventory_api.rs`, `client/agent_apps_skills/jobs_api.rs`, `client/agent_apps_skills/mod.rs`, `client/agent_apps_skills/repos_api.rs`, `client/agent_apps_skills/search_api.rs`, `client/agent_apps_skills/storage_api.rs`, `client/agent_apps_skills/uninstall_api.rs`, `client/agent_apps_skills/updates_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | get_json → gateway-http; delete_json → gateway-http; native_transport → gateway-http; post_json → gateway-http |
| 36 提示目录 | gateway-app-prompts | `client/agent_app_prompts_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop | native_transport → gateway-http |
| 37 提示资源 | gateway-app-prompt-resources | `client/agent_app_prompt_resources_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 共享错误/路径契约 → gateway-app-prompts; 共享错误/路径契约 → gateway-app-lifecycle; agent_apps_lifecycle_request → gateway-app-lifecycle |
| 38 外部会话 | gateway-external-session | `client/external_sessions/catalog.rs`, `client/external_sessions/compression.rs`, `client/external_sessions/content.rs`, `client/external_sessions/delete.rs`, `client/external_sessions/history_settings.rs`, `client/external_sessions/images.rs`, `client/external_sessions/maintenance.rs`, `client/external_sessions/markdown.rs`, `client/external_sessions/messages.rs`, `client/external_sessions/mod.rs`, `client/external_sessions/resume.rs`, `client/external_sessions/stream.rs` | reqwest, serde_json, vw_api_types (3) | vw-desktop, vw-agent | agent_apps_read_request → gateway-app-read; 共享错误/路径契约 → gateway-app-read |
| 39 工作区记忆 | gateway-workspace-memory | `client/workspace_memory_api.rs`, `client/workspace_directory_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent, vw-cli | native_transport → gateway-http |
| 40 Hermes记忆 | gateway-hermes-memory | `client/hermes_memory_api.rs`, `client/hermes_dashboard_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | native_transport → gateway-http |
| 41 OpenClaw设置 | gateway-openclaw-settings | `client/openclaw_settings_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop | native_transport → gateway-http |
| 42 应用额度 | gateway-app-usage | `client/apps_usage_read_api.rs`, `client/apps_usage_write_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 43 设备同步 | gateway-device-sync | `client/device_sync_operations_api.rs`, `client/device_sync_config_api.rs`, `client/device_sync_tasks_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | native_transport → gateway-http |
| 44 用量读取 | gateway-usage-read | `client/usage_read_types.rs`, `client/usage_read_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | native_transport → gateway-http |
| 45 用量扫描 | gateway-usage-scan | `client/usage_scan_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | native_transport → gateway-http |
| 46 用量价格 | gateway-usage-price | `client/usage_price_types.rs`, `client/usage_price_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | native_transport → gateway-http |
| 47 聚合成员 | gateway-aggregation | `client/aggregation_members_api.rs`, `client/aggregation_publish_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | native_transport → gateway-http |
| 48 聚合OMO | gateway-aggregation-omo | `client/aggregation_omo_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop | native_transport → gateway-http |
| 49 聚合原生目录 | gateway-aggregation-native | `client/aggregation_native_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop | 仅传输/领域内调用 |
| 50 聚合桌面 | gateway-aggregation-desktop | `client/aggregation_desktop_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop | native_transport → gateway-http |
| 51 聚合Codex | gateway-aggregation-codex | `client/aggregation_codex_api.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | native_transport → gateway-http |
| 52 MCP访问 | gateway-mcp | `client/mcp_api.rs` | serde_json, vw_api_types (2) | vw-desktop, vw-agent, vw-cli | get_json → gateway-http; post_json → gateway-http |
| 53 MCP读取 | gateway-mcp-read | `client/mcp_management_read.rs` | serde_json, vw_api_types (2) | vw-desktop | agent_apps_read_request → gateway-app-read |
| 54 MCP写入 | gateway-mcp-write | `client/mcp_management_write.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 55 MCP探测 | gateway-mcp-probe | `client/mcp_management_probe.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 56 MCP分发 | gateway-mcp-distribution | `client/mcp_management_distribution.rs` | reqwest, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent | 仅传输/领域内调用 |
| 57 推理与WebSocket | gateway-inference | `client/inference_api.rs` | futures_util, serde, serde_json, tokio, tokio_tungstenite, vw_api_types (6) | vw-desktop, vw-agent, vw-cli | post_json → gateway-http |
| 58 PTY终端 | gateway-pty | `client/pty_api.rs` | futures_util, reqwest, serde, serde_json, tokio, tokio_tungstenite, vw_api_types (7) | vw-desktop, vw-agent, vw-cli | get_json → gateway-http; put_json → gateway-http; delete_json → gateway-http; post_json → gateway-http |
| 59 语音HTTP与流 | gateway-voice | `client/voice_api.rs`, `voice_stream.rs` | futures_util, reqwest, serde, serde_json, tokio, tokio_tungstenite, vw_api_types (7) | vw-provider-resolver, vw-desktop, vw-agent, vw-cli | 仅传输/领域内调用 |
| 60 工作流 | gateway-workflow | `client/workflow_api.rs` | futures_util, serde, serde_json, vw_api_types (4) | vw-desktop, vw-agent, vw-cli | get_json → gateway-http; put_json → gateway-http; delete_json → gateway-http; post_json → gateway-http |
| 61 知识访问 | gateway-knowledge | `client/knowledge_api.rs` | serde_json, vw_api_types (2) | vw-desktop, vw-agent, vw-cli | get_json → gateway-http; delete_json → gateway-http; patch_json → gateway-http; post_json → gateway-http |
| 62 任务调度 | gateway-scheduler | `client/agent_scheduler_api.rs` | vw_api_types (1) | vw-desktop, vw-agent, vw-cli | put_json → gateway-http; endpoint → gateway-http; patch_json → gateway-http; get_json → gateway-http; post_json → gateway-http |
| 63 任务生命周期 | gateway-task-lifecycle | `client/task_lifecycle_api.rs` | serde_json, vw_api_types (2) | vw-desktop, vw-cli | get_json → gateway-http; put_json → gateway-http; delete_json → gateway-http; post_json → gateway-http |
| 64 任务池 | gateway-task-pool | `client/task_pool_api.rs` | futures_util, reqwest, serde, serde_json, vw_api_types (5) | vw-desktop, vw-agent, vw-cli | get_json → gateway-http; post_json → gateway-http |
| 65 Cron调度 | gateway-cron | `client/cron_api.rs` | serde (1) | vw-desktop, vw-cli | patch_json → gateway-http; put_json → gateway-http; delete_empty → gateway-http; get_json → gateway-http; post_json → gateway-http |
| 66 工具与交互 | gateway-interaction | `client/tools_api.rs`, `client/question_api.rs`, `client/permission_api.rs`, `client/control_api.rs`, `client/operations_api.rs` | serde, serde_json, vw_api_types, vw_shared (4) | vw-desktop, vw-agent, vw-cli | delete_json → gateway-http; put_json → gateway-http; get_json → gateway-http; post_json → gateway-http |
| 67 宿主工具 | gateway-host-tools | `client/dynamic_tools_api.rs` | futures_util, reqwest, serde_json, vw_api_types (4) | vw-agent | 仅传输/领域内调用 |
| 68 计算机控制 | gateway-computer-control | `client/computer_control_api.rs` | reqwest, serde, vw_api_types (3) | vw-desktop | native_transport → gateway-http |
| 69 系统访问 | gateway-system | `client/system_api.rs` | serde_json (1) | vw-desktop, vw-agent, vw-cli | get_json → gateway-http |
| 70 渠道授权 | gateway-channel-qr | `client/channel_qr_api.rs` | serde, serde_json (2) | vw-desktop, vw-agent | get_json → gateway-http; post_json → gateway-http |
| 71 识矩账号敏感请求 | gateway-shiju-account | `client/shiju_account_api.rs` | reqwest, serde, tracing (3) | vw-provider-resolver, vw-desktop, vw-agent, vw-cli | 仅传输/领域内调用 |
| 72 识矩应用 | gateway-shiju-app | `client/shiju_app_api.rs` | reqwest, serde (2) | vw-desktop, vw-agent, vw-cli | shiju_empty_sensitive_request → gateway-shiju-account |
| 73 识矩连接器 | gateway-shiju-connector | `client/shiju_connector_api.rs` | reqwest, serde, serde_json (3) | vw-desktop, vw-agent | shiju_account_sensitive_request → gateway-shiju-account |
| 74 识矩知识 | gateway-shiju-knowledge | `client/shiju_knowledge_api.rs` | reqwest, serde (2) | vw-desktop, vw-agent, vw-cli | shiju_account_sensitive_request → gateway-shiju-account |
| 75 识矩技能 | gateway-shiju-skill | `client/shiju_skill_api.rs` | reqwest, serde, serde_json (3) | vw-desktop, vw-agent, vw-cli | shiju_empty_sensitive_request → gateway-shiju-account |
| 76 技能市场 | gateway-skill-market | `client/skill_market_api.rs` | serde (1) | vw-desktop, vw-agent, vw-cli | get_json → gateway-http; post_json → gateway-http |
| 77 桌面设置访问 | gateway-desktop-settings | `client/desktop_settings_api.rs` | serde, serde_json, vw_api_types (3) | vw-desktop, vw-agent, vw-cli | delete_json → gateway-http; patch_json → gateway-http; get_json → gateway-http; 共享错误/路径契约 → gateway-config; global_config_get_path → gateway-config; post_json → gateway-http; global_config_patch_path → gateway-config |
| 78 发布门面与消费者收口 | vw-gateway-client | `lib.rs`, `session.rs`, `client/mod.rs`, `client/git_api.rs`, `client/test_support.rs` | reqwest, vw_api_types (2) | vw-provider-resolver, vw-desktop, vw-agent, vw-cli | post_json → gateway-http |

## 大文件、跨域模块与保留内容

- stream.rs 980行：客户端typed事件与规整函数、SSE帧解析、HTTP阻塞/异步/恢复订阅分模块放入 gateway-chat-stream；仍保留一个协议领域所有者，不能搬成新巨型lib.rs。
- session_api.rs 813行：在 gateway-session 内按CRUD、消息分页/原始载荷、归档、UI/标题拆文件；SessionUiLoadResult 的原始LLM保留语义不丢失。
- desktop_settings_api.rs 814行：按DTO目录、技能、插件、hooks、外部路径、service、preferences分文件；配置路径方法由 gateway-config 唯一拥有。
- workflow_api.rs 561行：记录CRUD与运行、流式事件解析分文件。cron_api.rs 528行：DTO/请求与调度约束分文件。shiju_skill_api.rs 518行：DTO/分页查询与私有技能请求分文件；共享敏感请求只由账号库拥有。
- app_providers_api.rs 281行多为path模块声明，24个基础切片和15个应用适配器按任务分阶段迁入同一领域；不复制 provider_path。agent_apps_skills 与 external_sessions 按原有子模块保留，不聚成单一文件。
- request_helpers 中 get_json_blocking 当前单独构造 blocking Client，保留其现状（不借拆分改代理语义）；get_json_with_404 的 Option/404行为不统一成别的helper。
- 门面 lib.rs、client/mod.rs 只保留公开重导出、GatewayClient组合与显式兼容转发；session.rs 只保留 deprecated 别名，不能以发布兼容为由保留业务实现。测试 tests.rs、client/tests.rs 保留门面兼容断言，test_support 仅属测试固件。

## 已发现的源码风险与静态限制

app_providers_api.rs 使用大量 #[path] 指向 app_providers 子目录；已逐项核对这些路径均存在。迁移适配器时必须同步修改父模块的 path 与测试路径，保持 dsh 等实际文件名与方法名不变；不根据名称推测路径缺失。

索引 status 有7988个文件，但 explore 返回大量重名/截断，未用索引完整性证明零遗漏；本表以工作树枚举284个Rust文件闭合。跨域边由 self方法定义匹配并人工核对代表性helper；同名方法可能有候选边，不作为动态调用证据。

后续统一验收（当前未验证）：四消费者编译/已有聚焦断言、Bearer与租户隔离、敏感账号/令牌日志脱敏、动态宿主禁止重定向重试、wasm明确拒绝原生授权请求、请求URL/查询与错误兼容、SSE分帧/恢复/idle、WebSocket关闭/语音二进制帧、文件目录回退、会话原始LLM载荷、任务与工作流取消及发布门面兼容。用户以后另行授权，不自动进入回归。
