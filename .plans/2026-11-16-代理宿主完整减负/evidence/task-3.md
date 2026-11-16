# 任务3：授权路由领域测试归属迁移

代码完成，编译/行为未验证。本轮遵守只写代码约束，未运行任何 Cargo（含 fmt/list）、构建、单元/实机测试。静态检查不能证明编译收益或行为通过。

## 真实生产入口与边界

`gateway/router.rs::management_auth_router` → 宿主 `agent_apps_auth_read::router` → 领域 `agent_apps_auth_read::router`。宿主 basic_auth 回调仍委托 agent-gateway 的真实 middleware；领域 router 注册 read/binding/default/remove/start/poll/cancel/logout。Runtime 由领域已有实现持有。以上生产下沉早已存在，本轮生产迁移收益为 0；没有添加算法或生产端口。

测试直接使用领域 router / router_with_runtime，鉴权注入 agent_gateway::middleware::basic_auth_middleware_with_credentials。外层使用既有 agent_gateway::router::build_router_with_auth，三个无关路由槽传空 Router。真实权限文件、数据库、临时凭证文件、OAuth HTTP fixture、子进程、Keychain、安全隔离断言保留；未用 mock 替代真实鉴权。

依赖方向：gateway-agent-apps 的 macOS dev-dependencies 新增 agent-gateway、gateway-app-auth、libc，生产依赖不变。静态遍历 agent-gateway 的 331 个本地生产依赖清单未见 gateway-agent-apps 反向路径；无 vw-agent 依赖、path/include 宿主源码、App/AppState 克隆。Cargo.lock 仅增对应三个已有包名。

## 全文件与逐条用例映射

以下相对来源均位于 crates/vw-agent/src/gateway/api/handlers，目标位于 crates/libs/gateway-agent-apps/src/host_migration_3_tests。同名模块、函数和断言保持；仅 imports 和路由组合适配。

### agent_apps_auth_binding_tests.rs
- 来源 463 行 → 目标 463 行；宿主保留 0 行。
- `auth6_api_binding_commits_expected_scoped_result` → `host_migration_3_tests::agent_apps_auth_binding_tests::auth6_api_binding_commits_expected_scoped_result`
- `auth6_api_binding_rejects_invalid_or_stale_input` → `host_migration_3_tests::agent_apps_auth_binding_tests::auth6_api_binding_rejects_invalid_or_stale_input`
- `auth6_api_binding_commits_expected_scoped_result_postgres` → `host_migration_3_tests::agent_apps_auth_binding_tests::auth6_api_binding_commits_expected_scoped_result_postgres`
- `auth6_api_binding_rejects_invalid_or_stale_input_postgres` → `host_migration_3_tests::agent_apps_auth_binding_tests::auth6_api_binding_rejects_invalid_or_stale_input_postgres`
- 其余夹具/辅助函数整体归属目标：`new`、`serve`、`drop`、`put`、`assert_binding`、`sqlite`、`check_success`、`check_denials`。

### agent_apps_auth_default_tests.rs
- 来源 340 行 → 目标 340 行；宿主保留 0 行。
- `auth6_api_default_commits_expected_scoped_result` → `host_migration_3_tests::agent_apps_auth_default_tests::auth6_api_default_commits_expected_scoped_result`
- `auth6_api_default_rejects_invalid_or_stale_input` → `host_migration_3_tests::agent_apps_auth_default_tests::auth6_api_default_rejects_invalid_or_stale_input`
- `auth6_api_default_commits_expected_scoped_result_postgres` → `host_migration_3_tests::agent_apps_auth_default_tests::auth6_api_default_commits_expected_scoped_result_postgres`
- `auth6_api_default_rejects_invalid_or_stale_input_postgres` → `host_migration_3_tests::agent_apps_auth_default_tests::auth6_api_default_rejects_invalid_or_stale_input_postgres`
- 其余夹具/辅助函数整体归属目标：`new`、`serve`、`drop`、`request`、`put`、`status`、`commits`、`rejects`。

### agent_apps_auth_integration_tests.rs
- 来源 419 行 → 目标 419 行；宿主保留 0 行。
- `auth6_integration_commits_expected_scoped_result` → `host_migration_3_tests::agent_apps_auth_integration_tests::auth6_integration_commits_expected_scoped_result`
- `auth6_integration_commits_expected_scoped_result_postgres` → `host_migration_3_tests::agent_apps_auth_integration_tests::auth6_integration_commits_expected_scoped_result_postgres`
- `auth6_integration_rejects_invalid_or_stale_input` → `host_migration_3_tests::agent_apps_auth_integration_tests::auth6_integration_rejects_invalid_or_stale_input`
- `auth6_integration_rejects_invalid_or_stale_input_postgres` → `host_migration_3_tests::agent_apps_auth_integration_tests::auth6_integration_rejects_invalid_or_stale_input_postgres`
- 其余夹具/辅助函数整体归属目标：`start`、`stop`、`sdk`、`drop`、`serve`、`start`、`poll`、`exchange`、`access_token`、`assert_requests`、`jwt`、`new`、`root`、`serve`、`material`、`start`、`assert_http`、`commit`、`reject`。

### agent_apps_auth_logout_tests.rs
- 来源 598 行 → 目标 598 行；宿主保留 0 行。
- `auth6_api_logout_commits_expected_scoped_result` → `host_migration_3_tests::agent_apps_auth_logout_tests::auth6_api_logout_commits_expected_scoped_result`
- `auth6_api_logout_rejects_invalid_or_stale_input` → `host_migration_3_tests::agent_apps_auth_logout_tests::auth6_api_logout_rejects_invalid_or_stale_input`
- `auth6_api_logout_commits_expected_scoped_result_postgres` → `host_migration_3_tests::agent_apps_auth_logout_tests::auth6_api_logout_commits_expected_scoped_result_postgres`
- `auth6_api_logout_rejects_invalid_or_stale_input_postgres` → `host_migration_3_tests::agent_apps_auth_logout_tests::auth6_api_logout_rejects_invalid_or_stale_input_postgres`
- 其余夹具/辅助函数整体归属目标：`new`、`serve`、`stored_default`、`has_credential`、`drop`、`request`、`revoke`、`logout`、`read`、`assert_error`、`commits`、`assert_kimi_logout`、`assert_claude_cleanup`、`drop`、`rejects`。

### agent_apps_auth_read_tests.rs
- 来源 325 行 → 目标 318 行；宿主保留 17 行。
- `auth6_api_read_commits_expected_scoped_result` → `host_migration_3_tests::agent_apps_auth_read_tests::auth6_api_read_commits_expected_scoped_result`
- `auth6_api_read_rejects_invalid_or_stale_input` → `host_migration_3_tests::agent_apps_auth_read_tests::auth6_api_read_rejects_invalid_or_stale_input`
- `auth6_api_read_commits_expected_scoped_result_postgres` → `host_migration_3_tests::agent_apps_auth_read_tests::auth6_api_read_commits_expected_scoped_result_postgres`
- `auth6_api_read_rejects_invalid_or_stale_input_postgres` → `host_migration_3_tests::agent_apps_auth_read_tests::auth6_api_read_rejects_invalid_or_stale_input_postgres`
- 其余夹具/辅助函数整体归属目标：`new`、`router`、`start`、`drop`、`query`、`sqlite`、`check_success`、`check_denials`。

### agent_apps_auth_security_tests.rs
- 来源 426 行 → 目标 426 行；宿主保留 0 行。
- `auth6_security_commits_expected_scoped_result` → `host_migration_3_tests::agent_apps_auth_security_tests::auth6_security_commits_expected_scoped_result`
- `auth6_security_rejects_invalid_or_stale_input` → `host_migration_3_tests::agent_apps_auth_security_tests::auth6_security_rejects_invalid_or_stale_input`
- `auth6_security_commits_expected_scoped_result_postgres` → `host_migration_3_tests::agent_apps_auth_security_tests::auth6_security_commits_expected_scoped_result_postgres`
- `auth6_security_rejects_invalid_or_stale_input_postgres` → `host_migration_3_tests::agent_apps_auth_security_tests::auth6_security_rejects_invalid_or_stale_input_postgres`
- 其余夹具/辅助函数整体归属目标：`scope`、`request`、`response`、`send`、`reject`、`codex_token`、`setup`、`select_default`、`bind`、`assert_account`、`assert_binding`、`commits`、`rejects`、`drop`、`check_flow_and_keychain`。

### agent_apps_auth_start_tests.rs
- 来源 346 行 → 目标 346 行；宿主保留 204 行。
- `auth6_api_start_commits_expected_scoped_result` → `host_migration_3_tests::agent_apps_auth_start_tests::auth6_api_start_commits_expected_scoped_result`
- `auth6_api_start_commits_expected_scoped_result_postgres` → `host_migration_3_tests::agent_apps_auth_start_tests::auth6_api_start_commits_expected_scoped_result_postgres`
- `auth6_api_start_rejects_invalid_or_stale_input` → `host_migration_3_tests::agent_apps_auth_start_tests::auth6_api_start_rejects_invalid_or_stale_input`
- `auth6_api_start_rejects_invalid_or_stale_input_postgres` → `host_migration_3_tests::agent_apps_auth_start_tests::auth6_api_start_rejects_invalid_or_stale_input_postgres`
- 其余夹具/辅助函数整体归属目标：`start`、`start_with_runtime`、`post`、`drop`、`check_start`、`check_rejections`。

read 的 check_denials 原末尾两项未版本化 alias 断言拆到宿主 `host_auth_read_rejects_unversioned_aliases`，使用真实 handler_router。原四条 read 用例其它断言保留。

## 每个宿主保留文件的职责

- agent_apps_auth_read.rs：22 行，basic_auth（真实宿主 middleware 端口）、router（生产管理路由接线）、router_with_runtime（剩余宿主用例 runtime 注入）；本轮不迁移。
- agent_apps_auth_start.rs：3 行，Runtime 兼容导出仍被本任务夹具与其它所有者的 cancel/poll/redaction 测试引用。
- agent_apps_auth_read_tests.rs：仅真实宿主 handler_router 不暴露两项别名的 1 条薄用例。
- agent_apps_auth_start_tests.rs：OWNER/OTHER/READER/BASIC/FLOWS/CLAUDE/CODEX/LINK/PASTE 常量；Gateway、start/start_with_runtime/post/Drop 为宿主 cancel/poll/redaction 的直接共享夹具；另有 host_auth_start_uses_real_auth_wiring 一条薄接线用例。原 start 的 4 条测试及 check_start/check_rejections 均已迁出。当前不能删除共享夹具而破坏其它任务来源；领域版本是最小真实存储/临时目录夹具，无 App/AppState。后续这些消费者迁移后可删宿主夹具。

## 静态登记与减量

原来源生产/转发 25 行，测试/夹具 2917 行，共 2942 行；现宿主生产/转发仍 25 行，测试/夹具 221 行，共 246 行，净减 2696 行（91.6%，超过 70%）。领域本轮生产新增 0 行，测试/夹具/注册模块新增 2946 行（不含 lib.rs 的 3 行注册）。保留夹具带来的测试物理重复不算生产迁移。

领域 lib.rs 自注册 host_migration_3_tests（macOS cfg 沿用原限制）。宿主父模块删除 binding/default/integration/logout/security 的旧挂载，read/start 仅挂载薄边界与必要夹具。外部挂载基线 evidence/external-mounts.json 无本组条目。静态逐条确认 28 条原测试名称均在目标，原五个迁空文件删除。git diff --check 无输出。未运行宿主 list，不能将静态消失声称运行验证。

## 未来验证（本轮全部未运行）

macOS：
```sh
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib host_migration_3_tests -- --test-threads=1
```
预期 28 条（每组 SQLite/Postgres 成功/拒绝各一；Postgres 需原有环境，Keychain/进程需真实权限）。
```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib gateway::api::handlers::agent_apps_auth_read_tests::host_auth_read_rejects_unversioned_aliases -- --exact
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib gateway::api::handlers::agent_apps_auth_start_tests::host_auth_start_uses_real_auth_wiring -- --exact
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib -- --list
```
两条薄用例分别预期 1 条；宿主 list 应不再含上述 28 条旧完整名称（原前缀 gateway::api::handlers::，模块名同上）。其它所有者的宿主测试仍需未来验证其夹具兼容；此处不执行、不报通过。
