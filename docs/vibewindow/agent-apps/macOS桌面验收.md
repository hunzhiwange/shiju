# macOS 桌面底座验收

2026-11-06（Asia/Shanghai），任务 93 交接审查通过。仅验收本计划的原生图标最小闭环；完整 CC 迁移与母任务 2 仍未完成。此前锁屏阻塞已由任务 90/91 的真实窗口回归解除。本轮只读取既有证据，未启动、构建程序或调用 Cargo。

## 证据及版本绑定

| 来源 | 实测版本、提交与证据 |
| --- | --- |
| 90 列表窗口 | macOS arm64、vw-desktop 0.6.0；工作区 `1f8f5496e15272ef9fc74f4ce670f32cdfde1564`，列表实现 `20e504469`。原会话 `01a1accc-941c-7cd0-b823-d8ec807950bd` 的 8 张截图、日志及记录在本次审查中逐字节归档至 [task90-macos](evidence/task90-macos/evidence.json)，[归档 SHA-256](evidence/task90-macos/archive-sha256.json) 在归档时生成，不冒充拍摄时签名。原记录未保存二进制哈希，不能用今天共享槽位的产物倒推。 |
| 91 改名窗口 | macOS 27.0（26A428）arm64、0.6.0；同一实测工作区提交，改名实现 `81d38ee4a`；证据提交 `747adf107`。详见 [原始验收报告](macOS桌面改名验收.md) 及 [evidence.json](evidence/task91-macos/evidence.json)，包含生产二进制 SHA-256、19 张原始截图哈希、API 重读及退出码。 |
| 92 双数据库无窗口 | 提交 `9fc7613a6`；[正式报告](macOS无窗口验收.md) 记录真实 SDK→HTTP Router→原生业务→SQLite/PostgreSQL 各 3 项，共 6 项通过。无窗口证据与窗口证据分开使用。 |

90/91 使用动态领取的槽位 2，绝对 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，`CARGO_BUILD_BUILD_DIR` 同值；均持锁到窗口及服务退出后释放。临时独立 bundle、隔离配置/租户、回环监听和真实临时 SQLite，不访问生产公司。临时服务夹具使用生产 Router/业务/存储，仅控制监听与传输时序，退出后已移除。核对实测提交到证据提交的 `crates/` 差异为空，未发现其间改变渲染行为的源码变更。

## 逐项审查

下表“实际”来自原桌面会话的实测记录、截图及 API 观测，本轮没有新增实测。

| 场景 | 期望与实际核对 | 证据 |
| --- | --- | --- |
| 列表刷新、中文与缩略图 | 读取真实库并显示中文；90 明暗主题下显示中文与缩略图，API 删除返回 204 后刷新为空库 | [浅色中文](evidence/task90-macos/light-chinese.jpg)、[暗色中文](evidence/task90-macos/dark-chinese.jpg)、[暗色空库](evidence/task90-macos/dark-empty.jpg) |
| 空库恢复 | 失败后可重新读取空集合；90 恢复真实服务并重试后显示“暂无图标” | [恢复空库](evidence/task90-macos/light-empty-recovered.jpg)、[恢复服务日志](evidence/task90-macos/recovery-sqlite-router.log) |
| 列表断网、失败与重试 | 停止隔离监听后报连接拒绝并出现重试；90 两主题实际点击，91 原窗口恢复后读取已保存中文名称 | [浅色失败](evidence/task90-macos/light-failure.jpg)、[浅色重试](evidence/task90-macos/light-retry.jpg)、[暗色失败](evidence/task90-macos/dark-failure.jpg)、[暗色重试](evidence/task90-macos/dark-retry.jpg)、[91恢复](evidence/task91-macos/dark-list-retry-recovered.jpg) |
| 中文改名、焦点与明暗主题 | 打开后直接粘贴，保存关闭并自动重读；91 两主题列表与 API 名称一致，中文可读 | [初始焦点](evidence/task91-macos/light-modal-focused.jpg)、[浅色成功](evidence/task91-macos/light-chinese-success.jpg)、[暗色成功](evidence/task91-macos/dark-chinese-success.jpg) |
| 取消及关闭 | 草稿不写入；91 取消后 PATCH 为 0，X 关闭不增加总计 5 次 PATCH，API 名称不变 | [取消草稿](evidence/task91-macos/light-cancel-draft.jpg)、[取消后](evidence/task91-macos/light-cancel-result.jpg)、[关闭后](evidence/task91-macos/dark-close-no-submit.jpg)、[API观测](evidence/task91-macos/api-observations.jsonl) |
| 重复提交 | 请求中禁止重复保存/回车/取消；暂停真实 PATCH 后服务计数始终为 1，释放后仅一次写入 | [保存中](evidence/task91-macos/light-saving-duplicate-blocked.jpg)、[传输夹具](evidence/task91-macos/transport-fixture.rs.txt) |
| 64/65 中文边界 | 64 字完整保存，65 字服务端拒绝且保留草稿；API 确认旧 64 字值、ID、图片、时间不变 | [64字](evidence/task91-macos/light-64-success.jpg)、[65字错误](evidence/task91-macos/light-65-server-error.jpg)、[API观测](evidence/task91-macos/api-observations.jsonl) |
| 改名断网与恢复 | 停止真实监听后 Connection refused、toast、草稿保留；同地址恢复后原窗口原草稿重试成功，自动重读及手动刷新一致 | [断网错误](evidence/task91-macos/dark-disconnected-error.jpg)、[恢复保存](evidence/task91-macos/dark-reconnected-success.jpg)、[恢复刷新](evidence/task91-macos/dark-reconnected-refresh.jpg) |
| 实际渲染无 panic | 中文、长中文、错误动态文本实际进入 Iced 渲染；90/91 应用和服务主动退出均为 0，91 应用日志无 `panicked at` | [90退出记录](evidence/task90-macos/evidence.json)、[91退出记录](evidence/task91-macos/evidence.json)、[应用日志](evidence/task91-macos/app.log) |

“断网”专指关闭本轮隔离网关的真实 TCP 监听，不代表切断整机网络。91 的 API 观测证明取消未写、65 字拒绝未写、断网未写，恢复后持久化一致；不将截图本身当数据库证明。GUI 使用 SQLite；PostgreSQL 只验收了 92 的真实无窗口链路，没有 PostgreSQL 桌面实测，也没有 Linux/Windows 验收。

## 验证审查结果

90 的 `cargo test -p vw-desktop --lib cc_base_icon_desktop_list_ -- --nocapture` 为 4 passed / 0 ignored，构建、Clippy 均退出 0；原始日志已归档。91 同类 `cc_base_icon_desktop_rename_` 为 6 passed / 0 failed / 0 ignored，构建、Clippy 均退出 0；92 `cc_base_icon_headless_` 为双库共 6 passed / 0 ignored，Clippy 退出 0。以上是前置运行证据，本轮未重跑 Cargo。链接器既有 `__eh_frame` 提示及隔离 Router 非图标端点 404 的限制保持原报告说明，不推广为整个应用通过。

本轮文档审查：19 张 91 截图 SHA-256 全部匹配；90 原始 8 图及 6 份记录/日志逐字节归档；API 观测的 8 个阶段名称、64 字长度、拒绝/断网/关闭前后不变及身份字段一致均通过；本任务新增文档链接与差异空白检查通过（退出 0）。底座需求与未迁移规则的唯一落点见 [底座交付映射](底座交付映射.md)。

归档日志保留原始字节及末尾空行：全体暂存文件 `git diff --cached --check` 退出 2，仅 focused-tests.log、real-sqlite-router.log、recovery-sqlite-router.log 报原始 EOF 空行；两份本任务 Markdown 定向检查退出 0。保留原证据而不改写日志；仅忽略 EOF 空行的全体检查 `git -c core.whitespace=-blank-at-eof diff --cached --check` 退出 0。
