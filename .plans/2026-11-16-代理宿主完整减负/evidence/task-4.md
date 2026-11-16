# 任务4证据

代码完成，编译/行为未验证。遵守本轮禁止 Cargo、单元测试、构建与实机测试的约束，仅阅读源码和静态核对。

## 全文件与生产边界

| 来源 | 修改前行数 | 保留行数 |
| --- | ---: | ---: |
| agent_apps_diagnostics.rs | 15 | 15 |
| agent_apps_diagnostics_tests.rs | 874 | 93 |
| agent_apps_diagnostics_acceptance_tests.rs | 555 | 88 |

宿主生产15行保持不变：router / router_with_policy 绑定真实 NativeProbeHost；endpoint_drafts_tests 挂载由任务20所有，本任务不迁移。没有将既有生产下沉计作本轮成果。
宿主测试1429行降为181行，净减1248行，超过来源1444行的70%。接收库新增测试与夹具1277行，生产新增0行。物理减量不代表编译或耗时收益。

## 全用例映射

| 原宿主完整测试后缀 | 实际所有者与新挂载 |
| --- | --- |
| agent_apps_diagnostics_tests::diagnostic_api_start_authorizes_before_execution | gateway-agent-apps::host_migration_4_tests::start::diagnostic_api_start_authorizes_before_execution |
| agent_apps_diagnostics_tests::diagnostic_api_read_and_cancel_are_scope_bound | gateway-agent-apps::host_migration_4_tests::scope::diagnostic_api_read_and_cancel_are_scope_bound |
| agent_apps_diagnostics_tests::diagnostic_endpoints_api_is_read_only_and_scoped | gateway-agent-apps::host_migration_4_tests::endpoints::diagnostic_endpoints_api_is_read_only_and_scoped |
| agent_apps_diagnostics_acceptance_tests::diagnostic_macos_jobs_acceptance | gateway-agent-apps::host_migration_4_tests::acceptance::diagnostic_macos_jobs_acceptance |
| agent_apps_diagnostics_tests::diagnostic_api_start_authorizes_before_execution_gateway | 保留宿主：真实子进程启动、租户头、鉴权先于请求与启动扩展接线 |
| agent_apps_diagnostics_acceptance_tests::diagnostic_macos_jobs_acceptance_gateway | 保留宿主：SDK到独立网关、实际取消TCP EOF、进程回收 |

四个领域函数整条迁出，旧文件不再定义或挂载这些函数；原mod仅保留两个真实宿主验收。所有断言和测试名保持，导入与泛型路由改为真实领域入口。
Servers / Drop、terminal、rejected、STDIO 随 acceptance 迁移；TOKEN 各模块隔离；stream_fixtures 保留既有 samples 原始线协议数据，无跨目录path/include。probe_host 仅连接已有真实策略授权、作业注册、mcp-client传输、plugin-mcp工具清单及tool-mcp DTO，不克隆App/AppState、不重写领域算法、不使用模拟授权。宿主真实 NativeProbeHost 继续由两个进程用例覆盖。

领域生产调用为宿主 router -> gateway_agent_apps::agent_apps_diagnostics::router::<NativeProbeHost> -> app-access-service / app-diagnostics-runtime / agent-app-services。接收库 lib.rs 自注册 host_migration_4_tests（macOS test）；新增依赖仅为macOS dev依赖及tokio test-util，无反向依赖vw-agent。Cargo.lock仅补已有包的接收库依赖边；未调用Cargo生成。

## 未来验证（本轮全部未运行）

- macOS：`python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib host_migration_4_tests -- --test-threads=1`，静态预期4条，须ran>0且failed=0。
- macOS：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib gateway::api::handlers::agent_apps_diagnostics_tests::diagnostic_api_start_authorizes_before_execution_gateway -- --exact --test-threads=1`，预期1条。
- macOS：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib gateway::api::handlers::agent_apps_diagnostics_acceptance_tests::diagnostic_macos_jobs_acceptance_gateway -- --exact --test-threads=1`，预期1条。
- 未来宿主 `--list` 核对四个迁出名称消失，不将零测试当通过。未运行任何编译、行为、实机或性能检查。
