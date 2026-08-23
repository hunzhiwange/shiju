# 任务 8: 技能插件 MCP 与记忆

批次：【批次 8】 依赖批次 7

状态：未开始

目的：从 Gateway 加载技能、插件、MCP 和记忆真值，生成动态命令与只读面板，并对允许的管理操作执行明确确认。

来源任务：无

预计会话范围：聚焦集成目录的发现、去重、展示、prompt expansion 与插件/技能启停；不接入外部账户商城或本地任意脚本执行。

## 文件

- 新建：`crates/vw-cli/src/cli/tui/runtime/integrations.rs`
- 修改：`crates/vw-cli/src/cli/tui/runtime/mod.rs`
- 修改：`crates/vw-cli/src/cli/tui/model/integration.rs`
- 修改：`crates/vw-cli/src/cli/tui/model/overlay.rs`
- 修改：`crates/vw-cli/src/cli/tui/commands.rs`
- 修改：`crates/vw-cli/src/cli/tui/app.rs`
- 修改：`crates/vw-cli/src/cli/tui/bridge.rs`
- 修改：`crates/vw-cli/src/cli/tui/state/reducer.rs`
- 修改：`crates/vw-cli/src/cli/tui/mod.rs`
- 测试：`crates/vw-cli/src/cli/tui/integration_tests.rs`

## 上下文

- Gateway client 已提供 `skills_get`、`skill_set_enabled`、`plugins_get`、`plugin_set_enabled`、`plugin_delete`、`mcp_inspect`；记忆使用已有只读 Gateway 来源。
- 动态命令只能使用网关枚举的稳定 ID；来源冲突需要确定性优先级，单一来源失败不能清空其他来源。

## 验证计划（仓库限制）

- 行为：动态条目可发现、去重、显示来源并安全扩展 prompt；启停与删除只作用于所选稳定 ID，删除需要类型化确认。
- 失败验证测试：新增 `dynamic_integration_catalog_merges_healthy_sources`、`integration_actions_use_stable_ids_and_confirmation`。
- 失败验证命令：`cargo clippy -p vw-cli -p vw-tui --all-targets --all-features -- -D warnings`
- 预期失败原因：活动 runtime 无 integration loader，命令目录不能合并动态项，面板模型无执行路由。
- 通过验证命令：`cargo clippy -p vw-cli -p vw-tui --all-targets --all-features -- -D warnings`
- 模拟策略：本地 HTTP 服务分别返回每个来源；真实执行合并、去重、确认 intent 与 prompt construction。
- 回归限制：未经授权不运行 `cargo test`；只编译测试时使用 `已完成，待回归`。

## 步骤

1. 添加多来源成功、单源失败、重名冲突、动态 prompt、启停和删除确认测试。
2. 运行失败验证命令，确认缺少 integration runtime 与动态目录合并入口。
3. 在 runtime 并发读取各来源并返回逐源结果；不要让一个来源失败覆盖健康来源。
4. 将健康条目合并到任务 3 的命令目录和 integration overlay；执行只接受稳定 ID，写操作成功后重新加载真值。
5. 运行聚焦与全仓 Clippy，检查删除确认、敏感配置脱敏和本地无任意副作用。

## 验证

- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：退出码 0；集成测试可编译；动态目录无未标明来源的冲突项；失败来源显示独立错误。
- 所需证据：失败验证、Clippy 成功、来源合并快照、稳定 ID 请求记录、取消删除时零请求和敏感字段审查。

## 完成

完成时报告 integration loader、冲突优先级、动态命令形状、确认规则和测试名。未执行测试时标记 `已完成，待回归`。
