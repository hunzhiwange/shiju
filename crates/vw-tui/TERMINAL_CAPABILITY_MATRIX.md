# 终端能力验收矩阵（2026-11-16）

清理说明（2026-11-16）：维护者随后授权删除 `vw-tui/src` 未激活旧源码；表内“参考保留”“恢复源码”描述清理前状态，不代表这些源码仍在当前目录。需要时按 `UPSTREAM.md` 指定 Git 版本恢复；能力接线状态不因删除参考文件而改变。

对应 `.plans/2026-11-16-终端能力对接/requirements.md` 的全部 38 项。提交列为实现来源，测试列为当前资产而非通过证据。任务 12 新增 6 个 `terminal_req_12_` 用例，由 cli-interaction/lib.rs 的 cfg(test) 接线。

用户本次明确要求只写代码，禁止 Cargo、单元测试、实机和最终回归；因此本表所有行为证据均待执行，没有红灯、通过数量或运行退出码。本轮仅静态阅读，运行平台证据为空；本机路径不构成 macOS 行为验证。Linux/Windows/WSL、SSH/tmux、IME、剪贴板、外部编辑器以及 inline/alternate 均未实机验收。

调用链：Crossterm Event → apply_ready_terminal_events → ChatWidget → ChatComposer → V3Action → V3GatewayApp::handle_action → 附件校验/控制器 → request_for_command。新用例真实构造请求但不连接 Gateway；队列用例验证真实 FIFO 取出与请求组装，自动流结束派发和网络失败仍需执行现有 runner 测试，不能用该用例声称自动发送已通过。

| ID | 能力 | 实现来源 | 当前测试/材料 | 实际边界 | 状态/限制 |
| --- | --- | --- | --- | --- | --- |
| 01 | 括号多行粘贴 | 01ee328f4 | cli-interaction terminal_req_02_/12_ | terminal → widget → composer | 接线；待验 |
| 02 | 快速键流粘贴 | c0235038d | tui-composer terminal_req_01_ | terminal → composer → PasteBurst | 接线；待验 |
| 03 | 无人输入到期刷新 | 01ee328f4 | cli-interaction terminal_req_02_ | event loop deadline → flush | 接线；待验 |
| 04 | 粘贴 Enter 保护 | c0235038d | tui-composer terminal_req_01_ | composer → PasteBurst | 接线；待验 |
| 05 | 中文与 emoji | c0235038d | terminal_req_01_/02_/12_ | composer → textarea | 接线；IME 实机待验 |
| 06 | AltGr 快捷键 | 4a1bb415e | tui-chat/clipboard terminal_req_03_ | widget → clipboard key classifier | 接线；平台待验 |
| 07 | 长文本折叠 | 7b669e8b5 | tui-composer terminal_req_04_；terminal_req_12_ | composer → expanded submission → request | 接线；待验 |
| 08 | 粘贴原文空白 | 7b669e8b5 | tui-composer terminal_req_04_；terminal_req_12_ | composer → request structured content | 接线；待验 |
| 09 | 原子占位符编辑 | 7b669e8b5 | tui-composer terminal_req_04_ | composer → textarea elements | 接线；待验 |
| 10 | 系统图片粘贴 | 4a1bb415e | tui-chat terminal_req_03_ | widget → clipboard → attachment | 接线；真实剪贴板待验 |
| 11 | 图片 Gateway 请求 | 既有入口 | cli-interaction attachment_tests；terminal_req_12_ | app → attachment validation → request | 接线；待验 |
| 12 | 图片资源上限 | 4a1bb415e | tui-clipboard terminal_req_03_ | clipboard dimensions → PNG encoder | 接线；待验 |
| 13 | 剪贴板错误恢复 | 4a1bb415e | tui-chat terminal_req_03_ | widget error event；draft retained | 接线；平台待验 |
| 14 | WSL 图片 | 未引入外部命令 | tui-clipboard clipboard_image | arboard only | 明确无 WSL 回退；实机未知 |
| 15 | 图片文件路径 | a3a1f6504 | cli-interaction terminal_req_06_/12_ | paste → widget path candidate → request | 接线；拖放实机待验 |
| 16 | 引号与 file URI | a3a1f6504 | cli-interaction terminal_req_06_ | widget normalize → attachment validation | 接线；待验 |
| 17 | 工作区边界 | 既有校验；a3a1f6504 | cli-interaction terminal_req_06_/12_ | model canonicalize → root boundary | 接线；待验 |
| 18 | 上下历史导航 | 29af7ca36 | tui-composer terminal_req_05_ | composer → local history | 接线；待验 |
| 19 | 富历史恢复 | 29af7ca36 | tui-composer terminal_req_05_ | history snapshot → composer | 接线；待验 |
| 20 | Ctrl-R 搜索 | c253fd7e9 | tui-composer terminal_req_08_ | composer → history search | 接线；待验 |
| 21 | 两类历史分离 | 既有入口 | cli-interaction session_restore_tests | sessions → Gateway；composer local history | 不新增输入历史存储；待验 |
| 22 | 运行中排队 | 29af7ca36 | tui-chat input_queue_tests；terminal_req_12_ | composer Queue → runner → widget FIFO | 接线；自动派发待验 |
| 23 | 重编队尾 | 29af7ca36 | tui-chat input_queue_tests | Alt-Up/Shift-Left → restore_submission | 接线；待验 |
| 24 | 失败提交恢复 | 29af7ca36 | attachment_tests；runner_tests；terminal_req_12_ | runner validation/network error → restore_submission | 接线；待验 |
| 25 | 外部编辑器 | 59def70d1 | cli-interaction terminal_req_07_ | runner_panels → editor → runner_host | 接线；真实编辑器待验 |
| 26 | 编辑器终端恢复 | 59def70d1 | cli-interaction terminal_req_07_ | terminal lifecycle suspend/resume | 接线；inline/alternate 实机待验 |
| 27 | Vim 设置 | 48155a028 | cli-interaction terminal_req_09_ | settings → widget → composer → textarea | 显式设置接线；待验 |
| 28 | 自定义键位 | 未增加配置系统 | tui-input keymap | RuntimeKeymap core | 参考保留；无完整产品配置入口 |
| 29 | @文件搜索 | a3a1f6504；既有 runner | runner_tests file_search_discards_stale_results_and_applies_latest_query | composer action → runner → latest result | 接线；待验 |
| 30 | pager 与终端历史 | 01ee328f4；59def70d1 | terminal_req_02_；vw-tui vt100_ | terminal pager branch → transcript | 接线；resize 实机待验 |
| 31 | SSH/OSC52 复制 | 744270411 | tui-clipboard terminal_req_10_；clipboard_copy_tests | copy_to_clipboard → environment backend | 接线；SSH 实机待验 |
| 32 | tmux/WSL/本机复制 | 744270411 | tui-clipboard terminal_req_10_ | clipboard backend dispatch/lease | 接线；各平台实机待验 |
| 33 | HTML 富复制 | 未新增调用者 | 仅上游 clipboard_html 参考 | 当前入口仅文本复制 | 参考保留；不支持 HTML 输出 |
| 34 | 审批与问题覆盖层 | 既有入口 | parity_tests；terminal_req_02_/12_ | widget overlay → existing permission command | 接线；待验 |
| 35 | 桌面提醒 | 06ec3269c | 终端提醒标题审查.md | NotificationStore 不等于系统通知 | 仅设计；未启用 |
| 36 | 终端窗口标题 | 06ec3269c | 终端提醒标题审查.md；session_title_tests | session title 不等于 OSC title | 仅设计；未启用 |
| 37 | 会话/模型/工具选择 | 既有 Gateway 入口 | session_restore_tests；settings_tests；integration_tests；parity_tests | sessions/settings/integrations → controller → Gateway | 接线；待验；不接 Codex 业务 |
| 38 | 宠物/IDE/内联可视化 | 44f78e22d | 恢复源码，无本轮完整验收 | 未建立产品运行协议 | 范围外参考保留 |

## 后续统一验证

旧计划 `.plans/2026-11-16-测试拆分与终端清理/4-统一回归.md` 仍为“未开始”，没有可复用的当前公共 Clippy 通过证据。PORTING_STATUS 的历史通过记录仅代表原时点，不覆盖后续接线。

后续先运行 cli-interaction 的 terminal_req_12_（预期 6 个有效用例）及表中直接相关测试，再执行快照矩阵（原有 689 上游 + 50 适配快照数量不可降低）；公共 Clippy 待统一安排。本轮没有运行这些命令，也没有修改快照、许可或恢复源码。快照矩阵属于 tests/upstream_snapshot_matrix.rs 集成目标，后续应以 `--test upstream_snapshot_matrix` 选择，避免仅 `--lib` 选不到该集成测试。
