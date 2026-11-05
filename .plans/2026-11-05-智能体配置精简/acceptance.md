# 智能体配置精简验收记录（2026-11-05）

状态：已完成。原生五标签交互、取消/失败重试与窄短窗口证据已补齐；此前所有聚焦回归和 Clippy 已通过。plan.md 保持只读。

## 范围与依赖

- 批次 6 已由 `d21adaa4d` 实现，编号任务文件已删除；批次 5 为 `422cff0f4`，保留待回归记录。
- 本任务仅扩展五标签实际渲染测试、真实目录页布局测试、全局配置 HTTP 路由及新会话绑定测试，以及隔离原生窗口样例。
- 工作区已有大量其他任务差异；不提交已有的取消/新建测试及其他模块修改。所有验证针对当前共享工作区，不代表仅本提交的干净检出结果。

## 新增行为证据

- `agent_config_cleanup_five_tabs_render_in_both_themes`：真实 tiny-skia layout/draw/screenshot，五个标签、两种主题、1100×900 与 760×520，共 20 张截图；断言唯一名称/描述/模型入口、身份页唯一编辑框、无旧资料/规则入口、标题及取消/保存按钮在窗口内。
- `agent_config_cleanup_catalog_keeps_three_columns_and_scrollbar_gutter`：直接布局真实 agents_view；两种宽度下检查 7px 左右留白、三列等宽与末行补位。4px 轨道宽度来自真实页面的 Scrollbar 配置，并由目录截图复核。
- `agent_config_cleanup_global_profile_roundtrips_to_new_session`：临时数据库及两个临时工作区，通过真实 Axum `/config` HTTP PATCH 保存并回读；Native/ACP 新绑定保持模型、固定空技能/MCP、系统提示词及完全访问初值；显式关闭后保持关闭，固定拒绝策略独立保留。
- 既有端到端验收通过时，另覆盖本地隔离模型 HTTP 服务、ACP Python 协议对端、服务稳定 ID 注入、安全拒绝及子进程重启后的会话绑定。
- 既有桌面 HTTP 测试及草稿测试覆盖多行描述保存、取消、失败保留及重试；本任务未将这些已有差异纳入提交。

## 首次验证与夹具修正

- 前六批已满足的场景不伪造功能缺失红灯。批次 5 原始红灯见其编号任务文件：缺少 MCP 模块及标签导致编译退出 101；本轮扩展现有真实渲染用例。
- 新后端测试首次执行 16 通过/1 失败，退出 101，`/tmp/shiju-task7-red-global-profile.log`：继承 permission 在会话绑定时被解析为固定策略，不能与未解析配置整体比较。将夹具设为显式固定拒绝策略，保留完整比较并覆盖安全边界。
- 真实目录页测试首次执行 24 通过/1 失败，退出 101，`/tmp/shiju-task7-final-vw-desktop-agent_config_cleanup_.log`：夹具 config_ready=false，读到只读等待页；修正夹具为加载完成后检查真实布局。
- 编写期间一次 Rust 类型检查错误（Option<String> 与 &str）已修正；不作为功能红灯。

## 既有失败基线

本任务编辑前运行 `cargo test -p vw-desktop --lib system_settings_agents -- --test-threads=2`，87 通过/14 失败，退出 101。原始日志 `/tmp/shiju-task7-baseline-vw-desktop-system_settings_agents.log`。

失败测试：

- `app::components::system_settings_agents::basic_tests::basic_view_keeps_current_provider_and_model_even_when_missing_from_catalog`
- `app::components::system_settings_agents::basic_tests::basic_view_main_hides_availability_and_shows_common_runtime_controls`
- `app::components::system_settings_agents::basic_tests::basic_view_preserves_fields_for_blank_selections_and_numeric_boundaries`
- `app::components::system_settings_agents::basic_tests::basic_view_renders_runtime_boolean_and_text_variants`
- `app::components::system_settings_agents::basic_tests::basic_view_worker_shows_availability_and_common_runtime_controls`
- `app::components::system_settings_agents::mod_tests::agents_view_builds_main_enabled_disabled_and_model_metadata`
- `app::components::system_settings_agents::skills_tests::discovery_order_text_distinguishes_open_project_and_global_context`
- `app::components::system_settings_agents::tests::agents_view_builds_cards_for_main_enabled_disabled_and_model_variants`
- `app::components::system_settings_agents::tests::agents_view_loading_state_adds_status_row`
- `app::components::system_settings_agents::tests::agents_view_missing_selection_uses_first_entry_and_unknown_tab_falls_back`
- `app::components::system_settings_agents::tools_tests::builtin_tool_cards_are_locked_and_badged`
- `app::components::system_settings_agents::tools_tests::tools_view_distinguishes_empty_and_all_grouped_tool_lists`
- `app::components::system_settings_agents::tools_tests::tools_view_keeps_zero_match_presets_visible_without_grouping_them`
- `app::components::system_settings_agents::tools_tests::tools_view_preserves_duplicate_available_tools_and_selection_state`

原始错误：basic_tests.rs:22 的旧子节点数量断言 `>= 8`；mod_tests.rs:147 及 tests.rs 的根节点数量 `left: 5 / right: 6`；skills_tests.rs:112 的旧项目路径提示断言；tools_tests.rs:350 的 `left: 2 / right: 3` 及 380/398/491 的索引越界。basic_tests.rs、skills_tests.rs、tools_tests.rs 未由本任务修改；mod_tests.rs、tests.rs 进入本任务前已有其他会话差异，未修改或提交。最终回归不能通过 skip 隐藏这些失败。

## 原生交互限制（历史，现已解除）

CUA 原生应用枚举明确返回：`The Mac is locked and automatic unlock could not unlock it`。因此无法操作原生窗口或取得其交互截图，不把 Headless 截图或窗口进程启动视为人工交互验收成功。不连接用户网关，隔离窗口始终使用端口 9。待解锁后完成基础参数/身份/工具/技能/MCP 的原生操作。

## 截图

- `/tmp/shiju-task7-{light,dark}-{basic,identity,tools,skills,mcp}-{1100x900,760x520}.png`（20 张真实绘制截图）。已查看基本参数、身份、工具、技能、MCP 的代表截图，中文正常，窄短窗口固定头尾可见。
- `/tmp/shiju-agent-catalog-light.png`、`/tmp/shiju-agent-catalog-dark.png`：本轮目录回归重新生成，三列、滚动轨道及内容留白可见。页面测试使用透明根背景，背景色不作为原生窗口主题验收结论。

## 首次执行命令结果（历史）

所有 Cargo 阶段均从仓库根目录通过原子 mkdir 动态领取槽位，设置相同绝对路径的 `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR`（仓库另有 build-dir 配置），所有子进程继承；阶段退出清理自有 owner 和锁，保留编译缓存。

- 基线：槽位 3，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。
- 首轮新增用例验证：槽位 4，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-4`，Clippy 退出 0。
- 后端最终验证及原生窗口：槽位 3，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。
- 修正目录夹具后的最终回归：槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。

| 确切命令 | 槽位 | 退出码 | 有效测试数量 | 原始日志 |
| --- | --- | --- | --- | --- |
| `cargo fmt --all` | 2 | 0 | 不适用 | `/tmp/shiju-task7-recheck-fmt.log` |
| `cargo test -p vw-config-types --lib agent_profile_tests -- --test-threads=2` | 3 | 0 | 7 通过 | `/tmp/shiju-task7-final-vw-config-types-agent_profile_tests.log` |
| `cargo test -p vw-desktop --lib agent_config_cleanup_ -- --test-threads=2` | 2 | 0 | 25 通过 | `/tmp/shiju-task7-recheck-vw-desktop-agent_config_cleanup_.log` |
| `cargo test -p vw-desktop --lib system_settings_agents -- --test-threads=2` | 2 | 101 | 88 通过、14 既有失败 | `/tmp/shiju-task7-recheck-vw-desktop-system_settings_agents.log` |
| `cargo test -p vw-desktop --lib app::message::settings::agents::tests -- --test-threads=2` | 3 | 0 | 63 通过 | `/tmp/shiju-task7-final-vw-desktop-app::message::settings::agents::tests.log` |
| `cargo test -p vw-agent --lib agent_config_cleanup_ -- --test-threads=2` | 3 | 0 | 17 通过 | `/tmp/shiju-task7-final-vw-agent-agent_config_cleanup_.log` |
| `cargo test -p vw-agent --lib agent_profile_acceptance_tests -- --test-threads=2` | 3 | 0 | 4 通过 | `/tmp/shiju-task7-final-vw-agent-agent_profile_acceptance_tests.log` |
| `cargo clippy --all-targets --all-features -- -D warnings` | 3 | 0 | 不适用 | `/tmp/shiju-task7-final-clippy.log` |
| `cargo test -p vw-desktop --test agent-profile-window` | 3 | 超时，180s 后发送 SIGTERM 并等待进程组结束；未记录数字退出码 | 非单元测试；编译并启动成功，原生交互未验收 | `/tmp/shiju-task7-native.log` |

额外复验：`cargo test -p vw-agent --lib request_tests -- --test-threads=2`，槽位 2，退出 101，27 通过/1 既有失败。日志 `/tmp/shiju-task7-recheck-vw-agent-request_tests.log`。失败名称 `session::llm::acp::request::request_tests::acp_model_source_launch_profile_production_request_rejects_unverified_driver`，request_tests.rs:274 原始错误：`assertion failed: message.contains("ACP codex session_model launch has not been verified")`。与任务 5 留存记录一致，本任务未修改 request_tests.rs 或生产请求路径，未 skip。

末轮 `cargo clippy --all-targets --all-features -- -D warnings`：槽位 2，退出 0，日志 `/tmp/shiju-task7-recheck-clippy.log`。本任务所有槽位占用记录和锁均已释放，编译缓存保留；复核无隔离窗口进程残留。

格式化后核对差异，恢复本任务以外的纯格式变化；提交只选择本任务拥有的函数修改及新增文件，保留其他会话原有差异。正常测试链接器提示 `__eh_frame section too large` 属既有 macOS 链接信息；完整 Clippy 无 Rust 诊断失败。

以上为首次执行的历史记录。旧断言现已按下列回归记录修复，当前剩余事项仅为解锁后的隔离原生交互。


## 延后回归（2026-11-05）

- 按新的调度授权修复过时断言：默认配置组移除权限/技能/MCP 重复控件后，基础页最小子节点数改为 6；目录页移除项目选择器后，原有加载状态夹具为 5 个根子节点；工具页移除重复入口后列表位于第 2 个子节点；技能发现提示包含“发现顺序：”前缀。保留所有分组、预设、重复工具、内建锁定徽标及运行参数检查，没有 skip 或修改生产行为。
- ACP 旧拒绝用例由任务 5 独立修复并提交为 `0c27574bc`：Codex 已支持会话模型，使用仍拒绝该启动路径的 Droid 验证同一拒绝边界，并保留密钥不泄漏断言。本任务未提交 request_tests.rs。
- 所有命令基于最新共享工作区运行。动态领取槽位 3，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`，`CARGO_BUILD_BUILD_DIR` 为相同绝对路径；阶段结束已释放自有锁，缓存保留。

| 确切命令 | 退出码 | 有效测试数量 | 日志 |
| --- | --- | --- | --- |
| `cargo fmt --all` | 0 | 不适用 | `/tmp/shiju-task7-resume-final-fmt.log` |
| `cargo test -p vw-config-types --lib agent_profile_tests -- --test-threads=2` | 0 | 7 通过 | `/tmp/shiju-task7-resume-final-vw-config-types-agent_profile_tests.log` |
| `cargo test -p vw-desktop --lib agent_config_cleanup_ -- --test-threads=2` | 0 | 25 通过 | `/tmp/shiju-task7-resume-final-vw-desktop-agent_config_cleanup_.log` |
| `cargo test -p vw-desktop --lib system_settings_agents -- --test-threads=2` | 0 | 102 通过 | `/tmp/shiju-task7-resume-final-vw-desktop-system_settings_agents.log` |
| `cargo test -p vw-desktop --lib app::message::settings::agents::tests -- --test-threads=2` | 0 | 63 通过 | `/tmp/shiju-task7-resume-final-vw-desktop-app::message::settings::agents::tests.log` |
| `cargo test -p vw-agent --lib agent_config_cleanup_ -- --test-threads=2` | 0 | 17 通过 | `/tmp/shiju-task7-resume-final-vw-agent-agent_config_cleanup_.log` |
| `cargo test -p vw-agent --lib agent_profile_acceptance_tests -- --test-threads=2` | 0 | 4 通过 | `/tmp/shiju-task7-resume-final-vw-agent-agent_profile_acceptance_tests.log` |
| `cargo test -p vw-agent --lib request_tests -- --test-threads=2` | 0 | 28 通过 | `/tmp/shiju-task7-resume-final-vw-agent-request_tests.log` |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 不适用 | `/tmp/shiju-task7-resume-final-clippy.log` |

三个指定新验收用例全部非零命中。20 张双主题五标签、两种尺寸的截图已由本轮重新绘制，路径仍为上文 `/tmp/shiju-task7-*.png`；抽查暗色技能窄窗口与亮色基础参数大窗口，中文绘制正常、标题和保存/取消区可见。

唯一阻塞：本轮 CUA `getState` 再次明确返回 `The Mac is locked and automatic unlock could not unlock it. Ask the user to unlock the Mac manually before continuing.`。原生应用清单为空，无法进行原生五页点击及取得交互截图。未绕过锁屏，也未把 Headless 绘制当作原生操作。首次原生窗口命令成功启动的历史记录保留，本轮因相同外部状态不重复空跑窗口。

下一步：用户解锁 Mac 后，在共享槽位内运行 `cargo test -p vw-desktop --test agent-profile-window`，实际操作五个标签并检查取消/失败重试及布局；取得原生证据后将任务 7 标记已完成并删除编号任务文件。当前按最新调度要求明确标记已阻塞并保留任务文件。


## 解锁后的原生验收完成（2026-11-05）

本次只补原生证据及文档，未修改 Rust 代码；复用上轮已通过的六组必需聚焦回归、额外 request_tests 与完整 Clippy，无重复运行无关检查。

- 动态领取槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，`CARGO_BUILD_BUILD_DIR` 同路径。
- 确切命令：`cargo test -p vw-desktop --test agent-profile-window`，日志 `/tmp/shiju-task7-native-final.log`；成功编译并启动固定端口 9 的窗口。本目标 harness=false，结果来自下列 10 个真实交互场景，不以零单元测试或编译成功代替验收。
- CUA 无法绑定无 bundle 的 Cargo 可执行文件（返回 Invalid app）。在自有槽位中添加 `AgentProfileAcceptance.app` 元数据，复制同一个编译产物，SHA-256 比较相同，未修改二进制或应用逻辑；独立 bundle ID 为 `com.rsung.shiju.profile-acceptance`。实际交互窗口标题明确为 `Agent Profile Acceptance — isolated gateway port 9`，不操作用户业务应用。
- 通过 CUA 原生坐标点击、键盘输入、滚动及拖拽完成以下场景。所有截图由原生窗口直接捕获，原始分辨率 2560×1944，缩小后 1560×1144；不是 Headless 图。

| 场景 | 实际观察 | 截图后缀（均带 shiju-task7-native- 前缀） |
| --- | --- | --- |
| 基础参数 | 修改名称和中文两行描述，跨页保留；运行迭代次数可改为 123 | `basic.png`、`basic-edited.png` |
| 身份 | 唯一系统提示词输入框可输入中文换行，字形绘制无 panic | `identity.png` |
| 工具 | 标签可切换，隔离 fixture 无可用工具时明确显示空态 | `tools.png` |
| 技能 | 继承改固定选择，依次选目录 a、b；原智能体草稿不变，端口 9 目录请求失败明确显示 | `skills-directory-a.png`、`skills-directory-b.png` |
| MCP | 继承改固定空选择，可点击刷新目录；不出现凭据配置表单 | `mcp.png` |
| 模型/权限 | 滚动至唯一模型组和完全访问开关，未连接 ACP 的能力说明可见，权限初值为关闭 | `model-permission.png` |
| 保存/重试 | 点击保存及再次重试，Toast 明确显示 `/v1/global/config` 端口 9 连接被拒绝；弹窗保留，身份草稿仍为新文本 | `save-failure.png`、`retry-preserves-identity.png` |
| 取消回滚 | 取消后列表保持原名称/描述；重新打开恢复原系统提示词，技能/MCP 模式恢复继承 | `cancel-catalog.png`、`cancel-reopen-basic.png`、`cancel-restores-identity.png`、`cancel-restores-skills.png`、`cancel-restores-mcp.png` |
| 新建/取消 | 新建 native-ui-fixture 默认进入基础参数，默认引擎 native；取消后仍只有原两张卡片 | `new-basic.png`、`new-cancel.png` |
| 窄短窗口 | 拖动真实窗口缩小至 1560×1144 截图尺寸，五标签、标题和底部取消/保存区保持可见，内容单独滚动 | `narrow-short.png` |

18 张原生截图及 SHA-256/尺寸清单已保存在：

`/Users/xiangmin/.codex/visualizations/2026/11/04/01a1a7ad-8390-7df1-93e7-0b4b69a0507f/task7-native/`

清单为该目录的 `manifest.json`。端口 9 的保存失败是预期负向场景；保存成功、全局配置回读以及 Native/ACP 新会话选择的正向证据仍由上轮临时数据库和隔离 HTTP/协议验收提供，不将连接失败声称为保存成功。

完成后通过原生关闭按钮退出 bundle 窗口，CUA 清单确认 `isRunning=false`；再发送 SIGTERM 清理原始未封装的 Cargo 窗口进程组，命令受控退出码为 -15（日志值，非测试断言失败），记录 `/tmp/shiju-task7-native-final-exit`。确认两个验收进程均结束后释放槽位 2，自有锁已删除，编译缓存保留。原有锁屏阻塞已解除，任务 7 及整个顺序计划验收完成；编号任务文件按要求删除。
