# 任务28阶段证据（未完成）

本任务尚未满足完整页面族验收，保留任务卡，不宣称完成或待回归。没有外部阻塞；剩余内容须继续在本任务实施。

## 已落实的调用链

- 宿主 `agent_apps_auth::update_at` → `desktop_provider_apps::auth::controller::update` → 同库 actions/binding/flow/state。原宿主登录转换已删除，保留 NativePorts 的浏览器、剪贴板、偏好路径及本机凭据接入；跨域打开授权页、清空供应商用量缓存及 toast 由宿主装配。
- desktop-apps-ui 原 auth 模块变为公开兼容重导出；授权状态、账号身份、登录轮询、取消、绑定和浏览器纯逻辑仅在 provider-apps 保留一份。
- 宿主 `app_providers::edit::load/dispatch_save` → provider-apps `message::edit_dispatch` → 已有各提供者 load/save；保留全部分支和消息载荷。
- provider-apps 依赖 settings-account 的真实账号类型，不依赖 desktop-apps-ui 或 vw-desktop，无循环依赖。Notice 由 provider-apps 提供，apps-ui 兼容重导出相同类型。

## 断言映射

- 原 `agent_apps_auth_actions_tests.rs` 整组迁至 `host_module_28_tests.rs`，保留两个测试名及全部断言：默认账号切换、本机登出、删除、撤销、非法账号、能力不足、断线、不确定结果、绑定引用、重试及租户过时代际隔离。网络端口仍使用真实 HTTP fixture；大型 App 夹具改为仅持有真实授权 aggregate 和 Notice 的测试夹具，驱动真实 controller。
- apps-ui `agent_apps_auth/login_modal_tests.rs` 和 `browser_tests.rs` 原样随实现迁至 provider-apps `auth/`，原注册保持，未删断言。
- 既有 `desktop_split_17_tests` 仍经兼容导出调用同一实现，尚需逐断言去重审计。

## 未完成项

- 提供者 list Open/Close/Refresh/Loaded 的状态策略仍在宿主；应完成领域更新入口及用量读取效果映射。
- 三组 provider UI refresh 测试、诊断测试及其真实绘制/字体/事件夹具仍在宿主，尚未迁移并完成逐断言映射。
- agent_apps 聚合/编辑器包装、provider 所有来源的逐函数保留理由尚未完成全面审计。原生路径范围检查仍需判断下沉职责。
- 未完成上述内容前不能删除任务文件；没有将其交给任务31或43代做。

## 静态验证

- 独立 rustfmt 解析了新增 controller、edit_dispatch、host_module_28_tests 及改接宿主文件；不代表类型检查。
- 本任务文件 `git diff --check -- <paths>` 结果见提交前检查。全工作区 diff --check 曾被其它任务的三个 vw-agent 测试文件 EOF 空白阻断，未修改无关文件。
- 遵守禁测：未执行 Cargo、单元测试、实机渲染或最终回归，不声称编译或行为通过。
- 后续获授权：`python3 scripts/cargo_slot.py cargo test -p desktop-provider-apps --lib host_module_28_tests -- --test-threads=1`；登录流程过滤 `auth::login_modal_tests`，浏览器过滤 `auth::browser_tests`；宿主目标检查由用户另行授权。

## baseline 同口径统计

来源总计 6697 → 5685，阶段净减少 1012 行；生产候选 2786 → 2189，净减少 597 行。未新建宿主文件。生产分类仍为文件名初筛，不能将测试减少视为生产减少。

| 来源 | 基线 | 当前 | 阶段归属 |
|---|---:|---:|---|
| `crates/vw-desktop/src/app/components/agent_apps/aggregation_clients.rs` | 7 | 7 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps/aggregation_desktop.rs` | 8 | 8 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps/aggregation_omo.rs` | 7 | 7 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps/aggregation_omp.rs` | 7 | 7 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps/aggregation_pi.rs` | 7 | 7 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps/aggregation_subagent.rs` | 7 | 7 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps/daily_memory.rs` | 32 | 32 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps/external_sessions.rs` | 69 | 69 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps/hermes_dashboard.rs` | 49 | 49 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps/hermes_memory.rs` | 45 | 45 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps/hermes_memory_limits.rs` | 1 | 1 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps/openclaw_catalog.rs` | 60 | 60 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps/openclaw_defaults.rs` | 60 | 60 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps/openclaw_env.rs` | 59 | 59 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps/openclaw_model.rs` | 31 | 31 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps/openclaw_tools.rs` | 59 | 59 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps/workspace_files.rs` | 64 | 64 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps_diagnostics/endpoints_modal.rs` | 41 | 41 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps_diagnostics/endpoints_modal_tests.rs` | 488 | 488 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps_diagnostics/mcp_panel.rs` | 40 | 40 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps_diagnostics/mcp_panel_tests.rs` | 325 | 325 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps_diagnostics/mod.rs` | 8 | 8 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps_diagnostics/model_panel.rs` | 7 | 7 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/agent_apps_diagnostics/model_panel_tests.rs` | 197 | 197 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/agy.rs` | 9 | 9 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/agy_ui_refresh_14_tests.rs` | 742 | 742 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/claude.rs` | 9 | 9 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/claude_ui_refresh_13_tests.rs` | 681 | 681 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/codex.rs` | 6 | 6 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/desktop.rs` | 16 | 16 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/desktop_ui_refresh_12_tests.rs` | 1063 | 1063 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/dsh.rs` | 6 | 6 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/edit.rs` | 41 | 41 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/endpoints.rs` | 7 | 7 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/env.rs` | 8 | 8 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/gemini.rs` | 6 | 6 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/grok.rs` | 6 | 6 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/hermes.rs` | 6 | 6 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/icons.rs` | 7 | 7 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/kimi.rs` | 6 | 6 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/list.rs` | 220 | 220 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/mcode.rs` | 6 | 6 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/mod.rs` | 35 | 35 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/mode.rs` | 8 | 8 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/omp.rs` | 6 | 6 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/openclaw.rs` | 6 | 6 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/opencode.rs` | 6 | 6 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/paths.rs` | 8 | 8 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/pi.rs` | 6 | 6 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/preview.rs` | 8 | 8 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/snippets.rs` | 8 | 8 | 待完整职责审计 |
| `crates/vw-desktop/src/app/components/app_providers/universal.rs` | 8 | 8 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/agent_apps_auth.rs` | 358 | 90 | 原生效果与跨域装配 |
| `crates/vw-desktop/src/app/message/agent_apps_auth_actions.rs` | 15 | 1 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/agent_apps_auth_actions_tests.rs` | 415 | 0 | 已迁领域测试 |
| `crates/vw-desktop/src/app/message/agent_apps_auth_browser.rs` | 26 | 26 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/agy.rs` | 9 | 9 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/claude.rs` | 10 | 10 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/codex.rs` | 10 | 10 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/desktop.rs` | 8 | 8 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/dsh.rs` | 10 | 10 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/edit.rs` | 374 | 59 | 领域更新/加载/保存转发 |
| `crates/vw-desktop/src/app/message/app_providers/endpoints.rs` | 19 | 19 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/env.rs` | 7 | 7 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/gemini.rs` | 4 | 4 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/grok.rs` | 4 | 4 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/hermes.rs` | 11 | 11 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/icons.rs` | 7 | 7 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/kimi.rs` | 11 | 11 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/leave.rs` | 81 | 81 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/list.rs` | 318 | 318 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/mcode.rs` | 15 | 15 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/mod.rs` | 50 | 50 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/mode.rs` | 9 | 9 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/omp.rs` | 15 | 15 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/openclaw.rs` | 15 | 15 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/opencode.rs` | 15 | 15 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/paths.rs` | 73 | 73 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/pi.rs` | 15 | 15 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/preview.rs` | 9 | 9 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/provider_update.rs` | 141 | 141 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/snippets.rs` | 9 | 9 | 待完整职责审计 |
| `crates/vw-desktop/src/app/message/app_providers/universal.rs` | 12 | 12 | 待完整职责审计 |
