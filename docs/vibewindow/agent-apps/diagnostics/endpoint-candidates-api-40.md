# 规范化端点候选 API（任务 40）

2026-11-12，macOS / Darwin arm64，vw-agent 与 vw-api-types 0.6.0。直接在原仓库执行，未创建工作树，plan.md 未修改。

## 实现和依赖

- 直接依赖任务 30 的 `vw_agent_apps::diagnostics::endpoints::collect`（`c34febb27`）及任务 38 的诊断作用域、路由和授权生产链（`9cb5aaa84`）。供应商记录读取复用 `AppProviderStore::preview_record`，自定义端点复用记录内 `customEndpoints`，预设复用母任务 5 的应用预设目录；没有新增端点存储。
- `POST /v1/agent-apps/diagnostics/endpoint-candidates/query?projectId=project&targetHostId=host&appId=claude`，JSON 正文为 `providerId`、可选 `draftUrls` 和可选 `presetName`；草稿 URL 不放在查询字符串。原查询 DTO 缺少编辑器选择的预设信息，本任务补充可选 `presetName`，通过应用目录校验，无新增配置或授权门槛。
- 处理器位于 `gateway/api/handlers/agent_apps_diagnostics.rs::endpoint_candidates`。先复用 `authorize_diagnostic_probe` 核对当前成员、租户、主机、项目、应用及供应商读取权限，再读取租户/应用记录并调用纯合并函数。没有创建凭据服务、诊断作业、HTTP 客户端或网络连接，不保存草稿。
- 成功返回 200、`Cache-Control: no-store` 及 `DiagnosticEndpointCandidatesResponse`。候选保留来源标记（draft/saved_config/preset/custom），重复 URL 合并，公开 ID 沿用测速执行端的 SHA-256 URL 标识。ID 不授予探测权限，测速执行端仍按已授权保存记录重新解析候选。
- 无效成员 401、越权 403、已授权但供应商不存在 404、非法草稿或未知预设 422；错误沿用脱敏诊断错误体。拒绝内嵌凭据、空 userinfo、非 HTTP(S)、含查询凭据及非回环明文 HTTP。32 KiB 正文限制沿用现有路由。
- bootstrap 的路由清单追加该 POST，实际路由复用已有 router 合入。独立测试沿用 `agent_apps_diagnostics_tests.rs` 的现有 macOS 注册，无重复模块。

## 验证

每个阶段原子领取共享槽位并在结束后只释放自己的锁，保留缓存。最终测试及格式化实际槽位为 5。

```text
CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-5
CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-5
CARGO_INCREMENTAL=0
CARGO_BUILD_JOBS=1
CARGO_PROFILE_TEST_DEBUG=0
CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_CODEGEN_UNITS=256
CARGO_PROFILE_TEST_LTO=off
CARGO_PROFILE_TEST_OPT_LEVEL=0
```

聚焦测试使用默认 features；Clippy 使用 workspace 全部 features。没有修改 Cargo 配置，也未运行全量测试或 Makefile。

真实行为红灯：`cargo test -p vw-agent --lib diagnostic_endpoints_api_is_read_only_and_scoped -- --nocapture`，退出 101，1 项测试失败、12397 项过滤；构建 5m05s，测试 0.30s。断言：

```text
assertion `left == right` failed: authorized candidate query must be available without starting a diagnostic
  left: 404
 right: 200
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12397 filtered out
```

首次夹具误用不存在的存储 `set` 方法，已改为真实 `upsert`，编译失败不计行为红灯。曾误启动第二个槽位 1 的构建，主动终止自己的 Cargo（退出 143），确认结束并释放自己的锁，未清除缓存；正式验证重新原子取得槽位 5。

`cargo fmt --all -- --emit stdout --config skip_children=true` 退出 0；仅输出全仓格式化结果，随后仅对本任务四个文件使用 rustfmt 回写。共享入口写入领取合入锁；审查确认未引入无关格式差异。共享处理器既有的 `read_job` 可见性改动排除本任务提交。

测试使用隔离 SQLite 真实供应商记录及私有策略文件，本地真实 HTTP 监听器用于观察连接计数；不代表真实供应商验证。校验四种来源合并、规范化和公开 ID、预设候选、项目/主机/应用/租户隔离、令牌和读取权限撤销、非法 URL 与未知供应商/预设拒绝、记录不变及无诊断作业文件。结束前观察本地服务请求数为零，abort 并 await 服务任务，确认退出。

绿灯：同一聚焦命令退出 0，1 passed、0 failed、12397 filtered，构建 7m23s，行为执行 0.06s。真实返回 200；合并后 6 个候选，第一项同时包含 draft/saved_config/preset/custom 四来源；没有泄漏供应商私钥。全部拒绝分支、保存记录不变及实际 HTTP 请求计数为 0 的断言均通过，服务任务确认取消退出。测试链接出现既有 `__eh_frame` 超过 16MB 警告（前置任务 38 已记录同类警告），不是本任务引入的代码警告。

最终 `cargo clippy --all-targets --all-features -- -D warnings` 退出 0，dev optimized，5m23s，无警告或错误；实际槽位 5，两个构建目录变量均为上述绝对路径。结束确认自己的 slot-5.lock 已释放，缓存保留。测试通过后四个本任务 Rust 文件 SHA-256 未改变；差异审查只包含候选查询及其直接合同、路由清单、独立测试和本证据。状态：已完成。
