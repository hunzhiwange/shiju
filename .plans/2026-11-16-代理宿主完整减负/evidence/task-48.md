# 任务48：MCP 应用服务迁移

代码完成，编译/行为未验证。仅迁移完整领域连接检查组；其它真实宿主边界逐项保留。减量未达70%，交任务59复核，不能视为整轮减量达标。

## 全文件与用例映射

|原文件|原行数|宿主现行数|职责/去向|
|---|---:|---:|---|
|acp_mount.rs|28|28|resolve_mounts 注入全局数据库；AcpMountLeases/is_current 从当前工具上下文取租户和会话；Deref/revoked_names 保持宿主句柄 API|
|acp_mount_tests.rs|227|227|add_server 供真实 ACP 后端装配；用例覆盖宿主上下文、工具调用、越权和恢复|
|binding_test_support.rs|35|35|private_root/installed 供 runtime_tests 真实安装探测与宿主文件操作|
|catalog_test_support.rs|33|33|with_database/http_draft 仍被保留的宿主测试共享；领域用例复用领域既有夹具|
|connection_probe_tests.rs|153|153|实际 NativeProbeHost、策略文件授权、DiagnosticJobs 与取消归属|
|connectivity.rs|28|28|NativeConnectionCheckRuntime/NativeCheckPeer 连接真实 AgentMcpHost 与插件工具目录|
|connectivity_tests.rs|401|0|迁至 agent-app-services/src/mcp/host_migration_48_tests.rs，保留所有测试函数、服务器夹具和断言|
|host_scope_tests.rs|226|226|真实 HTTP 路由授权与本机多用户文件隔离，不能用模拟授权替代|
|mod.rs|50|48|必要领域 re-export 与真实宿主端口/集成测试注册；删除 connectivity_tests 原注册|
|network_probe.rs|100|100|NativeProbeHost/NativeProbeAccess/NativeProbeGrant 绑定真实网关授权、任务注册、工具 DTO 与外部传输生命周期；start_* 为宿主真实调用入口|
|network_probe_tests.rs|462|462|Fixture/Server/report 使用真实策略重读、DiagnosticJobs、真实 HTTP/SSE 和会话清理；同时被连接检查、stdio、details 用例共享|
|probe_jobs.rs|45|45|NativeBatchHost 把任务所有者、错误及批次注册接到单一宿主 registry|
|probe_jobs_tests.rs|501|501|真实 DiagnosticJobs 活动/排队取消、结果代数和 OS 进程回收|
|revocation.rs|12|12|runtime_selection_is_current 读取当前工具或 UI 租户和全局数据库|
|revocation_tests.rs|251|251|真实缓存工具撤销、已准入调用与宿主插件连接生命周期；保留外部 fixture http_server.py 的 include_bytes|
|runtime.rs|33|33|selected_runtime_servers 读取 ProjectStore 和真实项目工作区边界，再调用领域 runtime_servers|
|runtime_tests.rs|261|261|register_project 与真实项目上下文、ToolCatalog、ACP/原生工具运行时装配|
|security_tests.rs|445|445|FixtureLogs/send/private_dir 和真实 gateway router，验证跨租户账户/任务、日志脱敏与拒绝路径|
|stdio_probe_tests.rs|173|173|真实 NativeProbeHost 授权与真实任务结束前进程树退出|
|tool_details_tests.rs|163|163|真实 NativeProbeAccess provider_config_read 权限、撤销、取消和任务返回 DTO|

宿主 3627 → 3224 行，净减 403 行（11.11%）。原生产/接线文件 7 个共 296 行；本次生产算法迁出 0 行，删除测试注册 2 行。领域新增测试/端口夹具 437 行，生产增加 0 行。不能声称生产迁移或编译性能收益。

## 全用例

- `acp_mount_tests::cc_mcp_t29_explicit_catalog_selection_mounts_only_the_selected_server` → `agent_apps::mcp::acp_mount_tests::cc_mcp_t29_explicit_catalog_selection_mounts_only_the_selected_server`
- `connection_probe_tests::cc_mcp_t39_revision_results_reuse_connection_checks_and_preserve_diagnostics` → `agent_apps::mcp::connection_probe_tests::cc_mcp_t39_revision_results_reuse_connection_checks_and_preserve_diagnostics`
- `connection_probe_tests::cc_mcp_t39_revision_checks_reject_revoked_success_and_cancel_owned_work` → `agent_apps::mcp::connection_probe_tests::cc_mcp_t39_revision_checks_reject_revoked_success_and_cancel_owned_work`
- `connectivity_tests::cc_mcp_t30_stdio_discovers_tools_using_args_env_and_cwd` → `mcp::host_migration_48_tests::cc_mcp_t30_stdio_discovers_tools_using_args_env_and_cwd`
- `connectivity_tests::cc_mcp_t30_streamable_http_discovers_tools_with_authorized_header` → `mcp::host_migration_48_tests::cc_mcp_t30_streamable_http_discovers_tools_with_authorized_header`
- `connectivity_tests::cc_mcp_t30_legacy_sse_discovers_tools_with_authorized_header` → `mcp::host_migration_48_tests::cc_mcp_t30_legacy_sse_discovers_tools_with_authorized_header`
- `connectivity_tests::cc_mcp_t30_handshake_and_discovery_failures_return_distinct_codes` → `mcp::host_migration_48_tests::cc_mcp_t30_handshake_and_discovery_failures_return_distinct_codes`
- `connectivity_tests::cc_mcp_t30_stalled_tool_discovery_times_out` → `mcp::host_migration_48_tests::cc_mcp_t30_stalled_tool_discovery_times_out`
- `connectivity_tests::cc_mcp_t30_foreign_endpoint_and_redirect_never_receive_headers` → `mcp::host_migration_48_tests::cc_mcp_t30_foreign_endpoint_and_redirect_never_receive_headers`
- `host_scope_tests::cc_mcp_t52_unavailable_hosts_never_fall_back_and_local_users_remain_isolated` → `agent_apps::mcp::host_scope_tests::cc_mcp_t52_unavailable_hosts_never_fall_back_and_local_users_remain_isolated`
- `network_probe_tests::diagnostic_mcp_http_handshake_is_scoped` → `agent_apps::mcp::network_probe_tests::diagnostic_mcp_http_handshake_is_scoped`
- `network_probe_tests::diagnostic_mcp_http_handshake_is_scoped_cancels_active_connections_and_limits_batches` → `agent_apps::mcp::network_probe_tests::diagnostic_mcp_http_handshake_is_scoped_cancels_active_connections_and_limits_batches`
- `network_probe_tests::diagnostic_mcp_http_handshake_is_scoped_rejects_foreign_endpoints_and_revoked_results` → `agent_apps::mcp::network_probe_tests::diagnostic_mcp_http_handshake_is_scoped_rejects_foreign_endpoints_and_revoked_results`
- `probe_jobs_tests::cc_mcp_t31_in_flight_connection_edit_discards_old_success` → `agent_apps::mcp::probe_jobs_tests::cc_mcp_t31_in_flight_connection_edit_discards_old_success`
- `probe_jobs_tests::cc_mcp_t31_cancel_stops_only_owned_active_and_queued_probes` → `agent_apps::mcp::probe_jobs_tests::cc_mcp_t31_cancel_stops_only_owned_active_and_queued_probes`
- `probe_jobs_tests::cc_mcp_t31_only_current_task_generation_and_connection_revision_apply` → `agent_apps::mcp::probe_jobs_tests::cc_mcp_t31_only_current_task_generation_and_connection_revision_apply`
- `probe_jobs_tests::diagnostic_mcp_stdio_cancellation_reaps_process_tree` → `agent_apps::mcp::probe_jobs_tests::diagnostic_mcp_stdio_cancellation_reaps_process_tree`
- `probe_jobs_tests::cc_mcp_t31_timed_out_stdio_is_reaped_before_job_completion` → `agent_apps::mcp::probe_jobs_tests::cc_mcp_t31_timed_out_stdio_is_reaped_before_job_completion`
- `probe_jobs_tests::cc_mcp_t31_cancel_drops_a_stalled_http_initialization_request` → `agent_apps::mcp::probe_jobs_tests::cc_mcp_t31_cancel_drops_a_stalled_http_initialization_request`
- `revocation_tests::cc_mcp_t28_update_and_delete_revoke_cached_tools_without_interrupting_admitted_call` → `agent_apps::mcp::revocation_tests::cc_mcp_t28_update_and_delete_revoke_cached_tools_without_interrupting_admitted_call`
- `revocation_tests::cc_mcp_t28_disable_revokes_loopback_tools_until_a_fresh_selection` → `agent_apps::mcp::revocation_tests::cc_mcp_t28_disable_revokes_loopback_tools_until_a_fresh_selection`
- `runtime_tests::cc_mcp_t27_selected_record_exposes_and_calls_the_real_tool` → `agent_apps::mcp::runtime_tests::cc_mcp_t27_selected_record_exposes_and_calls_the_real_tool`
- `runtime_tests::cc_mcp_t27_runtime_selection_keeps_project_and_acp_boundaries` → `agent_apps::mcp::runtime_tests::cc_mcp_t27_runtime_selection_keeps_project_and_acp_boundaries`
- `security_tests::cc_mcp_t51_foreign_tenants_accounts_and_job_owners_cannot_cross_boundaries` → `agent_apps::mcp::security_tests::cc_mcp_t51_foreign_tenants_accounts_and_job_owners_cannot_cross_boundaries`
- `stdio_probe_tests::diagnostic_mcp_stdio_cancellation_reaps_process_tree_handshake` → `agent_apps::mcp::stdio_probe_tests::diagnostic_mcp_stdio_cancellation_reaps_process_tree_handshake`
- `tool_details_tests::cc_mcp_t50_details_discover_real_specs_and_read_only_the_authorized_selection` → `agent_apps::mcp::tool_details_tests::cc_mcp_t50_details_discover_real_specs_and_read_only_the_authorized_selection`
- `tool_details_tests::cc_mcp_t50_discovery_rechecks_revocation_and_cancel_releases_the_owned_job` → `agent_apps::mcp::tool_details_tests::cc_mcp_t50_discovery_rechecks_revocation_and_cancel_releases_the_owned_job`

## 接线与约束

领域 mcp/mod.rs 在 macOS test 下自注册 host_migration_48_tests；原宿主 connectivity_tests 文件和 mod 注册均删除。6 条测试使用真实 McpCatalogStore/AuthAccountStore/SQLite（或环境指定 PostgreSQL）、凭证文件及 mcp-client 传输。FixtureHost 只提供外部 ClientHost 端口：真实 managed_stdio_cwd、无会话 HTTP 扩展、明确拒绝交互；不复制授权/检查算法。FixtureRuntime 通过真实 catalog::list_tools 分页，原宿主工具目录也转发同一领域函数。

生产调用仍为 gateway → NativeProbeHost → agent-app-services::mcp::network_probe；probe_jobs → NativeBatchHost → 领域批次算法；ACP → acp_mount 宿主上下文 → 领域 resolve_mounts。依赖方向 agent-app-services → mcp-client/agent-database/vw-agent-apps，不反向依赖 vw-agent，无 path/include 宿主源码。任务1外部清单只有 revocation_tests 的 Python include_bytes，原地保留真实宿主撤销验收。

补齐 dev 依赖 futures-util、rmcp client、axum 服务特性及 tokio test-util。Cargo.lock 仅补本包直接依赖，未运行 Cargo。

不足70%的后续可迁项：网络协议服务器夹具可由领域测试复用，但真实鉴权/任务生命周期断言仍需保留；runtime 的项目范围判断可在 ProjectStore 领域端口稳定后迁移；ACP 选择策略可增加领域独立用例，不能删除现有真实装配断言。其余文件逐文件原因见表；未用 mock 冒充宿主鉴权验收。

## 静态证据和未来验证

静态比较确认迁移前后 6 个测试名称与 assert!/assert_eq! 调用文本一致；仅替换 import 与外部 runtime 端口。所有 Cargo、fmt、编译、单元/实机测试均按计划未运行，未取得测试list或通过数。

未来：`python3 scripts/cargo_slot.py cargo test -p agent-app-services --lib host_migration_48_tests -- --test-threads=1`，macOS 预期 6 条，必须取得 ran>0、failed=0。agent-mcp 无新增模块，不执行零测试命令。

宿主未来分别运行以下精确过滤，均预期1条：

- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::acp_mount_tests::cc_mcp_t29_explicit_catalog_selection_mounts_only_the_selected_server -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::connection_probe_tests::cc_mcp_t39_revision_results_reuse_connection_checks_and_preserve_diagnostics -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::connection_probe_tests::cc_mcp_t39_revision_checks_reject_revoked_success_and_cancel_owned_work -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::host_scope_tests::cc_mcp_t52_unavailable_hosts_never_fall_back_and_local_users_remain_isolated -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::network_probe_tests::diagnostic_mcp_http_handshake_is_scoped -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::network_probe_tests::diagnostic_mcp_http_handshake_is_scoped_cancels_active_connections_and_limits_batches -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::network_probe_tests::diagnostic_mcp_http_handshake_is_scoped_rejects_foreign_endpoints_and_revoked_results -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::probe_jobs_tests::cc_mcp_t31_in_flight_connection_edit_discards_old_success -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::probe_jobs_tests::cc_mcp_t31_cancel_stops_only_owned_active_and_queued_probes -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::probe_jobs_tests::cc_mcp_t31_only_current_task_generation_and_connection_revision_apply -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::probe_jobs_tests::diagnostic_mcp_stdio_cancellation_reaps_process_tree -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::probe_jobs_tests::cc_mcp_t31_timed_out_stdio_is_reaped_before_job_completion -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::probe_jobs_tests::cc_mcp_t31_cancel_drops_a_stalled_http_initialization_request -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::revocation_tests::cc_mcp_t28_update_and_delete_revoke_cached_tools_without_interrupting_admitted_call -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::revocation_tests::cc_mcp_t28_disable_revokes_loopback_tools_until_a_fresh_selection -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::runtime_tests::cc_mcp_t27_selected_record_exposes_and_calls_the_real_tool -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::runtime_tests::cc_mcp_t27_runtime_selection_keeps_project_and_acp_boundaries -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::security_tests::cc_mcp_t51_foreign_tenants_accounts_and_job_owners_cannot_cross_boundaries -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::stdio_probe_tests::diagnostic_mcp_stdio_cancellation_reaps_process_tree_handshake -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::tool_details_tests::cc_mcp_t50_details_discover_real_specs_and_read_only_the_authorized_selection -- --exact --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::mcp::tool_details_tests::cc_mcp_t50_discovery_rechecks_revocation_and_cancel_releases_the_owned_job -- --exact --test-threads=1`

未来宿主 `cargo test -p vw-agent --lib -- --list`（同样通过 cargo_slot.py）应无 agent_apps::mcp::connectivity_tests；仅证明迁出归属，不能以零测试宣称通过。
