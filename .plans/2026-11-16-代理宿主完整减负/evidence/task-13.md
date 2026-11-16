# 任务13：静态交付证据

代码完成，编译/行为未验证。按用户要求未运行任何 Cargo、构建、单元或实机测试。

## 全文件映射

| 原文件 | 原行数 | 当前行数 | 实际所有者 / 保留职责 |

|---|---:|---:|---|

| `crates/vw-agent/src/gateway/agent.rs` | 40 | 40 | Host 实现 AgentHost；handle_agent 调用领域端点，run_agent_chat 选择宿主工具执行链；cfg(test) static 路径用于真实路由成功契约 |

| `crates/vw-agent/src/gateway/agent_tests.rs` | 451 | 451 | 保留真实宿主契约：Host 请求键、鉴权、聊天工厂/不支持配置错误、内存失败与观测事件接线；StaticProvider/FailingProvider/TrackingMemory/RecordingObserver 仅为本组实际端口夹具 |

| `crates/vw-agent/src/gateway/agent_apps/mod.rs` | 6 | 6 | 父模块注册与稳定命名空间，或 voice_router 的宿主租户中间件装配；无领域算法 |

| `crates/vw-agent/src/gateway/agent_apps/profile_operations.rs` | 2 | 0 | direct export: agent_gateway::agent_apps::profile_operations |

| `crates/vw-agent/src/gateway/agent_apps/profile_skills.rs` | 2 | 0 | direct export: agent_gateway::agent_apps::profile_skills |

| `crates/vw-agent/src/gateway/agent_apps/profile_store.rs` | 2 | 0 | direct export: agent_gateway::agent_apps::profile_store |

| `crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/mod.rs` | 4 | 4 | 父模块注册与稳定命名空间，或 voice_router 的宿主租户中间件装配；无领域算法 |

| `crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/model_check.rs` | 2 | 0 | direct export: agent_gateway::agent_apps_diagnostic_executors::model_check |

| `crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/reachability.rs` | 2 | 0 | direct export: agent_gateway::agent_apps_diagnostic_executors::reachability |

| `crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/speedtest.rs` | 2 | 0 | direct export: agent_gateway::agent_apps_diagnostic_executors::speedtest |

| `crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/stream_check.rs` | 2 | 0 | direct export: agent_gateway::agent_apps_diagnostic_executors::stream_check |

| `crates/vw-agent/src/gateway/agent_apps_diagnostic_jobs.rs` | 2 | 0 | direct export: agent_gateway::agent_apps_diagnostic_jobs |

| `crates/vw-agent/src/gateway/agent_apps_failover_runtime.rs` | 2 | 0 | direct export: app_routing_services::failover_runtime |

| `crates/vw-agent/src/gateway/agent_apps_failover_runtime_tests.rs` | 383 | 0 | crates/libs/app-routing-services/src/host_migration_13_tests/agent_apps_failover_runtime_tests.rs |

| `crates/vw-agent/src/gateway/agent_apps_failover_timeouts_tests.rs` | 397 | 0 | crates/libs/app-routing-services/src/host_migration_13_tests/agent_apps_failover_timeouts_tests.rs |

| `crates/vw-agent/src/gateway/agent_apps_high_availability_service.rs` | 2 | 0 | direct export: agent_gateway::agent_apps_high_availability_service |

| `crates/vw-agent/src/gateway/agent_apps_high_availability_service_tests.rs` | 549 | 0 | crates/libs/app-routing-services/src/host_migration_13_tests/agent_apps_high_availability_service_tests.rs |

| `crates/vw-agent/src/gateway/agent_apps_high_availability_store.rs` | 2 | 0 | direct export: app_routing_services::failover_store |

| `crates/vw-agent/src/gateway/agent_apps_skills_store.rs` | 2 | 0 | direct export: app_settings_store::agent_apps_skills_store |

| `crates/vw-agent/src/gateway/aggregation_desktop_record.rs` | 36 | 36 | PublicationHost::revision 读取真实宿主快照；native/publish_aggregation 连接实际文件发布上下文 |

| `crates/vw-agent/src/gateway/aggregation_lifecycle.rs` | 31 | 31 | AggregationLifecycle 保存数据库/租户/宿主，service/resume/detach 实现真实生命周期端口 |

| `crates/vw-agent/src/gateway/aggregation_resync.rs` | 2 | 2 | 父模块注册与稳定命名空间，或 voice_router 的宿主租户中间件装配；无领域算法 |

| `crates/vw-agent/src/gateway/api/auth.rs` | 2 | 0 | direct export: agent_gateway::api::auth |

| `crates/vw-agent/src/gateway/api/integrations.rs` | 2 | 0 | direct export: agent_gateway::api::integrations |

| `crates/vw-agent/src/gateway/api/mod.rs` | 115 | 115 | 父模块注册与稳定命名空间，或 voice_router 的宿主租户中间件装配；无领域算法 |

| `crates/vw-agent/src/gateway/api/secrets.rs` | 2 | 0 | direct export: agent_gateway::api::secrets |

| `crates/vw-agent/src/gateway/api/types.rs` | 2 | 0 | direct export: agent_gateway::api::types |

| `crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_files.rs` | 2 | 0 | direct export: gateway_agent_app::agent_app_prompt_files |

| `crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_shared_import.rs` | 2 | 0 | direct export: gateway_agent_app::agent_app_prompt_shared_import |

| `crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_sync.rs` | 2 | 0 | direct export: gateway_agent_app::agent_app_prompt_sync |

| `crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_templates.rs` | 2 | 0 | direct export: gateway_agent_app::agent_app_prompt_templates |

| `crates/vw-agent/src/gateway/api/handlers/agent_app_prompts_activation.rs` | 2 | 0 | direct export: gateway_agent_app::agent_app_prompts_activation |

| `crates/vw-agent/src/gateway/api/handlers/agent_app_prompts_read.rs` | 2 | 0 | direct export: gateway_agent_app::agent_app_prompts_read |

| `crates/vw-agent/src/gateway/api/handlers/agent_app_prompts_write.rs` | 2 | 0 | direct export: gateway_agent_app::agent_app_prompts_write |

| `crates/vw-agent/src/gateway/api/handlers/agent_apps_add_tests.rs` | 101 | 0 | crates/libs/gateway-agent-apps/src/host_migration_13_tests.rs |

| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_cancel_tests.rs` | 247 | 0 | crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_cancel_tests.rs |

| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_poll_tests.rs` | 222 | 0 | crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_poll_tests.rs |

| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_redaction_tests.rs` | 322 | 0 | crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_redaction_tests.rs |

## 全用例映射（原名及断言保持）

### `crates/vw-agent/src/gateway/agent_tests.rs` → `crates/vw-agent/src/gateway/agent_tests.rs`

- `handle_agent_rejects_unauthorized_requests_before_body_validation`

- `handle_agent_accepts_valid_bearer_before_body_validation`

- `handle_agent_rejects_invalid_json_body`

- `handle_agent_rejects_empty_message`

- `handle_agent_enforces_agent_rate_limit_before_auth`

- `handle_agent_uses_trusted_forwarded_client_for_rate_limit`

- `handle_agent_returns_bad_gateway_when_agent_engine_fails`

- `handle_agent_auto_saves_trimmed_message_and_ignores_store_failure`

- `handle_agent_records_failure_events_with_default_labels`

- `handle_agent_sanitizes_secrets_in_provider_errors`

- `handle_agent_returns_response_and_records_complete_success_lifecycle`

以上为真实宿主边界断言：直接调用 Host 路由及真实 AppState，不以伪造 Host 移动执行链错误场景。未来可进一步拆出纯 HTTP 策略断言，须保留真实宿主接线覆盖。

### `crates/vw-agent/src/gateway/agent_apps_failover_runtime_tests.rs` → `crates/libs/app-routing-services/src/host_migration_13_tests/agent_apps_failover_runtime_tests.rs`

- `ha_runtime_uses_single_retry_owner`

- `ha_switch_commits_once_without_client_config_write`

- `ha_switch_old_version_cannot_roll_back_new_route`

### `crates/vw-agent/src/gateway/agent_apps_failover_timeouts_tests.rs` → `crates/libs/app-routing-services/src/host_migration_13_tests/agent_apps_failover_timeouts_tests.rs`

- `ha_first_byte_timeout_stops_silent_upstream`

- `ha_idle_and_total_deadlines_bound_stream`

- `ha_idle_and_total_deadlines_bound_stream_retry_budget`

### `crates/vw-agent/src/gateway/agent_apps_high_availability_service_tests.rs` → `crates/libs/app-routing-services/src/host_migration_13_tests/agent_apps_high_availability_service_tests.rs`

- `ha_enable_requires_routing_mode_and_keeps_queue`

- `ha_enable_requires_routing_mode_and_keeps_queue_sqlite_preimages`

- `ha_enable_requires_routing_mode_and_keeps_queue_postgres_preimages`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps_add_tests.rs` → `crates/libs/gateway-agent-apps/src/host_migration_13_tests.rs`

- `cc_base_icon_api_add_sqlite_read_and_deduplicate`

- `cc_base_icon_api_add_postgres_read_and_deduplicate`

- `cc_base_icon_api_add_invalid_input_does_not_write`

- `cc_base_icon_api_add_body_limit`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_cancel_tests.rs` → `crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_cancel_tests.rs`

- `auth6_api_cancel_commits_expected_scoped_result`

- `auth6_api_cancel_commits_expected_scoped_result_postgres`

- `auth6_api_cancel_rejects_invalid_or_stale_input`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_poll_tests.rs` → `crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_poll_tests.rs`

- `auth6_api_poll_commits_expected_scoped_result`

- `auth6_api_poll_commits_expected_scoped_result_postgres`

- `auth6_api_poll_rejects_invalid_or_stale_input`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_redaction_tests.rs` → `crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_redaction_tests.rs`

- `auth6_redaction_commits_expected_scoped_result`

- `auth6_redaction_rejects_invalid_or_stale_input`

- `auth6_redaction_commits_expected_scoped_result_postgres`

- `auth6_redaction_rejects_invalid_or_stale_input_postgres`

## 行数与生产边界

原范围 2952 行，当前 685 行，减少 2267 行（76.8%）。本轮没有新增生产算法下沉收益：生产入口早已归属 agent-gateway/app-routing-services/gateway-agent-apps/gateway-agent-app，删除的两行文件仅是兼容转发。

测试独立文件：原 2672 行；宿主保留451行；接收测试保持整个函数迁移。生产/注册/转发及宿主端口粗分：原 280 行，当前 234 行。agent.rs 中含 cfg(test) 执行适配约11行，不能把它统计为生产下沉。

高可用三组由 app-routing-services 自注册 host_migration_13_tests；直接调用 high_availability_service/failover_runtime/failover_store。认证三组复用 gateway-agent-apps 已有 host_migration_3_tests 的真实 HTTP Gateway/Runtime/授权策略和数据库夹具；图标组自注册 host_migration_13_tests，调用 icons::router。均不反向依赖 vw-agent，不 path/include 宿主源码。

超时配置从全局 home 改为当前 fixture 数据库；四处 RoutingListener 使用 with_database 绑定同一真实存储。流协议样本复用 app-routing-services 既有 test_samples（四协议与 fixture-v1），不再跨宿主加载。新增 agent-config dev 依赖用于真实 SystemConfigStore。

任务1跨目录登记：本组直接夹具样本原跨 app-diagnostics-runtime 加载，迁移后使用已有领域样本；本组无遗留宿主外部 path 挂载。

## 未来验证（本轮全部未运行）

- `python3 scripts/cargo_slot.py cargo test -p app-routing-services --lib host_migration_13_tests -- --test-threads=1`；本次迁入预期 9 条非零用例。host_migration_3_tests 另含已有用例，应按上列完整尾名逐条过滤记录精确数量。

- `python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib host_migration_13_tests -- --test-threads=1`；本次迁入预期 4 条非零用例。host_migration_3_tests 另含已有用例，应按上列完整尾名逐条过滤记录精确数量。

- `python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib host_migration_3_tests -- --test-threads=1`；本次迁入预期 10 条非零用例。host_migration_3_tests 另含已有用例，应按上列完整尾名逐条过滤记录精确数量。

- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib gateway::agent::agent_tests -- --test-threads=1`：预期11条真实宿主用例。

- 未来同包 lib `-- --list` 核对上列迁出名称消失，不能以过滤后零测试冒充通过；macOS 验证全部高可用与认证模块。

静态核对：全38文件映射；七组迁出测试逐名、断言数量一致；旧模块声明删除；父模块直接导出保持原可见性和 cfg。未运行编译，因此不声称构建收益或行为通过。
