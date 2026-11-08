# 供应商领域实施契约

2026-11-06，任务 1；只冻结来源、边界与注册位置，不实现运行行为。适用 macOS 27.0（26A428）、arm64；Windows/Linux 及其他架构未验收。

## 开工判定与证据边界

原共同前置要求母任务 1–3 已验收；[底座交付映射](../../vibewindow/agent-apps/底座交付映射.md) 又要求母任务 4 等领域迁移完成后才能评估母任务 2 完成，构成循环。2026-11-06 本次继续执行指令已裁定：以底座子计划 1–93 的实际验收作为供应商开工门槛，逐项消费真实接口；母任务 2 完整领域迁移仍未收口，不把本裁定写成母任务 2 已完成，不修改只读 plan.md。

母任务 1 的[快照验收](../../../crates/vw-agent-apps/macOS快照验收.md)固定来源；底座提交 `1d7f1b192` 汇总图标最小 API/SDK/Iced、双库实测及领域交接；[无窗口验收](../../vibewindow/agent-apps/macOS无窗口验收.md)记录 6 项真实 SQLite/PostgreSQL 测试；[桌面验收](../../vibewindow/agent-apps/macOS桌面验收.md)记录实际窗口。母任务 3 当前已完成，14 应用身份、权限与执行主机证据可追溯到[目录](task-01.md)、[API](task-33.md)、[启动凭据](task-31.md)、[远程隔离](task-45.md)。这些是既有证据，本轮未重跑其测试或 GUI，不据此宣布供应商切换实机通过。

底座原调度会话 `01a1ab42-fb8e-7af1-86f1-41f7a5eda2a2` 与任务 93 会话 `01a1ace2-4bbf-7d80-9d7c-04db07691247` 均已完成、归档，没有正在执行的前置收口会话。原底座计划目录已清理，以持久文档和提交为准。

## 固定来源、调用者与相邻测试

来源 `/Users/xiangmin/worker/code/cc-switch`，HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，本轮 `git status --short` 无输出。以下路径相对该来源 `src-tauri/`；主仓 upstream 为冻结参考，不能成为第二运行所有者。

| 来源入口 | 直接消费者与审查结论 | 相邻验证入口（只读，未运行） |
| --- | --- | --- |
| `src/provider.rs` 的 `Provider`、`ProviderMeta`、`with_id` | `commands/provider.rs`、`services/provider/mod.rs` 使用供应商行、settingsConfig、备注/排序/端点/图标；现有原生 `AppProviderWrite` 不能代表全部来源字段 | `tests/provider_service.rs` 的 Codex/Claude/Gemini 切换及字段保留；`src/database/dao/providers_tests.rs` 列表/端点顺序 |
| `src/commands/provider.rs` 的 `get_providers`、`get_current_provider`、`add_provider`、`update_provider`、`delete_provider` | `src/lib.rs` generate_handler 注册；前端 `src/lib/api/providers.ts` 的 invoke 直接调用；委托 `ProviderService::list/current/add_from_editor/update_from_editor/delete/switch`。Tauri State、AppHandle、原始错误字符串不可照搬 | `tests/provider_commands.rs`；`tests/provider_service.rs::provider_service_switch_codex_updates_live_and_config` 在隔离 home 造配置并断言真实文件；不是单纯返回值验证 |
| `src/mode/current.rs` 的 `Purpose`、`provider_for`、`is_referenced` | `ProviderService::current/delete`、`mode/controller.rs`、`direct_provider` 消费；Direct 与 InUse 分开，删除检查直连指针和代理引用 | `current_provider_is_read_through_provider_for` 限制旁路读取；controller 相邻测试覆盖直连/代理与 pending；`tests/provider_service.rs` 覆盖代理下同步不覆盖契约 |
| `src/mode/state.rs`、`src/mode/operation.rs` | `Mode` 仅 Direct/Proxy，stack 另存；pending 先预演暂存，发布开始后前滚，外部修改保留；不能宣称多文件原子回滚 | operation/controller 相邻测试；本计划任务 17–21、45–46 重新取得原生证据 |

## 实际底座与唯一所有者

| 领域 | 当前真实文件 / 符号 | 冻结决策和差异 |
| --- | --- | --- |
| 应用身份与目录 | `crates/vw-api-types/src/agent_apps/catalog.rs::AppId`；`vw-agent-apps/src/apps/catalog.rs::catalog` | 固定 14 ID：claude、claude-desktop、codex、gemini、grokbuild、opencode、openclaw、hermes、pi、mcode、kimi、dsh、agy、omp。grok 是命令名，不新增 ID 别名。目录注册不授予配置写入能力 |
| 供应商校验 | `vw-agent-apps/src/providers/mod.rs::validate_identity/validate_profile`；调用者 `apps/launch/provider.rs` | 沿用已有 providers 根，后续逐行为扩充，不另建业务所有者；当前校验只服务终端启动，部分应用明确 unsupported |
| DTO | `vw-api-types/src/app_providers.rs::AppProviderWrite/AppProviderSummary/ProviderProtocol` | Write 含 name/protocol/apiKey/baseUrl/model，Debug 脱敏；Summary 仅 id/appId/name/configured。没有完整 CRUD、当前状态、编辑 revision 或秘密保留/清除意图 |
| 存储 | `vw-agent/src/app_providers.rs::AppProviderStore`；`gateway/settings_store.rs::GatewaySettingsStore`；`global_database.rs::GlobalDatabase` | 复用配置选择的 SQLite/PostgreSQL，namespace=`app_providers.records`，键为 JSON 序列化 app（含引号）加冒号和 id，作用域为 platform/company；现有 save 为整条 upsert、无激活副作用；内部 list 已有，HTTP 列表尚无；禁止另开 CC 数据库 |
| 图标 | `vw-agent-apps/src/icons/`、`vw-agent/src/agent_apps/store.rs::IconLibraryStore` | 复用已交付图标库，`agent_apps.custom_icons/default`；供应商图标关联归任务 9，不能再复制图标增删改库；当前 read_icons 对损坏数组降为空/过滤坏项，不把早期“损坏必须报错”的设计当作已实现保证，任务 9/51 验证消费边界 |
| 存储并发 | `agent_apps/sqlite.rs`、`postgres.rs` 的图标事务适配 | 图标已有原子读改写，不等于供应商多文件事务。供应商 revision/冲突、主机本地 pending 和跨上下文同物理文件锁归任务 17–24；不能用现有 settings get+upsert 冒充原子编辑 |
| 依赖 | `vw-agent-apps/Cargo.toml`、`src/lib.rs` | 正式包已依赖 vw-api-types、vw-acp，未反向依赖 agent/desktop；无 features 声明，聚焦测试用默认 features；不注册 upstream |

## API、SDK、Iced 注册契约

当前公开供应商路径为 **`/v1/app-providers/{app}/{id}`**，GET 调 `read`，PUT 调 `save`，16 KiB 上限。实现 `vw-agent/src/gateway/api/handlers/app_providers.rs::router`；SDK `vw-gateway-client/src/client/app_providers_api.rs::app_provider_get/app_provider_save`。它与识矩模型 `/v1/provider` 独立，不新增同义命名空间。现有 PUT 不激活，不写 CLI live 文件，不证明后续 add/update 行为已完成。

handler 模块由 `gateway/api/handlers/mod.rs` 声明；`gateway/router.rs::handler_router` 和 `gateway/bootstrap.rs` 的 stateful 路由均 merge `app_providers::router`，各自外层挂载 `/v1`。后续动作在此根下按行为加明确方法/DTO，不拼接任意上游命令；具体新增动作路径随对应任务真实 handler 固定，内部 patch/stage/recovery 不增加公共后门。

SDK 根 `src/client/mod.rs` 已注册 `app_providers_api`；DTO 根 `src/lib.rs` 已注册 `app_providers`。现有单文件与计划同名目录可以并存：保持 `app_providers.rs` 为根，在文件中声明子模块，不能同时创建同名 `app_providers/mod.rs` 形成双根。handler 同理；SDK 当前根继续用 `app_providers_api.rs`，计划子文件落在 `client/app_providers/`；在该根用显式 `#[path = "app_providers/<文件>"] mod <模块>;` 声明，避免 Rust 默认查找 app_providers_api 子目录，不另注册重复实现。

Iced 尚无本计划供应商编辑根。后续 `components/app_providers/mod.rs` 从现有 `components/mod.rs` 声明；`state/app_providers/mod.rs` 从 `state.rs` 声明；`message/app_providers/mod.rs` 从 `message/mod.rs` 声明。实际导航/视图在 `components/system_settings.rs`，顶层 app 模块为 `app/mod.rs`，Message 在 `ui_types.rs`，状态字段在 `state/app_state.rs`，设置消息分发在 `message/settings/mod.rs::update`；只加必要分支。复用 `system_settings_common/`、`system_settings_providers/custom_model_modal.rs` 的统一样式与 `toast.rs`，不把模型供应商编辑器当成本领域存储所有者。任务 52 增加 `cc04-provider-window` 隔离真实渲染入口；任务 54–75 仍须明暗主题、中文与动态表单实测，不能用 Element 构造替代。

## 权限、秘密与执行主机

| 操作类别 | 当前事实 | 后续必须满足的边界 / 责任 |
| --- | --- | --- |
| 管理读写 | build_router 走 Basic，bootstrap stateful 走 skey，各自再解析 TenantScope。凭据禁用时可放行是当前事实；公司头只建立存储作用域，不证明公司成员身份 | 不宣称所有部署都已认证。任务 2–12、51 复核各正式认证入口，禁止客户端指定任意 namespace/SQL，跨租户摘要读取不得泄露资源 |
| 主机读取 | `handlers/agent_apps/access.rs::authorize` 使用管理员 policy 与 `x-agent-apps-token`，绑定 platform/company/host；policy 私有权限、无跟随链接；token 摘要比较 | 应用路径从授权 Host.home/roots 解析，不把客户端 HOME、cwd 或远端路径当本机。目录查询与文件读取权限不得混为一谈 |
| 主机写入 | access 现有授权类别是 lifecycle actions、preferences_write、launch 和 identities；**尚无供应商 live-config 写权限**；现有 app-providers PUT 不经过该主机授权，因为只写租户记录 | 任务 12/17/26 及后续适配在首次外部文件动作前接入明确的主机配置写授权与拒绝测试；不得把 preferences_write 或 launch 自动解释成任意配置写权限。这是本计划待交付能力，不是底座缺少可接入位置 |
| 主机不可达/不支持 | access 返回离线 503、binding=remote 501；现有远程隔离证据是分别连接两个本地绑定网关，非任意远程执行协议 | 绝不回退客户端本机；Windows/Linux 不外推。SDK 只发送授权目标身份，文件/进程/密钥由执行主机持有 |
| 密钥写入与读取 | AppProviderWrite 为只写 DTO，现有 save 整体替换，get_summary/list 不返回 apiKey/baseUrl/model；秘密当前作为 settings JSON 存储，没有加密证明 | 任务 3/4 必须定义显式 Keep/Replace/Clear 写入意图；旧 DTO 未提供该保证。读取、预览、差异、日志、失败详情脱敏；绝不从 Debug、HTTP 原始错误或空值推断保留/清除，不能声称底座已提供安全秘密编辑 |
| 账号与启动 | `app_providers::prepare_launch` 从当前租户正式记录读凭据；`launch_identities` 再校验身份许可 | 只引用已有账号，OAuth/托管登录归母 6；不得复制账号或从环境/模型供应商全局密钥回退。账号缺失应明确失败 |

本计划当前不新增权限字段或通用 trait。任务 2 先做脱敏列表/当前只读契约；真正文件动作之前由对应任务实现并验证上述最小授权，不可依赖“未来会有授权”提前执行。

## 错误、状态与路由

现有 app-providers 错误体只有安全 `message`：未找到 404，存储不可用/存储解析失败 503，其他非法输入或不支持投影 400；没有稳定 code 或关联 ID，SDK 目前返回 String。不得写成已交付统一类型错误。图标已有 `AgentAppsErrorCode`（invalid_input/400、not_found/404、conflict/409、storage_unavailable/503）和受控错误适配，可作为后续供应商 DTO/handler/SDK 同码传递的既有模式；新增冲突、pending、取消语义随真实行为任务确定，任务 51 验证，无原始 SQL、路径、密钥、令牌和错误链出站。

主机权限已有 401（token 无效）、403（无 grant，不泄露主机存在）、404（授权主机缺失）、503（policy/离线）、501（remote 不支持）；不要把其改写成领域成功或空集合。

来源 `Purpose::Direct` 表示直连文件指针，`InUse` 在代理模式取路由，否则直连；删除须检查两类引用。Mode 仅 direct/proxy，stack 是独立代理状态，附着契约与代理运行状态也不能等同。累加应用没有唯一 current 时显式表达，不能制造默认 ID；Kimi 使用原生当前项。任务 2 只交付当前可证明状态，任务 45/46 负责完整模式 DTO/配置事务；真实代理协议归母 8、聚合目录归母 10，缺失时明确依赖失败，不冒充全链完成。

pending 状态归执行主机，先预演全部文件再暂存/登记意图，发布前失败不改文件；开始发布后前滚恢复，保留外部改动，提交状态与路由分开。任务 17–21 的物理路径锁、私有权限和 revision 验证需跨租户/项目上下文共享，不能把 tenant 锁当物理文件锁。进度取消归任务 50；现有 SSE 没有持久重放，不借用它承诺可靠恢复。

## 14 应用 macOS 来源矩阵

下表是固定来源的格式、默认路径和操作约束，**不是原生实现或实机通过矩阵**。`~` 指授权执行主机 HOME；override/环境仅由授权执行端解析，任务 12 覆盖真实优先级和边界；不读取用户实际配置。任务号为本计划责任，未实现项不得上推为底座已支持。

| AppId / 配置任务 | 来源符号（src-tauri/src） | 格式 / 默认路径 | 不支持操作与特别边界 |
| --- | --- | --- | --- |
| claude / 26–27 | `config.rs::get_claude_settings_path` | JSON，`~/.claude/settings.json`；已有 legacy `claude.json` 可回退 | 仅关键供应商字段，不覆盖其余设置；登录失败恢复；OAuth 不归本任务 |
| claude-desktop / 33–34 | `claude_desktop_config.rs::macos_paths_from_home/paths_from_dirs` | JSON，`~/Library/Application Support/Claude{,-3p}/claude_desktop_config.json`；3p `configLibrary/<PROFILE_ID>.json`、`_meta.json` | 无终端供应商启动能力；模型映射/托管账号直连拒绝，依赖路由；不能擅自写官方登录 |
| codex / 28–30 | `codex_config.rs::get_codex_config_dir/get_codex_auth_path` | TOML `~/.codex/config.toml`，JSON `auth.json`，目录指针/登录暂存另列事务 | 登录保留与轮换明确，不清除未知登录；模型目录消费者不得复制母 5/10 的目录所有者 |
| gemini / 31 | `gemini_config.rs::get_gemini_env_path/get_gemini_settings_path` | dotenv `~/.gemini/.env` + JSON `settings.json` | 双文件同事务，保留非托管变量，不能把 .env 写成功当完整切换 |
| grokbuild / 32 | `grok_config.rs::get_grok_config_path` | TOML `~/.grok/config.toml` | 模型表定点修改；官方 xAI OAuth 不由配置卡伪造；grok 只是 CLI 名 |
| opencode / 35 | `opencode_config.rs::resolve_config_path` | JSONC 优先 `~/.config/opencode/opencode.jsonc`，其次 `opencode.json`，新建 JSON | 累加 provider，不制造唯一 current；保留注释与其他成员；会话 SQLite 不归供应商写入 |
| openclaw / 36 | `openclaw_config.rs::get_openclaw_config_path` | JSON5，`~/.openclaw/openclaw.json` | 累加供应商，不全量重写未知字段；JSON5 语法不能假定普通 JSON patch 已支持，任务 13/36 补验证 |
| hermes / 37 | `hermes_config.rs::get_hermes_dir/get_hermes_config_path` | YAML `~/.hermes/config.yaml`，显式 override 优先于 HERMES_HOME | custom_providers 累加，不删除其他成员；环境目录按来源语义，不任意展开或回退客户端 |
| pi / 38 | `pi_config/mod.rs::get_pi_agent_dir/get_pi_models_path/read_pi_native_defaults` | JSON/JSON5 读取，`~/.pi/agent/models.json`；`settings.json` 只读原生默认 | 只管理显式成员；不管理原生登录、active provider/model，不由新增成员选为 current；PI_CODING_AGENT_DIR/override 必须绝对路径 |
| omp / 39 | `omp_config.rs::get_omp_native_agent_dir/get_omp_models_path` | YAML `~/.omp/agent/models.yml` 优先于 `models.yaml`；config.yml/yaml 默认项只读 | 只显式 providers；不修改默认项/登录，不抢 CLI 的 legacy models.json 迁移；PI_CONFIG_DIR、PI_PROFILE、PI_CODING_AGENT_DIR 与 XDG 数据目录须分别处理 |
| mcode / 40 | `mcode_config.rs::data_dir/config_path` | YAML `~/.minimax/config.yaml` | 累加供应商与应用 schema 校验；不得以整文件覆盖替代节点编辑，安装/技能归其他领域 |
| kimi / 41 | `kimi_config.rs::data_dir/config_path/current_provider` | TOML `~/.kimi-code/config.toml`，凭据另在 credentials | 来源新增可不选择；当前项读原生值，不能仅写数据库冒充激活；保留原生凭据所有权 |
| dsh / 42 | `dsh_config.rs::data_dir/profile_path/target_paths` | YAML `~/.dsh/profiles/{web,desktop,headless,acp}/cordis.patch.yml`，DSH_HOME 覆盖 | web/desktop 准备，headless/acp 仅已初始化时写；sdk/sdk-minimal 不写；多 profile 物理文件去重，共享登录不复制 |
| agy / 43 | `agy_config.rs::settings_path/key_profile_dir` | JSON，`~/.gemini/antigravity-cli/settings.json`；来源 key 私有目录下同结构 | 无全局启用；第三方 key/地址只进所选子进程环境，私有 --gemini_dir，官方真实 settings 不写；来源 `.cc-switch/agy/` 不作为目标产品数据目录照搬 |

JSONC/JSON5、TOML、YAML、dotenv 的保注释和未知字段要求均需原生补丁实际验证，不能由 serde 往返或来源注释推出已支持。各应用配置 override 与私有目录目标根由任务 12/17/对应适配验证；本任务不发明运行目录或迁移旧结构。

## 后续文件落点和真实注册入口

以下是计划文件登记，不是已创建模块。现有根表优先于编号任务的笼统“补 mod.rs”；已有实现能接通时不复制。所有独立测试在相应父模块 `#[cfg(test)] mod ...` 注册，集成测试由 Cargo tests 发现；不把测试混入业务文件。

| 文件组 | 拟新增路径 / 复用根 | 真实父入口 |
| --- | --- | --- |
| B：领域 | `crates/vw-agent-apps/src/providers/<文件>` | 已有 `providers/mod.rs` → `src/lib.rs`；live 子目录由 providers 根声明 `live` 再由 `live/mod.rs` 声明子模块 |
| D：DTO | `crates/vw-api-types/src/app_providers/<文件>` | 已有 `src/app_providers.rs` → `src/lib.rs`，保留已有共享类型/调用者兼容 |
| H：handler | `crates/vw-agent/src/gateway/api/handlers/app_providers/<文件>` | 已有 `handlers/app_providers.rs` → `handlers/mod.rs` → router.rs 和 bootstrap.rs 两处 merge；所有公开行为均走授权 |
| S：SDK | `crates/vw-gateway-client/src/client/app_providers/<文件>` | 已有 `client/app_providers_api.rs` 以显式 path 声明子模块 → `client/mod.rs`；保持 GatewayClient 方法为唯一桌面入口 |
| U：UI 三层 | `crates/vw-desktop/src/app/{components,state,message}/app_providers/<文件>` | 各自新 app_providers/mod.rs 接 components/mod.rs、state.rs、message/mod.rs；视图 system_settings.rs，消息 settings/mod.rs，根 app/mod.rs |
| agent 存储适配 | 沿用 `crates/vw-agent/src/app_providers.rs`，需要拆分时在同名目录挂子模块 | `vw-agent/src/lib.rs` 已有 app_providers 根；GlobalDatabase/settings 工厂不换所有者，业务包不持有第二数据库 |

下表从当前编号任务的“文件”段逐项登记；B/D/H/S 同名时列 BDHS，内部 live 只列 B，不暗示要暴露同名公共 API。各任务证据统一写本目录 provider-macos-acceptance.md 自己的条目。

| 任务 | 拟文件（按上表根展开） |
| --- | --- |
| 1 | 本契约及验收文档 |
| 2 | BDHS：`catalog.rs` |
| 3 | BDHS：`create.rs` |
| 4 | BDHS：`update.rs` |
| 5 | BDHS：`delete.rs` |
| 6 | BDHS：`duplicate.rs` |
| 7 | BDHS：`presentation.rs` |
| 8 | BDHS：`endpoints.rs` |
| 9 | BDHS：`icons.rs` |
| 10 | BDHS：`universal.rs` |
| 11 | BDHS：`universal_sync.rs` |
| 12 | BDHS：`paths.rs` |
| 13 | B：`live/patch_json.rs` |
| 14 | B：`live/patch_toml.rs` |
| 15 | B：`live/patch_yaml.rs` |
| 16 | B：`live/patch_dotenv.rs` |
| 17 | B：`live/write_guard.rs` |
| 18 | B：`live/stage.rs` |
| 19 | B：`live/commit.rs` |
| 20 | B：`live/recovery.rs` |
| 21 | B：`live/concurrency.rs` |
| 22 | BDHS：`preview.rs` |
| 23 | BDHS：`validation.rs` |
| 24 | BDHS：`editor_merge.rs` |
| 25 | BDHS：`snippets.rs` |
| 26 | BDHS：`claude.rs` |
| 27 | BDHS：`claude_login.rs` |
| 28 | BDHS：`codex.rs` |
| 29 | BDHS：`codex_login.rs` |
| 30 | BDHS：`codex_catalog.rs` |
| 31 | BDHS：`gemini.rs` |
| 32 | BDHS：`grok.rs` |
| 33 | BDHS：`claude_desktop.rs` |
| 34 | BDHS：`desktop_mapping.rs` |
| 35 | BDHS：`opencode.rs` |
| 36 | BDHS：`openclaw.rs` |
| 37 | BDHS：`hermes.rs` |
| 38 | BDHS：`pi.rs` |
| 39 | BDHS：`omp.rs` |
| 40 | BDHS：`mcode.rs` |
| 41 | BDHS：`kimi.rs` |
| 42 | BDHS：`dsh.rs` |
| 43 | BDHS：`agy.rs` |
| 44 | BDHS：`import.rs` |
| 45 | BDHS：`mode_state.rs` |
| 46 | BDHS：`mode_apply.rs` |
| 47 | BDHS：`env_diagnose.rs` |
| 48 | BDHS：`env_clean.rs` |
| 49 | BDHS：`env_restore.rs` |
| 50 | BDHS：`operations.rs` |
| 51 | H：`api_security_tests.rs` |
| 52 | U：`list.rs`；`crates/vw-desktop/src/cc04_provider_window_main.rs`，Cargo.toml 注册 cc04-provider-window（harness=false/test=false） |
| 53 | U：`edit.rs` |
| 54 | U：`claude.rs` |
| 55 | U：`gemini.rs` |
| 56 | U：`codex.rs` |
| 57 | U：`grok.rs` |
| 58 | U：`opencode.rs` |
| 59 | U：`openclaw.rs` |
| 60 | U：`hermes.rs` |
| 61 | U：`pi.rs` |
| 62 | U：`omp.rs` |
| 63 | U：`mcode.rs` |
| 64 | U：`kimi.rs` |
| 65 | U：`dsh.rs` |
| 66 | U：`agy.rs` |
| 67 | U：`desktop.rs` |
| 68 | U：`endpoints.rs` |
| 69 | U：`icons.rs` |
| 70 | U：`universal.rs` |
| 71 | U：`snippets.rs` |
| 72 | U：`preview.rs` |
| 73 | U：`env.rs` |
| 74 | U：`mode.rs` |
| 75 | U：`paths.rs` |
| 76 | `crates/vw-agent/tests/cc04_accept_transactions.rs` |
| 77 | 本目录正式验收文档（来源覆盖核对） |

## 完成判定与缺项归属

本任务清单已完成：14 ID/格式/路径/限制、管理与主机读写权限、秘密写入/脱敏、错误体与状态/路由差异、底座证据和全部拟文件注册均有来源。没有无法定位的必需底座入口；尚缺的是本计划要实现的完整 DTO/CRUD/显式秘密编辑（2–12、23）、授权路径和写锁（12、17）、补丁与 pending（13–24）、应用适配（26–44）、模式（45–46）、环境恢复（47–49）、操作进度（50）、权限脱敏回归（51）、真实供应商 Iced 与 macOS 交叉验收（52–77）。这些标为待实现，不因本任务完成而改为通过。

母任务 2 完整迁移/母任务 8 代理/母任务 10 聚合/母任务 6 OAuth 的完整领域验收仍未由此收口。后续遇到真实依赖缺失应给出明确拒绝和对应责任，不创建伪支持。任务 77 不得把未验证交叉项记作母任务完整。

## 供应商事务交接给统一账号授权批次 33

macOS 的 Kimi 官方 OAuth 供应商已接入主机事务。执行端使用
`crates/vw-agent/src/app_providers/kimi_binding.rs` 的
`AppProviderStore::{bind_kimi_account,kimi_account_binding,use_kimi_account_binding}`。
接入方从同一授权决策取得租户、主体、目标主机、HOME 和私有根。现有
`gateway/api/handlers/agent_apps/mod.rs::provider_editor_authorization` 同时返回主体和执行路径，
分别按配置读写权限检查。将这些值用于 `AuthAccountScope`、`ExecutionHostId` 和
`KimiNativeTarget`。账号 ID 只查询该作用域的 `agent_apps.auth_accounts.v2`。

首绑传空预期账号，替换传当前绑定的账号 ID。提交前对照目录与主机原生连接，核验 Kimi 官方
区域端点和 OAuth 槽。绑定要求有效 grant。同一 grant 直接比较，独立登录通过官方身份接口验证同一身份。提交通过
`KimiFiles` 和 `ProviderTransactions` 同步原生模型选择与绑定引用。相同绑定重试只确认已提交结果。
消费时由现有 Kimi 适配器重新核验原生 grant 身份，然后选择模型。无绑定时读取返回空，
消费返回 `NotFound`。当前支持范围为已存在原生官方 profile 和匹配的受管 grant。
统一账号语义和 HTTP 接入分别由批次 33、42 负责。

## 官方卡账号引用与删除事务交接

冻结来源为 cc-switch `ca58c0313360eae7c851fbd7553c1614c7af03e7`。
`src/lib/managedAccountUsage.ts::findManagedAccountUsers` 汇总 Claude、Claude Desktop、
Codex 和 AGY 的托管引用。Claude 官方卡的本机快照和隔离启动见
`services/provider/claude_official.rs`，Codex 官方卡和解除引用见
`services/provider/codex_direct.rs`、`commands/auth.rs::remove_codex_oauth_account_with_switch_lock`，
AGY 启动前切号见 `commands/misc.rs::switch_agy_to_bound_account`。
本地冻结合同采用显式绑定，账号仍被引用时返回 `account_in_use`。

| 应用与服务 | 执行端入口 | 参考范围与消费 |
| --- | --- | --- |
| Claude 官方卡，`claude_official` | `app_providers/claude_binding.rs` 的 `bind_claude_account`、`claude_account_binding`、`with_claude_provider_unreferenced` | 同租户、主体、主机、服务的 v2 元数据及独立目录。现有供应商切换和隔离启动消费绑定。目录引用与默认槽引用分别保护 |
| AGY 官方卡，`agy_oauth` | `app_providers/agy_binding.rs` 的 `bind_agy_account`、`agy_account_binding`、`with_agy_provider_unreferenced` | 同作用域钥匙串记录及原生独占槽。正式供应商启动切换所选账号，并持有槽锁至进程退出 |
| 原生 OpenAI 配置的 Codex 卡，`codex_oauth` | `app_providers/codex_binding.rs` 的 `bind_codex_account`、`codex_account_binding`、`with_codex_provider_unreferenced` | 同作用域 v2 元数据及 workspace、subject 凭据核验。使用隔离 `CODEX_HOME` 和原有轮换回写，不写主机默认 `auth.json` |
| Kimi 官方卡，`kimi_oauth` | 上节入口及 `app_providers/kimi_binding.rs::with_kimi_provider_unreferenced` | Kimi 主机状态、进行中的事务及受管账号租约 |

各 `with_*_provider_unreferenced` 在绑定写入方使用的账号租约或钥匙串锁内检查引用，
由账号删除方提供同锁回调。拒绝只返回稳定错误码。Claude 默认槽保持被引用，直至切换到另一份
已验证的受管登录。当前卡的绑定改动需先切离该卡。未知来源的既有默认登录保留并拒绝托管切换。
历史 Claude 绑定继续沿旧入口和旧授权读取原记录，历史 ID 不自动映射 v2 主体。

Claude 默认槽切换和 AGY 托管启动要求共同的管理员访问策略文件明确登记独占槽。
在 `SHIJU_AGENT_APPS_ACCESS_FILE` 所指文件的执行主机条目中使用 `native_login_owners`，
为每种服务仅登记一个已有主机授权的 `service`、`platform_id`、`company_id` 和 `principal`。
其他逻辑主机或主体的同应用启动无法借用该槽。后续统一账号接入从同一策略决定解析主体与路径。
现有启动和供应商路由继续使用原 API、SDK 和 UI。

Codex 官方卡的全局直连 checkout 尚缺默认 `auth.json` 的可证明归属和外部轮换对账，
现有直连入口拒绝隔离绑定。GitHub Copilot、xAI 及 Claude Desktop 的代理注入路径依赖
生产 `ProviderRouteRuntime`，当前执行端没有该消费者。DSH 为单一共享 grant。
这些路径未创建可用绑定或退回受管默认账号。
