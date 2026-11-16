# 任务57 静态交付证据

代码完成，编译/行为未验证。遵守用户禁令，未运行 Cargo（含 list/fmt/check）、构建、单元或实机测试。

## 所有权与接线

真实生产入口 gateway/bootstrap.rs → 宿主 gateway_owner::run_with_routing → app-routing-services::gateway_owner::run_with_routing(database, serve, shutdown)。本轮实际迁移 biased select、listener 所有权及 shutdown 等待/错误传播；数据库路径选择仍属宿主。没有反向依赖、include/path 引用宿主源码。

控制器鉴权测试放到既有 gateway-app-routing，避免 app-routing-services 反向 dev 依赖网关。复用 host_migration_7_tests::fixture::Lifecycle（真实 agent_gateway 聚合生命周期）与原 configure_with_policy；路径来自临时目录。队列测试使用真实 FailoverQueueStore 与临时数据库，消除全局 home 依赖。startup_policy 使用真实 app-access-service::access::routing_setup。

两个接收库 lib.rs 自注册 host_migration_57_tests；旧 aggregation/failover 测试 mod 已删除。所有 30 个未挂载转发文件删除；mod.rs 的实际公共重导出保留，真实调用者无需重接其它层。任务1跨目录来源核对：本组原测试均由原 mod.rs 本地注册，无外部 path 测试挂载。

## 全文件映射

| 来源 | 原行数 | 现行数 | 归属/保留职责 |
|---|---:|---:|---|
| `adapter_selection.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `aggregation_routing.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `aggregation_routing_tests.rs` | 395 | 0 | 整条测试 → gateway-app-routing::host_migration_57_tests::aggregation；保留真实策略授权及事务断言 |
| `body_limits.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `browser_guard.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `cancellation.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `capacity.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `config_apply.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `decode_request.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `decode_response.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `desktop_routing.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `dispatch.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `failover_execution.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `failover_routing.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `failover_routing_tests.rs` | 316 | 0 | 监听器顺序选择 → app-routing-services::host_migration_57_tests::failover；未授权发布 → gateway-app-routing::host_migration_57_tests::authorization |
| `gateway_owner.rs` | 19 | 11 | run_with_routing：全局路径选择与数据库注入；关闭算法委托领域库 |
| `gateway_owner_tests.rs` | 261 | 155 | gateway_child、GatewayProcess::drop、host、write_policy、cc_route_007_gateway_owner_expected_behavior：真实 gateway/daemon 子进程恢复、信号退出和原配置不变验收 |
| `header_wire.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `listener.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `mod.rs` | 18 | 14 | 保留实际公共兼容入口及宿主进程测试注册 |
| `outbound_config.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `outbound_custom.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `outbound_system.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `override_apply.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `protocol_errors.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `provider_publication.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `reference_scope.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `request_status.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `route_auth.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `shared_transport.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `shutdown.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `timeouts.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |
| `transport_security.rs` | 2 | 0 | 删除未挂载兼容转发；生产实现原已归属 app-routing-services，不计为本轮生产迁移 |

宿主总行数 1065 → 180，净减 885（83.1%），超过70%。原测试文件972行 → 155行；原生产/注册93行 → 25行（包含原未挂载转发60行，不能计为真实生产下沉）。

## 全用例映射与未来验证

名称和全部断言保留，仅修改导入、测试数据库/路径及显式数据库参数。

| 原测试名（原 app_routing 模块） | 新模块 |
|---|---|
| aggregation_routing_tests::aggregation_t11_published_revision_routes_and_revokes_exact_targets | gateway-app-routing::host_migration_57_tests::aggregation |
| failover_routing_tests::aggregation_t12_listener_preserves_queue_and_resumes_ordered_selection | app-routing-services::host_migration_57_tests::failover |
| failover_routing_tests::aggregation_t12_queue_publication_rejects_unauthorized_scope | gateway-app-routing::host_migration_57_tests::authorization |
| gateway_owner_tests::cc_route_007_gateway_owner_rejects_untrusted_startup | gateway-app-routing::host_migration_57_tests::startup_policy |
| gateway_owner_tests::cc_route_007_gateway_owner_waits_for_connections_before_returning | app-routing-services::host_migration_57_tests::gateway_owner |
| gateway_owner_tests::gateway_child | 宿主原模块保留（ignored 子进程入口） |
| gateway_owner_tests::cc_route_007_gateway_owner_expected_behavior | 宿主原模块保留 |

未来 macOS 执行，全部当前未运行：

- `python3 scripts/cargo_slot.py cargo test -p app-routing-services --lib host_migration_57_tests -- --test-threads=1`，预期2条。
- `python3 scripts/cargo_slot.py cargo test -p gateway-app-routing --lib host_migration_57_tests -- --test-threads=1`，预期3条。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib app_routing::gateway_owner_tests::cc_route_007_gateway_owner_expected_behavior -- --exact --test-threads=1`，预期1条并启动 gateway_child。
- 未来宿主 `--list` 应仅保留上述两个宿主测试，不以旧过滤零测试宣称通过。

静态核对：旧模块注册消失，测试名逐项对应，接收库现有直接依赖覆盖迁移导入；无删除断言。构建耗时、运行通过数和编译兼容性均无本轮证据。

crates/libs/app-routing-services/src: 新增测试/夹具 314 行。
crates/libs/gateway-app-routing/src: 新增测试/夹具 544 行。
app-routing-services 新增生产 gateway_owner.rs 19行；两个 lib.rs 注册及生产模块声明共8行。
