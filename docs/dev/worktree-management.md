# 工作树管理与环境

工作树策略由当前连接的网关按租户持久保存，目录属于网关宿主；不是桌面所在机器的任意远程目录选择器。空根目录使用 `task_worktrees_dir`（发布构建为网关用户的 `~/.shiju-harness/task-worktrees`，调试构建为 `~/.shiju-harness-dev/task-worktrees`），自定义根必须是绝对路径。

常规设置包括根目录、创建前获取更新（默认关闭）、自动清理（默认开启）、保留数量（默认 15，大于零）、删除前询问（默认开启）。获取更新失败会阻止创建，不自动合并或重置分支。开启询问时自动清理仅提出建议。

环境仅编辑所选项目的 `commands.start` 单条初始化命令，保存不会执行。创建时先运行项目命令，再运行本次额外初始化；失败工作树保留且不可新聊，可显式重试。成功的步骤不会因重试重复执行。

托管列表按项目分组，可在同一工作树新建多个聊天。主工作区、外部工作树、任务池、置顶聊天关联树、运行中或初始化中的树受保护。归属不明或状态读取失败时拒绝删除；第三方树不会自动接管。

删除先进行预检，确认绑定当时的设置和内容；内容变化、过期或新增保护要求重新预检。删除前持久保存索引、工作区和未跟踪文件；忽略文件不纳入快照，需要单独确认损失。不支持的仓库状态返回明确错误。聊天历史保留。

在“清理建议与恢复快照”中恢复已删除树。恢复创建新的托管路径，原分支或路径被占用时使用安全的新名称；不覆盖现存目录。恢复记录在重启后仍有效，成功的快照不能重复恢复。

## 验收

在任务约定的动态 Cargo 槽位中执行：

```sh
cargo test -p vw-agent worktree_management_e2e_ -- --nocapture
WORKTREE_WINDOW_OUTPUT=/tmp/worktree-window cargo test -p vw-desktop --test worktree-management-window -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

原生窗口复用生产设置组件和字体，自动采集常规、环境、删除、窄窗口策略弹窗的明暗主题截图并关闭。人工检查截图中的 4px 滚动条与两侧 7px 留白、固定弹窗头尾、中文路径、按钮状态和 tooltip；截图不能替代网关端到端验证。

原生验收以异步任务等待窗口稳定，再直接请求截图；不要订阅 `window::frames` 并逐帧更新应用状态来等待截图。后者反复重建布局，可能使 Iced 编辑器绘制缓冲的弱引用失效，出现“内容存在、截图空白”的验收假象。环境场景同时检查实际命令内容和编辑区文字像素，防止空白截图被当作通过。

需要人工交互时可设置 `WORKTREE_WINDOW_SINGLE_CASE=4 WORKTREE_WINDOW_CAPTURE_MS=45000`，只打开恢复区场景、保留 45 秒后截图并自动关闭；编号 0–4 为明主题，5–9 为对应暗主题。仅用于显式验收，不连接网关或写入用户配置。

## 2026-11-13 验证证据

| 验证 | 结果 | 日志或截图 |
| --- | --- | --- |
| `cargo test -p vw-agent worktree_management_e2e_ -- --nocapture` | 退出 0；主流程 1 passed，新进程恢复辅助断言通过；辅助测试只由父流程调用，顶层 ignored 1 | `/tmp/shiju-task14-agent-final.log` |
| `cargo test -p vw-desktop --test worktree-management-window -- --nocapture` | 退出 0；10 个明暗主题原生场景，无字形 panic；环境文字像素分别 2630、2171 | `/tmp/shiju-task14-accepted-window.log`、`/tmp/shiju-task14-accepted/` |
| `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0 | `/tmp/shiju-task14-accepted-clippy.log` |
| `cargo fmt --all`、`git diff --check` | 均退出 0 | `/tmp/shiju-task14-accepted-fmt.log` |

后端验证领取槽位 3，最终窗口与 Clippy 领取槽位 1；两者分别设置绝对路径 `CARGO_TARGET_DIR=$PWD/target/rust-slot-3`、`$PWD/target/rust-slot-1`，并设置 `CARGO_BUILD_BUILD_DIR=$CARGO_TARGET_DIR/build`。诊断阶段还动态使用槽位 5、8、9；结束后仅清理本会话锁，保留缓存。

失败证据为旧逐帧入口的环境文字像素 0（`/tmp/shiju-task14-check-ink.py` 检查退出 1）；修正截图驱动后同一区域检查通过，最终入口内置同样的文字像素断言。没有修改生产编辑器或工作树生命周期。

原生交互检查通过：滚动到恢复区，异常工作树的新聊与删除按钮呈禁用态；悬停删除按钮显示“删除前先检查保护状态”；窄策略弹窗显示输入框焦点边框、固定标题及保存/取消按钮。两主题截图已检查中文路径、长标题、二进制修改提示、4px 滚动条及内容留白。macOS 大型 agent 测试二进制存在 `__eh_frame section too large` 链接器提示，未改变测试成功结果；Clippy 无警告退出。
