# 配置源码审计与归属

审计依据：本包全部源文件、模块/跨域引用、清单和调用路径静态读取。RustCodeGraph status 有索引，explore 未及时返回，已取消；故回退源码枚举。本审计不代表编译通过。

## 现有库与计划去重

app-config-paths 导出 execution_environment/bounded/read/open_beneath 等外部应用配置探测，依赖 API 类型；不能承载本包纯目录命名。project-storage-paths 导出 project_data_dir/workspace_data_dir，当前依赖 vw-config-types；改为依赖新路径叶子以解耦。app-provider-validation 使用 AppProviderWrite，属于外部应用供应商校验，并非 ModelProviderConfig。均不重复搬实现。系统22—26没有旧全面子计划；后续 shared/API/provider-resolver 子计划接口尚未落地，不当成现有依赖。

## 清单、资源与状态

本包 Cargo.toml 无 features、build 配置、target 依赖或额外资源；唯一 derive feature 为 serde/derive。源码未发现 include_str!/include_bytes!、共享 Mutex/RwLock/可变静态。静态工具与代理目录常量只读。cfg 涉及 debug_assertions 路径与端口、agent/config/chat 的 wasm32 分支及 test。

直接清单消费者：vw-provider-resolver、vw-shared、vw-acp、vw-cli、vw-agent、vw-desktop、libs/gateway-pairing、libs/project-storage-paths。间接消费者还经 vw-shared 再导出获取类型，收口必须全仓查找再导出，不能只替换显式导入。

## 逐源文件归属

表内路径相对 crates/vw-config-types/src；完整源文件覆盖，测试与实现分别列出。分层卡的新增文件最终随迁库卡迁移。

| 源文件 | 行数 | 最终所有者/保留理由 |
| --- | ---: | --- |
| `acp_install.rs` | 181 | 任务2：config-acp-install |
| `acp_install_tests.rs` | 170 | 任务2：config-acp-install |
| `agent.rs` | 537 | 任务20：config-agent |
| `agent_profile.rs` | 85 | 任务19：config-agent-profile |
| `agent_profile_tests.rs` | 118 | 契约断言→任务19；AgentDefinitionConfig/合并断言→任务20 |
| `agent_tests.rs` | 545 | 任务20：config-agent |
| `automation.rs` | 97 | 任务6：config-automation |
| `automation_tests.rs` | 39 | 任务6：config-automation |
| `channels/chat.rs` | 421 | 任务26：config-channels |
| `channels/chat_tests.rs` | 110 | 任务26：config-channels |
| `channels/clawdtalk.rs` | 26 | 任务25：config-channel-services |
| `channels/clawdtalk_tests.rs` | 9 | 任务25：config-channel-services |
| `channels/email.rs` | 78 | 任务25：config-channel-services |
| `channels/email_tests.rs` | 9 | 任务25：config-channel-services |
| `channels/lark.rs` | 158 | 任务25：config-channel-services |
| `channels/lark_tests.rs` | 54 | 任务25：config-channel-services |
| `channels/mod.rs` | 26 | 任务26：config-channels |
| `channels/other.rs` | 199 | 任务25：config-channel-services |
| `channels/other_tests.rs` | 188 | 任务25：config-channel-services |
| `channels/types.rs` | 237 | 任务24：config-channel-types |
| `channels/types_tests.rs` | 236 | 任务24：config-channel-types |
| `channels/wechat.rs` | 31 | 任务25：config-channel-services |
| `channels/wechat_tests.rs` | 19 | 任务25：config-channel-services |
| `channels/wecom.rs` | 44 | 任务25：config-channel-services |
| `channels/wecom_tests.rs` | 40 | 任务25：config-channel-services |
| `computer_control.rs` | 41 | 任务14：config-computer-control |
| `computer_control_tests.rs` | 45 | 任务14：config-computer-control |
| `config.rs` | 484 | 任务29：config-aggregate |
| `config_tests.rs` | 375 | 任务29：config-aggregate |
| `gateway.rs` | 249 | 任务10：config-gateway |
| `gateway_tests.rs` | 49 | 任务10：config-gateway |
| `hooks.rs` | 57 | 任务7：config-hooks |
| `hooks_tests.rs` | 24 | 任务7：config-hooks |
| `lib.rs` | 42 | 原包薄发布门面（收口任务） |
| `memory.rs` | 264 | 任务11：config-memory |
| `memory_tests.rs` | 88 | 任务11：config-memory |
| `observability.rs` | 58 | 任务8：config-observability |
| `observability_tests.rs` | 26 | 任务8：config-observability |
| `paths.rs` | 91 | 任务1：config-runtime-paths |
| `paths_tests.rs` | 57 | 任务1：config-runtime-paths |
| `plugins.rs` | 48 | 任务16：config-plugins |
| `plugins_tests.rs` | 38 | PluginsConfig断言→config-plugins；Config组合断言→config-aggregate |
| `provider.rs` | 72 | 任务3：config-provider |
| `provider_tests.rs` | 80 | 任务3：config-provider |
| `proxy.rs` | 208 | 任务13：config-proxy |
| `proxy_tests.rs` | 158 | 任务13：config-proxy |
| `reliability.rs` | 65 | 任务5：config-reliability |
| `reliability_tests.rs` | 33 | 任务5：config-reliability |
| `routing.rs` | 99 | 任务4：config-routing |
| `routing_tests.rs` | 9 | 任务4：config-routing |
| `runtime.rs` | 213 | 任务9：config-runtime |
| `runtime_tests.rs` | 25 | 任务9：config-runtime |
| `security.rs` | 902 | 任务23：config-security |
| `security_tests.rs` | 166 | 任务23：config-security |
| `skills.rs` | 143 | 任务12：config-skills |
| `skills_tests.rs` | 197 | 任务12：config-skills |
| `tools.rs` | 485 | 任务18：config-tools |
| `tools_tests.rs` | 384 | 任务18：config-tools |
| `ui.rs` | 701 | 任务28：config-ui |
| `ui_tests.rs` | 715 | 任务28：config-ui |
| `voice.rs` | 141 | 任务15：config-voice |
| `voice_tests.rs` | 78 | VoiceConfig断言→config-voice；AppUiConfig组合断言→config-ui |
| `worktree.rs` | 56 | 任务21：config-worktree |
| `worktree_tests.rs` | 55 | 任务21：config-worktree |

## 领域外依赖、契约和直接调用文件

外依赖按源码实际引用列出，std 不计；serde_json 仅在测试出现时是 dev-dependency。路径及安装平台是 std-only 起点，之后同深度序列化叶子（serde/schemars 两项），额外 JSON、时区、环境依赖后置；有跨域边的组合根最后。清单最终以迁移后的真实引用裁剪。

### config-runtime-paths（任务1）

来源：paths.rs；目标：crates/libs/config-runtime-paths。职责：运行目录契约。外依赖：无。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/libs/project-storage-paths/src/paths.rs`
- `crates/libs/project-storage-paths/src/paths_tests.rs`
- `crates/vw-acp/src/config.rs`
- `crates/vw-acp/src/config_tests.rs`
- `crates/vw-acp/src/queue_paths.rs`
- `crates/vw-acp/src/queue_paths_tests.rs`
- `crates/vw-acp/src/session_event_log.rs`
- `crates/vw-acp/src/session_event_log_tests.rs`
- `crates/vw-acp/src/session_events_tests.rs`
- `crates/vw-acp/tests/port_helpers.rs`
- `crates/vw-acp/tests/queue_paths.rs`
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs`
- `crates/vw-agent/src/agent_scheduler/store/mod.rs`
- `crates/vw-agent/src/agent_scheduler/store/schema_tests.rs`
- `crates/vw-agent/src/app.rs`
- `crates/vw-agent/src/app_tests.rs`
- `crates/vw-agent/src/channels/runtime_command/task_mode_tests.rs`
- `crates/vw-agent/src/channels/telegram/config_tests.rs`
- `crates/vw-agent/src/channels/wecom/runtime.rs`
- `crates/vw-agent/src/config/migration.rs`
- `crates/vw-agent/src/config/migration_tests.rs`
- `crates/vw-agent/src/config/schema/config_load.rs`
- `crates/vw-agent/src/config/schema/workspace.rs`
- `crates/vw-agent/src/config/store_tests.rs`
- `crates/vw-agent/src/cron/tests/scheduler.rs`
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/mod.rs`
- `crates/vw-agent/src/gateway/api/handlers/aggregation_omo_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/auth_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/desktop_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/file_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/global_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/instance.rs`
- `crates/vw-agent/src/gateway/api/handlers/instance_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/knowledge.rs`
- `crates/vw-agent/src/gateway/api/handlers/knowledge_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/ui_ops_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/skill_market.rs`
- `crates/vw-agent/src/gateway/api/handlers/task_lifecycle_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/workflow_tests.rs`
- `crates/vw-agent/src/global/mod.rs`
- `crates/vw-agent/src/global_database.rs`
- `crates/vw-agent/src/global_database/config.rs`
- `crates/vw-agent/src/global_database/config_tests.rs`
- `crates/vw-agent/src/global_database/test_support.rs`
- `crates/vw-agent/src/global_database_tests.rs`
- `crates/vw-agent/src/installation/mod.rs`
- `crates/vw-agent/src/integrations/mod.rs`
- `crates/vw-agent/src/memory/hygiene/tests.rs`
- `crates/vw-agent/src/project/migration.rs`
- `crates/vw-agent/src/project/migration_tests.rs`
- `crates/vw-agent/src/runtime/docker/mod.rs`
- `crates/vw-agent/src/runtime/docker/tests.rs`
- `crates/vw-agent/src/runtime/native/mod.rs`
- `crates/vw-agent/src/runtime/native/tests.rs`
- `crates/vw-agent/src/security/gateway_skey_store_tests.rs`
- `crates/vw-agent/src/service/openrc.rs`
- `crates/vw-agent/src/session/instruction.rs`
- `crates/vw-agent/src/session/ui_config_tests.rs`
- `crates/vw-agent/src/session/ui_store.rs`
- `crates/vw-agent/src/session/ui_store_tests.rs`
- `crates/vw-agent/src/skills/discovery_tests.rs`
- `crates/vw-agent/src/skills/mod.rs`
- `crates/vw-agent/src/storage/mod.rs`
- `crates/vw-agent/src/storage/tests.rs`
- `crates/vw-agent/src/task_lifecycle/store_tests.rs`
- `crates/vw-agent/src/task_plan_run/prompts.rs`
- `crates/vw-agent/src/task_plan_run/prompts_tests.rs`
- `crates/vw-agent/src/tools/browser/installation.rs`
- `crates/vw-agent/src/workflow/store_tests.rs`
- `crates/vw-agent/src/worktree/cleanup.rs`
- `crates/vw-agent/src/worktree/cleanup_flow_tests.rs`
- `crates/vw-agent/src/worktree/cleanup_host.rs`
- `crates/vw-agent/src/worktree/recovery_restore_tests.rs`
- `crates/vw-agent/src/worktree/registry.rs`
- `crates/vw-agent/src/worktree/worktree_native.rs`
- `crates/vw-agent/src/worktree/worktree_native_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/identity.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/identity_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/skills.rs`
- `crates/vw-desktop/src/app/components/system_settings_security.rs`
- `crates/vw-desktop/src/app/components/system_settings_skills/browser_tests.rs`
- `crates/vw-desktop/src/app/config_cli_bootstrap.rs`
- `crates/vw-desktop/src/app/config_cli_bootstrap_tests.rs`
- `crates/vw-desktop/src/app/config_cli_bootstrap_windows.rs`
- `crates/vw-desktop/src/app/config_gateway_tests.rs`
- `crates/vw-desktop/src/app/config_system_settings.rs`
- `crates/vw-desktop/src/app/config_system_settings_tests.rs`
- `crates/vw-desktop/src/app/impls/app_init/new.rs`
- `crates/vw-desktop/src/app/impls/app_init/new/settings.rs`
- `crates/vw-desktop/src/app/message/project/file_tree_batch_tests.rs`
- `crates/vw-desktop/src/app/message/project/file_tree_external_drop_tests.rs`
- `crates/vw-desktop/src/app/message/project/file_tree_move_drop_http_tests.rs`
- `crates/vw-desktop/src/app/message/project/helpers.rs`
- `crates/vw-desktop/src/app/message/settings/agents.rs`
- `crates/vw-desktop/src/app/message/settings/providers/models.rs`
- `crates/vw-desktop/src/app/message/settings/security.rs`
- `crates/vw-desktop/src/app/message/settings/security_tests.rs`
- `crates/vw-desktop/src/app/message/view/panel/settings.rs`
- `crates/vw-desktop/src/app/session/ui_projection.rs`
- `crates/vw-desktop/src/app/skill_hub/installer.rs`
- `crates/vw-desktop/src/app/skill_hub/installer_tests.rs`
- `crates/vw-desktop/src/app/skill_hub/native_tests.rs`
- `crates/vw-desktop/src/app/skill_hub/package_version_cache.rs`
- `crates/vw-desktop/src/app/state/settings/infrastructure.rs`
- `crates/vw-desktop/src/app/state/settings/infrastructure_tests.rs`
- `crates/vw-desktop/src/app/task/executor/runner.rs`
- `crates/vw-desktop/src/app/task/executor/runner_tests.rs`
- `crates/vw-desktop/src/app/task/executor/state_tests.rs`
- `crates/vw-desktop/src/app/task/executor/tests.rs`
- `crates/vw-desktop/src/app/task/executor/worktree_admin_tests.rs`
- `crates/vw-desktop/src/app/task/executor/worktree_pool.rs`
- `crates/vw-desktop/src/app/task/executor/worktree_pool_tests.rs`
- `crates/vw-desktop/src/app/task/store/paths.rs`
- `crates/vw-desktop/src/app/task/store/paths_tests.rs`
- `crates/vw-desktop/src/app/task/store/persistence_tests.rs`
- `crates/vw-desktop/src/app/theme_catalog/resolve.rs`
- `crates/vw-desktop/src/app/views/expert_hub/install_guide.rs`
- `crates/vw-desktop/src/app/web_install/installer.rs`
- `crates/vw-provider-resolver/src/auth_tests.rs`
- `crates/vw-provider-resolver/src/global.rs`
- `crates/vw-provider-resolver/src/global_tests.rs`
- `crates/vw-shared/src/auth/store.rs`
- `crates/vw-shared/src/auth/store_tests.rs`
- `crates/vw-shared/src/local_apps.rs`
- `crates/vw-shared/src/local_apps_tests.rs`
- `crates/vw-shared/src/runtime_log.rs`
- `crates/vw-shared/src/task/store.rs`
- `crates/vw-shared/src/task/store_tests.rs`

### config-acp-install（任务2）

来源：acp_install.rs；目标：crates/libs/config-acp-install。职责：ACP安装平台。外依赖：无。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-acp/src/install.rs`
- `crates/vw-acp/tests/agent_registry.rs`
- `crates/vw-agent/src/gateway/api/handlers/desktop_acp_install.rs`
- `crates/vw-desktop/src/app/components/system_settings_acp.rs`
- `crates/vw-desktop/src/app/message/settings/acp.rs`

### config-provider（任务3）

来源：provider.rs；目标：crates/libs/config-provider。职责：模型供应商。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/channels/manager/start_tests.rs`
- `crates/vw-agent/src/config/schema/provider.rs`
- `crates/vw-agent/src/providers/mod.rs`

### config-routing（任务4）

来源：routing.rs；目标：crates/libs/config-routing。职责：模型路由。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/config/schema/mod.rs`
- `crates/vw-desktop/src/app/message/settings/embedding_routes.rs`
- `crates/vw-desktop/src/app/message/settings/messages.rs`
- `crates/vw-desktop/src/app/message/settings/model_routes.rs`
- `crates/vw-desktop/src/app/message/settings/query_classification.rs`

### config-reliability（任务5）

来源：reliability.rs；目标：crates/libs/config-reliability。职责：重试可靠性。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/config/schema/mod.rs`

### config-automation（任务6）

来源：automation.rs；目标：crates/libs/config-automation。职责：自动调度。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/config/schema/automation.rs`

### config-hooks（任务7）

来源：hooks.rs；目标：crates/libs/config-hooks。职责：钩子配置。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/config/schema/mod.rs`

### config-observability（任务8）

来源：observability.rs；目标：crates/libs/config-observability。职责：可观测配置。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/config/schema/mod.rs`

### config-runtime（任务9）

来源：runtime.rs；目标：crates/libs/config-runtime。职责：运行容器。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/config/schema/mod.rs`
- `crates/vw-desktop/src/app/message/settings/runtime.rs`

### config-gateway（任务10）

来源：gateway.rs；目标：crates/libs/config-gateway。职责：网关隧道。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/libs/gateway-pairing/src/pairing.rs`
- `crates/libs/gateway-pairing/src/pairing_tests.rs`
- `crates/vw-agent/src/config/mod.rs`
- `crates/vw-agent/src/config/mod_tests.rs`
- `crates/vw-agent/src/config/schema/mod.rs`
- `crates/vw-agent/src/config/store_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/config_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/global_tests.rs`
- `crates/vw-agent/src/package_boundaries/gateway_pairing_tests.rs`
- `crates/vw-agent/src/security/gateway_skey_store.rs`
- `crates/vw-agent/src/security/gateway_skey_store/postgres.rs`
- `crates/vw-cli/src/cli/tui/runtime/gateway.rs`
- `crates/vw-cli/src/cli/tui/runtime/gateway_tests.rs`
- `crates/vw-cli/src/handlers/api/mod.rs`
- `crates/vw-desktop/src/app/components/system_settings_gateway_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_gateway_ui_refresh_23_tests.rs`
- `crates/vw-desktop/src/app/config_cli_bootstrap.rs`
- `crates/vw-desktop/src/app/impls/app_init/new.rs`
- `crates/vw-desktop/src/app/impls/app_init/new/settings.rs`
- `crates/vw-desktop/src/app/impls/app_init/new/state.rs`
- `crates/vw-desktop/src/app/message/settings/gateway.rs`
- `crates/vw-desktop/src/app/message/settings/gateway_client.rs`
- `crates/vw-desktop/src/app/message/settings/tunnel.rs`
- `crates/vw-desktop/src/app/state/settings/runtime.rs`
- `crates/vw-desktop/src/app/state/settings/runtime_tests.rs`
- `crates/vw-provider-resolver/src/config.rs`

### config-memory（任务11）

来源：memory.rs；目标：crates/libs/config-memory。职责：记忆存储。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/config/schema/mod.rs`
- `crates/vw-desktop/src/app/message/settings/memory.rs`

### config-skills（任务12）

来源：skills.rs；目标：crates/libs/config-skills。职责：技能目录。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/config/schema/config_env_tests.rs`
- `crates/vw-agent/src/config/schema/skills.rs`
- `crates/vw-agent/src/gateway/api/handlers/desktop_skills.rs`
- `crates/vw-agent/src/gateway/api/handlers/desktop_skills_tests.rs`
- `crates/vw-agent/src/session/llm/acp/skill_mounts/mod.rs`
- `crates/vw-agent/src/skill/audit.rs`
- `crates/vw-agent/src/skill/audit_tests.rs`
- `crates/vw-agent/src/skill/install.rs`
- `crates/vw-agent/src/skill/loader.rs`
- `crates/vw-agent/src/skill/loader_tests.rs`
- `crates/vw-agent/src/skills/audit/manifest.rs`
- `crates/vw-agent/src/skills/audit/manifest_tests.rs`
- `crates/vw-agent/src/skills/audit/markdown.rs`
- `crates/vw-agent/src/skills/audit/markdown_tests.rs`
- `crates/vw-agent/src/skills/audit/mod.rs`
- `crates/vw-agent/src/skills/audit/scan.rs`
- `crates/vw-agent/src/skills/audit/scan_tests.rs`
- `crates/vw-agent/src/skills/audit/tests.rs`
- `crates/vw-agent/src/skills/installer.rs`
- `crates/vw-agent/src/skills/installer_tests.rs`
- `crates/vw-agent/src/skills/loader.rs`
- `crates/vw-agent/src/skills/loader_tests.rs`
- `crates/vw-agent/src/skills/marketplace/install.rs`
- `crates/vw-agent/src/skills/marketplace/install_tests.rs`
- `crates/vw-agent/src/skills/mod.rs`
- `crates/vw-agent/src/skills/session_mounts.rs`
- `crates/vw-agent/src/skills/session_mounts_tests.rs`
- `crates/vw-cli/src/cli/tui/model/integration.rs`
- `crates/vw-desktop/src/app/components/system_settings_skills.rs`
- `crates/vw-desktop/src/app/components/system_settings_skills/browser.rs`
- `crates/vw-desktop/src/app/components/system_settings_skills/browser_tests.rs`
- `crates/vw-desktop/src/app/message/settings/messages.rs`
- `crates/vw-desktop/src/app/message/settings/skills_tests.rs`
- `crates/vw-desktop/src/app/state/settings/runtime.rs`
- `crates/vw-desktop/src/app/state/settings/runtime_tests.rs`

### config-proxy（任务13）

来源：proxy.rs；目标：crates/libs/config-proxy。职责：代理选择。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/config/schema/proxy.rs`
- `crates/vw-desktop/src/app/components/system_settings_proxy.rs`
- `crates/vw-desktop/src/app/components/system_settings_proxy_tests.rs`
- `crates/vw-desktop/src/app/message/settings/proxy.rs`
- `crates/vw-desktop/src/app/message/settings/proxy_tests.rs`
- `crates/vw-desktop/src/app/state/settings/infrastructure.rs`
- `crates/vw-desktop/src/app/state/settings/infrastructure_tests.rs`

### config-computer-control（任务14）

来源：computer_control.rs；目标：crates/libs/config-computer-control。职责：计算机授权。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/computer_control/chrome/bridge_tests.rs`
- `crates/vw-agent/src/computer_control/chrome/origin_policy_tests.rs`
- `crates/vw-agent/src/computer_control/host_tests.rs`
- `crates/vw-agent/src/computer_control/lifecycle_tests.rs`
- `crates/vw-agent/src/computer_control/native/input_tests.rs`
- `crates/vw-agent/src/computer_control/native/observation_tests.rs`
- `crates/vw-agent/src/computer_control/policy.rs`
- `crates/vw-agent/src/computer_control/policy_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/computer_control.rs`
- `crates/vw-agent/src/gateway/api/handlers/computer_control_tests.rs`
- `crates/vw-agent/src/tools/computer_control.rs`
- `crates/vw-agent/src/tools/computer_control/chrome.rs`
- `crates/vw-agent/src/tools/computer_control_tests.rs`
- `crates/vw-agent/src/tools/excel_control.rs`
- `crates/vw-agent/src/tools/excel_control_tests.rs`
- `crates/vw-agent/src/tools/word_control.rs`
- `crates/vw-agent/src/tools/word_control_tests.rs`

### config-voice（任务15）

来源：voice.rs；目标：crates/libs/config-voice。职责：语音配置。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/gateway/api/handlers/voice.rs`
- `crates/vw-agent/src/gateway/api/handlers/voice_realtime.rs`
- `crates/vw-agent/src/gateway/api/handlers/voice_settings.rs`
- `crates/vw-agent/src/gateway/api/handlers/voice_settings_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/voice_tests.rs`
- `crates/vw-agent/src/provider/voice/mod.rs`
- `crates/vw-agent/src/provider/voice/realtime.rs`
- `crates/vw-agent/src/provider/voice/realtime_handoff_tests.rs`
- `crates/vw-agent/src/provider/voice/realtime_preview_tests.rs`
- `crates/vw-agent/src/provider/voice/realtime_tests.rs`
- `crates/vw-agent/src/provider/voice/transcription.rs`
- `crates/vw-agent/src/provider/voice/transcription_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_voice.rs`
- `crates/vw-desktop/src/app/message/settings/voice.rs`
- `crates/vw-desktop/src/app/message/voice_dictation.rs`
- `crates/vw-desktop/src/app/message/voice_recordings_tests.rs`
- `crates/vw-desktop/src/app/voice/acceptance_tests.rs`
- `crates/vw-desktop/src/app/voice/conversation.rs`
- `crates/vw-desktop/src/app/voice/conversation_tests.rs`
- `crates/vw-desktop/src/app/voice/hotkeys_tests.rs`
- `crates/vw-desktop/src/app/voice/preferences.rs`
- `crates/vw-desktop/src/app/voice/preferences_tests.rs`
- `crates/vw-desktop/src/app/voice/preview.rs`
- `crates/vw-desktop/src/app/voice/preview_tests.rs`
- `crates/vw-desktop/src/voice_window_main.rs`

### config-plugins（任务16）

来源：plugins.rs；目标：crates/libs/config-plugins。职责：插件配置。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/gateway/api/handlers/hooks_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/plugins.rs`
- `crates/vw-agent/src/gateway/api/handlers/plugins_tests.rs`
- `crates/vw-agent/src/plugins/cli_tests.rs`
- `crates/vw-agent/src/plugins/compat/t06_tests.rs`
- `crates/vw-agent/src/plugins/compat/t07_tests.rs`
- `crates/vw-agent/src/plugins/compat/t08_tests.rs`
- `crates/vw-agent/src/plugins/compat/t14_tests.rs`
- `crates/vw-agent/src/plugins/compat/t15_tests.rs`
- `crates/vw-agent/src/plugins/compat/t16_tests.rs`
- `crates/vw-agent/src/plugins/compat/t17_tests.rs`
- `crates/vw-agent/src/plugins/compat/t18_tests.rs`
- `crates/vw-agent/src/plugins/compat/t21_tests.rs`
- `crates/vw-agent/src/plugins/compat/t51_tests.rs`
- `crates/vw-agent/src/plugins/hook_handler_tests.rs`
- `crates/vw-agent/src/plugins/loader.rs`
- `crates/vw-agent/src/plugins/loader_tests.rs`
- `crates/vw-agent/src/plugins/mcp_tests.rs`
- `crates/vw-agent/src/tools/mcp_common/tests.rs`

### config-tools（任务18）

来源：tools.rs；目标：crates/libs/config-tools。职责：工具配置迁库。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/config/schema/mod.rs`
- `crates/vw-agent/src/session/processor/helpers.rs`
- `crates/vw-agent/src/session/processor/helpers_tests.rs`
- `crates/vw-agent/src/tools/delegated_tools.rs`
- `crates/vw-agent/src/tools/http_request/mod.rs`
- `crates/vw-agent/src/tools/web_fetch/mod.rs`
- `crates/vw-agent/src/tools/web_fetch/tests.rs`
- `crates/vw-agent/src/tools/web_search_tool/mod.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/tools.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/tools_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_http_request.rs`
- `crates/vw-desktop/src/app/components/system_settings_web_search.rs`
- `crates/vw-desktop/src/app/config_gateway.rs`
- `crates/vw-desktop/src/app/impls/app_init/new.rs`
- `crates/vw-desktop/src/app/impls/app_init/new/settings.rs`
- `crates/vw-desktop/src/app/impls/app_update.rs`
- `crates/vw-desktop/src/app/impls/app_update_tests.rs`
- `crates/vw-desktop/src/app/message/settings/agents.rs`
- `crates/vw-desktop/src/app/message/settings/browser.rs`
- `crates/vw-desktop/src/app/message/settings/http_request.rs`
- `crates/vw-desktop/src/app/message/settings/http_request_tests.rs`
- `crates/vw-desktop/src/app/message/settings/multimodal.rs`
- `crates/vw-desktop/src/app/message/settings/web_search.rs`
- `crates/vw-desktop/src/app/state.rs`
- `crates/vw-desktop/src/app/state/settings/agents.rs`
- `crates/vw-desktop/src/app/state/settings/runtime.rs`
- `crates/vw-desktop/src/app/state/settings/runtime_tests.rs`
- `crates/vw-desktop/src/app/ui_types.rs`

### config-agent-profile（任务19）

来源：agent_profile.rs；目标：crates/libs/config-agent-profile。职责：代理档案契约。外依赖：serde、schemars、serde_json。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- 未找到显式模块路径外部调用；由 Config 字段/再导出间接消费，迁移时复核。

### config-agent（任务20）

来源：agent.rs；目标：crates/libs/config-agent。职责：代理工作区与定义。外依赖：serde、schemars、serde_json、shellexpand。原跨域引用：agent_profile、tools。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/agent/agent/builder.rs`
- `crates/vw-agent/src/agent/agent/builder_tests.rs`
- `crates/vw-agent/src/agent/agent/core.rs`
- `crates/vw-agent/src/agent/agent/core_tests.rs`
- `crates/vw-agent/src/agent/agent/turn_skills_tests.rs`
- `crates/vw-agent/src/agent/agent/turn_tests.rs`
- `crates/vw-agent/src/agent/catalog.rs`
- `crates/vw-agent/src/agent/catalog_tests.rs`
- `crates/vw-agent/src/agent/defaults.rs`
- `crates/vw-agent/src/agent/defaults_tests.rs`
- `crates/vw-agent/src/agent/loop_/core/constants.rs`
- `crates/vw-agent/src/agent/loop_/core/skills.rs`
- `crates/vw-agent/src/agent/loop_/query_engine.rs`
- `crates/vw-agent/src/agent/loop_/query_engine_skills_tests.rs`
- `crates/vw-agent/src/agent/mod.rs`
- `crates/vw-agent/src/agent/mod_tests.rs`
- `crates/vw-agent/src/agent/tests/helpers.rs`
- `crates/vw-agent/src/agent/tests/loop_history.rs`
- `crates/vw-agent/src/agent/tests/loop_tools.rs`
- `crates/vw-agent/src/channels/config_tests.rs`
- `crates/vw-agent/src/channels/manager/execution_binding.rs`
- `crates/vw-agent/src/channels/manager/execution_binding_tests.rs`
- `crates/vw-agent/src/channels/manager/start.rs`
- `crates/vw-agent/src/channels/manager/start_tests.rs`
- `crates/vw-agent/src/config/schema/config_save_tests.rs`
- `crates/vw-agent/src/config/schema/config_validate.rs`
- `crates/vw-agent/src/config/schema/mod.rs`
- `crates/vw-agent/src/cron/tests/scheduler.rs`
- `crates/vw-agent/src/gateway/api/handlers/config.rs`
- `crates/vw-agent/src/gateway/api/handlers/config_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/agent_profile_acceptance_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/stream.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/stream_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/ui_ops_tests.rs`
- `crates/vw-agent/src/gateway/bootstrap.rs`
- `crates/vw-agent/src/plugins/compat/t10_tests.rs`
- `crates/vw-agent/src/session/agent_binding.rs`
- `crates/vw-agent/src/session/agent_binding_tests.rs`
- `crates/vw-agent/src/session/capability_policy.rs`
- `crates/vw-agent/src/session/capability_policy_tests.rs`
- `crates/vw-agent/src/session/child_agent.rs`
- `crates/vw-agent/src/session/child_agent_tests.rs`
- `crates/vw-agent/src/session/llm/acp/config.rs`
- `crates/vw-agent/src/session/llm/acp/config_tests.rs`
- `crates/vw-agent/src/session/llm/acp/request_tests.rs`
- `crates/vw-agent/src/session/processor/runner.rs`
- `crates/vw-agent/src/session/processor/turn_policy.rs`
- `crates/vw-agent/src/task_plan_run/runner_tests.rs`
- `crates/vw-agent/src/tools/agent_tool/tests.rs`
- `crates/vw-agent/src/tools/client_tool_tests.rs`
- `crates/vw-agent/src/tools/config/mod.rs`
- `crates/vw-agent/src/tools/context/mod.rs`
- `crates/vw-agent/src/tools/decision/tests.rs`
- `crates/vw-agent/src/tools/delegated_tools.rs`
- `crates/vw-agent/src/tools/skill/mod_tests.rs`
- `crates/vw-agent/src/tools/toolset/mod.rs`
- `crates/vw-agent/src/tools/toolset/mod_tests.rs`
- `crates/vw-cli/src/cli/setup.rs`
- `crates/vw-cli/src/cli/single_message.rs`
- `crates/vw-cli/src/lib.rs`
- `crates/vw-desktop/src/agent_profile_window_main.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tool_selector.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tool_selector_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/basic.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/basic_profile.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/basic_profile_ui_refresh_15_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/basic_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/basic_ui_refresh_16_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/catalog.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/catalog_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/defaults.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/defaults_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/mcp.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/mcp_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/profile_modal.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/profile_modal_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/shared_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/skills.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/skills_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/common_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/execution.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/execution_tests.rs`
- `crates/vw-desktop/src/app/config_gateway_tests.rs`
- `crates/vw-desktop/src/app/message/chat/session.rs`
- `crates/vw-desktop/src/app/message/chat/session_tests.rs`
- `crates/vw-desktop/src/app/message/settings/agents.rs`
- `crates/vw-desktop/src/app/message/settings/agents_profile.rs`
- `crates/vw-desktop/src/app/message/settings/agents_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/execution.rs`
- `crates/vw-desktop/src/app/message/settings/channels/persist_tests.rs`
- `crates/vw-desktop/src/app/message/settings/messages.rs`
- `crates/vw-desktop/src/app/state/settings/agents.rs`
- `crates/vw-desktop/src/app/state/settings/agents_tests.rs`
- `crates/vw-desktop/src/app/state/settings/channels.rs`
- `crates/vw-desktop/src/app/task/agent_activity.rs`
- `crates/vw-desktop/src/app/views/task_board/panel/mod.rs`

### config-worktree（任务21）

来源：worktree.rs；目标：crates/libs/config-worktree。职责：工作树设置。外依赖：serde、schemars。原跨域引用：paths。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/gateway/api/handlers/worktree_management.rs`
- `crates/vw-agent/src/worktree/cleanup.rs`
- `crates/vw-agent/src/worktree/cleanup_flow_tests.rs`
- `crates/vw-agent/src/worktree/cleanup_tests.rs`
- `crates/vw-agent/src/worktree/creation_policy.rs`
- `crates/vw-agent/src/worktree/creation_policy_tests.rs`
- `crates/vw-agent/src/worktree/management.rs`
- `crates/vw-agent/src/worktree/management_e2e_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_worktrees/recovery_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_worktrees_tests.rs`
- `crates/vw-desktop/src/app/message/settings/worktrees.rs`
- `crates/vw-desktop/src/app/state/settings/worktrees.rs`
- `crates/vw-desktop/tests/worktree_management_window.rs`

### config-security（任务23）

来源：security.rs；目标：crates/libs/config-security。职责：安全配置迁库。外依赖：serde、schemars。原跨域引用：paths。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/config/schema/security.rs`
- `crates/vw-agent/src/security/policy/mod.rs`
- `crates/vw-agent/src/security/policy/types.rs`
- `crates/vw-agent/src/security/tests/policy/config.rs`
- `crates/vw-agent/src/tools/shell/permissions/rules.rs`
- `crates/vw-agent/src/tools/shell/security/mod.rs`
- `crates/vw-agent/src/tools/shell/security/mod_tests.rs`
- `crates/vw-agent/src/tools/shell/security/validators/mod.rs`
- `crates/vw-agent/src/tools/shell/security/validators/tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_autonomy.rs`
- `crates/vw-desktop/src/app/components/system_settings_autonomy_tests.rs`
- `crates/vw-desktop/src/app/impls/app_init/new.rs`
- `crates/vw-desktop/src/app/impls/app_init/new/settings.rs`
- `crates/vw-desktop/src/app/message/settings/autonomy.rs`
- `crates/vw-desktop/src/app/message/settings/autonomy_tests.rs`
- `crates/vw-desktop/src/app/message/settings/messages.rs`
- `crates/vw-desktop/src/app/message/settings/security.rs`
- `crates/vw-desktop/src/app/message/settings/security_tests.rs`
- `crates/vw-desktop/src/app/state/settings/infrastructure.rs`
- `crates/vw-desktop/src/app/state/settings/infrastructure_tests.rs`

### config-channel-types（任务24）

来源：channels/types.rs；目标：crates/libs/config-channel-types。职责：渠道协议配置。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/channels/clawdtalk/mod.rs`
- `crates/vw-agent/src/channels/manager/channels_config_tests.rs`
- `crates/vw-agent/src/channels/manager/execution_binding.rs`
- `crates/vw-agent/src/channels/scope.rs`
- `crates/vw-agent/src/channels/wechat/mod.rs`
- `crates/vw-agent/src/config/schema/channels/chat.rs`
- `crates/vw-agent/src/config/schema/channels/chat_tests.rs`
- `crates/vw-agent/src/config/schema/channels/email.rs`
- `crates/vw-agent/src/config/schema/channels/lark.rs`
- `crates/vw-agent/src/config/schema/channels/other.rs`
- `crates/vw-agent/src/config/schema/channels/types.rs`
- `crates/vw-agent/src/cron/tests/scheduler.rs`
- `crates/vw-agent/src/daemon/tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/channel_qr/wechat.rs`
- `crates/vw-agent/src/gateway/api/handlers/channel_qr_tests.rs`
- `crates/vw-agent/src/gateway/api/secrets_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/basic_channels_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/common.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/common_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/enterprise_channels_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/extended_channels_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels_ui_refresh_17_tests.rs`
- `crates/vw-desktop/src/app/impls/app_init/settings_builders.rs`
- `crates/vw-desktop/src/app/impls/app_init/settings_builders_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/changes_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/execution_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/helpers.rs`
- `crates/vw-desktop/src/app/message/settings/channels/helpers_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/persist.rs`
- `crates/vw-desktop/src/app/message/settings/channels/persist_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/projects_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/toggles.rs`
- `crates/vw-desktop/src/app/message/settings/channels/toggles_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/updates.rs`
- `crates/vw-desktop/src/app/message/settings/channels/updates_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/wecom_delivery.rs`
- `crates/vw-desktop/src/app/message/settings/cron_tests.rs`
- `crates/vw-desktop/src/app/state/settings/channels.rs`
- `crates/vw-desktop/src/app/state/settings/channels_tests.rs`

### config-channel-services（任务25）

来源：channels/clawdtalk.rs、channels/email.rs、channels/lark.rs、channels/other.rs、channels/wechat.rs、channels/wecom.rs；目标：crates/libs/config-channel-services。职责：渠道服务配置。外依赖：serde、schemars。原跨域引用：无。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/channels/clawdtalk/mod.rs`
- `crates/vw-agent/src/channels/manager/channels_config_tests.rs`
- `crates/vw-agent/src/channels/manager/execution_binding.rs`
- `crates/vw-agent/src/channels/scope.rs`
- `crates/vw-agent/src/channels/wechat/mod.rs`
- `crates/vw-agent/src/config/schema/channels/chat.rs`
- `crates/vw-agent/src/config/schema/channels/chat_tests.rs`
- `crates/vw-agent/src/config/schema/channels/email.rs`
- `crates/vw-agent/src/config/schema/channels/lark.rs`
- `crates/vw-agent/src/config/schema/channels/other.rs`
- `crates/vw-agent/src/config/schema/channels/types.rs`
- `crates/vw-agent/src/cron/tests/scheduler.rs`
- `crates/vw-agent/src/daemon/tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/channel_qr/wechat.rs`
- `crates/vw-agent/src/gateway/api/handlers/channel_qr_tests.rs`
- `crates/vw-agent/src/gateway/api/secrets_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/basic_channels_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/common.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/common_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/enterprise_channels_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/extended_channels_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels_ui_refresh_17_tests.rs`
- `crates/vw-desktop/src/app/impls/app_init/settings_builders.rs`
- `crates/vw-desktop/src/app/impls/app_init/settings_builders_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/changes_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/execution_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/helpers.rs`
- `crates/vw-desktop/src/app/message/settings/channels/helpers_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/persist.rs`
- `crates/vw-desktop/src/app/message/settings/channels/persist_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/projects_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/toggles.rs`
- `crates/vw-desktop/src/app/message/settings/channels/toggles_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/updates.rs`
- `crates/vw-desktop/src/app/message/settings/channels/updates_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/wecom_delivery.rs`
- `crates/vw-desktop/src/app/message/settings/cron_tests.rs`
- `crates/vw-desktop/src/app/state/settings/channels.rs`
- `crates/vw-desktop/src/app/state/settings/channels_tests.rs`

### config-channels（任务26）

来源：channels/chat.rs、channels/mod.rs；目标：crates/libs/config-channels。职责：渠道实例组合。外依赖：serde、schemars。原跨域引用：agent。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/channels/clawdtalk/mod.rs`
- `crates/vw-agent/src/channels/manager/channels_config_tests.rs`
- `crates/vw-agent/src/channels/manager/execution_binding.rs`
- `crates/vw-agent/src/channels/scope.rs`
- `crates/vw-agent/src/channels/wechat/mod.rs`
- `crates/vw-agent/src/config/schema/channels/chat.rs`
- `crates/vw-agent/src/config/schema/channels/chat_tests.rs`
- `crates/vw-agent/src/config/schema/channels/email.rs`
- `crates/vw-agent/src/config/schema/channels/lark.rs`
- `crates/vw-agent/src/config/schema/channels/other.rs`
- `crates/vw-agent/src/config/schema/channels/types.rs`
- `crates/vw-agent/src/cron/tests/scheduler.rs`
- `crates/vw-agent/src/daemon/tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/channel_qr/wechat.rs`
- `crates/vw-agent/src/gateway/api/handlers/channel_qr_tests.rs`
- `crates/vw-agent/src/gateway/api/secrets_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/basic_channels_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/common.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/common_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/enterprise_channels_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/extended_channels_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels_ui_refresh_17_tests.rs`
- `crates/vw-desktop/src/app/impls/app_init/settings_builders.rs`
- `crates/vw-desktop/src/app/impls/app_init/settings_builders_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/changes_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/execution_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/helpers.rs`
- `crates/vw-desktop/src/app/message/settings/channels/helpers_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/persist.rs`
- `crates/vw-desktop/src/app/message/settings/channels/persist_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/projects_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/toggles.rs`
- `crates/vw-desktop/src/app/message/settings/channels/toggles_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/updates.rs`
- `crates/vw-desktop/src/app/message/settings/channels/updates_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/wecom_delivery.rs`
- `crates/vw-desktop/src/app/message/settings/cron_tests.rs`
- `crates/vw-desktop/src/app/state/settings/channels.rs`
- `crates/vw-desktop/src/app/state/settings/channels_tests.rs`

### config-ui（任务28）

来源：ui.rs；目标：crates/libs/config-ui。职责：UI配置迁库。外依赖：serde、schemars、chrono_tz。原跨域引用：gateway、voice。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/config/schema/mod.rs`
- `crates/vw-desktop/src/acp_options_window_main.rs`
- `crates/vw-desktop/src/app/components/about_modal_ui_refresh_41_tests.rs`
- `crates/vw-desktop/src/app/components/account_status_ui_refresh_31_tests.rs`
- `crates/vw-desktop/src/app/components/app_providers/agy_ui_refresh_14_tests.rs`
- `crates/vw-desktop/src/app/components/app_providers/claude_ui_refresh_13_tests.rs`
- `crates/vw-desktop/src/app/components/app_providers/desktop_ui_refresh_12_tests.rs`
- `crates/vw-desktop/src/app/components/app_providers/list.rs`
- `crates/vw-desktop/src/app/components/archived_chats_window/native.rs`
- `crates/vw-desktop/src/app/components/chat_panel/latest_user_question_button_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/message_view/message_view_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/message_view/render_cache.rs`
- `crates/vw-desktop/src/app/components/chat_panel/message_view/render_cache_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/message_view/text_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/message_view/tool_summaries.rs`
- `crates/vw-desktop/src/app/components/chat_panel/mod_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/permission_view_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tool_selector_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/advanced_view_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/apply_patch_view_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/bash_view_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/config_view_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/explore_summary_view_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/files_view/common_view_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/files_view/list_ui_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/files_view/write_view_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/lsp_view_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/mcp_view_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/skill_view_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/tests/config_view.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/text_view_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/tool_meta.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/tool_permission_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/web_view_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/tools/workflow_view_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/trajectory/details.rs`
- `crates/vw-desktop/src/app/components/chat_panel/trajectory/details_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/trajectory/json_tree_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/trajectory/ledger_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/trajectory/timeline_tests.rs`
- `crates/vw-desktop/src/app/components/chat_panel/utils/ui_tests.rs`
- `crates/vw-desktop/src/app/components/editor.rs`
- `crates/vw-desktop/src/app/components/editor_settings_fields_tests.rs`
- `crates/vw-desktop/src/app/components/editor_tests.rs`
- `crates/vw-desktop/src/app/components/git_panel/diff_view/context_menu.rs`
- `crates/vw-desktop/src/app/components/git_panel/diff_view/context_menu_tests.rs`
- `crates/vw-desktop/src/app/components/git_panel/diff_view/file_view/gaps.rs`
- `crates/vw-desktop/src/app/components/git_panel/diff_view/file_view/mod.rs`
- `crates/vw-desktop/src/app/components/git_panel/diff_view/file_view/mod_tests.rs`
- `crates/vw-desktop/src/app/components/git_panel/diff_view/file_view/tests.rs`
- `crates/vw-desktop/src/app/components/git_panel/left_panel_tests.rs`
- `crates/vw-desktop/src/app/components/input_panel/acp_options_ui_refresh_42_tests.rs`
- `crates/vw-desktop/src/app/components/input_panel/agent_sessions_tests.rs`
- `crates/vw-desktop/src/app/components/input_panel/attachment_tests.rs`
- `crates/vw-desktop/src/app/components/input_panel/file_references_tests.rs`
- `crates/vw-desktop/src/app/components/input_panel/input_editor_tests.rs`
- `crates/vw-desktop/src/app/components/input_panel/queue_panel_tests.rs`
- `crates/vw-desktop/src/app/components/input_panel/quick_messages.rs`
- `crates/vw-desktop/src/app/components/input_panel/quick_messages_tests.rs`
- `crates/vw-desktop/src/app/components/input_panel/send_controls_tests.rs`
- `crates/vw-desktop/src/app/components/input_panel/tests.rs`
- `crates/vw-desktop/src/app/components/install_cli_modal.rs`
- `crates/vw-desktop/src/app/components/markdown_editor_tests.rs`
- `crates/vw-desktop/src/app/components/preview_panel/tabs_tests.rs`
- `crates/vw-desktop/src/app/components/preview_panel/view/editor_actions_tests.rs`
- `crates/vw-desktop/src/app/components/preview_panel/view/editor_options_tests.rs`
- `crates/vw-desktop/src/app/components/preview_panel/view/editor_popovers_tests.rs`
- `crates/vw-desktop/src/app/components/preview_panel/view/editor_search_tests.rs`
- `crates/vw-desktop/src/app/components/preview_panel/view/editor_tools_acceptance_tests.rs`
- `crates/vw-desktop/src/app/components/preview_panel/view/settings_ui_refresh_40_tests.rs`
- `crates/vw-desktop/src/app/components/startup_transition_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_acp_diagnostic.rs`
- `crates/vw-desktop/src/app/components/system_settings_acp_ui_refresh_18_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/basic_profile_ui_refresh_15_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/basic_ui_refresh_16_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/defaults.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/identity.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/identity_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/mod.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/skills.rs`
- `crates/vw-desktop/src/app/components/system_settings_apps/catalog_ui_refresh_06_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_apps/launch_ui_refresh_07_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_apps/preferences_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_archived_chats.rs`
- `crates/vw-desktop/src/app/components/system_settings_archived_chats_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_autonomy.rs`
- `crates/vw-desktop/src/app/components/system_settings_autonomy_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_autonomy_ui_refresh_22_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_browser_ui_refresh_26_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/common.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/execution_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels/wecom_delivery_modal_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_channels_ui_refresh_17_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_common/help.rs`
- `crates/vw-desktop/src/app/components/system_settings_common/help_ui_refresh_03_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_common/tab_appearance_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_companion/size_modal_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_companion_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_cron_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_cron_ui_refresh_27_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_dialogue_flow_ui_refresh_29_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_editor_ui_refresh_05_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_embedding_routes_ui_refresh_11_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_gateway.rs`
- `crates/vw-desktop/src/app/components/system_settings_gateway_client_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_gateway_client_ui_refresh_24_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_gateway_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_gateway_ui_refresh_23_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_general.rs`
- `crates/vw-desktop/src/app/components/system_settings_general_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_help.rs`
- `crates/vw-desktop/src/app/components/system_settings_hooks.rs`
- `crates/vw-desktop/src/app/components/system_settings_hooks_ui_refresh_28_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_http_request_ui_refresh_25_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_mcp.rs`
- `crates/vw-desktop/src/app/components/system_settings_mcp_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_memory.rs`
- `crates/vw-desktop/src/app/components/system_settings_memory_ui_refresh_20_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_models_ui_refresh_10_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_projects_ui_refresh_30_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_providers/catalog.rs`
- `crates/vw-desktop/src/app/components/system_settings_providers/catalog_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_providers/custom_model_modal_ui_refresh_09_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_providers_ui_refresh_08_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_proxy_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_quick_messages.rs`
- `crates/vw-desktop/src/app/components/system_settings_quick_messages_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_reliability_ui_refresh_21_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_runtime_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_search.rs`
- `crates/vw-desktop/src/app/components/system_settings_search_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_security.rs`
- `crates/vw-desktop/src/app/components/system_settings_security_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_skills/browser.rs`
- `crates/vw-desktop/src/app/components/system_settings_skills/browser_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_skills/catalog.rs`
- `crates/vw-desktop/src/app/components/system_settings_skills/catalog_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_skills/help.rs`
- `crates/vw-desktop/src/app/components/system_settings_skills/help_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_skills/plugins.rs`
- `crates/vw-desktop/src/app/components/system_settings_skills/plugins_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_skills_ui_refresh_19_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_themes/editor.rs`
- `crates/vw-desktop/src/app/components/system_settings_themes/gallery.rs`
- `crates/vw-desktop/src/app/components/system_settings_themes/preview.rs`
- `crates/vw-desktop/src/app/components/system_settings_ui_refresh_04_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_worktrees_tests.rs`
- `crates/vw-desktop/src/app/components/task_pet_tests.rs`
- `crates/vw-desktop/src/app/components/text_editor_context_menu_tests.rs`
- `crates/vw-desktop/src/app/components/toast_ui_refresh_33_tests.rs`
- `crates/vw-desktop/src/app/components/top_bar/controls_tests.rs`
- `crates/vw-desktop/src/app/components/top_bar/gateway_tests.rs`
- `crates/vw-desktop/src/app/config_cli_bootstrap.rs`
- `crates/vw-desktop/src/app/config_cron_jobs_tests.rs`
- `crates/vw-desktop/src/app/config_desktop_tests.rs`
- `crates/vw-desktop/src/app/config_gateway_tests.rs`
- `crates/vw-desktop/src/app/config_system_settings.rs`
- `crates/vw-desktop/src/app/config_system_settings_tests.rs`
- `crates/vw-desktop/src/app/desktop_lifecycle.rs`
- `crates/vw-desktop/src/app/desktop_lifecycle/native.rs`
- `crates/vw-desktop/src/app/desktop_notification.rs`
- `crates/vw-desktop/src/app/i18n/apps_tests.rs`
- `crates/vw-desktop/src/app/i18n/catalog.rs`
- `crates/vw-desktop/src/app/i18n/chat_tests.rs`
- `crates/vw-desktop/src/app/i18n/editor.rs`
- `crates/vw-desktop/src/app/i18n/gettext_area_10_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_area_11_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_area_15_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_area_16_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_area_17_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_area_19_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_area_4_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_area_5_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_area_8_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_area_9_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_catalog_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_selection_tests.rs`
- `crates/vw-desktop/src/app/i18n/gettext_sprintf_tests.rs`
- `crates/vw-desktop/src/app/i18n/input.rs`
- `crates/vw-desktop/src/app/i18n/modals_tests.rs`
- `crates/vw-desktop/src/app/i18n/project.rs`
- `crates/vw-desktop/src/app/i18n/project_tests.rs`
- `crates/vw-desktop/src/app/i18n/settings_extensions.rs`
- `crates/vw-desktop/src/app/i18n/settings_extensions_tests.rs`
- `crates/vw-desktop/src/app/i18n/settings_models.rs`
- `crates/vw-desktop/src/app/i18n/settings_models/connection_text/mod.rs`
- `crates/vw-desktop/src/app/i18n/settings_models/help.rs`
- `crates/vw-desktop/src/app/i18n/settings_models/message_catalog.rs`
- `crates/vw-desktop/src/app/i18n/settings_models/messages.rs`
- `crates/vw-desktop/src/app/i18n/settings_models_tests.rs`
- `crates/vw-desktop/src/app/i18n/settings_runtime.rs`
- `crates/vw-desktop/src/app/i18n/settings_runtime_tests.rs`
- `crates/vw-desktop/src/app/i18n/settings_security.rs`
- `crates/vw-desktop/src/app/i18n/tasks.rs`
- `crates/vw-desktop/src/app/i18n/tasks_tests.rs`
- `crates/vw-desktop/src/app/i18n/tests.rs`
- `crates/vw-desktop/src/app/i18n/workflow_tests.rs`
- `crates/vw-desktop/src/app/impls/agent_stream.rs`
- `crates/vw-desktop/src/app/impls/app_basic.rs`
- `crates/vw-desktop/src/app/impls/app_init/new.rs`
- `crates/vw-desktop/src/app/impls/app_init/new/load.rs`
- `crates/vw-desktop/src/app/impls/app_init/new/state.rs`
- `crates/vw-desktop/src/app/impls/app_update.rs`
- `crates/vw-desktop/src/app/impls/app_update_tests.rs`
- `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs`
- `crates/vw-desktop/src/app/impls/app_view_modals/permission_modal_tests.rs`
- `crates/vw-desktop/src/app/installer_update.rs`
- `crates/vw-desktop/src/app/message/chat/input.rs`
- `crates/vw-desktop/src/app/message/chat/input/slash_palette.rs`
- `crates/vw-desktop/src/app/message/chat/input/tool_detail_tests.rs`
- `crates/vw-desktop/src/app/message/chat/input_tests.rs`
- `crates/vw-desktop/src/app/message/chat/mod.rs`
- `crates/vw-desktop/src/app/message/git/diff.rs`
- `crates/vw-desktop/src/app/message/project/file_tree_batch_tests.rs`
- `crates/vw-desktop/src/app/message/project/file_tree_external_drop_tests.rs`
- `crates/vw-desktop/src/app/message/project/file_tree_move_drop_http_tests.rs`
- `crates/vw-desktop/src/app/message/project/session/lifecycle_tests.rs`
- `crates/vw-desktop/src/app/message/settings/agents_tests.rs`
- `crates/vw-desktop/src/app/message/settings/aggregation_desktop_tests.rs`
- `crates/vw-desktop/src/app/message/settings/archived_chats_tests.rs`
- `crates/vw-desktop/src/app/message/settings/channels/updates_tests.rs`
- `crates/vw-desktop/src/app/message/settings/cron.rs`
- `crates/vw-desktop/src/app/message/settings/cron_tests.rs`
- `crates/vw-desktop/src/app/message/settings/dialogue_flow.rs`
- `crates/vw-desktop/src/app/message/settings/dialogue_flow_tests.rs`
- `crates/vw-desktop/src/app/message/settings/gateway_client.rs`
- `crates/vw-desktop/src/app/message/settings/gateway_client_tests.rs`
- `crates/vw-desktop/src/app/message/settings/general.rs`
- `crates/vw-desktop/src/app/message/settings/general_tests.rs`
- `crates/vw-desktop/src/app/message/settings/mcp.rs`
- `crates/vw-desktop/src/app/message/settings/messages.rs`
- `crates/vw-desktop/src/app/message/settings/model_routes.rs`
- `crates/vw-desktop/src/app/message/settings/query_classification.rs`
- `crates/vw-desktop/src/app/message/settings/quick_messages.rs`
- `crates/vw-desktop/src/app/message/settings/skills.rs`
- `crates/vw-desktop/src/app/message/settings/themes.rs`
- `crates/vw-desktop/src/app/message/skill_hub_tests.rs`
- `crates/vw-desktop/src/app/message/task_board/helpers/logs.rs`
- `crates/vw-desktop/src/app/message/task_board/update/execution_tests.rs`
- `crates/vw-desktop/src/app/message/terminal/focus_tests.rs`
- `crates/vw-desktop/src/app/mod.rs`
- `crates/vw-desktop/src/app/native_menu/actions.rs`
- `crates/vw-desktop/src/app/native_menu/actions_tests.rs`
- `crates/vw-desktop/src/app/preview.rs`
- `crates/vw-desktop/src/app/session/active_tests.rs`
- `crates/vw-desktop/src/app/session/tests.rs`
- `crates/vw-desktop/src/app/skill_hub/cache_sync.rs`
- `crates/vw-desktop/src/app/skill_hub/client.rs`
- `crates/vw-desktop/src/app/skill_hub/client_tests.rs`
- `crates/vw-desktop/src/app/skill_hub/installer_tests.rs`
- `crates/vw-desktop/src/app/state/app_state.rs`
- `crates/vw-desktop/src/app/state/settings/runtime.rs`
- `crates/vw-desktop/src/app/state/settings/runtime_tests.rs`
- `crates/vw-desktop/src/app/task/executor/runner_tests.rs`
- `crates/vw-desktop/src/app/task/models.rs`
- `crates/vw-desktop/src/app/terminal.rs`
- `crates/vw-desktop/src/app/ui_types.rs`
- `crates/vw-desktop/src/app/views/apps/modals_tests.rs`
- `crates/vw-desktop/src/app/views/apps/ui_tests.rs`
- `crates/vw-desktop/src/app/views/expert_hub/connectors_ui_refresh_39_tests.rs`
- `crates/vw-desktop/src/app/views/expert_hub/experts_tests.rs`
- `crates/vw-desktop/src/app/views/expert_hub/install_guide_tests.rs`
- `crates/vw-desktop/src/app/views/expert_hub/list_tests.rs`
- `crates/vw-desktop/src/app/views/expert_hub/public_skills_tests.rs`
- `crates/vw-desktop/src/app/views/home/recent_tests.rs`
- `crates/vw-desktop/src/app/views/knowledge_tests.rs`
- `crates/vw-desktop/src/app/views/project/components/projects_list_tests.rs`
- `crates/vw-desktop/src/app/views/project/components/session_row_tests.rs`
- `crates/vw-desktop/src/app/views/project/components/sessions_panel_tests.rs`
- `crates/vw-desktop/src/app/views/project/layout/work_sidebar_tests.rs`
- `crates/vw-desktop/src/app/views/task_board/common_ui_refresh_34_tests.rs`
- `crates/vw-desktop/src/app/views/task_board/control/worktree_office/floor_tests.rs`
- `crates/vw-desktop/src/app/views/task_board/drag_tests.rs`
- `crates/vw-desktop/src/app/views/task_board/modals_tests.rs`
- `crates/vw-desktop/src/app/views/task_board/panel/executor_selector_ui_refresh_35_tests.rs`
- `crates/vw-desktop/src/app/views/task_board/panel/logs_modal_tests.rs`
- `crates/vw-desktop/src/app/views/task_board/panel/reasoning_selector.rs`
- `crates/vw-desktop/src/app/views/task_board/panel/reasoning_selector_tests.rs`
- `crates/vw-desktop/src/app/views/task_board/panel/tests.rs`
- `crates/vw-desktop/src/app/views/usage/session_menu_tests.rs`
- `crates/vw-desktop/src/app/views/usage/steps_tests.rs`
- `crates/vw-desktop/src/app/views/usage/utils_tests.rs`
- `crates/vw-desktop/src/app/views/workflow_manager/form_controls_tests.rs`
- `crates/vw-desktop/src/apps/workflow/dify_node_editor_round_trip_tests.rs`
- `crates/vw-desktop/src/apps/workflow/view/app_editor_ui_refresh_38_tests.rs`
- `crates/vw-desktop/src/apps/workflow/view/data_nodes_ui_refresh_37_tests.rs`
- `crates/vw-desktop/src/apps/workflow/view/mod_ui_refresh_36_tests.rs`
- `crates/vw-desktop/src/desktop_update_helper.rs`
- `crates/vw-desktop/src/voice_window_main.rs`

### config-aggregate（任务29）

来源：config.rs；目标：crates/libs/config-aggregate。职责：配置组合根。外依赖：serde、schemars、serde_json、directories、tracing。原跨域引用：agent、automation、channels、computer_control、gateway、hooks、memory、observability、paths、plugins、provider、proxy、reliability、routing、runtime、security、skills、tools、ui、worktree。共享契约为原公开类型身份及 serde/schemars 默认语义，不新增协议。

实际显式模块路径调用文件：
- `crates/vw-agent/src/config/migration.rs`
- `crates/vw-agent/src/config/migration_tests.rs`
- `crates/vw-agent/src/config/schema/config.rs`
- `crates/vw-agent/src/config/schema/config_env.rs`
- `crates/vw-agent/src/config/schema/config_io.rs`
- `crates/vw-agent/src/config/schema/config_load.rs`
- `crates/vw-agent/src/config/schema/config_save.rs`
- `crates/vw-agent/src/config/schema/config_validate.rs`
- `crates/vw-cli/src/cli/tui/runtime/gateway.rs`
- `crates/vw-desktop/src/app/components/system_settings_acp.rs`
- `crates/vw-desktop/src/app/components/system_settings_acp_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_agents/catalog_tests.rs`
- `crates/vw-desktop/src/app/config_gateway_tests.rs`
- `crates/vw-desktop/src/app/impls/app_init.rs`
- `crates/vw-desktop/src/app/impls/app_init/new.rs`
- `crates/vw-desktop/src/app/impls/app_init/new/load.rs`
- `crates/vw-desktop/src/app/impls/app_init/new/settings.rs`
- `crates/vw-desktop/src/app/impls/app_init/new/settings_tests.rs`
- `crates/vw-desktop/src/app/impls/app_init/new/state.rs`
- `crates/vw-desktop/src/app/impls/app_init_tests.rs`
- `crates/vw-desktop/src/app/impls/app_update_tests.rs`
- `crates/vw-desktop/src/app/message/settings/acp.rs`
- `crates/vw-desktop/src/app/message/settings/acp_tests.rs`
- `crates/vw-desktop/src/app/message/settings/agents.rs`
- `crates/vw-desktop/src/app/message/settings/agents_tests.rs`
- `crates/vw-desktop/src/app/message/settings/messages.rs`
- `crates/vw-desktop/src/app/message/settings/model_routes_tests.rs`
- `crates/vw-desktop/src/app/message/settings/models.rs`
- `crates/vw-desktop/src/app/message/settings/models_tests.rs`
- `crates/vw-desktop/src/app/message/settings/providers/helpers.rs`
- `crates/vw-desktop/src/app/message/settings/providers/helpers_tests.rs`
- `crates/vw-desktop/src/app/state/settings/agents.rs`
- `crates/vw-desktop/src/app/state/settings/infrastructure.rs`
- `crates/vw-desktop/src/app/ui_types.rs`
- `crates/vw-provider-resolver/src/config.rs`
- `crates/vw-provider-resolver/src/config_tests.rs`

## 图与未拆内容

生产模块边已在 plan.md 图中逐项覆盖，无强连通环。测试产生 voice→ui→voice、plugins→config→plugins，以及 agent_profile_tests→agent 的反向边；将组合断言上移，不能加入反向 dev-dependency。channels/chat 原引用 agent 模型身份，改为 agent-profile 契约即可减少依赖深度。

保留门面因现有发布模块路径；保留 Config 为跨领域配置组装，保留 SecurityConfig/ChannelsConfig/AppUiConfig 为各自领域组装。工具、UI、安全大文件先有聚焦分层卡；没有未分配业务源文件。未来行为/schema、性能与编译隔离收益均未验证。
