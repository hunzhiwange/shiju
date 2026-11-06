# 会话恢复与撤销接入

日期：2026-11-06。依赖批次 26 已由提交 `23144ca61` 的 TRAE 当前授权接入及相邻供应商实现提供。

## 实现边界

- `vw-agent/session/llm/acp/controls.rs::prepare_access_restore` 将非提升偏好保存在 SessionRecord 的 desired 字段，独立于生命周期快照中的 current 回读。自动同步的完全访问值不成为下一轮恢复的偏好；在完全访问期间显式选择提升值也保留之前的非提升基线。
- `vw-acp/client/access_restore.rs` 复用供应商已存在的模式定义处理旧提升值。关闭授权后，request 重放和 controls 错误回滚都不能以保存值重新授予提升权限；模型、思考和计划选项继续沿原路径处理。Vibe 等没有安全自动撤销路径的模式继续明确报错。
- Codex、Claude、CodeBuddy、Qoder 的最终原生同步保留已恢复的已知非提升模式；已提升的旧基线使用供应商既有非提升模式。最终策略继续由现有 new/load/resume 与 prompt 前的 `sync_native_access` 应用，保留原生会话 ID 和历史。
- 缓存键继续包含租户、逻辑会话、目录与权限。`stop_incompatible_cached_clients` 在 prompt_lock 内成功关闭旧客户端后才移除缓存；关闭失败阻止替代进程启动，并保留可重试的旧项。
- 运行中切换继续遵循逻辑会话锁和现有取消路径；忙碌时明确要求当前轮结束后重试，不撤回已经发生的副作用。

## 验证范围

用户明确要求不运行测试、不索取账号或受控上游，因此未执行红绿测试或真实供应商验收，有效行为测试执行数为 0。本说明是源码接入和静态检查记录，不新增 E3 声明。

静态检查使用仓库动态领取的槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`：

- `cargo fmt --all`：退出码 0；仅保留本任务源码差异。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出码 0（编译所有 targets/features，不执行测试）。
- `git diff --check`（仅本任务暂存文件）：退出码 0。

槽位阶段结束后释放锁，保留构建缓存。实机开启、关闭、恢复及失败重试的工具效果未测；后续可按需要进行实际会话回归。
