# TOOLS.md - 本地笔记

技能定义工具 _如何_ 工作。这个文件记录的是 _你自己的_ 具体信息 —— 也就是只属于你这套环境的内容。

## 这里放什么

例如：

- 摄像头名称和位置
- SSH 主机和别名
- TTS 偏好的声音
- 扬声器或房间名称
- 设备昵称
- 任何与环境相关的特定信息

## 示例

```markdown
### 摄像头

- 客厅 → 主要区域，180° 广角
- 前门 → 入口处，动作触发

### SSH

- 家用服务器 → 192.168.1.100，用户：admin

### TTS

- 偏好声音："Nova"（温暖，略带英式口音）
- 默认扬声器：厨房 HomePod
```

## 为什么要分开？

技能是可共享的，而你的环境配置只属于你。把它们分开，意味着你可以在不丢失个人笔记的前提下更新技能，也能共享技能而不泄露你的基础设施信息。

---

把任何有助于你完成工作的内容都写进来。这就是你的速查表。

## Thread 工具

用户可见任务使用 `create_thread`、`fork_thread`、`list_threads`、`read_thread`、`send_message_to_thread`、`handoff_thread`、`set_thread_title`、`set_thread_pinned`、`set_thread_archived`、`wait_threads`。这些命令直接出现在工具目录中；`SessionTool` 仅供内部适配，不再向模型注册。

目标参数使用 `thread_id`，项目参数使用 `project_id`。`set_thread_title` 使用 `title`，`set_thread_pinned` 使用布尔 `pinned`，`set_thread_archived` 使用布尔 `archived`。未实现的跨主机交接由 `handoff_thread` 明确报错；不支持的参数也明确拒绝。

工具仅允许同租户、同项目、同规范化目录的可见任务，并拒绝当前任务、忙碌目标、未知 ID 和跨作用域目标。
跨任务用户消息显示“由 VibeWindow 从另一项任务发起”；内部来源标记在展示和复制前被剔除。
