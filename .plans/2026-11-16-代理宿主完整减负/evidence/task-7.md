# 任务7 静态交付证据

代码完成，编译/行为未验证。遵照公共计划，未运行任何 Cargo、单元测试、实机或构建。

实际所有者是 gateway-app-routing：源文件已转发至其 server；agent-gateway 是它的下层依赖，若将 HTTP 测试迁入 agent-gateway 将形成反向依赖，因此选择直接所有者。

## 全文件映射

| 原文件 | 前行数 | 后行数 | 去向或保留职责 |
|---|---:|---:|---|
| outbound_scan_tests.rs | 239 | 0 | gateway-app-routing/src/host_migration_7_tests/outbound_scan_tests.rs；全部断言保留 |
| outbound_scan.rs | 31 | 0 | 移除转发；server/router.rs 直接引用原有领域入口（未声称原有算法为本轮迁移成果） |
| lifecycle_api_tests.rs | 351 | 234 | 3条领域用例迁至 lifecycle_tests.rs；保留 gateway_child 和 cc_route_010_lifecycle_api_expected_behavior 真实启动、路由、鉴权、绑定冲突及停机契约；共享进程/策略/客户端夹具仍被相邻 HA/diagnostics 测试调用 |
| outbound_api_tests.rs | 124 | 0 | gateway-app-routing/src/host_migration_7_tests/outbound_api_tests.rs；全部断言保留 |
| config_api_tests.rs | 234 | 0 | gateway-app-routing/src/host_migration_7_tests/config_api_tests.rs；全部断言保留 |
| mod.rs | 42 | 11 | 只注册宿主装配、真实启动测试及相邻任务所有的 HA 模块 |
| outbound_probe.rs | 31 | 0 | 移除转发；server/router.rs 直接引用原有领域入口（未声称原有算法为本轮迁移成果） |
| client_host.rs | 24 | 0 | 移除转发；server/router.rs 直接引用原有领域入口（未声称原有算法为本轮迁移成果） |
| lifecycle_api.rs | 22 | 21 | 保留 paths、lifecycle、configure、configure_with_policy：全局路径、具体宿主生命周期装配及既有测试注入端口 |
| outbound_probe_tests.rs | 263 | 0 | gateway-app-routing/src/host_migration_7_tests/outbound_probe_tests.rs；全部断言保留 |
| outbound_api.rs | 2 | 0 | 移除转发；server/router.rs 直接引用原有领域入口（未声称原有算法为本轮迁移成果） |
| request_status.rs | 15 | 0 | 移除转发；server/router.rs 直接引用原有领域入口（未声称原有算法为本轮迁移成果） |
| config_api.rs | 2 | 0 | 移除转发；server/router.rs 直接引用原有领域入口（未声称原有算法为本轮迁移成果） |
| mode_attach.rs | 2 | 0 | 移除转发；server/router.rs 直接引用原有领域入口（未声称原有算法为本轮迁移成果） |

来源共 1382 → 266 行，净减 1116 行（80.8%）。

## 全用例映射
- `cc_route_030_outbound_scan_expected_behavior` → crates/libs/gateway-app-routing/src/host_migration_7_tests/outbound_scan_tests.rs（名称、断言保留）。
- `cc_route_030_tcp_candidate_closes_without_protocol_claim` → crates/libs/gateway-app-routing/src/host_migration_7_tests/outbound_scan_tests.rs（名称、断言保留）。
- `cc_route_030_cancel_and_deadlines_stop_pending_io` → crates/libs/gateway-app-routing/src/host_migration_7_tests/outbound_scan_tests.rs（名称、断言保留）。
- `cc_route_030_cancel_drops_inflight_connection_before_return` → crates/libs/gateway-app-routing/src/host_migration_7_tests/outbound_scan_tests.rs（名称、断言保留）。
- `gateway_child` → crates/vw-agent/src/gateway/api/handlers/app_routing/lifecycle_api_tests.rs（名称、断言保留）。
- `cc_route_011_lifecycle_reads_the_persisted_listener_config` → crates/libs/gateway-app-routing/src/host_migration_7_tests/lifecycle_tests.rs（名称、断言保留）。
- `cc_route_010_lifecycle_api_expected_behavior` → crates/vw-agent/src/gateway/api/handlers/app_routing/lifecycle_api_tests.rs（名称、断言保留）。
- `cc_route_010_lifecycle_stop_preserves_attached_apps` → crates/libs/gateway-app-routing/src/host_migration_7_tests/lifecycle_tests.rs（名称、断言保留）。
- `cc_route_010_lifecycle_policy_reload_cannot_move_a_running_listener` → crates/libs/gateway-app-routing/src/host_migration_7_tests/lifecycle_tests.rs（名称、断言保留）。
- `cc_route_028_outbound_api_expected_behavior` → crates/libs/gateway-app-routing/src/host_migration_7_tests/outbound_api_tests.rs（名称、断言保留）。
- `cc_route_012_config_api_expected_behavior` → crates/libs/gateway-app-routing/src/host_migration_7_tests/config_api_tests.rs（名称、断言保留）。
- `cc_route_029_outbound_probe_expected_behavior` → crates/libs/gateway-app-routing/src/host_migration_7_tests/outbound_probe_tests.rs（名称、断言保留）。
- `cc_route_029_probe_reports_headers_without_following_redirects` → crates/libs/gateway-app-routing/src/host_migration_7_tests/outbound_probe_tests.rs（名称、断言保留）。

## 生产接线与依赖

宿主 gateway/router.rs 的 app_routing_router 调用 gateway_app_routing::server::router；路由、1024 字节限制和 no-store 注册从真实生产入口迁入领域库。所有泛型端点显式使用 RoutingControl。7个旧转发文件和4个旧测试挂载删除；不使用 path/include 加载宿主代码。领域 lib.rs 以 cfg(all(test,target_os="macos")) 自注册。

测试直接使用 GatewayTransport，避免 vw-gateway-client 对本库的反向依赖。新增 dev 依赖仅 tempfile、hex、model-http-client；Cargo.lock 仅补本包三项。测试 fixture 使用真实 RoutingControl、数据库、策略文件、诊断任务及 AggregationLifecycle 执行器；显式注入租户和目录，不设置进程环境变量。Publication 仅适配现有快照/发布入口，不复制领域算法。它不替代宿主启动与全局中间件验收，后者保留。

## 行数分类

领域新增生产：56 行。

领域测试：1106 行。

源宿主生产（含注册）171 → 32 行；测试1211 → 234 行。宿主 router 注册另净减52行。只声明物理归属减量，不声明编译收益。

## 未来验证

`python3 scripts/cargo_slot.py cargo test -p gateway-app-routing --lib host_migration_7_tests -- --test-threads=1`：预期11条非零测试。

`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib gateway::api::handlers::app_routing::lifecycle_api_tests::cc_route_010_lifecycle_api_expected_behavior -- --exact --test-threads=1`：预期1条。gateway_child 是该用例启动的 ignored 子进程入口；未来在真实宿主 --list 中核对其余11条原完整路径消失，不能以过滤零测试声明通过。

静态审查：旧挂载/转发引用、真实生产调用、清单依赖、完整名称与断言映射及 diff 空白检查已核对；全部编译、类型、运行与行为检查未运行。
