# 系统通知（任务 29）

实现：权限拒绝和发送失败复用 `desktop_notification::failure_feedback`，保留现有 toast 的内容、种类和 ID；没有 toast 时显示错误。权限拒绝仍关闭通知开关。通知结果消息不修改对话完成结果；流完成入口已先更新运行状态，再批量调度通知任务。

依赖：任务 1 的入口证据见 acceptance-1.md；现有 `permission_task`、`completion_task` 和通用设置消息链已交付。macOS 使用 mac-usernotifications 0.3.1，Iced 0.14（清单声明）。系统权限拒绝返回错误，不伪装发送成功。

已新增独立行为测试，覆盖拒绝/发送失败保留成功 toast，以及无 toast 时显示错误；未执行。按本轮用户覆盖要求，没有运行 Cargo、单元测试、实机测试或最终回归，没有领取构建槽位，也没有设置 CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR。实现待未来统一验证，不宣称验证通过。
