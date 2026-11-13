# 定时任务配置与验收

## 配置步骤

1. 打开系统设置的定时任务页，选择新建任务；编辑已有任务使用同一弹窗。
2. 填写名称、提示词及所需的目录、模型或 Agent。目录是执行上下文，不代表云端运行来源。
3. 选择单次、固定间隔、每天、每周、每双周、每月、每年，或高级 Cron。每周和双周至少选择一个星期；每月至少选择一个日期。
4. 检查时区。新建默认 UTC，不会推断操作系统时区；例如选择 Asia/Shanghai 后，当地 09:30 对应 UTC 01:30。单次使用日期和时间选择器；确认才写回草稿，取消保留原值。
5. 循环任务可选长期或起止日期。检查规则摘要、有效期和未来最多五次触发，再保存。预览不创建任务或运行；没有窗口内触发点时拒绝保存。
6. 取消弹窗不写入任务；保存错误保留草稿，可修正后重试。编辑后重新打开应保留规则、时区和有效期。

## 时间语义

| 类型 | 规则及边界 |
| --- | --- |
| 单次 | 当地时间必须在未来。DST 不存在或有歧义的当地时间直接报错；使用明确时区和有效时间重新配置。存储为 UTC 绝对时刻，重新打开可能显示 UTC。 |
| 固定间隔 | 数量加秒/分/时/天，按经过的时间重新调度；1 天为 24 小时。运行及重新调度耗时可能改变后续相位，不保证墙钟固定时间。 |
| 每天 | 在所选时区的固定日历时间触发，与每隔 24 小时有区别。 |
| 每周 | 所选多个星期的当地时间触发，星期至少选一个。 |
| 每双周 | 锚点所在当地周的周一为第零周，此后隔周有效；活动周内按多选星期触发。跨年保持日历周相位，不按 ISO 周号奇偶计算，也不等同 14×24 小时。 |
| 每月 | 多选 1–31 日；短月缺少的日期跳过，不顺延到月底。 |
| 每年 | 固定月日；2 月 29 日只在闰年触发，2 月 30 日等永不存在日期拒绝保存。 |
| 高级 Cron | 保留不能无损映射为普通规则的表达式。五字段为分、时、日、月、周；六字段前加秒，七字段再加年。网关保存后的零秒六字段仍可回读为普通日历规则；非零秒、带年限制和分钟步长保留高级模式。 |

Cron 分钟步长每小时重新起算：`*/40 * * * *` 在每小时 00、40 分触发；00:50 后的下一次为 01:00。固定间隔 40 分钟从 00:50 计算为 01:30。它们不能互换。

循环 Cron 使用实际 chrono/cron 库处理 DST；不要承诺重复小时一定执行一次。双周按当地日历寻找候选，缺失时间跳过，歧义候选按 UTC 先后选择。预览和运行共用底层计算；跨 DST 的计划应检查预览结果。固定间隔不随当地时钟调整。

有效期包含界面所选起止日期，存储为 UTC 半开窗口 `[starts_at, ends_at)`，截止日转换为当地次日零点。结束时不启动新运行，也不取消已经启动的运行；离线后的到期扫描同样受窗口约束。单次不额外显示有效期。

立即运行不改变原计划；不在有效期内的手动运行会被拒绝，需先编辑有效期。列表可启停、筛选、批量管理并查看历史；输出中的会话标记不等于完整产物时间线。

## 权限、投递与在线要求

任务的“完全访问”默认关闭，执行仍受运行环境的权限配置约束。开启完全访问必须显式确认并保存；取消确认保留受限权限，取消整个表单不保存权限变化。不同执行器可能等待确认或在后台拒绝；ACP 非交互请求默认拒绝额外未授权操作。运行错误应在历史中查看；本期不提供持久化人工问答、重启后恢复审批或“无人在线必定完成敏感操作”的保证。

投递选择已配置的真实通道及目标。本期已验收的桌面投递通道为 telegram、discord、slack、mattermost、qq、email。未配置、凭证不可用、未知通道、缺少目标均应显示明确错误。best_effort 为 false 时投递失败使运行记为错误；为 true 时保留任务执行成功并记录投递失败。桌面 Toast 是本地反馈，不等于系统通知或手机推送。

关闭桌面窗口后是否继续调度取决于 daemon 是否仍在运行。离线执行必须保持机器及 daemon 在线；不能在关机后执行，依赖网络的模型和投递还需要网络可用。本说明不承诺云端沙箱、模板市场、手机端、技能/连接器独立选择或新的离线追赶策略；企业微信 Cron 投递属于独立改动，不在本任务验收范围。

## 回归清单与证据

自动化和实际窗口验收分开记录；仅构建 Element 不算字体整形或窗口验收。每项必须有本次运行证据才可标为通过。

| 场景 | 自动化入口 | 窗口检查 |
| --- | --- | --- |
| 七类规则、周/月多选、UTC/上海、有效期、编辑往返、到期判定 | `wb_schedule_acceptance_roundtrips_window_and_timezone`；`wb_schedule_api_`；`wb_schedule_draft_` | 新建、编辑、预览摘要 |
| 双周跨年、多星期、DST | `wb_schedule_biweekly_` | 锚点、星期显示 |
| 月末、闰日、Cron 步长 | `wb_schedule_acceptance_month_end_and_leap_day_skip_missing_dates`；`wb_schedule_acceptance_cron_step_differs_from_elapsed_interval` | 日期选择及说明 |
| DST 错误与草稿保留 | `wb_schedule_api_`；`wb_schedule_modal_`；`wb_schedule_picker_` | 错误后字段保留 |
| 立即运行、过期/未开始拒绝、排队过期 | `wb_schedule_acceptance_manual_cron_run_preserves_next_run`；`wb_schedule_window_`；`cron_` | Toast 和历史 |
| 取消、重复保存、陈旧预览 | `wb_schedule_draft_`；`wb_schedule_modal_` | 取消不写入 |
| 默认权限、完全访问确认、投递失败 | `wb_schedule_feedback_` | 受限说明、确认、投递目标及错误 |
| 中文、动态摘要、明暗主题、窄窗口 | `wb_schedule_modal_renders_all_rules_and_fixed_actions`；`wb_schedule_feedback_renders_permissions_and_delivery`（真实字体渲染） | 另需启动 shi-ju 实际窗口，检查 4px 滚动条、两侧 7px 留白及固定标题/按钮 |

2026-11-13 回归使用动态槽位 5，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-5`，`CARGO_BUILD_BUILD_DIR` 同值。所有 Cargo 命令继承此环境，缓存保留。

| 命令 | 退出码 / 有效测试数 |
| --- | --- |
| `cargo test -p vw-agent --lib wb_schedule_acceptance_ --all-features` | 0 / 4 通过 |
| `cargo test -p vw-agent --lib wb_schedule_ --all-features` | 0 / 64 通过，含双周 12 项 |
| `cargo test -p vw-agent --lib cron_ --all-features` | 0 / 127 通过 |
| `cargo test -p vw-gateway-client --lib wb_schedule_` | 0 / 7 通过 |
| `cargo test -p vw-gateway-client --lib cron_` | 0 / 14 通过 |
| `cargo test -p vw-desktop --bin shi-ju wb_schedule_`、`cargo test -p vw-desktop --bin shi-ju cron_` | 各 0 / 0，不算行为验收；测试已迁入 library |
| `cargo test -p vw-desktop --lib wb_schedule_acceptance_` | 0 / 2 通过 |
| `cargo test -p vw-desktop --lib wb_schedule_` | 0 / 64 通过 |
| `cargo test -p vw-desktop --lib cron_` | 0 / 161 通过 |
| `cargo fmt --all` | 0，已检查任务差异 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0，静态检查无测试数 |

失败证据与调整：首轮 Agent 编译因并发企微代码访问私有 `runtime_home` 返回 101、0 项执行，该问题已由所属改动消除。六字段首次失败验证另被并发 `delivery.rs` 宏歧义阻塞（101、0 项，`/tmp/shiju-cron9-normalization-red.log`），不算行为红灯；所属改动修复后新增两项验收通过。Agent 周边回归曾 123 通过 / 3 失败，修正三个审批夹具，显式开启其预期测试的中风险审批选项。桌面周边曾 152 通过 / 3 失败，修正旧页内新增表单夹具，使其使用当前共用表单和新增弹窗矩阵；保留原有几何、取消及固定按钮断言。

真实字体渲染已生成 80 张图片于 `/tmp/shiju-cron9-render`，覆盖规则、新增/编辑、明暗、宽/窄、权限投递和滚动后固定按钮。抽查路径：`rule-4-edit-dark-narrow.png`、`rule-6-add-light-wide.png`、`feedback-false-dark-380.png`、`scrolled-edit-light-narrow.png`。渲染夹具的预览数据仅用于布局，时间结果以真实 chrono/SQLite 测试为准。

实际窗口使用隔离的本地 HTTP 夹具，执行、保存和投递均被拒绝，不连接模型或外部通知。按要求以 `cargo run -p vw-desktop --bin shi-ju` 启动后，CUA 无法绑定裸可执行文件，改用同一槽位产物的临时 `.app` 外壳，配置与工作目录位于 `/tmp/shiju-cron9-main`。裸进程主动退出码为 143，不记为成功运行退出码。

窗口已观察中文、明暗主题、新建/编辑、约 935×685 逻辑尺寸、滚动内容和固定标题/预览/按钮。编辑改名后取消，列表保留旧值；保存返回 400 时 Toast 和错误可见，中文名称及命令草稿仍保留。窗口截图随本次验收工具记录保存；矩阵另有字体渲染 PNG，时间语义由 chrono/SQLite 测试证明。未实测真实模型调用、外部通知送达或长时间 daemon 离线恢复。

必要修复：窗口暴露列表硬编码 UTC 和双周空标签；新增卡片渲染测试先失败，再最小修复并通过。真实网关零秒六字段也会被旧草稿识别降为高级模式，现已补齐普通规则回读；非零秒、年份限制和分钟步长继续保留高级模式。两主题卡片截图为 `/tmp/shiju-cron9-render/wb-acceptance-card-zones-Light.png`、`/tmp/shiju-cron9-render/wb-acceptance-card-zones-Dark.png`。

并发企微能力新增期间，旧反馈测试曾把 wecom 当作不支持的通道，导致 wb 过滤 62 通过 / 2 失败，cron 过滤 160 通过 / 1 失败。所属改动修正夹具后，本任务重跑得到 wb 64 项、cron 161 项全部通过；本任务未提交企微相关更改。最终日志为 `/tmp/shiju-cron9-desktop-wb-confirmed.log`、`/tmp/shiju-cron9-desktop-cron-confirmed.log`。

最终 `cargo build -p vw-desktop --bin shi-ju` 退出 0（`/tmp/shiju-cron9-native-final-build.log`）。最新产物窗口再次确认每日规则、双周摘要、上海时区、2096-12-24 至 2097-01-31 有效日期和中文 Agent 提示词均正确显示，取消不写入；验收后退出窗口并停止隔离夹具。运行日志保留为 `/tmp/shiju-cron9-*.log`，最终 Clippy 记录为 `/tmp/shiju-cron9-clippy-confirmed.log`。
