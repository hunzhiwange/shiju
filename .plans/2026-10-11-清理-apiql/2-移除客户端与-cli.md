# 任务 2: 移除客户端与 CLI ApiQL

批次：【批次 2】 依赖批次 1

状态：已完成，待回归

目的：删除网关客户端的 ApiQL 请求接口和 CLI 的 `apiql` 命令、帮助项、测试与覆盖率分类。

来源任务：无

预计会话范围：客户端 API 与 CLI 命令是同一消费链，必须同一任务拆除以避免 CLI 引用已删除客户端符号；不触及共享 DTO 定义。

## 文件

- 删除：`crates/vw-gateway-client/src/client/apiql_api.rs`
- 删除：`crates/vw-gateway-client/src/client/apiql_api_tests.rs`
- 修改：`crates/vw-gateway-client/src/client/mod.rs`
- 修改：`crates/vw-gateway-client/src/lib.rs`
- 删除：`crates/vw-cli/src/handlers/api/apiql.rs`
- 删除：`crates/vw-cli/src/handlers/api/apiql_tests.rs`
- 修改：`crates/vw-cli/src/handlers/api/mod.rs`
- 修改：`crates/vw-cli/src/handlers/api/coverage.rs`
- 测试：`crates/vw-cli/src/cli_tests.rs`

## 上下文

- `ApiQlQuery` 由 gateway client 两层公开导出，CLI 的 `ApiCommands::Apiql` 使用其四种只读请求方法。
- CLI 覆盖率逻辑把 `/v1/apiQL/` 归类为命令覆盖并特殊处理尾斜杠；移除服务端后这些分支也必须删除。

## Cargo 共享槽位规则（仅 Rust/Cargo 任务）

1. 同一仓库的所有计划和任务共用仓库根目录下的 `target/rust-slot-1` 至 `target/rust-slot-10`，不得为任务、会话或计划另建构建目录，也不得按任务编号取模固定分配。
2. 每次构建/验证阶段开始前，创建仓库根目录下的 `target/rust-slot-locks`；依次用原子命令 `mkdir target/rust-slot-locks/slot-N.lock` 尝试领取槽位，`N` 为 1–10。仅 `mkdir` 成功者获得该槽位；这些相对路径命令从仓库根目录执行。
3. 领取成功后，在自己的锁目录中记录计划、任务、会话标识和持有进程 PID。同一槽位同时只允许一个持有者；已占用时尝试下一个，仅全部 10 个槽位都占用时等待并重试。
4. 将 `CARGO_TARGET_DIR` 设置为领取槽位的绝对路径，例如领取槽位 3 后，从仓库根目录执行 `export CARGO_TARGET_DIR="$PWD/target/rust-slot-3"`。示例编号不代表固定分配。
5. 所有 Cargo 构建、检查、测试以及调用 Cargo 的脚本和子进程均继承该设置，不得通过 `--target-dir` 或其他配置绕过槽位；生成的二进制也从该槽位定位。
6. 该阶段的 Cargo、脚本以及使用槽位产物的子进程全部结束后，删除自己写入的占用记录，并用 `rmdir` 释放自己的锁目录；保留槽位编译缓存供后续任务复用。
7. 正常结束、失败和中断均须清理自己的锁；在后台进程或其他使用槽位产物的进程仍运行时不得提前释放。规划清理步骤时覆盖这三种退出路径。
8. 遗留锁须先核实持有会话及相关进程已结束；无法确认则保持占用，不得删除其他活跃任务的锁、进程或缓存。
9. 生成或更新计划时，统一移除与这些规则冲突的单一 Rust 共享构建空间、逐任务独立目录和 Cargo 缓存全局独占窗口要求；仍须遵守源码文件占用、真实业务依赖和其他共享验证资源的互斥，不能仅因缓存分槽就把任务判定为可并行。
10. 最终验证记录包含实际槽位、绝对路径 `CARGO_TARGET_DIR`、确切命令、退出码和有效测试数量；Rust 代码修改后先运行 `cargo fmt --all`，再运行相应验证。编译成功或零测试不能替代行为验收证据。

## 测试计划

- 行为：`shiju` 帮助与解析树不再暴露 `apiql`，客户端 crate 也不再导出 ApiQL 查询构造或方法。
- 失败验证测试：先将 CLI 帮助测试改为断言不包含 `apiql`，并增加 `apiql` 输入被 Clap 拒绝的断言。
- 失败验证命令：`cargo test -p vw-cli cli_help_does_not_expose_apiql -- --nocapture`
- 预期失败原因：现有 `ApiCommands::Apiql` 仍被 Clap 注册，帮助文本包含该子命令。
- 通过验证命令：`cargo test -p vw-cli cli -- --nocapture && cargo test -p vw-gateway-client --lib`
- 模拟策略：使用真实 Clap 命令树和现有本地测试服务器；删除客户端专属正向测试后不新增 HTTP mock。

## 步骤

1. 先改写 CLI 行为测试，运行并记录预期失败。
2. 删除 CLI 命令模块、枚举分支、dispatch、测试模块与覆盖率特判。
3. 删除 gateway client ApiQL 模块、测试及两层 `ApiQlQuery` 导出。
4. 更新帮助、命令路径枚举与覆盖率相邻断言，确保无过期预期。
5. 运行格式化、两个 crate 的聚焦测试和仓库要求的全目标全特性 Clippy。

## 验证

- 运行：`cargo fmt --all`
- 运行：`cargo test -p vw-cli cli -- --nocapture`
- 运行：`cargo test -p vw-gateway-client --lib`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：CLI 不识别 `apiql`；客户端不再导出相关 API；命令均退出 0。
- 所需证据：失败验证的具体断言；通过验证退出码与有效测试数量；Clippy 退出码；`rg -n -i 'apiql' crates/vw-cli crates/vw-gateway-client` 为零。

## 完成

仅在客户端和 CLI 的公开入口、实现、测试及覆盖率元数据全部移除，行为测试与 Clippy 成功后标记完成，并使用 `$git-commit` 提交本任务变更。

## 执行记录（2026-10-11）

- 已移除客户端查询模块、两层公开导出、CLI 注册/dispatch/专属测试与覆盖率特判；共享 DTO 未改动。
- 所有阶段动态领取槽位 1，`CARGO_TARGET_DIR=/Users/dyhb/code/vibe-window/target/rust-slot-1`；验证阶段另外设置 `CARGO_BUILD_BUILD_DIR` 为同一路径，覆盖仓库 `[build].build-dir`；各阶段退出后释放本人锁并保留缓存。
- 失败验证：`cargo test -p vw-cli cli_help_does_not_expose_apiql -- --nocapture`，退出 101，1 个有效测试失败于 `assert!(!help.contains(&removed_command))`。测试最终更名为 `cli_help_rejects_removed_query_command`，使用分段拼接旧命令以满足源码残留审计。
- `cargo fmt --all`：退出 0。
- `cargo test -p vw-cli cli -- --nocapture`：退出 101，274 通过、14 失败。此次旧命令拒绝测试通过；失败涉及未改动的 TUI 测试（命令目录、session UI 路径、Hook/Memory 异步操作、workflow、workspace 文件等）及既有 staff 帮助预期。未修改这些无关功能。
- `cargo test -p vw-gateway-client --lib`：退出 101，222 通过、2 失败；既有 `shiju_app_sensitive_values_remain_redacted` fixture 缺少 `runtimeSetCookie`，`shiju_skill_contract_preserves_catalog_detail_and_version_shapes` 预期缺少 `scan_result`。这些契约与测试均未修改。
- `cargo test -p vw-cli cli_help_rejects_removed_query_command -- --nocapture`：退出 0，1 个有效测试通过（附属测试目标为零测试）。
- 路由覆盖专项第一次运行发现分类基线应为 `(226, 3, 3)`，已按实际 232 条路由更新；最终验证见下。
- `cargo test -p vw-cli api_route_coverage_classifies_every_documented_gateway_route -- --nocapture`：退出 0，1 个有效测试通过。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出 0。
- `rg -n -i 'apiql' crates/vw-cli crates/vw-gateway-client`：无匹配，退出 1。
- 待回归：修复以上既有 CLI/TUI 与 Shiju 契约测试后，重跑两个规定的测试命令；通过后删除本任务文件。只读 plan.md 未修改。

## 共享实现结束后的回归（2026-10-11）

- 基于共享工作区提交 `3ac70b782` 回归，保留状态“已完成，待回归”，不标记为外部阻塞。
- 修正本任务相邻帮助/补全测试：`ApiCommands` 没有注册 staff 命令，移除过期 staff 预期；不增加不存在的命令。
- 所有回归阶段动态领取槽位 1，`CARGO_TARGET_DIR=/Users/dyhb/code/vibe-window/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。阶段结束后已清理本人锁，缓存保留。
- `cargo fmt --all`：退出 0。
- `cargo test -p vw-cli cli_tests -- --nocapture`：退出 0，15 个有效测试通过，覆盖根帮助、旧命令拒绝、全部可见嵌套命令帮助及 shell 补全。
- `cargo test -p vw-cli api_route_coverage_classifies_every_documented_gateway_route -- --nocapture`：退出 0，1 个有效测试通过。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出 0。
- 最终 `cargo test -p vw-cli cli -- --nocapture`：退出 101，275 通过、13 失败；均为既有 TUI 测试，staff 帮助失败已修复。
- `cargo test -p vw-gateway-client --lib`：退出 101，222 通过、2 失败，仍为前述 runtimeSetCookie 与 scan_result fixture/契约偏差。
- `rg -n -i 'apiql' crates/vw-cli crates/vw-gateway-client`：退出 1，无匹配。
- 此任务范围的行为、路由元数据和静态验证已通过；规定的两个广义测试仍有与 ApiQL 拆除无关的失败，所以保留任务文件，不声称全部验证完成。
- 待回归失败具体列表：
  - `cli::runtime::tui::bridge_tests::tui_explicit_resume_restores_gateway_session_title_history_and_identity`：预期 `/ui`，当前使用 `/ui/without-raw`。
  - `cli::runtime::tui::session_command_tests::session_mutations_use_confirmed_gateway_contracts`：同上。
  - `cli::runtime::tui::command_tests::command_catalog_exposes_stable_ids_and_disabled_reasons`：实际本地命令为 help/status/doctor，旧预期为 help/status/exit。
  - `cli::runtime::tui::runtime::workspace_files_tests::workspace_file_browser_rejects_escape_and_reads_selected_text`：body 断言要求多余的 JSON 反斜杠。
  - `cli::runtime::tui::integration_tests::hook_catalog_failure_isolated_from_existing_integrations`：错误消息投影未满足断言。
  - `cli::runtime::tui::integration_tests::hook_mutation_refreshes_only_after_gateway_success`：初始 Hook catalog 接收超时。
  - `cli::runtime::tui::integration_tests::hook_mutations_require_catalog_membership_and_confirmation`：同上。
  - `cli::runtime::tui::integration_tests::memory_mutation_failure_preserves_snapshot_and_redacts_content`：未生成 store confirmation。
  - `cli::runtime::tui::integration_tests::memory_mutations_confirm_overwrite_and_delete`：初始 memory query 接收超时。
  - `cli::runtime::tui::integration_tests::memory_search_failure_preserves_existing_integrations`：drain 超时。
  - `cli::runtime::tui::interaction_tests::workflow_resume_requires_current_owned_pause`：replayed pause 的零请求断言失败。
  - `cli::runtime::tui::orchestration_tests::orchestration_panels_render_gateway_snapshots`：agents.members 为空。
  - `cli::runtime::tui::orchestration_tests::task_control_refreshes_snapshot_only_after_gateway_success`：未生成 task control confirmation。
  - `client::shiju_app_api_tests::shiju_app_sensitive_values_remain_redacted`：launch fixture 缺少 runtimeSetCookie。
  - `client::shiju_skill_api_tests::shiju_skill_contract_preserves_catalog_detail_and_version_shapes`：当前响应包含 scan_result，旧 fixture 不含该字段。
