# 任务28：静态交付证据

已完成生产接线与静态职责审查；未编译、未测试、未运行绘制。不因静态检查推断类型正确、运行通过或编译提速。

## 生产归属及真实调用链

- App 的 `agent_apps_auth` 通过宿主 state 别名直接持有 provider-apps `auth::state::State<Cards>`；初始化继续读取宿主偏好文件。`app_providers` 仍为 provider-apps `ListState` 的现有具体跨域槽位实例。没有复制 App/AppState、没有 Deref 或反向源码加载。
- `message::agent_apps_auth::update_at` → provider-apps `auth::controller::update` → 同库 actions/binding/state/flow。登录启动、轮询、取消、回填、账号身份、凭据、代际与回环拒绝策略保留。宿主 NativePorts 只接浏览器/剪贴板、偏好路径和本机凭据接入；`Origin` 是既有局部输入，`Effects` 仅打开授权页及失效供应商用量缓存。apps-ui auth 路径兼容重导出同一实现。
- 提供者 `edit::load/dispatch_save` → provider-apps `message::edit_dispatch` → 已有各 provider SDK。所有 provider 分支和原消息载荷保留；字段分派由 `edit::change` 接管，宿主 `Effect::Field` 经现有局部状态投影调用。
- `list::Open/Refresh/Close/QueryInput/Loaded` → provider-apps `ListState::update_catalog`。领域维护范围、草稿保护、代际、读取状态、错误和迟到响应拒绝；宿主只提供当前网关/项目/主机并装配 clear/read usage 和 toast。
- provider-apps `components::list::local_card` 持有局部弹窗优先级与关闭守卫，宿主和视图测试共用。删除旧 fields/utility wrapper 文件和旧 mod 挂载。宿主 list 只组合跨域用量、健康、聚合、模型目录、授权绑定与诊断界面，并映射消息。
- 原生路径纯验证归 provider-apps `message::paths::open_local`；文件选择器、执行主机归属核验和系统资源仍由宿主接入。聚合桌面 mapping_fields 直接使用已有 provider 视图，编辑性守卫归该函数。
- 外部历史事件的压缩/维护互斥、打开/关闭、时间分组和本地事件分类归 apps-ui `external_sessions::prepare_host_event`；宿主仅执行 OpenSettings 跨页效果、连接装配和本机归属核验。

## 依赖方向

provider-apps → settings-account / gateway-app-auth / 既有 API 类型；apps-ui → provider-apps。provider-apps 不依赖 apps-ui、vw-desktop 或宿主源码；无新增生产循环。账号数据结构复用 settings-account，未重新定义身份或密钥结构。
apps-ui 新增的 chat-view、settings-providers、config-ui、provider-profiles/extended、model-catalog、toml 仅为迁移测试所需 dev-dependencies。Cargo.lock 手工同步对应包依赖边，未执行 Cargo。

## 断言与测试注册映射

| 原测试组 | 新归属 | 保留证据 |
|---|---|---|
| agent_apps_auth_actions_tests | provider-apps `host_module_28_tests` | 原两个测试名、全部请求与断言；真实 auth aggregate/HTTP 端口，替换大型 App 夹具 |
| apps-ui auth/login_modal_tests、browser_tests | provider-apps auth 同名模块 | 原样随实现移动；原 cfg(test) 注册保留 |
| desktop_ui_refresh_12_tests | apps-ui `host_module_28_tests::desktop_ui_refresh_12_tests` | 67 个 assert 宏保留；中文、明暗主题、真实布局/draw/screenshot、值保护、点击和菜单断言保留 |
| claude_ui_refresh_13_tests | apps-ui 同名子模块 | 34 个 assert 宏保留；所有原测试名和绘制/交互代码保留 |
| agy_ui_refresh_14_tests | apps-ui 同名子模块 | 44 个 assert 宏保留；所有原测试名、声明编辑、加载/错误/空态及帮助测试保留 |
| mcp_panel_tests | apps-ui `host_module_28_tests::mcp_panel_tests` | 全部状态、服务身份、迟到结果、真实 draw 和协议断言整体迁移 |
| endpoint selection other_forms | apps-ui `host_module_28_tests::endpoint_forms_tests` | 原测试名与断言完整迁移；原 TOML 注释、其它部署及模型保护、Gemini 应用验证 |
| endpoint selection requires_save | 宿主原 tests 模块 | 跨域契约：诊断候选与测速响应经全局路由写回 provider draft、切换自定义端点、取消和最终保存请求；真实 App/update 与两类 SDK 连线不可由单库代替 |
| model cancel/routes/resumes | 宿主原 tests 模块 | 跨域契约：设置页退出触发清理、App catalog 切换、provider 消息路由和迟到结果；context/run/snapshot 为该契约的最小 SDK 载荷构造 |

UI `ui_support::Fixture` 仅持有真实 provider reducer State，没有 App/AppState 复刻。视图适配只负责将真实卡片和局部消息接到测试场景，字段编辑调用生产 reducer；请求无发送断言改为实际非阻塞 TCP listener 的 accept 检查。局部弹窗守卫直接调用生产 local_card，没有另写测试算法。原 draw、截图和字体加载代码保留；本轮未执行。

前轮去重：本域未找到 host_split 测试挂载；已有 `desktop_split_17_tests` 是授权列表读取/服务顺序、脱敏标签、凭据状态、权限错误与租户隔离，而迁入 actions 组验证默认切换/登出/删除/撤销/引用冲突和重试，非重复断言。provider state/list_tests 检查 begin/finish 身份隔离，UI 列表组验证实际绘制和事件，均继续保留。未按文件名推断等价或删断言。

## 静态验证与以后命令

已对本任务修改及新增 Rust 源码使用独立 rustfmt 解析（skip_children）；已核对本域 path 依赖存在、mod/cfg 注册、旧挂载删除、实际调用者与依赖方向，并完成限定本任务路径的 git diff --check。全工作区早期 diff --check 曾受其它任务 vw-agent EOF 空白影响，不修改无关内容。
未运行 Cargo（含 metadata/check/test/fmt）、单元测试、实机、窗口或最终回归。静态解析不等于编译/行为通过，跨平台系统效果及字体整形风险须在以后授权时验证。

以后获用户明确授权后：
- `python3 scripts/cargo_slot.py cargo test -p desktop-provider-apps --lib host_module_28_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-provider-apps --lib auth::login_modal_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-provider-apps --lib auth::browser_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-apps-ui --lib host_module_28_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib diagnostic_desktop_endpoint_selection_requires_save -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib diagnostic_desktop_cancel_keeps_terminal_state_routes_and_resumes -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`
以上命令本轮均未执行，没有通过数。

## baseline 同口径净减量

来源 6697 → 2680 行，净减少 4017 行；文件名口径生产候选 2786 → 2085 行，净减少 701 行。splitlines 包含空行/注释；未新建宿主文件。候选差异不能代替 cfg(test) 审计：删除 fields wrapper 的测试注册与生产转发分列上文，UI 测试减少未计为生产减少。真实生产迁出包括 auth controller、edit dispatch、catalog lifecycle、local_card、字段/路径守卫与历史预处理。

## 全来源逐函数/模块保留审查

下表列出每个来源的现存函数（或已删除模块）及保留原因；相同职责字段按前述 State/Origin/Effects 分组审查。

| 来源 | 基线→当前 | 现存函数/模块 | 归属或必要宿主理由 |
|---|---:|---|---|
| `crates/vw-desktop/src/app/components/agent_apps/aggregation_clients.rs` | 7→7 | overlay | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps/aggregation_desktop.rs` | 8→25 | overlay | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps/aggregation_omo.rs` | 7→7 | overlay | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps/aggregation_omp.rs` | 7→7 | overlay | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps/aggregation_pi.rs` | 7→7 | overlay | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps/aggregation_subagent.rs` | 7→7 | overlay | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps/daily_memory.rs` | 32→32 | message, entry, overlay, update, subscription | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps/external_sessions.rs` | 69→144 | message, view, update, update_compression | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps/hermes_dashboard.rs` | 49→49 | message, view, update | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps/hermes_memory.rs` | 45→45 | message, view, update | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps/hermes_memory_limits.rs` | 1→1 | mod/use 装配 | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps/openclaw_catalog.rs` | 60→60 | message, entry, overlay, update | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps/openclaw_defaults.rs` | 60→60 | message, entry, overlay, update | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps/openclaw_env.rs` | 59→59 | message, entry, overlay, update | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps/openclaw_model.rs` | 31→31 | message, entry, overlay, update | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps/openclaw_tools.rs` | 59→59 | message, entry, overlay, update | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps/workspace_files.rs` | 64→64 | message, entry, overlay, update | 既有 apps-ui 实现的 Element/Subscription 消息映射、连接/项目/主机装配及跨域效果；领域状态/更新/绘制已在库 |
| `crates/vw-desktop/src/app/components/agent_apps_diagnostics/endpoints_modal.rs` | 41→41 | message, overlay | 诊断真实局部类型→View 的借用投影及全局消息映射；实际绘制在 apps-ui |
| `crates/vw-desktop/src/app/components/agent_apps_diagnostics/endpoints_modal_tests.rs` | 488→398 | diagnostic_desktop_endpoint_selection_requires_save, render_panel | 仅保留上述真实 App/global routing 跨域契约；不复制领域算法 |
| `crates/vw-desktop/src/app/components/agent_apps_diagnostics/mcp_panel.rs` | 40→37 | message, view | 诊断真实局部类型→View 的借用投影及全局消息映射；实际绘制在 apps-ui |
| `crates/vw-desktop/src/app/components/agent_apps_diagnostics/mcp_panel_tests.rs` | 325→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/agent_apps_diagnostics/mod.rs` | 8→8 | mod/use 装配 | 诊断真实局部类型→View 的借用投影及全局消息映射；实际绘制在 apps-ui |
| `crates/vw-desktop/src/app/components/agent_apps_diagnostics/model_panel.rs` | 7→7 | overlay | 诊断真实局部类型→View 的借用投影及全局消息映射；实际绘制在 apps-ui |
| `crates/vw-desktop/src/app/components/agent_apps_diagnostics/model_panel_tests.rs` | 197→197 | context, run, snapshot, diagnostic_desktop_cancel_keeps_terminal_state_routes_and_resumes | 仅保留上述真实 App/global routing 跨域契约；不复制领域算法 |
| `crates/vw-desktop/src/app/components/app_providers/agy.rs` | 9→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/agy_ui_refresh_14_tests.rs` | 742→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/claude.rs` | 9→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/claude_ui_refresh_13_tests.rs` | 681→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/codex.rs` | 6→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/desktop.rs` | 16→9 | overlay | provider 卡片消息映射、跨域聚合/健康/用量/授权/诊断叠加及共享帮助 renderer 接口 |
| `crates/vw-desktop/src/app/components/app_providers/desktop_ui_refresh_12_tests.rs` | 1063→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/dsh.rs` | 6→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/edit.rs` | 41→39 | map, render_help, overlay_with_binding | provider 卡片消息映射、跨域聚合/健康/用量/授权/诊断叠加及共享帮助 renderer 接口 |
| `crates/vw-desktop/src/app/components/app_providers/endpoints.rs` | 7→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/env.rs` | 8→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/gemini.rs` | 6→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/grok.rs` | 6→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/hermes.rs` | 6→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/icons.rs` | 7→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/kimi.rs` | 6→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/list.rs` | 220→199 | provider_help, new, with_usage, overlay, map, map_editor | provider 卡片消息映射、跨域聚合/健康/用量/授权/诊断叠加及共享帮助 renderer 接口 |
| `crates/vw-desktop/src/app/components/app_providers/mcode.rs` | 6→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/mod.rs` | 35→3 | mod/use 装配 | provider 卡片消息映射、跨域聚合/健康/用量/授权/诊断叠加及共享帮助 renderer 接口 |
| `crates/vw-desktop/src/app/components/app_providers/mode.rs` | 8→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/omp.rs` | 6→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/openclaw.rs` | 6→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/opencode.rs` | 6→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/paths.rs` | 8→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/pi.rs` | 6→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/preview.rs` | 8→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/snippets.rs` | 8→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/components/app_providers/universal.rs` | 8→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/message/agent_apps_auth.rs` | 358→90 | message, update, update_at, store_preference, deliver, connect_auth | 授权领域重导出及 NativePorts（进程浏览器/剪贴板/偏好路径/凭据）和跨页面效果映射 |
| `crates/vw-desktop/src/app/message/agent_apps_auth_actions.rs` | 15→1 | mod/use 装配 | 授权领域重导出及 NativePorts（进程浏览器/剪贴板/偏好路径/凭据）和跨页面效果映射 |
| `crates/vw-desktop/src/app/message/agent_apps_auth_actions_tests.rs` | 415→0 | 已删除 | 旧实现/测试及挂载已删除；对应领域实现或测试见上文 |
| `crates/vw-desktop/src/app/message/agent_apps_auth_browser.rs` | 26→26 | preference_path, load_preference, store_preference, edit_paste, submit_paste, deliver | 授权领域重导出及 NativePorts（进程浏览器/剪贴板/偏好路径/凭据）和跨页面效果映射 |
| `crates/vw-desktop/src/app/message/app_providers/agy.rs` | 9→9 | finish | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/claude.rs` | 10→10 | finish | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/codex.rs` | 10→10 | finish | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/desktop.rs` | 8→8 | message, update | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/dsh.rs` | 10→10 | change_membership, membership_changed | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/edit.rs` | 374→59 | message, update, load, dispatch_save | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/endpoints.rs` | 19→19 | message, update | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/env.rs` | 7→7 | message, update | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/gemini.rs` | 4→4 | finish | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/grok.rs` | 4→4 | finish | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/hermes.rs` | 11→11 | finish, change_membership, membership_changed | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/icons.rs` | 7→7 | message, update | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/kimi.rs` | 11→11 | finish, change_membership, membership_changed | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/leave.rs` | 81→80 | busy, intercept, choose, resume, overlay | 全局导航消息到领域离开策略的分类及恢复/保存效果；busy/choose/intercept/resume 算法在库 |
| `crates/vw-desktop/src/app/message/app_providers/list.rs` | 318→379 | message, current, update, catalog | 全局 ListMessage 到领域/其它页面事件分发、当前网关/主机/项目投影和 usage/toast 效果装配 |
| `crates/vw-desktop/src/app/message/app_providers/mcode.rs` | 15→15 | finish, change_membership, membership_changed | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/mod.rs` | 50→50 | mod/use 装配 | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/mode.rs` | 9→9 | update, message | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/omp.rs` | 15→15 | finish, change_membership, membership_changed | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/openclaw.rs` | 15→15 | finish, change_membership, membership_changed | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/opencode.rs` | 15→15 | finish, change_membership, membership_changed | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/paths.rs` | 73→45 | update, pick_local, message, pick, open | 操作系统文件对话框、已验证本机主机连接及前台 Task 装配；纯路径安全规则在库 |
| `crates/vw-desktop/src/app/message/app_providers/pi.rs` | 15→15 | finish, change_membership, membership_changed | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/preview.rs` | 9→9 | update, message | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/provider_update.rs` | 141→124 | mapped, run, edit, desktop, change_membership, membership_changed, effect, request_is_current, endpoints, env, icons, hermes_change, hermes_changed, kimi_change, kimi_changed, list | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/snippets.rs` | 9→9 | update, message | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |
| `crates/vw-desktop/src/app/message/app_providers/universal.rs` | 12→12 | message, update | 真实局部 State 及 SDK 类型重导出、当前 scope 投影、领域 Effect 到宿主消息/反馈映射；不含提供者业务算法 |

额外直接调用者：宿主 `state/agent_apps_auth.rs` 仅将原兼容别名指向真实 provider 类型；`message/agent_apps_auth/binding.rs` 和 `flow.rs` 移除旧转发更新/登录启动，仅保留 entry 消息映射或类型重导出。这些均非新宿主替代文件。
