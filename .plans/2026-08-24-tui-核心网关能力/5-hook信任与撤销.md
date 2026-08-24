# 任务 5: 接通 Hook 信任与撤销

批次：【批次 5】 依赖批次：批次 4

状态：未开始

目的：允许用户对当前 Gateway Hook 目录中的稳定声明执行确认后的信任或撤销。

来源任务：无

预计会话范围：增加两项安全敏感 mutation、确认层和刷新；不执行 Hook、不编辑 Hook 配置。

## 文件

- 修改：`crates/vw-cli/src/cli/tui/commands.rs`
- 修改：`crates/vw-cli/src/cli/tui/app.rs`
- 修改：`crates/vw-cli/src/cli/tui/model/overlay.rs`
- 修改：`crates/vw-cli/src/cli/tui/runtime/hooks.rs`
- 测试：`crates/vw-cli/src/cli/tui/command_tests.rs`
- 测试：`crates/vw-cli/src/cli/tui/integration_tests.rs`

## 上下文

- 调用 `hook_trust(plugin_id, hook_hash)` 与 `hook_revoke(plugin_id, hook_hash)`。
- 只接受任务 4 当前 catalog 中精确匹配的 plugin ID/hash，不接受自由构造的外部 hash。
- 两种操作都先展示声明摘要和 hash 确认；响应后重新加载 catalog/audit，失败不乐观更新。

## 测试计划

- 行为：`/hooks trust|revoke <plugin-id> <hash>` 只控制当前快照项，确认后调用 Gateway，取消或失败不改本地状态。
- 失败验证测试：新增 `hook_mutations_require_catalog_membership_and_confirmation`、`hook_mutation_refreshes_only_after_gateway_success`。
- 失败验证命令：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期失败原因：确认 intent、mutation runtime 与命令语法尚不存在。
- 通过验证命令：`cargo clippy --all-targets --all-features -- -D warnings`
- 模拟策略：HTTP fixture 记录真实 POST body 与响应；不模拟确认状态机。

## 步骤

1. 添加非目录 ID/hash、取消、成功和 Gateway 失败测试。
2. 运行失败验证命令并确认缺失 Hook mutation 路径。
3. 新增显式 confirm intent 和 runtime mutation，严格绑定当前 catalog。
4. 成功后刷新目录与审计，失败只写错误 transcript。
5. 运行通过验证命令并审查无跳过确认或乐观授权。

## 验证

- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：退出码 0；测试编译；无 warning。
- 所需证据：失败/通过证据、精确 POST 路径与 body、确认取消零请求、失败不更新快照。

## 完成

仅当信任与撤销都要求当前目录稳定凭据和用户确认，并以 Gateway 成功作为唯一提交条件后，任务可完成。
