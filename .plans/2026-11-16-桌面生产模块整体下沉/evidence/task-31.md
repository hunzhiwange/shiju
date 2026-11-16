# 任务31：宿主状态初始化与网关薄化

静态交付完成。前置任务38已完成；上次阻塞已解除。本轮未编译、未测试、未运行绘制或系统授权验收。

## 生产归属与真实调用链

- App::new 直接构造既有库的领域状态；任务看板81字段归 desktop-task-board::state，47个会话草稿/请求/历史字段归 desktop-session::session::state，28个聊天工作集字段复用既有 desktop-chat-view::view_snapshot::SessionViewSnapshot，22个聊天瞬态字段归 chat-view::interaction，41个工程字段归 workspace::project_state，32个编辑器字段归 editor::preview::state。泛型仅沿用已有渲染状态与快照端口，没有复制 App 或引入总控 trait。
- 宠物18字段、通知6字段、toast2字段、一般设置16字段、主题5字段、Todo5字段、复制反馈2字段分别由真实领域库持有。工具文件过滤器复用既有 suggestions::State；错误及本地化标题归 desktop-i18n::input::State。全部真实 update/view、缓存恢复和宿主测试调用者改为显式领域字段访问，无 Deref 全局状态隐藏。
- new.rs 的主题种子、解析、编辑器指标/语言、会话草稿、任务看板计时器、窗口内反馈和工程索引初始化由领域构造拥有。已有 Default 与原逐项空值完全相同时直接复用，不再添加同义工厂。
- app_update.rs 将启动/健康事件路由到既有 app_init 的宿主适配器；普通领域事件继续路由既有消息处理器。dispatch::Update::{Continue,Return} 显式保留原来提前返回与执行后置鉴权/探针/语音清理的区别，没有把所有启动事件一律提前返回。
- 偏好转换由 session::state::{restore_access_preferences,restore_history_preferences,apply_project_defaults}、workspace::{project_state,file_manager}、task-board::state、git-ui::state、providers::app_init、appearance::{theme_state,general_state}、update::DesktopUpdate 和 navigation::app_init 分别执行。覆盖本地编辑优先、完整权限覆盖保护、历史数量边界、拖拽保护与无效主题保留。
- StartupConfiguration 直接使用 desktop-settings-network 的真实状态与 load/accept/accept_sessions 接口，删除宿主包装与两个 Deref 实现。端点与全局 Message 映射留宿主。pending ACP 的移除、验证与回填、已知健康服务器过滤、控制身份更换时代际重置均归原领域。
- 上次提交87b22db33已删除114个网关转发并改接真实调用者。session_gateway 仅保留活动租户端点装配及宿主契约测试；请求直接使用 desktop-session::session_gateway，异步端点解析时机保持在任务被轮询时。
- 没有新增 Cargo 依赖，没有反向加载宿主源码，没有把 App 改名搬入库。TodoPanelPlacement 移到双方已依赖的 ui-controls，导航原路径重导出，避免 chat-view 与导航建立循环依赖。

## 宿主逐入口保留理由

| 入口/字段组 | 保留理由 |
| --- | --- |
| new.rs::App::new | 聚合领域构造、读取当前端点/本地安装快照、创建原生LSP通道、持有窗口/终端等系统资源，装配启动Task；没有复制领域构造。 |
| new.rs::apply_bootstrap_agent_config / apply_startup_agent_config | 将同一配置快照按既有顺序分发给多个设置领域并回填当前ACP列表。 |
| new/settings.rs::apply_bootstrap_agent_config | 对当前端点的编辑保护逐节分发到现有构造；控制身份重置调用安全领域。导入项只重导出真实领域构造。 |
| startup.rs::build_startup_task | 原生服务启动与WASM全局Task/Message路由、账户仓库及会话加载等跨域装配。 |
| startup.rs::note_startup_setting_edit / note_startup_worktree_edit | 把当前系统端点映射到领域PreferencesScope。 |
| startup.rs::merge_startup_worktree_settings | 调用网络领域的本地编辑保护合并，再把同一快照交给工程编辑器和工作区。 |
| preferences.rs | 先执行全局编辑保护，再依序调用各领域偏好方法；终端可见性、当前会话运行时和外部平台与领域选择间的装配。 |
| system_settings.rs | 保留各设置分区编辑/持久化保护、WASM端点安装、原生LSP启停、各编辑器和终端主题应用顺序；主题解析、快捷消息升级及Shell解析已委托领域。 |
| startup_snapshot.rs | 领域代际/端点检查后，将网络、设置、代理、会话、外部应用与任务看板快照按原顺序装配；保持用户在请求期间创建/编辑的草稿。 |
| dispatch.rs | 仅宿主全局消息到各领域结果/Task的映射、原生服务生命周期门控与重试任务调度；健康集合过滤由网络领域负责。 |
| app_update.rs | 主消息路由、原生剪贴板/文件读取、跨窗口焦点与选择路由，以及多个领域的后置清理。复制反馈状态转换归导航库。 |
| session_gateway.rs | 当前系统端点租户绑定与宿主契约挂载；不存在重复请求实现。 |
| app_state.rs | 领域状态与系统资源的聚合根。116项初始化静态集合核对无缺漏、无重复；随后工具过滤器归入现有建议状态，最终115项。 |
| 外部平台、焦点、光标、动画帧、滚动Id | 进程平台探测与Iced生命周期/控件资源；供多个领域共享。 |
| 原生启动/窗口/终端/LSP通道与进程控制器 | 系统资源所有权及生命周期。 |
| launch_at_login/close_to_tray及保存状态 | 操作系统登录与关闭生命周期、系统写入中状态。 |
| task_pet_size_pending、desktop_preferences_persistence | 当前端点持久化操作的跨域在途确认，不把本地领域预览值视为已保存。 |

## 测试与断言映射

- new/load_tests.rs::load_tests_module_is_wired → navigation::host_module_31_tests::load_tests，同名断言原样保留，删除旧挂载/文件。
- new/state_tests.rs::state_tests_module_is_wired → navigation::host_module_31_tests::state_tests，同名断言原样保留，删除旧挂载/文件。
- new/settings_tests.rs::settings_tests_module_is_wired → navigation::host_module_31_tests::settings_tests，保留模块后缀断言。
- legacy_task_board_switches_are_ignored_after_capability_cutover 的默认序列化字段缺失及旧配置反序列化后字段缺失两组断言 → task-board::host_module_31_tests 同名用例，直接使用生产State构造与真实TaskBoardSettings；删除宿主仅测试使用的构造函数。UI控制项、弹窗、工程刷新三组源代码契约留原宿主用例，不在领域库 include 宿主源码。
- startup_tests.rs 的注册、原生启动Task构造、App::new不等待网关、native_startup_backfills_once_and_retries_only_failed_reads 全部保留。它们验证真实App与网关边界、跨领域本地编辑保护、实际headless布局代码，不能由单一领域夹具代替；所有原HTTP、主题、中文/布局、配置、草稿、重试断言和真实绘制代码均未删除。
- 其它直接调用者测试只调整字段路径与StartupConfiguration显式端点/Message映射；静态核对没有删除原测试/夹具函数。唯一移走的三个注册函数均有上述真实归属。没有复制大型App夹具到库。
- 新增领域源码用例覆盖导航首屏/中文首页/宽度/拖拽保护、复制反馈忽略过期清理、看板计时器/代理/草稿默认值、晚到权限/历史偏好保留本地编辑，以及工程树列表与集合一致性。测试均通过真实领域入口；缓存快照只使用最小ViewSnapshot端口，未mock被测转换。
- 没有删除旧行为测试凑减量；来源测试净减少单列，新增测试源码没有当作生产减量。

## 静态验证与已知风险

- 独立rustfmt解析覆盖本轮全部修改Rust源码，未调用Cargo；限定本任务路径git diff --check通过。
- 对App字段与初始化集合、七组新领域State构造字段逐项作静态集合核对；App无漏项/重复。核对cfg分支、父模块注册和实际调用者，不使用干净工作区作为验收。
- 本次初始化相邻修正：原new.rs引用未定义editor_language（87b22db33亦如此），领域构造改用已有desktop-i18n::editor::language(system.desktop_locale)；不新增语言策略。
- 编译、类型检查、目标平台链接、测试运行、字体整形、原生绘制和系统授权结果均未知。按用户禁测要求，本轮没有运行任何这些验证，也没有声明通过数。
- 工作区其它任务变更保持原样；tree_list_tests.rs原有git字段修正与本任务字段改接同文件，提交仅收录本任务部分。

## 同口径行数

口径：Python pathlib.read_text().splitlines，含注释和空行；计入本任务四个新宿主适配文件，不把移动到宿主新文件当作净迁出。

| 来源 | 基线 | 当前 | 减少 |
| --- | ---: | ---: | ---: |
| `crates/vw-desktop/src/app/impls/app_init/external_apps.rs` | 2 | 2 | 0 |
| `crates/vw-desktop/src/app/impls/app_init/new/load_tests.rs` | 6 | 0 | 6 |
| `crates/vw-desktop/src/app/impls/app_init/new/settings.rs` | 146 | 106 | 40 |
| `crates/vw-desktop/src/app/impls/app_init/new/settings_tests.rs` | 63 | 24 | 39 |
| `crates/vw-desktop/src/app/impls/app_init/new/state_tests.rs` | 6 | 0 | 6 |
| `crates/vw-desktop/src/app/impls/app_init/new.rs` | 1099 | 372 | 727 |
| `crates/vw-desktop/src/app/impls/app_init/settings_builders.rs` | 7 | 2 | 5 |
| `crates/vw-desktop/src/app/impls/app_init/startup.rs` | 129 | 116 | 13 |
| `crates/vw-desktop/src/app/impls/app_init/startup_tests.rs` | 260 | 260 | 0 |
| `crates/vw-desktop/src/app/impls/app_update.rs` | 1054 | 435 | 619 |
| `crates/vw-desktop/src/app/session_gateway.rs` | 1008 | 14 | 994 |
| `crates/vw-desktop/src/app/state/app_state.rs` | 1251 | 275 | 976 |

来源合计 5031 → 1606，减少 3425 行。生产候选 4696 → 1322，减少 3374 行；测试来源减少 51 行。

- 新宿主 `crates/vw-desktop/src/app/impls/app_init/dispatch.rs`：210 行，均为上述跨域装配。
- 新宿主 `crates/vw-desktop/src/app/impls/app_init/preferences.rs`：67 行，均为上述跨域装配。
- 新宿主 `crates/vw-desktop/src/app/impls/app_init/startup_snapshot.rs`：112 行，均为上述跨域装配。
- 新宿主 `crates/vw-desktop/src/app/impls/app_init/system_settings.rs`：145 行，均为上述跨域装配。

计入新宿主 534 行后，来源范围真实净减量 2891 行，生产候选净减量 2840 行。包含前置任务已完成下沉，不将全部基线差值归本轮；直接调用者只改变字段路径，不用其行数变化扩大减量。

## 以后明确授权才运行的验证

- `python3 scripts/cargo_slot.py cargo test -p desktop-navigation --lib host_module_31_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-task-board --lib host_module_31_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-session --lib host_module_31_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-workspace --lib host_module_31_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::impls::app_init -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::impls::app_update_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::session::view_cache_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`

本轮均未执行；原生UI/权限验收不自动启动。
