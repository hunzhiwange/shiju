# 其他应用专属入口静态核对

任务 40，2026-11-14。直接依赖为任务 1 的 [正式合同](workspace-contracts.md)，已读取；来源映射与接口边界保持只读。来源根为同级 cc-switch（合同登记 ca58c0313360eae7c851fbd7553c1614c7af03e7）。本轮仅完成源码核对，不新增业务所有者，不宣称功能或 macOS 验收通过。

## 入口与唯一所有者

下列目标路径以仓库根为基准。供应商共同链路为 `crates/vw-agent-apps/src/providers/{app}.rs` → `crates/vw-agent/src/gateway/api/handlers/app_providers/{app}.rs` → `crates/vw-gateway-client/src/client/app_providers/{app}.rs` → `crates/vw-desktop/src/app/message/app_providers/{app}.rs`；五种 app 均已定位对应文件。网关前缀 `/v1`，领域前缀 `/app-providers/{app}`。存在入口不代表所有来源语义已等价。

| 应用 | 来源文件 / 命令 | 应用管理（母 3） | 供应商（母 4/5） | 账号（母 6） | 额度（母 7） |
| --- | --- | --- | --- | --- | --- |
| AGY | `commands/agy_account.rs::agy_account_is_running/agy_account_quota`、`agy_config.rs` | 复用 catalog/lifecycle；旧 is_running 是账号写回风险提示，不能只以启动按钮替代 | `agy_form_read/create/update/delete`、`agy_provider_save/switch`；目标 settings.json 与共享定制目录不同 | `auth/agy_login.rs::start_on_host/poll`、`agy_account.rs`、`agy_refresh.rs`；Keychain 与绑定账号归该领域 | 来源按绑定账号查询，运行状态和真实 quota 错误不可吞掉；当前未核实完整 quota API→SDK→Iced 等价链 |
| DSH | `commands/dsh.rs::dsh_account_status/sign_out/login_start/login_poll/login_cancel`、`dsh_config.rs` | 既有应用目录及启动生命周期 | `dsh_state`、form、save/add/remove-live；四个 profile 配置由现有适配拥有 | `auth/dsh_login.rs::start/poll/cancel`、`dsh_account.rs`；SDK `agent_apps_auth_flow.rs` 与 accounts 接点。五来源动作仍需逐一核验 | 本卡来源 dsh.rs 无独立额度命令；不能据此推断全应用无额度能力，通用供应商用量仍属母 7 |
| MiniMax | `mcode_config.rs`、`commands/misc.rs` 安装分支；Hermes MiniMax preset/key | AppId 为 Mcode；沿用安装/启动，不能新造 MiniMax 记忆 ID | `mcode_state/import`、form/editor/save/add/remove-live；Hermes key 归 `providers/hermes/form.rs` 与 `models/preset_hermes_mcode.rs` | 上述专属来源没有独立 MiniMax OAuth 命令；凭据为供应商字段，账号投影复用母 6，不编造登录流程 | `usage/coding/minimax.rs` 已有所有者；当前未核实额度 UI 与凭据绑定闭环 |
| Pi | `commands/pi.rs::get_pi_current_state/update_pi_provider_usage_script/get_pi_session_discovery`、`pi_config.rs`、`services/pi_state` | 既有 Pi 目录与生命周期 | `providers/pi.rs::state/read/patch`、SDK `pi_state/pi_usage_save`；POST config/state、PUT `{id}/usage-script` | pi.rs 无独立 OAuth 动作；配置凭据仍归供应商/授权所有者 | usage script 保存入口已定位；保存不是脚本执行、超时、拒绝或额度返回的证据 |
| OMP | `commands/omp.rs::get_omp_current_state`、`omp_config.rs` | 既有 Omp 生命周期 | `providers/omp.rs::declarations/state/patch`、`omp_state/import`；显式 providers 与 modelRoles.default 分开，内建 default 可以没有显式节点 | omp.rs 无独立 OAuth 动作；不能把 builtin default 当成已绑定账号 | omp.rs 无独立额度命令；通用供应商用量是否覆盖需母 7 核实，不能填写已通过 |

## 提示词与外部会话

| 应用 | 来源与唯一领域所有者 | 当前接点与缺项 |
| --- | --- | --- |
| AGY | `prompt_files.rs` 的 Agy 分支、`session_manager/providers/agy`；母 13/14 | `prompts/targets.rs::resolve` 指向 `~/.gemini/config/AGENTS.md`；不能误写供应商 settings 目录。外部会话解析、恢复、拒绝路径的完整生产链未在本次定位 |
| DSH | `prompt_files.rs` 的 Dsh 分支、`session_manager/providers/dsh`；母 13/14 | resolve 返回原生数据目录 AGENTS.md；尚无本次提示词读写/激活与外部会话 API→SDK→Iced 闭环证据 |
| MiniMax | `services/prompt.rs::upsert_mcode_prompt/enable_mcode_prompt`、`prompt_files.rs`、`session_manager/providers/mcode`；母 13/14 | resolve 返回 Mcode AGENTS.md。来源有限长及数据库/文件协调语义，不能仅凭路径解析宣称保留；外部会话闭环未核实 |
| Pi | `services/pi_prompt_files`、`services/prompt.rs`、`commands/pi.rs::get_pi_session_discovery`、`session_manager/providers/pi`；母 13/14 | resolve 明确 `pi_native_resources=true`，AGENTS.md 之外 SYSTEM.md、APPEND_SYSTEM.md、prompts/ 不得遗漏；current 从原生状态派生。发现接口、读写/激活与会话恢复生产链仍缺本次证据 |
| OMP | `prompt_files.rs` 的 Omp 分支、`session_manager/providers/omp`；母 13/14 | resolve 返回 Omp AGENTS.md；不沿用 Pi 的扩展提示词资源标记。提示词与外部会话完整链及错误路径未核实 |

五应用均有来源提示词与会话分支，不能填写“不适用”。`apps/launch/terminal_session.rs` 是启动终端会话，不能充当外部历史解析/恢复的证据。账号和额度栏的“无独立命令”只限所列来源文件，不是全仓不存在的结论。

Hermes 对照已读 `src/hooks/useHermes.ts`、`src/lib/api/hermes.ts` 及源 `App.tsx`：记忆、provider/model 缓存与 Web 面板有独立入口；离线触发 dashboard 确认，深层配置由 Hermes Web UI 所有，不能将其扩展成上述五应用的通用记忆页。MiniMax 的 Hermes provider key 仍归供应商，不产生 MiniMax workspace 功能。

## 证据及后续交接

历史应用管理证据分别见 `macos/agy.md`、`macos/dsh.md`、`macos/mcode.md`、`macos/pi.md`、`macos/omp.md`；仅作为生命周期历史索引，不替代供应商、账号、额度、提示词和外部会话的逐功能验收。授权历史文档见 `docs/vibewindow/agent-apps/auth/完成AGY浏览器授权.md`、`完成DSH浏览器授权.md`，本次不重签历史结论。

本次只读命令：RustCodeGraph status（退出 0，7988 文件）、explore（退出 0，宽查询有截断，不能作穷尽证明）；rg 定位真实源码与路由（有效匹配）。检索 services/session.rs 返回不存在，已改读真实 session_manager/mod.rs；shell auth*.rs 空 glob 已改为按文件清单定位，均不作为功能缺失证明。源码审查确认上述命名与职责，未执行文档断言脚本或任何测试。

缺项按母 3/4/6/7/13/14 消费接口交接：AGY 运行风险提示及绑定 quota、DSH 五动作状态机、MiniMax 内建 key/额度投影、Pi usage 执行与 discovery，以及五应用提示词/外部会话完整 API/SDK/Iced 和失败路径。缺项没有被通用 UI 替代，也没有被标为通过。本卡在用户本轮约束下完成静态盘点并删除；后续统一验证另行开展，不据卡片删除推断运行验收完成。
