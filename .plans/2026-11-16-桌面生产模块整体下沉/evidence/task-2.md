# 任务2阶段证据（未完成）

本次只完成发送准备入口下沉，**未满足任务2完整验收，不删除任务卡**。

## 已落实调用链

宿主 `send_current_session_input_checked` 保留来源保存/历史加载拦截与跨域装配，调用 `desktop_session::message::chat::send_input::prepare_send_input`。领域复用 `SessionRuntimeState<R>`、`QueueItem`、工具选择器与既有内容组合函数；返回 `SendInputEffect`，由宿主映射任务看板、输入编辑器镜像与队列派发。未增加依赖、未引用 App、未反向加载宿主源码。

已迁出：空输入判断、内联模式转换、手动上下文消费、附件消费、任务模式选项投影、模型/ACP来源选择、委派默认值、ACP重建判断、历史数量钳制、工作流/目标模式子会话限制、完整 QueueItem 构造。

允许工具计算仍属于宿主既有工具库存接口；在运行状态副本清除手动上下文后计算，保持原先计算发生于清除之后的顺序语义。

## 断言映射

`apply_inline_mode_command_toggles_task_mode_and_clears_runtime_editor` 整个用例迁入 `host_module_2_tests`，三项原断言原样保留：任务模式开启、编辑器清空、聊天模式关闭任务开关。夹具从 App 改为真实 `SessionRuntimeState<()>`。其余来源测试未删，尚未逐项完成归属迁移。host_split 的全面逐断言去重尚未完成。

## 尚未完成的明确范围

- `start`：项目身份解析、PendingSessionSend、草稿创建、重试与权限队列、请求生命周期启动、持久化装配尚未整体下沉。
- `update`：会话偏好、工具选择、异步历史/分叉身份、完成转换仍需分离领域转换和宿主效果。
- 提交消息生成与 Git handoff 尚未改为完整具名领域效果。
- `send_to_existing_session`、历史重置、压缩及其测试尚未完成归属核对。
- 来源整组测试和每个宿主残留函数的保留理由审计尚未完成；当前残留不能统称必要装配。

这些是剩余实施工作，不是外部阻塞，也不归入以后统一回归。

## 行数（pathlib splitlines，同基线口径）

| 来源 | 基线 | 当前 | 宿主减少 |
| --- | ---: | ---: | ---: |
| `crates/vw-desktop/src/app/message/chat/session.rs` | 1896 | 1802 | 94 |
| `crates/vw-desktop/src/app/message/chat/session_tests.rs` | 2431 | 2414 | 17 |

无新增宿主替代文件。生产候选减少与测试减少分别按上述两个来源计算；不得将此阶段数字当完整领域迁出验收。

## 静态检查与限制

独立 rustfmt 解析新领域实现和迁移测试；`git diff --check` 无输出。已核对 lib.rs 的生产及 cfg(test) 注册、宿主真实调用及新模块依赖方向。用户禁测：未执行任何 Cargo 命令、单元测试、运行或绘制，不能声称编译或行为通过。

以后另行授权的定向命令：`python3 scripts/cargo_slot.py cargo test -p desktop-session --lib host_module_2_tests -- --test-threads=1`；宿主检查 `python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`。此阶段只有一个迁移用例，整组覆盖尚不完整。
