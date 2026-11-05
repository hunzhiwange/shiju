# macOS 桌面图标改名验收

2026-11-06（Asia/Shanghai）完成任务 91 的真实原生窗口验收。前置任务 90 已完成并释放 GUI；本轮独占操作隔离应用。仅验收 macOS，不据此声明 Linux/Windows 通过。

## 版本与隔离环境

- macOS 27.0（26A428），arm64；`vw-desktop` 0.6.0。
- 实测工作区提交 `1f8f5496e15272ef9fc74f4ce670f32cdfde1564`，改名实现提交 `81d38ee4a`，列表实现提交 `20e504469`。生产 Rust 源码无最终改动；本文件所在提交保存本轮验收证据。
- 实际应用为当前源码构建的 `target/rust-slot-2/debug/shi-ju`，临时 `.app` 仅以符号链接和独立 bundle ID `com.shiju.task91.acceptance` 使 CUA 能准确选中此进程。二进制 SHA-256 见 [evidence.json](evidence/task91-macos/evidence.json)。
- `VIBEWINDOW_CONFIG_DIR` 与 `VIBEWINDOW_CONFIG` 均指向新建临时目录，租户为 `task91-platform/task91-company`，只连接本机随机回环端口，使用独立临时 SQLite 和临时随机认证键。未操作用户生产公司或配置，证据不包含认证键。
- Iced → 生产 SDK → 生产 Axum Router → 原生图标业务 → 真实 SQLite。复用现有 `Server` 夹具初始化数据；临时传输中间件仅暂停 PATCH、计数和停启真实监听，不替代 handler、校验或数据库。夹具在窗口退出后移除，完整副本见 [传输控制夹具](evidence/task91-macos/transport-fixture.rs.txt)。
- “断网”是停止本次隔离网关的真实 TCP 监听，实际确认 `ConnectionRefusedError`；恢复时在原地址、原认证、原数据库重新监听，桌面进程没有重启。未切断机器全局网络或修改防火墙。

## 窗口操作的期望与实际

截图为 CUA 原生窗口原始输出，未重绘或合成。完整清单、SHA-256、命令及结构化期望/实际见 [evidence.json](evidence/task91-macos/evidence.json)；真实存储的 API 重读见 [api-observations.jsonl](evidence/task91-macos/api-observations.jsonl)。

| 场景与操作 | 期望 | 实际 | 截图 |
| --- | --- | --- | --- |
| 浅色打开改名 | 输入框自动聚焦；中文清晰 | 未点击输入框，直接全选粘贴中文成功；可见焦点边框与光标 | [初始焦点](evidence/task91-macos/light-modal-focused.jpg)、[中文草稿](evidence/task91-macos/light-chinese-draft.jpg) |
| 修改后取消 | 关闭弹窗，不发送 PATCH，不改存储 | 输入“取消不应保存”后点取消，PATCH 计数仍为 0，API 仍为“工作图标·中文验收” | [取消前](evidence/task91-macos/light-cancel-draft.jpg)、[取消后](evidence/task91-macos/light-cancel-result.jpg) |
| 中文保存 | SDK 改名成功，关闭并自动重读 | 提交“中文改名成功”，列表自动更新，API 重读一致 | [中文成功](evidence/task91-macos/light-chinese-success.jpg) |
| 保存中重复点击 | 输入、保存、取消不产生重复操作 | 真实 PATCH 在传输门闩暂停；反复点保存、按回车、点取消，弹窗仍为“保存中”，服务端 PATCH 计数保持 **1**；释放后只写入一次 | [保存中](evidence/task91-macos/light-saving-duplicate-blocked.jpg) |
| 64 个中文字符 | 后端接受，完整保存 | 粘贴 64 个“中”，保存成功，API 返回名称严格等于 64 个“中” | [64 字草稿](evidence/task91-macos/light-64-draft.jpg)、[64 字结果](evidence/task91-macos/light-64-success.jpg) |
| 65 个中文字符 | 后端拒绝，保留输入并显示错误 | 真正 PATCH 返回 **400 / invalid_input**，弹窗保留输入并显示错误、toast；API 仍严格为之前的 64 字名称 | [65 字错误](evidence/task91-macos/light-65-server-error.jpg) |
| 暗色中文保存 | 中文、焦点、按钮和背景有可读对比 | 切换应用 Dark 主题，在真实弹窗输入“暗色中文图标”并保存，列表与 API 一致 | [暗色弹窗](evidence/task91-macos/dark-chinese-draft.jpg)、[暗色结果](evidence/task91-macos/dark-chinese-success.jpg) |
| 桌面改名断网 | 失败且保留草稿，解除请求中状态 | 停止真实监听后保存“断网后保留草稿”，出现 **Connection refused (os error 61)** 错误和 toast，输入保留、保存可重试 | [断网草稿](evidence/task91-macos/dark-offline-draft.jpg)、[断网错误](evidence/task91-macos/dark-disconnected-error.jpg) |
| 桌面改名恢复 | 原窗口、原草稿可重试，自动重读 | 恢复前数据库仍为“暗色中文图标”；同地址恢复后点原弹窗保存，自动显示“断网后保留草稿”，再点刷新及 API 重读均一致 | [恢复保存](evidence/task91-macos/dark-reconnected-success.jpg)、[恢复刷新](evidence/task91-macos/dark-reconnected-refresh.jpg) |
| 桌面列表断网与恢复 | 刷新失败显示重试；恢复后可读列表 | 再次停止监听并点刷新，错误横幅及“重试”出现；恢复后在原窗口点重试，读取已保存中文名称 | [列表断网](evidence/task91-macos/dark-list-offline-error.jpg)、[列表恢复](evidence/task91-macos/dark-list-retry-recovered.jpg) |
| 关闭按钮 | 不提交当前草稿 | 输入“关闭按钮不保存”后点 X；API 名称未变，整个序列 PATCH 总计 **5**，关闭未增加请求 | [关闭后](evidence/task91-macos/dark-close-no-submit.jpg) |
| 中文字形与退出 | 实际渲染不 panic，正常退出 | 明暗主题、长中文、错误动态文本均实际渲染；应用持续运行至主动退出，退出码 **0**，服务端退出码 **0**，日志无 `panicked at` | [应用日志](evidence/task91-macos/app.log)、[真实 Router 日志](evidence/task91-macos/real-router.log) |

各次 API 重读还验证图标 ID、图片内容及创建时间始终不变。65 字错误和网络错误均未造成部分写入。传输计数 5 对应中文保存、64 字保存、65 字拒绝、暗色保存、断网恢复后的保存；连接拒绝未到达服务端。

## 命令与退出码

从槽位 1 开始逐个原子 `mkdir`，实际领取 **槽位 2**。所有 Cargo、临时服务器及桌面进程整个阶段持有该槽位；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，`CARGO_BUILD_BUILD_DIR` 同值。全部进程退出后已清理自己的锁，保留缓存。

| 命令 | 退出码与有效数量 |
| --- | --- |
| `cargo fmt --all` | 0；临时夹具格式化，退出后恢复原源码，无最终 Rust 差异 |
| `cargo build -p vw-desktop --bin shi-ju` | 0，当前桌面源码构建成功 |
| `cargo test -p vw-agent --lib cc_base_icon_task91_ui_server -- --ignored --nocapture` | 0，显式运行 1 个真实服务夹具，最终 1 passed；业务验收来自上述实际 GUI 操作 |
| 隔离环境下运行 `ShiJuTask91.app/Contents/MacOS/shi-ju` | 0，链接到当前槽位生产二进制 |
| `cargo test -p vw-desktop --lib cc_base_icon_desktop_rename_ -- --nocapture` | 0，**6 passed / 0 failed / 0 ignored**，8552 filtered out；恢复原源码后的最终运行 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0，恢复原源码后的最终运行，无 Clippy 警告 |

聚焦用例完整名见 [测试日志](evidence/task91-macos/focused-tests.log)，共同前缀为 `cc_base_icon_desktop_rename_`，后缀分别为 `modal_chinese_input`、`sdk_chinese_64_65_and_reread`、`failure_preserves_input_and_blocks_duplicate`、`context_change_never_writes_or_applies`、`success_rereads_and_stale_response_is_ignored`、`close_does_not_submit`。

构建和测试链接器仍有既有 `__eh_frame section too large` 提示；应用日志包含最小隔离 Router 对非图标端点返回 404，以及 macOS 临时 bundle 的 sandbox/window-move 提示。它们没有阻止本轮图标窗口验收，不能据此宣称整个应用或所有路由通过。未运行全量测试、wasm 或 Makefile。本轮桌面窗口使用 SQLite；PostgreSQL 的独立真实无窗口验收见 [macOS无窗口验收](macOS无窗口验收.md)，不冒充 PostgreSQL 桌面实测。
