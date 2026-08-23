# OpenAI Codex TUI 移植状态

本表是 `src/lib.rs` 顶层模块的唯一删改清单。批次 5 已激活 Markdown、代码高亮、表格识别与流式文本状态；批次 6 已激活复制的 history/exec/diff/goal 展示，并以纯 `V3DisplayEvent` 做数据映射；批次 7 已激活纯本地 textarea、Unicode wrapping、Vim、paste burst 与草稿历史核心；批次 8 已激活无边框 `›` composer、排队预览、动态 footer 与纯 `V3Action` 动作边界；批次 9 已激活复制的通用选择列表、过滤、命令补全、文件结果、会话/主题选择，并以 `V3SelectionItem`/`V3Action` 隔离 VibeWindow 数据与动作；批次 10 已激活审批与用户问答 overlay，所有决定和答案均只输出 `V3Action`；批次 11 已原地裁剪并激活复制的 ChatWidget，只保留 committed history、active cell、live tail、bottom pane、status、input queue、overlay 与布局缓存；批次 12 已用纯 UI `V3Event` 激活 thinking/text/tool/turn 的流式归约，保留 copied exec cell 分组、中间帧、终帧、状态隐藏和中断终态；批次 14 已激活复制的 welcome、session header、working indicator 和 status card，由 CLI bridge 提供只读 VibeWindow 会话数据；批次 15 已激活 copied custom terminal、history insertion 与 resize reflow 状态机，并以纯 v3 transcript pager、终端生命周期和 CLI bridge 接入 committed/active cells。后续批次修改文件时必须同步本表。

## 状态定义和删除门禁

- `未触及`：仍与来源快照一致，或尚未进入编译激活批次。
- `已激活`：复制模块已进入 v3 编译图，仍保持上游行为。
- `已改写`：模块已为 VibeWindow 做过有证据的适配。
- `允许删除`：替代链路和对应快照已通过，且无生产引用，下一批次可删除。
- `已删除`：明确路径已删除；本表必须同时保留删除理由、替代文件和验证证据。

“删除候选”不是状态，也不代表已经允许删除。候选只有同时满足以下条件才能改为“允许删除”：

1. 不再由 `src/lib.rs` 声明，且生产引用搜索为空。
2. 表中写明删除理由和实际替代文件，不接受“无用”等含糊理由。
3. 对应移植批次的聚焦测试和快照已通过。
4. Gateway 请求、session、cancel、permission 和 terminal 语义回归通过。

批次 16 已在逐项满足门禁后删除 37 个顶层 Codex 业务模块及明确的 onboarding auth 子路径；对应理由、替代文件和验证证据保留在本表。

## 顶层模块状态表

`后续批次` 是首次负责激活、改写或作最终处置判断的批次。`tests` 是 `lib.rs` 内联测试模块；条件编译模块同样计入。每个名称在表中只出现一次。

| 模块 | 当前状态 | 后续批次 | 计划处置 | 理由或替代边界 |
| --- | --- | --- | --- | --- |
| `additional_dirs` | 已删除 | 16 | 已删除 | Codex 启动目录业务；由 CLI v3 app 和现有 VibeWindow 启动参数替代 |
| `app` | 已删除 | 16 | 已删除 | Codex 应用主循环；由 `crates/vw-cli/src/cli/tui_v3/app.rs` 替代 |
| `app_backtrack` | 已删除 | 16 | 已删除 | Codex app 导航状态；由 v3 action/CLI controller 替代 |
| `app_command` | 已删除 | 16 | 已删除 | Codex app 命令分派；由 v3 action/CLI controller 替代 |
| `app_event` | 不进入 v3 编译图 | 12 | 由 v3 边界替代 | 生命周期输入由 `vibe::event::V3Event` 替代，不编译 Codex app event |
| `app_event_sender` | 不进入 v3 编译图 | 12 | 由 v3 边界替代 | v3 UI reducer 直接接收 `V3Event`，发送与 Gateway 转换留在 CLI bridge |
| `app_info` | 已删除 | 16 | 已删除 | Codex app 信息业务；只读产品数据由 CLI bridge 提供 |
| `app_server_approval_conversions` | 已删除 | 16 | 已删除 | Codex app-server 审批转换；由 v3 action 与现有权限回复链路替代 |
| `app_server_session` | 已删除 | 16 | 已删除 | Codex app-server session；由现有 Gateway runtime/session 替代 |
| `approval_events` | 已改写 | 10 | 保留并激活 | 审批 overlay 只输出 `ResolveApproval`，不发送权限回复 |
| `ascii_animation` | 不进入 v3 编译图 | 14 | 由 welcome 通用首帧替代 | welcome 保留原断点和复制的 brand-neutral 首帧，不编译 Codex/OpenAI 动画变体与帧调度依赖 |
| `bottom_pane` | 已改写 | 7-10 | 改写 | 批次 7 激活纯本地 textarea、粘贴突发和草稿历史；批次 8 接入 composer/footer 与 `V3Action`；批次 9 接选择；批次 10 接审批和用户问答安全 overlay |
| `branch_summary` | 未触及 | 6 | 改写 | 作为历史展示 cell 接收纯 UI 数据 |
| `chatwidget` | 已改写 | 14 | 保留并激活 | 批次 11 保留主布局、history/active/live tail、composer/status/overlay；批次 12 接入 `V3Event` 流式、工具、turn 终态和 interrupt 状态；批次 14 首屏插入复制 session header，运行状态直接委托 copied working indicator |
| `cli` | 已删除 | 16 | 已删除 | Codex CLI 参数面；由 `crates/vw-cli/src/cli/tui_v3/` 替代 |
| `clipboard_copy` | 未触及 | 8 | 保留并激活 | composer 通用剪贴板 UI 行为 |
| `clipboard_paste` | 已激活 | 7 | 保留并激活 | textarea 通用粘贴行为，不产生外部请求 |
| `collaboration_modes` | 已改写 | 9 | 保留并激活 | 仅保留共享选择 UI，数据改由 VibeWindow 提供 |
| `color` | 已激活 | 4 | 保留并激活 | 上游颜色基础 |
| `config_update` | 已删除 | 16 | 已删除 | Codex 配置写入；VibeWindow 现有配置链路负责 |
| `custom_terminal` | 已改写 | 15 | 保留并激活 | 保留 copied double buffer、viewport/resize、宽字符与 cursor-style 行为；仅增加 Ratatui/Crossterm 颜色桥接，版权头与 NOTICE 保持 |
| `pets` | 未触及 | 14 | 改写 | 作为 VibeWindow 状态展示，保持 UI 边界 |
| `auto_review_denials` | 未触及 | 10 | 改写 | 显式保留拒绝路径，改接 VibeWindow 权限语义；当前由 `V3ApprovalDecision::Deny`/`Cancel` 输出 |
| `cwd_prompt` | 已删除 | 16 | 已删除 | Codex 启动 cwd 流程；由现有 CLI/session restore 替代 |
| `debug_config` | 已删除 | 16 | 已删除 | Codex 配置调试面；不进入 v3 生产依赖图 |
| `diff_model` | 已改写 | 6 | 保留并激活 | 纯 UI 文件变更数据，无协议或执行职责 |
| `diff_render` | 已改写 | 6 | 保留并激活 | 复制 diff 展示边界，使用 VibeWindow 数据 |
| `exec_cell` | 已改写 | 6 | 保留并激活 | 复制工具单元只读渲染，不执行命令 |
| `exec_command` | 未触及 | 6 | 不进入 v3 编译图 | 工具命令来源由 `V3DisplayEvent` 提供，UI 不执行命令 |
| `external_agent_config_migration` | 已删除 | 16 | 已删除 | Codex 配置迁移；VibeWindow 不接入该配置系统 |
| `external_editor` | 未触及 | 8 | 改写 | 仅保留 composer 动作，执行由 CLI 边界决定 |
| `file_search` | 已改写 | 9 | 保留并激活 | 只接收 VibeWindow 提供的搜索结果 |
| `frames` | 未触及 | 17 | 保留并验证 | 上游视觉资源，纳入最终快照矩阵 |
| `get_git_diff` | 未触及 | 6 | 改写 | diff 数据由外部输入，不在 UI crate 执行业务查询 |
| `git_action_directives` | 已激活 | 6 | 改写 | 批次 5 仅提前激活纯解析器供 Markdown copy target 使用；Git 副作用仍不进入 UI crate |
| `goal_display` | 已改写 | 6 | 保留并激活 | 纯 UI step/goal 文本格式化 |
| `goal_files` | 未触及 | 6 | 改写 | 只保留 goal 附件展示 |
| `history_cell` | 已改写 | 14 | 保留并激活 | 所有 `V3DisplayEvent` 显式映射到复制 history/exec cell；批次 14 恢复 bordered session header、路径截断和窄屏折叠 |
| `hooks_rpc` | 已删除 | 16 | 已删除 | Codex hooks RPC；不接入 VibeWindow UI crate |
| `ide_context` | 未触及 | 8 | 改写 | 仅作为 composer 附件/上下文展示输入 |
| `inline_visualization` | 未触及 | 6 | 改写 | 作为历史 cell 展示，无外部副作用 |
| `insert_history` | 已改写 | 15 | 保留并激活 | copied inline/full-screen history insertion 接收 `ChatWidget::transcript_lines`，保留 URL、宽字符、Zellij soft-wrap 与背景样式测试 |
| `key_hint` | 未触及 | 8 | 保留并激活 | 通用键位提示渲染 |
| `keymap` | 已改写 | 7 | 保留并激活 | 输入核心使用自包含编辑/Vim 默认键位，配置解析留待后续边界 |
| `keymap_setup` | 未触及 | 7 | 改写 | 移除 Codex 配置来源，使用 v3 明确设置 |
| `line_truncation` | 已激活 | 4 | 保留并激活 | 上游行截断算法 |
| `live_wrap` | 已激活 | 4 | 保留并激活 | 上游实时换行算法 |
| `local_chatgpt_auth` | 已删除 | 16 | 已删除 | OpenAI/ChatGPT 登录；v3 不接入登录系统 |
| `managed_new_thread_defaults` | 已删除 | 16 | 已删除 | Codex 托管配置；会话默认值由现有 VibeWindow state 提供 |
| `markdown` | 已改写 | 5 | 保留并激活 | 保留上游解析与 fence 算法；inline visualization 暂用上游无上下文回退，不引入 Codex 协议 |
| `markdown_render` | 已改写 | 5 | 保留并激活 | 上游渲染与快照不变；内联单个 Codex string helper 解除外部 crate 依赖 |
| `markdown_stream` | 已激活 | 5 | 改写 | 已接纯文本 chunk，保留上游 newline commit 与 UTF-8 边界行为 |
| `markdown_text_merge` | 已激活 | 5 | 保留并激活 | 上游文本合并算法与测试直接通过 |
| `mention_codec` | 未触及 | 8 | 改写 | composer mention 转为 VibeWindow 附件数据 |
| `model_catalog` | 已删除 | 16 | 已删除 | Codex 模型目录；展示数据由 CLI bridge 提供 |
| `model_migration` | 已删除 | 16 | 已删除 | Codex 模型迁移；VibeWindow 不接入该配置业务 |
| `motion` | 未触及 | 14 | 保留并激活 | 通用状态动效 |
| `multi_agents` | 已改写 | 9 | 保留并激活 | 仅保留选择/状态 UI，数据由 VibeWindow 提供 |
| `named_session_lookup` | 已删除 | 16 | 已删除 | Codex session 查询；由现有 Gateway session restore 替代 |
| `notifications` | 不进入 v3 编译图 | 12 | 由 v3 边界替代 | Codex notification 分派由 `ChatWidget::handle_v3_event` 的穷尽 match 替代 |
| `npm_registry` | 已删除 | 16 | 已删除 | Codex 更新/分发查询；v3 不访问 npm registry |
| `onboarding` | 已改写 | 14 | 保留并激活 | welcome 保留上游留白、bold 层级、动画断点和通用首帧；auth/trust/account 步骤不进入编译图 |
| `oss_selection` | 已删除 | 16 | 已删除 | Codex provider/model 选择；由现有 VibeWindow 配置负责 |
| `pager_overlay` | 已改写 | 15 | 保留并激活 | 全文 pager 只接收 v3 committed/active lines，保留 top/middle/end、page/jump、end pin、resize 与三尺寸快照 |
| `public_widgets` | 未触及 | 8 | 改写 | 暴露移植后的 composer 公共 UI |
| `render` | 已激活 | 4 | 保留并激活 | 上游 Renderable 基础与原快照；仅适配 crate 重命名后的快照查找 |
| `resize_reflow_cap` | 已改写 | 15 | 保留并激活 | 保留上游保守 row cap，改用显式 v3 override 与本地终端环境探测，不依赖 Codex config/detection |
| `resume_picker` | 已改写 | 9 | 保留并激活 | 选择 UI 接收 VibeWindow session 数据 |
| `selection_list` | 已改写 | 9 | 保留并激活 | 通用列表组件接收 `V3SelectionItem` |
| `service_tier_resolution` | 已删除 | 16 | 已删除 | OpenAI service tier 业务；v3 无对应所有权 |
| `session_archive_commands` | 已删除 | 16 | 已删除 | Codex session 命令；由现有 controller/session API 替代 |
| `session_log` | 已删除 | 16 | 已删除 | Codex session 日志；由现有 Gateway/state 记录链路替代 |
| `session_queue_commands` | 已删除 | 16 | 已删除 | Codex queue 命令；由现有 controller 行为替代 |
| `session_resume` | 已删除 | 16 | 已删除 | Codex resume 业务；由现有 VibeWindow session restore 替代 |
| `session_start` | 已删除 | 16 | 已删除 | Codex session 启动；由 CLI v3 app/Gateway runtime 替代 |
| `session_state` | 未触及 | 13 | 改写 | 映射为 CLI bridge 提供的只读会话状态 |
| `skills_helpers` | 已改写 | 9 | 保留并激活 | 只保留命令面板展示数据，不发现或执行技能 |
| `slash_command` | 已改写 | 9 | 保留并激活 | 命令清单改为 VibeWindow 已支持项 |
| `startup_draft` | 已删除 | 16 | 已删除 | Codex 启动草稿恢复；由现有 VibeWindow state 替代 |
| `startup_error` | 未触及 | 13 | 改写 | 只保留明确 UI 错误展示，错误源来自 CLI bridge |
| `startup_hooks_review` | 已删除 | 16 | 已删除 | Codex hooks 启动流程；不接入 v3 |
| `startup_orchestration` | 已删除 | 16 | 已删除 | Codex 启动编排；由 CLI v3 app 替代 |
| `startup_preflight` | 已删除 | 16 | 已删除 | Codex 启动预检；沿用现有 VibeWindow 启动检查 |
| `status` | 已改写 | 14 | 保留并激活 | copied bordered status card 仅消费 `V3SessionDisplayData`；无 account、rate-limit、plan 或服务查询 |
| `status_indicator_widget` | 已改写 | 14 | 保留并激活 | ChatWidget 直接使用 copied `• Working`、elapsed/interrupt、details 与窄屏截断布局 |
| `streaming` | 已改写 | 5、12 | 保留并激活 | 批次 5 保留 Markdown chunking/holdback；批次 12 将 ChatWidget thinking/answer 增量段接入 active/committed cell，并在工具和 turn 边界提交 |
| `style` | 已激活 | 4 | 保留并激活 | 上游样式基础 |
| `terminal_hyperlinks` | 已改写 | 15 | 保留并激活 | 批次 5 作为 Markdown 几何依赖提前激活；两项 backend 测试暂直接验证 Buffer，生产算法未改 |
| `terminal_palette` | 已改写 | 15 | 保留并激活 | 保留上游调色与默认色探测；仅移除 Codex 终端类型依赖 |
| `terminal_probe` | 已激活 | 15 | 改写 | 提前作为样式基础依赖激活原探测与输入回放；运行模式适配仍归批次 15 |
| `terminal_title` | 未触及 | 15 | 改写 | 标题改为允许的产品文案差异 |
| `terminal_visualization_instructions` | 未触及 | 15 | 改写 | 删除 Codex 业务提示，只保留实际支持的终端说明 |
| `text_formatting` | 已激活 | 4 | 保留并激活 | 上游文本格式基础 |
| `theme_picker` | 已改写 | 9 | 保留并激活 | 通用选择 UI 接收 VibeWindow 主题数据 |
| `thread_transcript` | 已改写 | 15 | 保留并激活 | 将纯 `V3DisplayEvent` 映射为 scrollback/pager 共用的 source-backed history cells |
| `token_usage` | 不进入 v3 编译图 | 14 | 由 `status::V3TokenUsage` 替代 | CLI bridge 从现有 Gateway UI state 投影只读计数，不查询额度服务 |
| `tooltips` | 未触及 | 8 | 改写 | 保留布局，命令文案按白名单替换 |
| `transcript_reflow` | 已激活 | 15 | 保留并激活 | copied width observation、debounce、visible-row cache 与 stream-final repair 状态机；CLI bridge 消费 width change |
| `tui` | 已改写 | 15 | 保留并激活 | v3 run mode 选择 inline/alternate；RAII 在成功、错误、初始化失败与 panic 路径恢复 cursor/paste/screen/raw mode |
| `ui_consts` | 已激活 | 4 | 保留并激活 | 上游 UI 常量 |
| `unarchive_prompt` | 已删除 | 16 | 已删除 | Codex session 业务；由现有 VibeWindow session UI/控制器替代 |
| `update_action` | 已删除 | 16 | 已删除 | Codex 更新业务；v3 不负责产品更新 |
| `update_prompt` | 已删除 | 16 | 已删除 | Codex 更新提示；无对应 v3 UI 能力 |
| `update_versions` | 已删除 | 16 | 已删除 | Codex 版本更新查询；v3 不访问该服务 |
| `updates` | 已删除 | 16 | 已删除 | Codex 更新流程；不进入 v3 生产依赖图 |
| `updates_cache` | 已删除 | 16 | 已删除 | Codex 更新缓存；v3 不保存该状态 |
| `version` | 不进入 v3 编译图 | 14 | 由 CLI bridge 包版本替代 | 展示 `vw-cli` 包版本，不查询 Codex 更新服务 |
| `width` | 已激活 | 4 | 保留并激活 | 上游 Unicode width 算法 |
| `windows_sandbox` | 已删除 | 16 | 已删除 | Codex sandbox 业务；安全决策仍由现有 VibeWindow 链路负责 |
| `workspace_command` | 已改写 | 9 | 保留并激活 | 仅保留命令/选择 UI，动作交给 controller |
| `workspace_messages` | 未触及 | 6 | 不进入 v3 编译图 | 由 `vibe::event::V3DisplayEvent` 作为纯显示边界替代 |
| `wrapping` | 已激活 | 4 | 保留并激活 | 上游 Unicode 换行算法 |
| `table_detect` | 已激活 | 5 | 保留并激活 | 上游 Markdown 表格与 fence 识别测试直接通过 |
| `test_backend` | 已验证 | 17 | 保留并验证 | 上游测试 backend 与复制的 `ChatWidget` 共同服务最终快照矩阵 |
| `test_support` | 已改写并验证 | 17 | 改写 | 保留测试工具并适配 v3 构造，不引入独立 renderer |
| `tests` | 已改写并验证 | 17 | 改写 | 内联上游测试与最终 Gateway/尺寸矩阵均通过 |

## 批次 16 删除汇总

## 批次 16 删除前门禁证据

- 上表 37 个批次 16 候选均不由 `src/lib.rs` 声明，因此不进入 v3 生产编译图；聚焦入口编译和 Gateway 回归通过。
- 替代边界由批次 5–15 的已激活/已改写项提供；前置任务文件已按 `do-task-plans` 的验证后删除协议移除。
- `cargo test -p vw-cli tui_v3 --lib`：2 通过，包含默认入口委托和 Gateway request/terminal 语义。
- `cargo test -p vw-tui-v3 terminal --lib`：57 通过，包含 copied terminal、resize reflow、宽字符、OSC8 和终端恢复路径。
- `cargo tree -p vw-tui-v3 | rg 'codex-'`：无输出；删除前 `codex-*` 生产依赖为零。
- UI、`custom_terminal.rs`、729 个快照、`LICENSE-CODEX` 和 `NOTICE-CODEX` 均存在；许可文件与上游仓库逐字节一致。

| 子路径 | 当前状态 | 替代边界 |
| --- | --- | --- |
| `onboarding/auth.rs` 与 `onboarding/auth/headless_chatgpt_login.rs` | 已删除 | OpenAI/ChatGPT 登录不进入 v3；welcome 由 `onboarding/welcome.rs` 保留，会话展示数据由 CLI bridge 提供 |

已删除类别包括 app-server、OpenAI/ChatGPT 登录、Codex config、provider/model、更新/分发、启动编排和 Codex session 命令，共解析并删除 53 个明确文件/目录入口。`src/app/` 中 11 个、`src/external_agent_config_migration/` 中 9 个上游快照已按原相对路径迁至 `upstream_snapshots/`，仓库总快照数保持 729。

`onboarding` 本身不是删除候选：批次 14 保留 welcome/status UI；其 `onboarding/auth.rs` 子路径在批次 16 单独审查并由 CLI bridge 的只读身份/会话数据边界替代。

## 批次 16 删除后终验

- `cargo test -p vw-cli run_interactive_delegates_to_copied_and_ported_tui_v3 --lib`：1 通过；默认交互入口只委托 v3，并保持错误传播。
- `cargo test -p vw-cli tui_v2 --lib -- --test-threads=1`：459 通过；v2 完整保留。
- `cargo test -p vw-cli tui_v3 --lib`：2 通过；入口与 Gateway request/terminal 语义通过。
- `cargo test -p vw-tui-v3 terminal --lib`：57 通过；终端、宽字符、OSC8、resize 与恢复语义通过。
- `cargo tree -p vw-tui-v3 | rg 'codex-'`：无输出；v3 直接依赖仅为 UI、终端、文本、序列化与测试支持依赖，无 Codex 业务 crate。
- `cargo clippy --all-targets --all-features -- -D warnings`：通过。
- 明确删除目标复核：37 个顶层模块及 onboarding auth 子路径均不存在；729 个快照、UI、终端、v2、`LICENSE-CODEX` 与 `NOTICE-CODEX` 均保留。

## 批次 17 最终快照与 Gateway 回归

- 上游快照保留率：689/689（100%）；名称与内容均与来源逐文件一致，其中 20 个随已删除业务模块迁入 `upstream_snapshots/`。
- v3 适配快照 50 个，仓库快照总数 739；产品文案差异仍只能使用 `UPSTREAM.md` 的 5 类白名单。
- 尺寸/场景矩阵：40x18、80x24、120x40 × empty、conversation、working、tools、approval、question、overlay、transcript、completed、error，共 30 个完整帧。
- 组件证明：矩阵直接实例化 `vw_tui_v3::chatwidget::ChatWidget`，并调用其 `render`、copied approval/user-input overlay 和 `TranscriptPager`；无新 renderer。

| Gateway 边界 | 回归字段/语义 |
| --- | --- |
| request | `session_id`、`messages`、`system`、`model`、`reasoning_effort`、`agent`、`allowed_tools`、`acp_agent`、`acp_allowed_tools`、`options` |
| session | workspace root、session ID、message ID/parent ID、usage、snapshot 持久化 |
| security | permission once/always/reject 路由仍经 v2 controller，cancel 仍为 `CancelActiveSubmission` |
| terminal | done/error/cancel/timeout 仍经 stream adapter 与 runtime reducer，再投影为 v3 terminal event |

真实回归链路为 `GatewayChatStreamEvent -> stream adapter -> TuiState reducer/controller -> GatewayV3Bridge -> ChatWidget -> Ratatui Buffer`；仅 Gateway 网络 endpoint 使用不可达的测试端口，未 fake 中间层。

最终依赖树的一级依赖仅包含 Crossterm、Ratatui、文本/Markdown、样式、URL、序列化与测试支持库；`cargo tree -p vw-tui-v3 | rg 'codex-'` 无输出。

- `cargo test -p vw-tui-v3 --lib`：725 通过，1 个 Windows 限定用例忽略，0 失败。
- `cargo test -p vw-tui-v3 upstream_snapshot_matrix`：1 通过。
- 两个批次 17 具名 CLI 回归：各 1 通过；`cargo test -p vw-cli tui_v3 --lib`：4 通过，0 失败。
- `cargo test -p vw-cli stream_adapter --lib`：14 通过；`cargo test -p vw-cli security --lib`：14 通过。
- `cargo clippy --all-targets --all-features -- -D warnings`：通过。
