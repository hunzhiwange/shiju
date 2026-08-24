# 任务 4: 接通 Hook 目录与审计

批次：【批次 4】 依赖批次：批次 3

状态：未开始

目的：让 `/hooks` 与 `/hooks audit` 展示 Gateway 返回的 Hook 声明和安全审计记录。

来源任务：无

预计会话范围：只读加载两类 Hook DTO 并渲染；不信任、撤销或执行 Hook。

## 文件

- 修改：`crates/vw-cli/src/cli/tui/commands.rs`
- 修改：`crates/vw-cli/src/cli/tui/app.rs`
- 新建：`crates/vw-cli/src/cli/tui/runtime/hooks.rs`
- 新建：`crates/vw-cli/src/cli/tui/runtime/hooks_tests.rs`
- 修改：`crates/vw-cli/src/cli/tui/runtime/mod.rs`
- 测试：`crates/vw-cli/src/cli/tui/command_tests.rs`
- 测试：`crates/vw-cli/src/cli/tui/integration_tests.rs`

## 上下文

- 使用 `hooks_get` 与 `hook_audit_get` 的强类型 DTO；Hook hash 和 plugin ID 是后续 mutation 的稳定凭据。
- 审计记录只能展示 DTO 已提供的 bounded safe fields，不展示环境、原始命令载荷或认证数据。
- Hook 来源失败不能破坏现有 Skill/Plugin/MCP/Memory 目录。

## 测试计划

- 行为：Hook 目录与审计能独立加载和展示，空结果、单来源失败与畸形字段有明确状态。
- 失败验证测试：新增 `hooks_commands_route_read_only_views`、`hook_catalog_and_audit_render_safe_fields`。
- 失败验证命令：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期失败原因：Hook 命令、runtime 和 view model 尚未接线。
- 通过验证命令：`cargo clippy --all-targets --all-features -- -D warnings`
- 模拟策略：HTTP fixture 返回真实 Hook catalog/audit DTO；不模拟解析后的 item。

## 步骤

1. 添加命令路由、独立失败、空状态和敏感字段不展示测试。
2. 运行失败验证命令并确认缺失 Hook 活动路径。
3. 新增 Hook 只读 runtime 与稳定 view model。
4. 接通 selection/transcript 展示，保持与 integration catalog 的失败隔离。
5. 运行通过验证命令并审查无 mutation 请求。

## 验证

- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：退出码 0；测试编译；无 warning。
- 所需证据：失败与通过证据、请求路径 `/v1/hooks` 和 `/v1/hooks/audit`、只读结果及敏感字段审查。

## 完成

仅当 Hook 目录和审计均活动可达、失败隔离且只展示安全字段后，任务可完成。
