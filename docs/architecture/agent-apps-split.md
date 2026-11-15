# vw-agent-apps 拆分边界与完整清单

任务 1 基线：2026-11-14。仅完成源码边界分析，尚未搬迁。后续编号任务在本文更新实际落点、留存符号和接收编号；不修改 plan.md。

未运行 Cargo、rustc、单元测试、实机测试或最终回归；静态审阅不代表编译或行为验收。Cargo.lock 未修改，清单一致性及三平台条件留待授权统一验收。

## 任务 9：模型预设与能力迁移结果

15 个模块（capabilities、codex_template、identity、pi_thinking、preset_catalog、
八类 preset 实现、preset_search、user_agent_presets）及其独立测试和整个 presets
资源目录现归 `crates/libs/app-model-catalog/src/`。models 根 validate、
codex_projection 一并下沉；原 models 模块显式重导出，lib.rs 继续经 models
提供原路径，不增加顶层别名。配置填充、发现与 metadata 服务仍由任务 26 接收。

任务 8 移交的 media_sanitize::prepare 已下沉本包，原 routing 位置重导出；
本包依赖 app-routing-policy 执行 replace，能力目录没有反向依赖。纯端点投影、
身份校验和 User-Agent 校验直接使用 app-provider-validation。其它直接依赖为
vw-api-types、serde、serde_json（arbitrary_precision）及仅用于 URL 解析的 reqwest
（关闭默认特性）；toml_edit 仅为模板测试依赖。原包这些依赖仍有真实消费者。

codex_template_tests 的 prepare_provider、preset_catalog_tests 的 native_projection、
preset_omp_tests 的 complete_native_catalog 涉及上层配置编排，断言原样留在门面，
分别由任务 24、11、25 接收；同文件其它测试随目录迁移，未新增或丢弃测试。
私有 Pi thinking 规则仍在同包，未扩大可见性。真实调用者包括 fill_pi、metadata、
providers/codex_catalog、aggregation/exit_restore 及网关模型发现，经门面共享类型。

静态证据：Python 标准库对照 15 组测试函数名及计数全部保留，所有 include_str
资源相对路径存在，直接 path 清单引用均存在；系统 Python 无 tomllib，未执行 TOML
解析。源码检索未发现新包依赖门面，workspace 成员及直接依赖已注册，git diff --check
无空白错误。原有 cfg 未变，无 HTTP 或进程发现实现迁入。
未运行 Cargo、单元及实机测试，统一验收待授权：新包与调用者编译、迁移单测、
媒体策略编排、三平台行为及锁文件一致性。Cargo.lock 未修改。

## 任务 8：路由载荷规则迁移结果

所有权表任务 8 的 budget_rectify、cache_policy、classifier_guard、compaction_replay、
gemini_schema、inline_think、lifecycle_state、moonshot_schema、private_params、
thinking_options 实现及对应八个独立测试现归 `crates/libs/app-routing-policy/src/`。
根 supports_app 叶子函数一并下沉，原 routing/mod.rs 显式 re-export，lib.rs 继续
通过 routing 保留调用路径，无需新增顶层别名。override_validate_tests、user_agent_tests
一并迁入新包，直接引用任务 4 已下沉的 app-provider-validation（仅 dev 依赖）。

media_sanitize 的 repair、replace 及私有遍历/拒绝识别规则下沉；replace 因原门面
prepare 的真实跨包调用最小公开。prepare 仍在原位置解析模型能力再调用 replace，
已由任务 9 接收其能力编排，依赖 app-routing-policy，无反向依赖模型目录。
target_model::resolve 及其私有 helper、target_model_tests 暂留原包，由任务 10 接收；
其 desktop_mapping::defaults 尚未下沉，测试还引用任务 27 的 model_ids，任务 10
如未闭合须按既有桌面映射交接到任务 28。无复制实现或门面回依赖。

新包依赖 base64、sha2、serde_json（arbitrary_precision）、vw-api-types 和仅用于
Url 解析的 reqwest（关闭默认特性）；测试依赖 app-provider-validation。原包这些
生产依赖仍有其它调用，因此保留。未改变平台条件、错误语义或资源路径，无 include
资源；独立测试仍仅 cfg(test) 注册。调用者 app_routing/dispatch、provider_publication、
model_relay 以及网关路由授权继续使用原门面，类型身份保持一致。

静态证据：迁移文件与 HEAD 内容对照（仅两个测试 import 改为底层包）、模块注册、
真实调用者和依赖检索、清单路径审阅、git diff --check。系统 Python 无 tomllib，未解析 TOML；
使用标准库核对两个包的全部直接路径均存在。媒体拆分仅提取 prepare，
replace 增加公开可见性及说明。未运行 Cargo、单元及实机测试，统一验收待授权：
新包及直接调用者编译、迁移测试、后继编排收口、锁文件一致性及三平台行为。

## 任务 7：高可用状态与重试迁移结果

所有权表任务 7 的 circuit_breaker、health、retry 实现及三个独立测试现归
`crates/libs/app-availability-core/src/`，原 high_availability 模块重导出同名模块，
lib.rs 继续通过 high_availability 提供原路径，无需新增顶层别名。
health 与 circuit_breaker 同包互引，保留私有可见性；health_tests 的包内导入随之调整。
retry 继续仅在非 wasm 平台导出，最多 10 次重试、5 秒退避上限和截止时间保持不变。
原根模块的 queue、selection 及其测试仍归任务 30，无本任务遗留符号。

新包生产依赖仅 vw-api-types 与非 wasm tokio（macros/rt/time）；非 wasm 测试
依赖 tokio（io-util/net/sync）及 reqwest（关闭默认特性，rustls）。原包仍有其它
调用者使用 tokio、reqwest 与 vw-api-types，因此保留其依赖。真实消费者包括
selection、网关高可用服务与 app_routing，经原门面共享相同类型身份。

静态证据：六文件移动与原内容对照、原模块导出和调用者检索、清单路径审阅、
无门面回依赖、测试注册及 cfg 对照、git diff --check。无 include 资源路径。
当前 Python 无 tomllib，未执行 TOML 解析；标准库检查直接依赖路径均存在。
未运行 Cargo、单元及实机测试，统一验收待授权：新包和调用者编译、迁移测试、
锁文件一致性及三平台行为。

## 任务 6：应用目录与偏好迁移结果

`apps/{acp_projection,catalog,preferences}.rs` 及三个独立测试现归
`crates/libs/app-catalog/src/`；原 apps 模块 re-export 保留现有调用路径。
AppId、目录能力、ACP 注册及审查门禁、偏好归一化和错误语义保持不变。
原 lib.rs 通过 apps 提供接口，无需新增顶层别名。

偏好校验复用 `discovery/wsl` 的纯规则，为避免依赖发现层，WslShell、WslTarget、
parse_target、resolve_target 及原独立测试一并迁入 app-catalog::wsl；resolve_target
仅检查 cfg!(windows) 并解析，无探测或执行。原 discovery re-export 保留类型身份
及调用路径，任务 32 不再重复迁移这两个文件，其发现/探测职责仍归任务 32。
所有权表中上述八个原文件均由 app-catalog 接收，无留存实现。

新包仅依赖 serde_json（保留 arbitrary_precision）、vw-api-types、vw-acp；
根 workspace 和原包直接路径依赖已接线，原包 vw-acp 已无其它调用，移除。
真实消费者包括 discovery、lifecycle、launch、providers/live/recovery、
data_management/preferences 及网关目录/偏好接口，继续沿用原门面。
新包无启动、安装或门面依赖；原测试仍独立且仅在 cfg(test) 注册，
WSL 平台拒绝条件保持不变，无 include 资源路径。

静态证据：迁移内容与 HEAD 对照（仅偏好 import 路径调整）、导出及调用者检索、
清单路径与依赖人工审阅、Python 标准库路径检查、git diff --check。
系统 Python 无 tomllib，未执行 TOML 解析。未运行 Cargo、单元及实机测试，
统一验收待授权：新包和直接调用者编译、迁移测试、锁文件一致性、三平台行为。

## 任务 5：配置补丁迁移结果

所有权表任务 5 的十个 `providers/live/patch_*.rs` 文件（dotenv、JSON、JSON5、
TOML、YAML 实现与各自独立测试）现归 `crates/libs/app-config-patch/src/`。
新 lib.rs 显式注册模块和测试；原 providers/live 以 re-export 保留路径，
原 lib.rs 继续通过 providers 提供接口，无需顶层别名。无留存实现或后继接收项。

JSON/JSON5 的 object_value、replace_member_preserving_trivia、
replace_object_preserving_trivia，以及 YAML 的 patch_mcp_server、
patch_hermes_memory_enabled、has_yaml_reference_tokens 因真实跨包调用最小提升为 pub。
调用者包括 providers、aggregation、mcp/adapters、models 和 OpenClaw/Hermes 配置模块；
其余实现及独立测试原样搬迁，无 include 资源路径。JSON5 模块、测试及依赖保持 macOS
条件，JSON 的 trivia 替换和 YAML MCP 入口保留原 cfg。未知字段、注释与错误语义未改。

workspace 和原包直接路径依赖已接线。新包依赖 serde_json（arbitrary_precision）、
serde_yaml、toml_edit、vw-api-types、macOS json-five，测试依赖 tempfile。
原包 json-five 已无其他调用，移除；其他格式依赖仍有调用，保留。无反向依赖。
静态审阅包含迁移内容对照、导出/调用者检索、清单人工审阅、Python 标准库路径检查和
差异空白检查（系统 Python 缺少 tomllib，未执行 TOML 解析）。未运行 Cargo、单元及实机测试，统一验收待授权：新库与调用者编译、
迁移单测、锁文件一致性及三平台条件和原配置行为。

## 任务 2：图标规则迁移结果

`icons/{library,name,validation}.rs` 及三个对应独立测试原样迁入
`crates/libs/app-icons/src/`；原 `icons/mod.rs` 成为新包 `src/lib.rs`，保留显式模块树与测试注册。
所有权表中任务 2 的七个原路径现由 `app-icons` 所有；原目录已移除，无留存符号或后继接收项。

根 workspace 注册 `app-icons`，`vw-agent-apps` 直接路径依赖并以
`pub use app_icons as icons` 保留调用路径。新包仅依赖 base64、png、vw-api-types，
后者不依赖原门面，无反向依赖。原包 base64 仍有生产调用，png 仅供应商测试使用，移至 dev-dependencies。

公开接口保持 `library::{add_icon,rename_icon,delete_icon}`、`name::normalize_icon_name`、
`validation::{is_valid_custom_icon,CUSTOM_ICON_PREFIX,MAX_CUSTOM_ICON_LENGTH}`。
真实调用者 providers/icons、providers/universal、agent_apps/store 和 gateway 相邻测试沿用门面。
PNG 完整解码、64 KiB URI / 256 像素边界、名称限制和错误码均未改动。
迁移文件无平台 cfg 或 include 资源路径；三个测试模块仍只在 cfg(test) 注册。

静态审阅：文件内容与原版逐字对照，源码检索确认导出和调用接线、无重复实现或反向依赖，
Python 标准库 TOML 审阅确认成员及直接依赖路径存在；git diff --check 无空白错误。
未运行 Cargo、单元及实机测试，统一验收待授权：新包和调用者编译、迁移测试、锁文件一致性及平台行为。

## 任务 3：应用路径规则迁移结果

所有权表中 `providers/paths.rs`、`providers/paths_tests.rs` 现归
`crates/libs/app-config-paths/src/`；新增 lib.rs 显式注册实现和独立测试。
原 providers 模块以 `pub use app_config_paths as paths` 保留调用路径，
无需新增 lib.rs 顶层别名。没有留存实现或后继接收项。

`execution_environment`、`read` 保持公开；真实跨包调用所需的 `bounded`、
unix `open_beneath`、macOS `agy_profile_directory` 从 pub(crate) 最小提升为 pub。
profile/probe 仍私有。调用者包括 providers/live、providers/agy、auth、aggregation、
models、mcp、skills、sessions、prompts、usage 和 data_management，继续沿用原门面。
环境白名单、HOME 约束、描述符相对遍历、符号链接拒绝、错误及平台 cfg 均保持。
测试内容原样搬迁，仍在 macOS 条件下注册；无 include 资源路径。

新包仅依赖 vw-api-types、unix libc 及 macOS base64/sha2/serde_json，
测试依赖 tempfile；原包这些依赖仍有其它真实调用，不删除。
根 workspace 与原包直接路径依赖已接线；无反向依赖或重复实现。
静态审阅使用源码差异、导出/调用者检索、Python 标准库 TOML 路径与依赖图检查，
并执行 git diff --check。未运行 Cargo、单元及实机测试，统一验收待授权：
新包与调用者编译、迁移单测、锁文件一致性及三平台路径行为。

## 任务 4：供应商校验规则迁移结果

所有权表中 providers/endpoints.rs、endpoints_tests.rs、request_options.rs、
validation.rs 已迁入 crates/libs/app-provider-validation/src/。
providers/mod.rs 的 validate_identity、validate_profile 及 universal_sync 的
projected_endpoint 纯规则随包迁移；旧位置以显式 re-export 保留真实调用路径。
原 lib.rs 仍通过 providers 模块提供接口，无需增加顶层别名。

validation_tests.rs 的普通/代理请求头、端点与清空规则测试迁入新包独立测试文件。
其余五个测试依赖上层 create/update、preview、apps::launch，保留原包注册：
创建/更新测试由任务 29 接收，预览测试由任务 10 接收，启动测试由任务 35 接收。
universal_sync 的同步编排仍由任务 11 接收，不依赖新包以外的上层实现。
Kimi 原生 validate_identity 返回事务 WriteError，保留任务 24 接收，未改变其规则。

protected_header、header_value、native_request 因 models、usage 与原生投影的
真实跨包调用最小提升为 pub，其余私有函数保持私有。真实调用者 auth、models、
providers、usage、apps::launch 继续沿用门面；无重复实现及反向依赖。
新库仅依赖 reqwest（关闭默认特性）、serde_json（保留 arbitrary_precision）、
toml_edit、vw-api-types，测试依赖 tempfile。原包这些依赖均仍有其它调用，保留。
迁移内容没有平台 cfg 或 include 资源路径；测试保持 cfg(test) 独立注册。

静态证据：源码/符号检索、原始函数体及测试差异对照、Python 标准库 TOML
成员和依赖路径审阅、git diff --check。未运行 Cargo、单元及实机测试，
统一验收待授权：新库与直接调用者编译、迁移测试、锁文件一致性与三平台行为。

## 规模与责任

源码根下共 883 个文件，其中 871 个 Rust 文件、317 个独立测试文件，共 217017 行；非 Rust 资源 12 个。独立测试按文件名 `_tests.rs` 计数，集成测试另列。

主责任是文件的最终接收任务；根模块内混合职责按下节符号拆分，不能因文件归属而整文件提前搬迁。任务 1 不改实现与清单。

## 具体拆边与延后接收

| 当前引用/闭环 | 叶子落点与上层接收 |
| --- | --- |
| auth::{flow,accounts,kimi,copilot_token,gemini_refresh,xai_refresh} → providers::validate_identity；models::{identity,discovery_url,discovery_http} → validation | validate_identity、validate_profile 随任务 4 进入 app-provider-validation；调用方只依赖规则包。validate_profile 本身仅依赖 API 类型与 validation，不依赖原生写入。 |
| auth::{codex_device,codex_refresh} → codex_login_material::parse_codex_jwt_claims | 任务 14 接收 JWT claims 解析与身份提取的内聚纯逻辑，任务 23 登录接线依赖认证包；不保留认证→登录服务反向依赖。当前提供者文件主责任 23，须拆符号并更新表。 |
| credential_file → claude_login::LoginMaterial；claude_login → auth::{claude_keychain,…}；live::claude_directory → LoginHost | LoginMaterial/LoginHost 的现有定义与必要内聚方法由任务 13/15 接收至认证基础/Claude 包；任务 23 接收登录编排。任务 12 只搬不依赖 LoginHost 的目录受控操作；依赖登录的目录方法留任务 23，禁止 app-config-write 依赖 app-provider-login。 |
| live::commit::{AppState,ModeState} → mode_state::StackState；mode_state::observe/ClientConfiguration::contract → commit::{AppState,Contract,Mode} | StackState 数据与 is_empty 随任务 12 的持久化状态下沉；任务 29 接收 observe 与模式服务，依赖写入状态及任务 10 投影。project_binding 的 observe 调用属于任务 29 接收的绑定编排，不可整文件塞入写入叶子。 |
| live::commit → providers::operations::{planned,publication,advanced} → apps::lifecycle::operations::Operation | 现有操作进度/发布门禁的内聚状态随任务 12 下沉，任务 33 生命周期和任务 29 供应商服务共同依赖；不新增回调 trait，也不让写入包依赖生命周期服务。 |
| live::commit → kimi::validate_identity；live::recovery → apps::catalog::catalog | 身份规则任务 4 接收；catalog 任务 6 接收后，任务 12 可直接依赖 app-catalog。Kimi 原生写入留任务 24。 |
| auth::{kimi_account,dsh_account,claude_rotation} → 原生供应商/事务；供应商登录与 kimi_form → auth | 任务 18/20/15 先搬纯协议及凭据逻辑；依赖原生写入的账户绑定、旋转发布留任务 22，未具备原生依赖时继续留至任务 24 后由任务 29 收完；kimi_form 的 binding_region 纯解析随任务 18 下沉，任务 24 只依赖该叶子。 |
| providers::agy → apps::launch::{ProviderLaunch,LaunchMaterial}；apps::launch::provider → providers::agy::Lease | LaunchMaterial 请求数据保持任务 35 所有；agy 中消费它并构造 ProviderLaunch 的启动组装随任务 35 移动，任务 24 仅保留 Scope、Lease 与原生配置操作。 |
| discovery::read → lifecycle::{local_agy,local_omp}::owns；lifecycle::wsl → discovery::wsl；local_agy/local_omp → uninstall::native::is_native_executable | 路径所有权识别及可执行文件纯判定随任务 32 的发现叶子接收，任务 33/34 依赖发现包；WSL 类型/解析已由任务 6 提前迁入 app-catalog::wsl，任务 32 保留发现/探测，生命周期执行归 33。任务 31 node_tests 对 discovery::version 探测的集成场景留任务 32，避免测试反向依赖。 |
| models::preset_catalog → universal_sync::projected_endpoint、validation::endpoint；providers::codex_catalog → models::codex_projection | projected_endpoint 的纯端点投影随任务 4 接收；models 根 validate/codex_projection 随任务 9 下沉目录包，任务 24/28 依赖目录包，不依赖任务 26 发现填充服务。 |
| providers::desktop_mapping::catalog → aggregation::model_ids::{Decoded,decode,encode}；aggregation 发布 → providers 原生配置 | model_ids 编解码随任务 27 下沉 app-aggregation-catalog；任务 24 暂留相关桌面映射到任务 28 接收（依赖 27 和已迁移原生叶子），任务 29 只从 28 调用聚合服务。 |
| aggregation::{exit_restore,claude_publish} → claude_login::configuration_has_pending_login；aggregation → codex_catalog/CodexFiles、live::commit | 发布/恢复任务 28 依赖任务 23 登录、24 原生、12 写入及9目录；与聚合目录 27 分开，27 不反向依赖 28。 |
| providers::{codex_catalog::form,grok_form} → preview::editor_category | 表单纯投影与分类规则任务 24 接收所需内聚实现；跨适配器 preview 分发留任务 29。无法独立的表单编排一并留 29，不让 native 包依赖 providers 服务。 |

以上落点均搬迁已有符号，不新增配置或抽象。编号不是无条件拓扑序；前置依赖未就绪时保留原实现并记录，指定接收者负责在依赖齐备时收完。任务 72 最后核对所有延后项，不能把编排永久留在门面。

## 根函数与分发

| 符号/模块 | 接收任务 | 边界 |
| --- | --- | --- |
| routing::supports_app | 8 | 仅匹配 AppId 四个变体，直接随 app-routing-policy 下沉，无供应商服务依赖。 |
| models::{validate,codex_projection} | 9 | 校验目录身份与序列化已保存行，只依赖任务 4 和 API/serde_json。 |
| mcp::native_capability | 40 | 保留应用能力匹配，与 native 适配/分发同属 app-mcp；目录规则 39 不调用分发层。 |
| sessions/providers/mod.rs | 54 | 子适配器声明分别归 45—50；保留必要重导出，分发/会话操作归 app-sessions。私有 blocks/messages 随所属解析器移动。 |
| providers::{validate_identity,validate_profile} | 4 | 根文件主责任 29，函数必须提前迁出。 |
| providers/mod.rs 与 live/mod.rs | 29 / 12 | 分别保留服务分发与写入模块树；各叶子被搬走后只作导出，最终 72 清理。 |
| lib.rs | 72 | macOS cfg 与真实外部路径保持；不放业务实现。 |

## 目标包职责

| 任务 | 目标包 | 职责 |
| --- | --- | --- |
| 2 | `app-icons` | 图标规则 |
| 3 | `app-config-paths` | 应用路径规则 |
| 4 | `app-provider-validation` | 供应商校验规则 |
| 5 | `app-config-patch` | 配置补丁 |
| 6 | `app-catalog` | 应用目录与偏好 |
| 7 | `app-availability-core` | 高可用状态与重试 |
| 8 | `app-routing-policy` | 路由载荷规则 |
| 9 | `app-model-catalog` | 模型预设与能力 |
| 10 | `app-provider-projection` | 供应商原生投影 |
| 11 | `app-provider-core` | 供应商基础业务 |
| 12 | `app-config-write` | 受控配置写入 |
| 13 | `app-auth-core` | 凭据文件与认证流程基础 |
| 14 | `app-auth-codex` | Codex认证 |
| 15 | `app-auth-claude` | Claude认证 |
| 16 | `app-auth-copilot` | Copilot认证 |
| 17 | `app-auth-xai` | XAI认证 |
| 18 | `app-auth-kimi` | Kimi认证 |
| 19 | `app-auth-agy` | AGY认证 |
| 20 | `app-auth-dsh` | DSH认证 |
| 21 | `app-auth-gemini` | Gemini认证 |
| 22 | `app-auth` | 认证账户编排 |
| 23 | `app-provider-login` | 供应商登录接线 |
| 24 | `app-provider-native` | 供应商原生适配 |
| 25 | `app-provider-environment` | 供应商环境治理 |
| 26 | `app-models` | 模型发现与填充 |
| 27 | `app-aggregation-catalog` | 聚合目录规则 |
| 28 | `app-aggregation` | 聚合发布与恢复 |
| 29 | `app-providers` | 供应商模式与服务 |
| 30 | `app-availability` | 高可用选择编排 |
| 31 | `app-versions` | 应用版本查询 |
| 32 | `app-discovery` | 应用发现 |
| 33 | `app-lifecycle` | 应用生命周期 |
| 34 | `app-uninstall` | 应用卸载 |
| 35 | `app-launch` | 应用启动 |
| 36 | `app-operations` | 应用操作门面 |
| 37 | `app-workspace-memory` | 工作区与记忆文件 |
| 38 | `app-openclaw` | OpenClaw配置工具 |
| 39 | `app-mcp-catalog` | MCP目录规则 |
| 40 | `app-mcp` | MCP原生适配与分发 |
| 41 | `app-prompts` | 提示词管理 |
| 42 | `app-skill-catalog` | 技能目录与发现 |
| 43 | `app-skills` | 技能部署与更新 |
| 44 | `app-session-core` | 会话解析基础 |
| 45 | `app-session-codex` | Codex会话解析 |
| 46 | `app-session-claude` | Claude会话解析 |
| 47 | `app-session-gemini` | Gemini会话解析 |
| 48 | `app-session-pi` | Pi会话解析 |
| 49 | `app-session-sqlite` | SQLite会话解析 |
| 50 | `app-session-native` | 其它应用会话解析 |
| 51 | `app-session-terminal` | 会话终端集成 |
| 52 | `app-session-migration` | 会话迁移 |
| 53 | `app-session-content` | 会话内容与存储 |
| 54 | `app-sessions` | 会话操作与维护 |
| 55 | `app-usage-core` | 用量计费规则 |
| 56 | `app-usage-store` | 用量账本与查询 |
| 57 | `app-usage-balances` | 用量余额查询 |
| 58 | `app-usage-coding` | 用量套餐查询 |
| 59 | `app-usage-subscriptions` | 用量订阅查询 |
| 60 | `app-usage-scripts` | 用量脚本执行 |
| 61 | `app-usage-codex` | Codex用量导入 |
| 62 | `app-usage-import` | 其它应用用量导入 |
| 63 | `app-usage` | 用量扫描编排 |
| 64 | `app-sync-core` | 设备同步格式与归档 |
| 65 | `app-device-sync` | 设备同步传输 |
| 66 | `app-data-format` | 数据包格式与暂存 |
| 67 | `app-data-backup` | 数据备份文件 |
| 68 | `app-data-projection` | 数据领域投影 |
| 69 | `app-data-management` | 数据恢复编排 |
| 70 | `app-project-profiles` | 项目配置档案 |
| 71 | `app-diagnostics` | 应用诊断 |

## 全量文件所有权

路径相对 `crates/vw-agent-apps/src/`。当前落点均为原路径，状态均为未搬迁；后续任务逐行更新实际落点。资源与独立测试必须随实现迁移。

| 文件 | 行数 | 主责任任务 |
| --- | ---: | ---: |
| `aggregation/catalog_ack.rs` | 60 | 28 |
| `aggregation/catalog_ack_tests.rs` | 222 | 28 |
| `aggregation/catalog_generation.rs` | 223 | 28 |
| `aggregation/catalog_generation_tests.rs` | 198 | 28 |
| `aggregation/claude_catalog.rs` | 200 | 27 |
| `aggregation/claude_catalog_tests.rs` | 107 | 27 |
| `aggregation/claude_publish.rs` | 258 | 28 |
| `aggregation/claude_publish_tests.rs` | 359 | 28 |
| `aggregation/claude_refresh_tests.rs` | 140 | 28 |
| `aggregation/codex_catalog.rs` | 236 | 27 |
| `aggregation/codex_catalog_tests.rs` | 154 | 27 |
| `aggregation/codex_classic.rs` | 49 | 27 |
| `aggregation/codex_classic_tests.rs` | 99 | 27 |
| `aggregation/codex_effort_tests.rs` | 86 | 28 |
| `aggregation/codex_publish.rs` | 213 | 28 |
| `aggregation/codex_publish_tests.rs` | 348 | 28 |
| `aggregation/codex_resync_tests.rs` | 26 | 28 |
| `aggregation/codex_review.rs` | 52 | 27 |
| `aggregation/codex_review_tests.rs` | 70 | 27 |
| `aggregation/codex_settings.rs` | 175 | 27 |
| `aggregation/codex_subagent.rs` | 127 | 27 |
| `aggregation/codex_subagent_tests.rs` | 94 | 27 |
| `aggregation/daemon_restart.rs` | 415 | 28 |
| `aggregation/daemon_restart_desktop_tests.rs` | 145 | 28 |
| `aggregation/daemon_restart_tests.rs` | 319 | 28 |
| `aggregation/default_provider_tests.rs` | 180 | 28 |
| `aggregation/exit_restore.rs` | 340 | 28 |
| `aggregation/exit_restore_tests.rs` | 317 | 28 |
| `aggregation/failover_guard.rs` | 27 | 28 |
| `aggregation/failover_guard_tests.rs` | 36 | 28 |
| `aggregation/lifecycle_restore.rs` | 123 | 28 |
| `aggregation/lifecycle_restore_tests.rs` | 402 | 28 |
| `aggregation/member_keys.rs` | 47 | 27 |
| `aggregation/member_keys_tests.rs` | 78 | 27 |
| `aggregation/member_validation.rs` | 57 | 27 |
| `aggregation/member_validation_tests.rs` | 72 | 27 |
| `aggregation/membership.rs` | 205 | 28 |
| `aggregation/membership_tests.rs` | 220 | 28 |
| `aggregation/mod.rs` | 134 | 28 |
| `aggregation/model_ids.rs` | 118 | 27 |
| `aggregation/model_ids_tests.rs` | 71 | 27 |
| `aggregation/omo_read.rs` | 162 | 27 |
| `aggregation/omo_read_tests.rs` | 109 | 27 |
| `aggregation/omo_scope.rs` | 64 | 28 |
| `aggregation/omo_slim.rs` | 293 | 28 |
| `aggregation/omo_slim_tests.rs` | 313 | 28 |
| `aggregation/omo_write.rs` | 198 | 28 |
| `aggregation/omo_write_tests.rs` | 301 | 28 |
| `aggregation/preview.rs` | 326 | 28 |
| `aggregation/preview_tests.rs` | 299 | 28 |
| `aggregation/resolve.rs` | 110 | 28 |
| `aggregation/resolve_tests.rs` | 164 | 28 |
| `aggregation/stale_clients.rs` | 207 | 28 |
| `aggregation/stale_clients_tests.rs` | 204 | 28 |
| `apps/acp_projection.rs` → `app-catalog/src/acp_projection.rs` | 60 | 6 |
| `apps/acp_projection_tests.rs` → `app-catalog/src/acp_projection_tests.rs` | 36 | 6 |
| `apps/catalog.rs` → `app-catalog/src/catalog.rs` | 115 | 6 |
| `apps/catalog_tests.rs` → `app-catalog/src/catalog_tests.rs` | 56 | 6 |
| `apps/discovery/claude_desktop.rs` | 99 | 32 |
| `apps/discovery/claude_desktop_tests.rs` | 78 | 32 |
| `apps/discovery/mod.rs` | 27 | 32 |
| `apps/discovery/posix.rs` | 185 | 32 |
| `apps/discovery/posix_tests.rs` | 115 | 32 |
| `apps/discovery/read.rs` | 162 | 32 |
| `apps/discovery/report.rs` | 128 | 32 |
| `apps/discovery/report_tests.rs` | 124 | 32 |
| `apps/discovery/version.rs` | 150 | 32 |
| `apps/discovery/version_tests.rs` | 144 | 32 |
| `apps/discovery/windows.rs` | 109 | 32 |
| `apps/discovery/windows_tests.rs` | 45 | 32 |
| `apps/discovery/wsl.rs` → `app-catalog/src/wsl.rs` | 90 | 6（提前接收 32 的纯规则） |
| `apps/discovery/wsl_tests.rs` → `app-catalog/src/wsl_tests.rs` | 69 | 6 |
| `apps/launch/account.rs` | 267 | 35 |
| `apps/launch/account_lifecycle_tests.rs` | 203 | 35 |
| `apps/launch/account_tests.rs` | 344 | 35 |
| `apps/launch/host.rs` | 45 | 35 |
| `apps/launch/host_tests.rs` | 46 | 35 |
| `apps/launch/mod.rs` | 32 | 35 |
| `apps/launch/posix.rs` | 283 | 35 |
| `apps/launch/posix_tests.rs` | 150 | 35 |
| `apps/launch/provider.rs` | 311 | 35 |
| `apps/launch/provider_omp_tests.rs` | 148 | 35 |
| `apps/launch/provider_tests.rs` | 322 | 35 |
| `apps/launch/request.rs` | 112 | 35 |
| `apps/launch/request_tests.rs` | 202 | 35 |
| `apps/launch/terminal_session.rs` | 277 | 35 |
| `apps/launch/terminal_session_tests.rs` | 77 | 35 |
| `apps/launch/windows.rs` | 194 | 35 |
| `apps/launch/windows_tests.rs` | 131 | 35 |
| `apps/lifecycle/coordinator.rs` | 68 | 33 |
| `apps/lifecycle/coordinator_tests.rs` | 130 | 33 |
| `apps/lifecycle/local.rs` | 293 | 33 |
| `apps/lifecycle/local_agy.rs` | 132 | 33 |
| `apps/lifecycle/local_agy_tests.rs` | 238 | 33 |
| `apps/lifecycle/local_node.rs` | 50 | 33 |
| `apps/lifecycle/local_node_tests.rs` | 69 | 33 |
| `apps/lifecycle/local_omp.rs` | 132 | 33 |
| `apps/lifecycle/local_omp_tests.rs` | 245 | 33 |
| `apps/lifecycle/mod.rs` | 45 | 33 |
| `apps/lifecycle/native_grok_mcode.rs` | 82 | 33 |
| `apps/lifecycle/native_grok_mcode_tests.rs` | 132 | 33 |
| `apps/lifecycle/native_hermes_agy.rs` | 81 | 33 |
| `apps/lifecycle/native_hermes_agy_tests.rs` | 103 | 33 |
| `apps/lifecycle/native_kimi_omp.rs` | 94 | 33 |
| `apps/lifecycle/native_kimi_omp_tests.rs` | 113 | 33 |
| `apps/lifecycle/native_primary.rs` | 48 | 33 |
| `apps/lifecycle/native_primary_tests.rs` | 85 | 33 |
| `apps/lifecycle/npm.rs` | 53 | 33 |
| `apps/lifecycle/npm_tests.rs` | 55 | 33 |
| `apps/lifecycle/operations.rs` | 178 | 33 |
| `apps/lifecycle/operations_tests.rs` | 98 | 33 |
| `apps/lifecycle/process.rs` | 334 | 33 |
| `apps/lifecycle/process_tests.rs` | 159 | 33 |
| `apps/lifecycle/update.rs` | 332 | 33 |
| `apps/lifecycle/update_tests.rs` | 275 | 33 |
| `apps/lifecycle/wsl.rs` | 222 | 33 |
| `apps/lifecycle/wsl_tests.rs` | 278 | 33 |
| `apps/mod.rs` | 18 | 36 |
| `apps/preferences.rs` → `app-catalog/src/preferences.rs` | 147 | 6 |
| `apps/preferences_tests.rs` → `app-catalog/src/preferences_tests.rs` | 213 | 6 |
| `apps/uninstall/execute.rs` | 115 | 34 |
| `apps/uninstall/execute_tests.rs` | 110 | 34 |
| `apps/uninstall/mod.rs` | 11 | 34 |
| `apps/uninstall/native.rs` | 272 | 34 |
| `apps/uninstall/native_tests.rs` | 153 | 34 |
| `apps/uninstall/packages.rs` | 202 | 34 |
| `apps/uninstall/packages_tests.rs` | 189 | 34 |
| `apps/versions/latest.rs` | 218 | 31 |
| `apps/versions/latest_tests.rs` | 112 | 31 |
| `apps/versions/mod.rs` | 2 | 31 |
| `apps/versions/node.rs` | 265 | 31 |
| `apps/versions/node_tests.rs` | 136 | 31 |
| `auth/accounts.rs` | 72 | 22 |
| `auth/accounts_tests.rs` | 282 | 22 |
| `auth/agy_account.rs` | 504 | 19 |
| `auth/agy_account/logout.rs` | 182 | 19 |
| `auth/agy_account_tests.rs` | 296 | 19 |
| `auth/agy_login.rs` | 404 | 19 |
| `auth/agy_login_tests.rs` | 386 | 19 |
| `auth/agy_provider_binding_tests.rs` | 47 | 19 |
| `auth/agy_refresh.rs` | 152 | 19 |
| `auth/agy_refresh_tests.rs` | 221 | 19 |
| `auth/binding_tests.rs` | 139 | 22 |
| `auth/browser_callback.rs` | 331 | 13 |
| `auth/claude_keychain.rs` | 86 | 15 |
| `auth/claude_login.rs` | 572 | 15 |
| `auth/claude_login_files.rs` | 247 | 15 |
| `auth/claude_login_tests.rs` | 389 | 15 |
| `auth/claude_recover_tests.rs` | 302 | 15 |
| `auth/claude_rotation.rs` | 113 | 15 |
| `auth/claude_rotation_tests.rs` | 209 | 15 |
| `auth/claude_snapshot_tests.rs` | 216 | 15 |
| `auth/codex_browser.rs` | 201 | 14 |
| `auth/codex_browser_tests.rs` | 385 | 14 |
| `auth/codex_device.rs` | 493 | 14 |
| `auth/codex_device_tests.rs` | 553 | 14 |
| `auth/codex_refresh.rs` | 303 | 14 |
| `auth/codex_refresh_tests.rs` | 488 | 14 |
| `auth/copilot.rs` | 504 | 16 |
| `auth/copilot_tests.rs` | 531 | 16 |
| `auth/copilot_token.rs` | 220 | 16 |
| `auth/copilot_token_tests.rs` | 302 | 16 |
| `auth/credential_file.rs` | 636 | 13 |
| `auth/credential_file/host.rs` | 480 | 13 |
| `auth/credential_file_tests.rs` | 200 | 13 |
| `auth/credential_lease.rs` | 439 | 13 |
| `auth/credential_lease_tests.rs` | 61 | 13 |
| `auth/dsh_account.rs` | 482 | 20 |
| `auth/dsh_account_tests.rs` | 460 | 20 |
| `auth/dsh_login.rs` | 410 | 20 |
| `auth/dsh_login_tests.rs` | 331 | 20 |
| `auth/flow.rs` | 562 | 13 |
| `auth/flow_tests.rs` | 749 | 13 |
| `auth/gemini_refresh.rs` | 179 | 21 |
| `auth/gemini_refresh_tests.rs` | 63 | 21 |
| `auth/keychain.rs` | 266 | 13 |
| `auth/keychain_tests.rs` | 76 | 13 |
| `auth/kimi.rs` | 440 | 18 |
| `auth/kimi_account.rs` | 533 | 18 |
| `auth/kimi_account/binding.rs` | 215 | 18 |
| `auth/kimi_account/binding_tests.rs` | 84 | 18 |
| `auth/kimi_account/logout.rs` | 156 | 18 |
| `auth/kimi_account/native.rs` | 208 | 18 |
| `auth/kimi_account/usage.rs` | 81 | 18 |
| `auth/kimi_account_tests.rs` | 321 | 18 |
| `auth/kimi_tests.rs` | 370 | 18 |
| `auth/logout.rs` | 158 | 22 |
| `auth/logout_tests.rs` | 472 | 22 |
| `auth/mod.rs` | 134 | 22 |
| `auth/xai.rs` | 395 | 17 |
| `auth/xai_refresh.rs` | 161 | 17 |
| `auth/xai_refresh_tests.rs` | 252 | 17 |
| `auth/xai_tests.rs` | 349 | 17 |
| `daily_memory_files.rs` | 76 | 37 |
| `daily_memory_files_tests.rs` | 143 | 37 |
| `daily_memory_list.rs` | 49 | 37 |
| `daily_memory_list_tests.rs` | 67 | 37 |
| `daily_memory_search.rs` | 139 | 37 |
| `daily_memory_search_tests.rs` | 77 | 37 |
| `data_management/apply.rs` | 70 | 69 |
| `data_management/apply_tests.rs` | 109 | 69 |
| `data_management/backup_api.rs` | 130 | 69 |
| `data_management/backup_create.rs` | 201 | 67 |
| `data_management/backup_create_files.rs` | 253 | 67 |
| `data_management/backup_create_tests.rs` | 175 | 67 |
| `data_management/backup_delete.rs` | 77 | 67 |
| `data_management/backup_delete_tests.rs` | 86 | 67 |
| `data_management/backup_list.rs` | 78 | 67 |
| `data_management/backup_list_tests.rs` | 118 | 67 |
| `data_management/backup_rename.rs` | 30 | 67 |
| `data_management/backup_rename_tests.rs` | 161 | 67 |
| `data_management/backup_restore.rs` | 68 | 69 |
| `data_management/backup_restore_tests.rs` | 73 | 69 |
| `data_management/export.rs` | 137 | 67 |
| `data_management/export_files.rs` | 47 | 67 |
| `data_management/export_tests.rs` | 46 | 67 |
| `data_management/fixtures/schema_18.sql` | 72 | 66 |
| `data_management/location_delete.rs` | 202 | 67 |
| `data_management/location_delete_tests.rs` | 97 | 67 |
| `data_management/locations.rs` | 279 | 67 |
| `data_management/locations_tests.rs` | 88 | 67 |
| `data_management/mcp.rs` | 154 | 68 |
| `data_management/mcp_tests.rs` | 55 | 68 |
| `data_management/mod.rs` | 139 | 69 |
| `data_management/operations.rs` | 146 | 69 |
| `data_management/package_import.rs` | 47 | 69 |
| `data_management/package_import_tests.rs` | 51 | 69 |
| `data_management/periodic.rs` | 37 | 69 |
| `data_management/portable_policy.rs` | 190 | 66 |
| `data_management/portable_policy_tests.rs` | 88 | 66 |
| `data_management/post_import.rs` | 72 | 69 |
| `data_management/preferences.rs` | 92 | 68 |
| `data_management/preview.rs` | 154 | 66 |
| `data_management/preview_tests.rs` | 63 | 66 |
| `data_management/profiles.rs` | 139 | 68 |
| `data_management/profiles_tests.rs` | 60 | 68 |
| `data_management/prompts.rs` | 70 | 68 |
| `data_management/prompts_tests.rs` | 27 | 68 |
| `data_management/providers.rs` | 216 | 68 |
| `data_management/providers_tests.rs` | 54 | 68 |
| `data_management/recovery.rs` | 69 | 69 |
| `data_management/recovery_tests.rs` | 79 | 69 |
| `data_management/retention.rs` | 68 | 67 |
| `data_management/retention_tests.rs` | 101 | 67 |
| `data_management/rollback.rs` | 94 | 69 |
| `data_management/rollback_tests.rs` | 71 | 69 |
| `data_management/safety_snapshot.rs` | 249 | 67 |
| `data_management/safety_snapshot_files.rs` | 134 | 67 |
| `data_management/safety_snapshot_tests.rs` | 232 | 67 |
| `data_management/shared_snapshot.rs` | 75 | 66 |
| `data_management/skills.rs` | 479 | 68 |
| `data_management/skills_tests.rs` | 163 | 68 |
| `data_management/source.rs` | 245 | 66 |
| `data_management/source_files.rs` | 124 | 66 |
| `data_management/source_json.rs` | 55 | 66 |
| `data_management/source_tests.rs` | 175 | 66 |
| `data_management/source_upgrade.rs` | 72 | 66 |
| `data_management/source_upgrade_tests.rs` | 52 | 66 |
| `data_management/sql_stage.rs` | 75 | 66 |
| `data_management/sql_stage_tests.rs` | 73 | 66 |
| `data_management/usage.rs` | 142 | 68 |
| `data_management/usage_tests.rs` | 44 | 68 |
| `device_sync/archive_extract.rs` | 151 | 64 |
| `device_sync/archive_pack.rs` | 210 | 64 |
| `device_sync/artifact.rs` | 132 | 64 |
| `device_sync/conflict.rs` | 120 | 64 |
| `device_sync/manifest.rs` | 119 | 64 |
| `device_sync/mod.rs` | 18 | 65 |
| `device_sync/s3_config.rs` | 130 | 64 |
| `device_sync/s3_remote.rs` | 62 | 65 |
| `device_sync/s3_signing.rs` | 163 | 64 |
| `device_sync/s3_transport.rs` | 190 | 65 |
| `device_sync/s3_transport/download.rs` | 205 | 65 |
| `device_sync/skill_update.rs` | 117 | 65 |
| `device_sync/webdav_config.rs` | 113 | 64 |
| `device_sync/webdav_remote.rs` | 91 | 65 |
| `device_sync/webdav_transport.rs` | 237 | 65 |
| `device_sync/webdav_transport/download.rs` | 147 | 65 |
| `device_sync/webdav_url.rs` | 52 | 64 |
| `diagnostics/endpoints.rs` | 135 | 71 |
| `diagnostics/endpoints_tests.rs` | 95 | 71 |
| `diagnostics/mod.rs` | 5 | 71 |
| `hermes_dashboard.rs` | 86 | 37 |
| `hermes_dashboard_tests.rs` | 63 | 37 |
| `hermes_memory.rs` | 63 | 37 |
| `hermes_memory_limits.rs` | 63 | 37 |
| `hermes_memory_limits_tests.rs` | 47 | 37 |
| `hermes_memory_tests.rs` | 54 | 37 |
| `high_availability/circuit_breaker.rs` | 304 | 7 |
| `high_availability/circuit_breaker_tests.rs` | 144 | 7 |
| `high_availability/health.rs` | 128 | 7 |
| `high_availability/health_tests.rs` | 132 | 7 |
| `high_availability/mod.rs` | 14 | 30 |
| `high_availability/queue.rs` | 96 | 30 |
| `high_availability/queue_tests.rs` | 77 | 30 |
| `high_availability/retry.rs` | 158 | 7 |
| `high_availability/retry_tests.rs` | 149 | 7 |
| `high_availability/selection.rs` | 95 | 30 |
| `high_availability/selection_tests.rs` | 123 | 30 |
| `icons/library.rs` | 55 | 2 |
| `icons/library_tests.rs` | 149 | 2 |
| `icons/mod.rs` | 15 | 2 |
| `icons/name.rs` | 13 | 2 |
| `icons/name_tests.rs` | 31 | 2 |
| `icons/validation.rs` | 45 | 2 |
| `icons/validation_tests.rs` | 56 | 2 |
| `lib.rs` | 107 | 72 |
| `mcp/adapters/agy.rs` | 237 | 40 |
| `mcp/adapters/agy_tests.rs` | 175 | 40 |
| `mcp/adapters/claude.rs` | 189 | 40 |
| `mcp/adapters/claude_tests.rs` | 221 | 40 |
| `mcp/adapters/codex.rs` | 260 | 40 |
| `mcp/adapters/codex_tests.rs` | 209 | 40 |
| `mcp/adapters/dsh.rs` | 458 | 40 |
| `mcp/adapters/dsh_tests.rs` | 219 | 40 |
| `mcp/adapters/gemini.rs` | 208 | 40 |
| `mcp/adapters/gemini_tests.rs` | 182 | 40 |
| `mcp/adapters/grokbuild.rs` | 172 | 40 |
| `mcp/adapters/grokbuild_tests.rs` | 182 | 40 |
| `mcp/adapters/hermes.rs` | 259 | 40 |
| `mcp/adapters/hermes_tests.rs` | 170 | 40 |
| `mcp/adapters/kimi.rs` | 197 | 40 |
| `mcp/adapters/kimi_tests.rs` | 252 | 40 |
| `mcp/adapters/mcode.rs` | 216 | 40 |
| `mcp/adapters/mcode_tests.rs` | 295 | 40 |
| `mcp/adapters/mod.rs` | 46 | 40 |
| `mcp/adapters/omp.rs` | 279 | 40 |
| `mcp/adapters/omp_tests.rs` | 280 | 40 |
| `mcp/adapters/opencode.rs` | 230 | 40 |
| `mcp/adapters/opencode_tests.rs` | 224 | 40 |
| `mcp/adapters/pi.rs` | 256 | 40 |
| `mcp/adapters/pi_tests.rs` | 251 | 40 |
| `mcp/catalog.rs` | 74 | 39 |
| `mcp/distribution.rs` | 362 | 40 |
| `mcp/import.rs` | 222 | 40 |
| `mcp/mod.rs` | 47 | 40 |
| `mcp/native_file.rs` | 120 | 40 |
| `mcp/native_file_tests.rs` | 308 | 40 |
| `mcp/presets.rs` | 94 | 39 |
| `mcp/presets_tests.rs` | 90 | 39 |
| `mcp/validation.rs` | 177 | 39 |
| `mcp/validation_tests.rs` | 187 | 39 |
| `models/capabilities.rs` | 157 9 → `crates/libs/app-model-catalog/src/capabilities.rs` |
| `models/capabilities_tests.rs` | 162 9 → `crates/libs/app-model-catalog/src/capabilities_tests.rs` |
| `models/codex_template.rs` | 27 9 → `crates/libs/app-model-catalog/src/codex_template.rs` |
| `models/codex_template_tests.rs` | 69 9 → `crates/libs/app-model-catalog/src/codex_template_tests.rs`（上层编排断言保留原文件，见本任务记录） |
| `models/defaults_tests.rs` | 204 | 26 |
| `models/discovery_agy.rs` | 107 | 26 |
| `models/discovery_agy_tests.rs` | 232 | 26 |
| `models/discovery_http.rs` | 126 | 26 |
| `models/discovery_http_tests.rs` | 336 | 26 |
| `models/discovery_opencode.rs` | 83 | 26 |
| `models/discovery_opencode_tests.rs` | 180 | 26 |
| `models/discovery_process.rs` | 78 | 26 |
| `models/discovery_url.rs` | 85 | 26 |
| `models/discovery_url_tests.rs` | 268 | 26 |
| `models/edits.rs` | 263 | 26 |
| `models/edits_tests.rs` | 261 | 26 |
| `models/fill_codex.rs` | 70 | 26 |
| `models/fill_codex_tests.rs` | 149 | 26 |
| `models/fill_hermes.rs` | 48 | 26 |
| `models/fill_hermes_tests.rs` | 130 | 26 |
| `models/fill_kimi.rs` | 70 | 26 |
| `models/fill_kimi_tests.rs` | 147 | 26 |
| `models/fill_openclaw.rs` | 78 | 26 |
| `models/fill_openclaw_tests.rs` | 201 | 26 |
| `models/fill_opencode.rs` | 97 | 26 |
| `models/fill_opencode_tests.rs` | 218 | 26 |
| `models/fill_pi.rs` | 190 | 26 |
| `models/fill_pi_tests.rs` | 275 | 26 |
| `models/fill_service.rs` | 147 | 26 |
| `models/fill_service_tests.rs` | 347 | 26 |
| `models/identity.rs` | 111 9 → `crates/libs/app-model-catalog/src/identity.rs` |
| `models/identity_tests.rs` | 160 9 → `crates/libs/app-model-catalog/src/identity_tests.rs` |
| `models/metadata.rs` | 340 | 26 |
| `models/metadata_tests.rs` | 261 | 26 |
| `models/mod.rs` | 146 | 26 |
| `models/pi_thinking.rs` | 83 9 → `crates/libs/app-model-catalog/src/pi_thinking.rs` |
| `models/pi_thinking_tests.rs` | 163 9 → `crates/libs/app-model-catalog/src/pi_thinking_tests.rs` |
| `models/preset_catalog.rs` | 220 9 → `crates/libs/app-model-catalog/src/preset_catalog.rs` |
| `models/preset_catalog_tests.rs` | 121 9 → `crates/libs/app-model-catalog/src/preset_catalog_tests.rs`（上层编排断言保留原文件，见本任务记录） |
| `models/preset_claude.rs` | 104 9 → `crates/libs/app-model-catalog/src/preset_claude.rs` |
| `models/preset_claude_tests.rs` | 132 9 → `crates/libs/app-model-catalog/src/preset_claude_tests.rs` |
| `models/preset_codex.rs` | 88 9 → `crates/libs/app-model-catalog/src/preset_codex.rs` |
| `models/preset_codex_tests.rs` | 141 9 → `crates/libs/app-model-catalog/src/preset_codex_tests.rs` |
| `models/preset_gemini_grok.rs` | 41 9 → `crates/libs/app-model-catalog/src/preset_gemini_grok.rs` |
| `models/preset_gemini_grok_tests.rs` | 109 9 → `crates/libs/app-model-catalog/src/preset_gemini_grok_tests.rs` |
| `models/preset_hermes_mcode.rs` | 77 9 → `crates/libs/app-model-catalog/src/preset_hermes_mcode.rs` |
| `models/preset_hermes_mcode_tests.rs` | 108 9 → `crates/libs/app-model-catalog/src/preset_hermes_mcode_tests.rs` |
| `models/preset_kimi_dsh.rs` | 165 9 → `crates/libs/app-model-catalog/src/preset_kimi_dsh.rs` |
| `models/preset_kimi_dsh_tests.rs` | 148 9 → `crates/libs/app-model-catalog/src/preset_kimi_dsh_tests.rs` |
| `models/preset_omp.rs` | 120 9 → `crates/libs/app-model-catalog/src/preset_omp.rs` |
| `models/preset_omp_tests.rs` | 141 9 → `crates/libs/app-model-catalog/src/preset_omp_tests.rs`（上层编排断言保留原文件，见本任务记录） |
| `models/preset_opencode_openclaw.rs` | 97 9 → `crates/libs/app-model-catalog/src/preset_opencode_openclaw.rs` |
| `models/preset_opencode_openclaw_tests.rs` | 117 9 → `crates/libs/app-model-catalog/src/preset_opencode_openclaw_tests.rs` |
| `models/preset_pi.rs` | 157 9 → `crates/libs/app-model-catalog/src/preset_pi.rs` |
| `models/preset_pi_tests.rs` | 131 9 → `crates/libs/app-model-catalog/src/preset_pi_tests.rs` |
| `models/preset_search.rs` | 273 9 → `crates/libs/app-model-catalog/src/preset_search.rs` |
| `models/preset_search_tests.rs` | 163 9 → `crates/libs/app-model-catalog/src/preset_search_tests.rs` |
| `models/presets/claude.json` | 6668 9 → `crates/libs/app-model-catalog/src/presets/claude.json` |
| `models/presets/codex.json` | 4727 9 → `crates/libs/app-model-catalog/src/presets/codex.json` |
| `models/presets/families.json` | 94 9 → `crates/libs/app-model-catalog/src/presets/families.json` |
| `models/presets/gemini_grok.json` | 1443 9 → `crates/libs/app-model-catalog/src/presets/gemini_grok.json` |
| `models/presets/hermes_mcode.json` | 12071 9 → `crates/libs/app-model-catalog/src/presets/hermes_mcode.json` |
| `models/presets/kimi_dsh.json` | 19517 9 → `crates/libs/app-model-catalog/src/presets/kimi_dsh.json` |
| `models/presets/opencode_openclaw.json` | 12854 9 → `crates/libs/app-model-catalog/src/presets/opencode_openclaw.json` |
| `models/presets/pi.json` | 8087 9 → `crates/libs/app-model-catalog/src/presets/pi.json` |
| `models/presets/pi_thinking.json` | 299 9 → `crates/libs/app-model-catalog/src/presets/pi_thinking.json` |
| `models/presets/search_aliases.json` | 64 9 → `crates/libs/app-model-catalog/src/presets/search_aliases.json` |
| `models/presets/universal.json` | 40 9 → `crates/libs/app-model-catalog/src/presets/universal.json` |
| `models/user_agent_presets.rs` | 12 9 → `crates/libs/app-model-catalog/src/user_agent_presets.rs` |
| `models/user_agent_presets_tests.rs` | 35 9 → `crates/libs/app-model-catalog/src/user_agent_presets_tests.rs` |
| `models/vendor_fallback.rs` | 73 | 26 |
| `models/vendor_fallback_tests.rs` | 224 | 26 |
| `openclaw_catalog.rs` | 61 | 38 |
| `openclaw_catalog_tests.rs` | 57 | 38 |
| `openclaw_defaults.rs` | 119 | 38 |
| `openclaw_defaults_tests.rs` | 55 | 38 |
| `openclaw_env.rs` | 85 | 38 |
| `openclaw_env_tests.rs` | 43 | 38 |
| `openclaw_health.rs` | 57 | 38 |
| `openclaw_health_tests.rs` | 61 | 38 |
| `openclaw_model.rs` | 103 | 38 |
| `openclaw_model_tests.rs` | 76 | 38 |
| `openclaw_tools.rs` | 92 | 38 |
| `openclaw_tools_tests.rs` | 56 | 38 |
| `project_profiles/apply.rs` | 181 | 70 |
| `project_profiles/apply_mcp.rs` | 129 | 70 |
| `project_profiles/apply_prompt.rs` | 60 | 70 |
| `project_profiles/apply_provider.rs` | 131 | 70 |
| `project_profiles/apply_skills.rs` | 79 | 70 |
| `project_profiles/autosave.rs` | 46 | 70 |
| `project_profiles/create.rs` | 63 | 70 |
| `project_profiles/mod.rs` | 15 | 70 |
| `project_profiles/preview.rs` | 124 | 70 |
| `project_profiles/rename.rs` | 22 | 70 |
| `project_profiles/resnapshot.rs` | 35 | 70 |
| `project_profiles/snapshot.rs` | 102 | 70 |
| `prompts/activation.rs` | 127 | 41 |
| `prompts/copy.rs` | 14 | 41 |
| `prompts/deactivation.rs` | 26 | 41 |
| `prompts/import.rs` | 88 | 41 |
| `prompts/listing.rs` | 32 | 41 |
| `prompts/live_file.rs` | 101 | 41 |
| `prompts/mcode.rs` | 116 | 41 |
| `prompts/mod.rs` | 40 | 41 |
| `prompts/omp.rs` | 31 | 41 |
| `prompts/pi_agents.rs` | 126 | 41 |
| `prompts/pi_system_files.rs` | 141 | 41 |
| `prompts/shared_import.rs` | 156 | 41 |
| `prompts/sync.rs` | 72 | 41 |
| `prompts/targets.rs` | 76 | 41 |
| `prompts/template_metadata.rs` | 125 | 41 |
| `prompts/template_rename.rs` | 80 | 41 |
| `prompts/template_slug.rs` | 56 | 41 |
| `prompts/templates.rs` | 145 | 41 |
| `providers/agy.rs` | 190 | 24 |
| `providers/agy_files.rs` | 254 | 24 |
| `providers/agy_tests.rs` | 288 | 24 |
| `providers/basic.rs` | 108 | 11 |
| `providers/catalog.rs` | 254 | 11 |
| `providers/catalog_tests.rs` | 134 | 11 |
| `providers/claude.rs` | 377 | 10 |
| `providers/claude_desktop.rs` | 420 | 24 |
| `providers/claude_desktop/project_tests.rs` | 112 | 24 |
| `providers/claude_desktop/publication.rs` | 300 | 24 |
| `providers/claude_desktop/publish_tests.rs` | 384 | 24 |
| `providers/claude_desktop_tests.rs` | 358 | 24 |
| `providers/claude_form.rs` | 70 | 10 |
| `providers/claude_login.rs` | 528 | 23 |
| `providers/claude_login_tests.rs` | 257 | 23 |
| `providers/claude_tests.rs` | 60 | 10 |
| `providers/codex.rs` | 88 | 10 |
| `providers/codex_catalog.rs` | 278 | 29 |
| `providers/codex_catalog/form.rs` | 292 | 29 |
| `providers/codex_catalog_tests.rs` | 333 | 29 |
| `providers/codex_login.rs` | 109 | 23 |
| `providers/codex_login_material.rs` | 182 | 23 |
| `providers/codex_login_policy.rs` | 300 | 23 |
| `providers/codex_login_policy_tests.rs` | 95 | 23 |
| `providers/codex_login_tests.rs` | 445 | 23 |
| `providers/codex_login_transaction_tests.rs` | 71 | 23 |
| `providers/codex_tests.rs` | 157 | 10 |
| `providers/create.rs` | 40 | 11 |
| `providers/create_tests.rs` | 82 | 11 |
| `providers/delete.rs` | 28 | 11 |
| `providers/delete_tests.rs` | 53 | 11 |
| `providers/desktop_mapping.rs` | 358 | 10 |
| `providers/desktop_mapping/catalog.rs` | 249 | 10 |
| `providers/desktop_mapping_tests.rs` | 249 | 10 |
| `providers/dsh.rs` | 402 | 24 |
| `providers/dsh_edit.rs` | 125 | 24 |
| `providers/dsh_files.rs` | 366 | 24 |
| `providers/dsh_form.rs` | 240 | 10 |
| `providers/dsh_tests.rs` | 258 | 24 |
| `providers/duplicate.rs` | 47 | 11 |
| `providers/duplicate_tests.rs` | 85 | 11 |
| `providers/editor_merge.rs` | 203 | 11 |
| `providers/editor_merge_tests.rs` | 97 | 11 |
| `providers/endpoints.rs` | 85 | 4 |
| `providers/endpoints_tests.rs` | 114 | 4 |
| `providers/env_backup.rs` | 195 | 25 |
| `providers/env_clean.rs` | 285 | 25 |
| `providers/env_clean_tests.rs` | 283 | 25 |
| `providers/env_diagnose.rs` | 238 | 25 |
| `providers/env_diagnose_tests.rs` | 145 | 25 |
| `providers/env_restore.rs` | 135 | 25 |
| `providers/env_restore_tests.rs` | 271 | 25 |
| `providers/failover.rs` | 168 | 11 |
| `providers/gemini.rs` | 264 | 10 |
| `providers/gemini_form.rs` | 51 | 10 |
| `providers/gemini_tests.rs` | 253 | 10 |
| `providers/grok.rs` | 225 | 10 |
| `providers/grok_form.rs` | 178 | 10 |
| `providers/grok_tests.rs` | 244 | 10 |
| `providers/hermes.rs` | 507 | 24 |
| `providers/hermes/form.rs` | 188 | 24 |
| `providers/hermes_tests.rs` | 214 | 24 |
| `providers/icons.rs` | 42 | 11 |
| `providers/icons_tests.rs` | 70 | 11 |
| `providers/import.rs` | 148 | 29 |
| `providers/import_tests.rs` | 78 | 29 |
| `providers/kimi.rs` | 520 | 24 |
| `providers/kimi_form.rs` | 238 | 10 |
| `providers/kimi_tests.rs` | 229 | 24 |
| `providers/live/claude_directory.rs` | 125 | 12 |
| `providers/live/claude_directory_tests.rs` | 36 | 12 |
| `providers/live/commit.rs` | 750 | 12 |
| `providers/live/commit_tests.rs` | 292 | 12 |
| `providers/live/concurrency.rs` | 117 | 12 |
| `providers/live/concurrency_tests.rs` | 351 | 12 |
| `providers/live/mod.rs` | 55 | 12 |
| `providers/live/patch_dotenv.rs` | 148 | 5 |
| `providers/live/patch_dotenv_tests.rs` | 70 | 5 |
| `providers/live/patch_json.rs` | 440 | 5 |
| `providers/live/patch_json5.rs` | 391 | 5 |
| `providers/live/patch_json5_tests.rs` | 55 | 5 |
| `providers/live/patch_json_tests.rs` | 118 | 5 |
| `providers/live/patch_toml.rs` | 130 | 5 |
| `providers/live/patch_toml_tests.rs` | 82 | 5 |
| `providers/live/patch_yaml.rs` | 382 | 5 |
| `providers/live/patch_yaml_tests.rs` | 158 | 5 |
| `providers/live/project_binding.rs` | 127 | 12 |
| `providers/live/project_binding_tests.rs` | 214 | 12 |
| `providers/live/record_publication.rs` | 144 | 12 |
| `providers/live/recovery.rs` | 330 | 12 |
| `providers/live/recovery_tests.rs` | 187 | 12 |
| `providers/live/stage.rs` | 117 | 12 |
| `providers/live/stage_tests.rs` | 155 | 12 |
| `providers/live/write_guard.rs` | 412 | 12 |
| `providers/live/write_guard_tests.rs` | 152 | 12 |
| `providers/mcode.rs` | 447 | 24 |
| `providers/mcode_form.rs` | 226 | 10 |
| `providers/mcode_tests.rs` | 245 | 24 |
| `providers/mod.rs` | 255 | 29 |
| `providers/mode_apply.rs` | 244 | 29 |
| `providers/mode_apply_publication.rs` | 143 | 29 |
| `providers/mode_apply_publication_tests.rs` | 81 | 29 |
| `providers/mode_apply_tests.rs` | 257 | 29 |
| `providers/mode_state.rs` | 204 | 29 |
| `providers/mode_state_tests.rs` | 379 | 29 |
| `providers/omp.rs` | 549 | 24 |
| `providers/omp/form.rs` | 268 | 24 |
| `providers/omp_read_tests.rs` | 127 | 24 |
| `providers/omp_tests.rs` | 321 | 24 |
| `providers/omp_validation.rs` | 206 | 10 |
| `providers/omp_write_tests.rs` | 123 | 24 |
| `providers/openclaw.rs` | 276 | 24 |
| `providers/openclaw_form.rs` | 213 | 10 |
| `providers/openclaw_tests.rs` | 206 | 24 |
| `providers/opencode.rs` | 363 | 24 |
| `providers/opencode_form.rs` | 192 | 10 |
| `providers/opencode_schema.rs` | 185 | 10 |
| `providers/opencode_tests.rs` | 232 | 24 |
| `providers/operations.rs` | 118 | 29 |
| `providers/operations_tests.rs` | 215 | 29 |
| `providers/paths.rs` | 314 | 3 |
| `providers/paths_tests.rs` | 161 | 3 |
| `providers/pi.rs` | 336 | 24 |
| `providers/pi/directory_project_tests.rs` | 81 | 24 |
| `providers/pi/form.rs` | 287 | 24 |
| `providers/pi/read_tests.rs` | 120 | 24 |
| `providers/pi/write.rs` | 197 | 24 |
| `providers/pi/write_tests.rs` | 286 | 24 |
| `providers/pi_tests.rs` | 149 | 24 |
| `providers/presentation.rs` | 41 | 11 |
| `providers/presentation_tests.rs` | 54 | 11 |
| `providers/preview.rs` | 292 | 29 |
| `providers/preview_tests.rs` | 382 | 29 |
| `providers/projection/codex.rs` | 268 | 10 |
| `providers/projection/codex_route.rs` | 338 | 10 |
| `providers/projection/fields.rs` | 177 | 10 |
| `providers/projection/grok.rs` | 141 | 10 |
| `providers/projection/json.rs` | 200 | 10 |
| `providers/projection/mod.rs` | 72 | 10 |
| `providers/request_options.rs` | 50 | 4 |
| `providers/snippets.rs` | 209 | 11 |
| `providers/snippets_tests.rs` | 110 | 11 |
| `providers/universal.rs` | 125 | 11 |
| `providers/universal_sync.rs` | 227 | 11 |
| `providers/universal_sync_tests.rs` | 85 | 11 |
| `providers/universal_tests.rs` | 90 | 11 |
| `providers/update.rs` | 72 | 11 |
| `providers/update_tests.rs` | 55 | 11 |
| `providers/validation.rs` | 226 | 4 |
| `providers/validation_tests.rs` | 181 | 4 |
| `routing/budget_rectify.rs` | 33 | 8 |
| `routing/cache_policy.rs` | 148 | 8 |
| `routing/cache_policy_tests.rs` | 50 | 8 |
| `routing/classifier_guard.rs` | 66 | 8 |
| `routing/classifier_guard_tests.rs` | 82 | 8 |
| `routing/compaction_replay.rs` | 98 | 8 |
| `routing/compaction_replay_tests.rs` | 19 | 8 |
| `routing/gemini_schema.rs` | 449 | 8 |
| `routing/inline_think.rs` | 201 | 8 |
| `routing/inline_think_tests.rs` | 41 | 8 |
| `routing/lifecycle_state.rs` | 80 | 8 |
| `routing/lifecycle_state_tests.rs` | 41 | 8 |
| `routing/media_sanitize.rs` | 176 | 8 |
| `routing/mod.rs` | 57 | 8 |
| `routing/moonshot_schema.rs` | 159 | 8 |
| `routing/moonshot_schema_tests.rs` | 253 | 8 |
| `routing/override_validate_tests.rs` | 129 | 8 |
| `routing/private_params.rs` | 78 | 8 |
| `routing/private_params_tests.rs` | 68 | 8 |
| `routing/target_model.rs` | 69 | 8 |
| `routing/target_model_tests.rs` | 56 | 8 |
| `routing/thinking_options.rs` | 96 | 8 |
| `routing/thinking_options_tests.rs` | 56 | 8 |
| `routing/user_agent_tests.rs` | 33 | 8 |
| `sessions/activity.rs` | 46 | 54 |
| `sessions/block_preview.rs` | 143 | 44 |
| `sessions/blocks.rs` | 394 | 44 |
| `sessions/cache.rs` | 121 | 53 |
| `sessions/catalog.rs` | 170 | 54 |
| `sessions/codex_state.rs` | 83 | 44 |
| `sessions/compression.rs` | 219 | 54 |
| `sessions/content_file.rs` | 160 | 53 |
| `sessions/content_sqlite.rs` | 87 | 53 |
| `sessions/delete/claude.rs` | 92 | 54 |
| `sessions/delete/codex.rs` | 91 | 54 |
| `sessions/delete/dispatch.rs` | 64 | 54 |
| `sessions/delete/gemini.rs` | 150 | 54 |
| `sessions/delete/grokbuild.rs` | 100 | 54 |
| `sessions/delete/hermes.rs` | 65 | 54 |
| `sessions/delete/hermes_sqlite.rs` | 90 | 54 |
| `sessions/delete/hermes_tests.rs` | 88 | 54 |
| `sessions/delete/kimi.rs` | 173 | 54 |
| `sessions/delete/mod.rs` | 20 | 54 |
| `sessions/delete/omp.rs` | 107 | 54 |
| `sessions/delete/openclaw.rs` | 107 | 54 |
| `sessions/delete/opencode.rs` | 172 | 54 |
| `sessions/delete/opencode_sqlite.rs` | 97 | 54 |
| `sessions/delete/pi.rs` | 45 | 54 |
| `sessions/delete_batch.rs` | 81 | 54 |
| `sessions/identity.rs` | 57 | 44 |
| `sessions/images.rs` | 96 | 53 |
| `sessions/maintenance.rs` | 346 | 54 |
| `sessions/markdown.rs` | 183 | 53 |
| `sessions/migration/mod.rs` | 14 | 52 |
| `sessions/migration/provider_refs.rs` | 267 | 52 |
| `sessions/migration/restore.rs` | 278 | 52 |
| `sessions/migration/rollout.rs` | 200 | 52 |
| `sessions/migration/state_db.rs` | 244 | 52 |
| `sessions/migration/unify.rs` | 348 | 52 |
| `sessions/mod.rs` | 43 | 54 |
| `sessions/providers/agy.rs` | 301 | 50 |
| `sessions/providers/claude.rs` | 320 | 46 |
| `sessions/providers/claude_blocks.rs` | 407 | 46 |
| `sessions/providers/claude_jsonl.rs` | 104 | 46 |
| `sessions/providers/claude_messages.rs` | 677 | 46 |
| `sessions/providers/claude_wire.rs` | 358 | 46 |
| `sessions/providers/codex.rs` | 429 | 45 |
| `sessions/providers/codex_items.rs` | 1072 | 45 |
| `sessions/providers/codex_messages.rs` | 444 | 45 |
| `sessions/providers/codex_parser_1.rs` | 327 | 45 |
| `sessions/providers/codex_parser_2.rs` | 391 | 45 |
| `sessions/providers/codex_parser_3.rs` | 240 | 45 |
| `sessions/providers/codex_parser_4.rs` | 268 | 45 |
| `sessions/providers/dsh.rs` | 276 | 50 |
| `sessions/providers/gemini.rs` | 431 | 47 |
| `sessions/providers/gemini_document.rs` | 93 | 47 |
| `sessions/providers/grokbuild.rs` | 175 | 50 |
| `sessions/providers/hermes.rs` | 332 | 49 |
| `sessions/providers/hermes_sqlite.rs` | 403 | 49 |
| `sessions/providers/kimi.rs` | 287 | 50 |
| `sessions/providers/kimi_requests.rs` | 175 | 50 |
| `sessions/providers/mcode.rs` | 144 | 50 |
| `sessions/providers/mod.rs` | 39 | 50 |
| `sessions/providers/omp.rs` | 129 | 50 |
| `sessions/providers/openclaw.rs` | 271 | 50 |
| `sessions/providers/opencode.rs` | 131 | 49 |
| `sessions/providers/opencode_blocks.rs` | 433 | 49 |
| `sessions/providers/opencode_sqlite.rs` | 267 | 49 |
| `sessions/providers/pi.rs` | 235 | 48 |
| `sessions/providers/pi_blocks.rs` | 495 | 48 |
| `sessions/providers/pi_jsonl.rs` | 113 | 48 |
| `sessions/providers/pi_tree.rs` | 429 | 48 |
| `sessions/resume.rs` | 272 | 54 |
| `sessions/rollout.rs` | 103 | 53 |
| `sessions/source.rs` | 148 | 53 |
| `sessions/terminal/alacritty.rs` | 65 | 51 |
| `sessions/terminal/apple.rs` | 115 | 51 |
| `sessions/terminal/custom.rs` | 123 | 51 |
| `sessions/terminal/ghostty_kitty.rs` | 108 | 51 |
| `sessions/terminal/mod.rs` | 15 | 51 |
| `sessions/terminal/otty.rs` | 122 | 51 |
| `sessions/terminal/warp.rs` | 71 | 51 |
| `sessions/terminal/wezterm_kaku.rs` | 93 | 51 |
| `sessions/tool_normalization.rs` | 195 | 44 |
| `sessions/turns.rs` | 247 | 53 |
| `sessions/utils.rs` | 83 | 44 |
| `skills/app_paths.rs` | 216 | 42 |
| `skills/app_paths_tests.rs` | 135 | 42 |
| `skills/archive.rs` | 316 | 42 |
| `skills/archive_tests.rs` | 142 | 42 |
| `skills/backup_catalog.rs` | 146 | 43 |
| `skills/backup_restore.rs` | 122 | 43 |
| `skills/backup_restore_tests.rs` | 37 | 43 |
| `skills/bulk_sync.rs` | 92 | 43 |
| `skills/claude_onboarding.rs` | 149 | 43 |
| `skills/claude_onboarding_tests.rs` | 139 | 43 |
| `skills/claude_status.rs` | 108 | 43 |
| `skills/claude_status_tests.rs` | 117 | 43 |
| `skills/claude_toggle.rs` | 175 | 43 |
| `skills/claude_toggle_tests.rs` | 156 | 43 |
| `skills/copy_deploy.rs` | 219 | 43 |
| `skills/copy_deploy_tests.rs` | 50 | 43 |
| `skills/disable_deploy.rs` | 156 | 43 |
| `skills/disable_deploy_tests.rs` | 72 | 43 |
| `skills/discovery.rs` | 257 | 42 |
| `skills/discovery_cache.rs` | 142 | 42 |
| `skills/discovery_cache_tests.rs` | 108 | 42 |
| `skills/discovery_tests.rs` | 147 | 42 |
| `skills/inventory.rs` | 164 | 43 |
| `skills/inventory_tests.rs` | 58 | 43 |
| `skills/job_cancel.rs` | 10 | 42 |
| `skills/job_cancel_tests.rs` | 46 | 42 |
| `skills/jobs.rs` | 78 | 42 |
| `skills/jobs_tests.rs` | 25 | 42 |
| `skills/link_deploy.rs` | 15 | 43 |
| `skills/link_deploy_tests.rs` | 52 | 43 |
| `skills/local_install.rs` | 350 | 43 |
| `skills/local_install_tests.rs` | 76 | 43 |
| `skills/market_search.rs` | 176 | 42 |
| `skills/market_search_tests.rs` | 152 | 42 |
| `skills/mod.rs` | 143 | 43 |
| `skills/repo_install.rs` | 111 | 43 |
| `skills/repo_install_tests.rs` | 93 | 43 |
| `skills/repos.rs` | 142 | 42 |
| `skills/repos_tests.rs` | 55 | 42 |
| `skills/storage_move.rs` | 166 | 43 |
| `skills/storage_move_tests.rs` | 33 | 43 |
| `skills/uninstall.rs` | 182 | 43 |
| `skills/uninstall_tests.rs` | 154 | 43 |
| `skills/unmanaged_import.rs` | 66 | 43 |
| `skills/unmanaged_import_tests.rs` | 61 | 43 |
| `skills/unmanaged_scan.rs` | 122 | 43 |
| `skills/unmanaged_scan_tests.rs` | 60 | 43 |
| `skills/update_apply.rs` | 124 | 43 |
| `skills/update_apply_tests.rs` | 74 | 43 |
| `skills/update_check.rs` | 121 | 43 |
| `skills/update_check_tests.rs` | 103 | 43 |
| `skills/zip_install.rs` | 81 | 43 |
| `skills/zip_install_tests.rs` | 45 | 43 |
| `usage/agy.rs` | 248 | 62 |
| `usage/agy_native.rs` | 189 | 62 |
| `usage/balances/deepseek.rs` | 154 | 57 |
| `usage/balances/deepseek_tests.rs` | 110 | 57 |
| `usage/balances/mod.rs` | 6 | 57 |
| `usage/balances/novita.rs` | 112 | 57 |
| `usage/balances/novita_tests.rs` | 121 | 57 |
| `usage/balances/openrouter.rs` | 100 | 57 |
| `usage/balances/openrouter_tests.rs` | 110 | 57 |
| `usage/balances/siliconflow.rs` | 149 | 57 |
| `usage/balances/siliconflow_tests.rs` | 155 | 57 |
| `usage/balances/stepfun.rs` | 101 | 57 |
| `usage/balances/stepfun_tests.rs` | 111 | 57 |
| `usage/balances/zhipu.rs` | 70 | 57 |
| `usage/balances/zhipu_tests.rs` | 172 | 57 |
| `usage/binding.rs` | 100 | 55 |
| `usage/binding_tests.rs` | 225 | 55 |
| `usage/cache.rs` | 120 | 56 |
| `usage/cache_tests.rs` | 149 | 56 |
| `usage/claude.rs` | 96 | 62 |
| `usage/claude_scan.rs` | 312 | 62 |
| `usage/codex_delta.rs` | 245 | 61 |
| `usage/codex_lineage.rs` | 377 | 61 |
| `usage/codex_rebuild.rs` | 148 | 61 |
| `usage/codex_timing.rs` | 201 | 61 |
| `usage/coding/command_code.rs` | 373 | 58 |
| `usage/coding/command_code_tests.rs` | 259 | 58 |
| `usage/coding/kimi.rs` | 247 | 58 |
| `usage/coding/kimi_tests.rs` | 209 | 58 |
| `usage/coding/minimax.rs` | 205 | 58 |
| `usage/coding/minimax_tests.rs` | 193 | 58 |
| `usage/coding/mod.rs` | 17 | 58 |
| `usage/coding/opencode_go.rs` | 158 | 58 |
| `usage/coding/opencode_go_tests.rs` | 176 | 58 |
| `usage/coding/volcengine.rs` | 299 | 58 |
| `usage/coding/volcengine_sign.rs` | 110 | 58 |
| `usage/coding/volcengine_sign_tests.rs` | 127 | 58 |
| `usage/coding/volcengine_tests.rs` | 216 | 58 |
| `usage/coding/zenmux.rs` | 214 | 58 |
| `usage/coding/zenmux_tests.rs` | 189 | 58 |
| `usage/coding/zhipu.rs` | 299 | 58 |
| `usage/coding/zhipu_team.rs` | 88 | 58 |
| `usage/coding/zhipu_team_tests.rs` | 213 | 58 |
| `usage/coding/zhipu_tests.rs` | 200 | 58 |
| `usage/cost.rs` | 148 | 55 |
| `usage/cursors.rs` | 252 | 55 |
| `usage/dedup.rs` | 101 | 55 |
| `usage/detect.rs` | 115 | 55 |
| `usage/detect_tests.rs` | 117 | 55 |
| `usage/dsh.rs` | 189 | 62 |
| `usage/exchange.rs` | 219 | 55 |
| `usage/exchange_store.rs` | 128 | 55 |
| `usage/gemini.rs` | 224 | 62 |
| `usage/grokbuild.rs` | 199 | 62 |
| `usage/groups.rs` | 368 | 56 |
| `usage/import_store.rs` | 46 | 56 |
| `usage/kimi.rs` | 217 | 62 |
| `usage/limits.rs` | 266 | 55 |
| `usage/logs.rs` | 244 | 63 |
| `usage/mcode.rs` | 172 | 62 |
| `usage/mod.rs` | 82 | 63 |
| `usage/model_names.rs` | 239 | 55 |
| `usage/models_selection.rs` | 156 | 63 |
| `usage/models_sync.rs` | 163 | 63 |
| `usage/omp.rs` | 74 | 62 |
| `usage/opencode.rs` | 319 | 62 |
| `usage/pi.rs` | 233 | 62 |
| `usage/pi_children.rs` | 180 | 62 |
| `usage/presentation/balance.rs` | 89 | 55 |
| `usage/presentation/balance_tests.rs` | 154 | 55 |
| `usage/presentation/credits.rs` | 1 | 55 |
| `usage/presentation/credits_tests.rs` | 75 | 55 |
| `usage/presentation/mod.rs` | 12 | 55 |
| `usage/presentation/tiers.rs` | 1 | 55 |
| `usage/presentation/tiers_tests.rs` | 102 | 55 |
| `usage/pricing.rs` | 159 | 55 |
| `usage/pricing_batch.rs` | 109 | 55 |
| `usage/ranges.rs` | 120 | 55 |
| `usage/retention.rs` | 266 | 56 |
| `usage/scan_jobs.rs` | 199 | 63 |
| `usage/scan_schedule.rs` | 127 | 63 |
| `usage/scripts/config.rs` | 313 | 60 |
| `usage/scripts/config_tests.rs` | 134 | 60 |
| `usage/scripts/extract.rs` | 84 | 60 |
| `usage/scripts/extract_tests.rs` | 127 | 60 |
| `usage/scripts/mod.rs` | 23 | 60 |
| `usage/scripts/network.rs` | 189 | 60 |
| `usage/scripts/network_tests.rs` | 211 | 60 |
| `usage/scripts/run.rs` | 63 | 60 |
| `usage/scripts/run_tests.rs` | 153 | 60 |
| `usage/scripts/runtime.rs` | 182 | 60 |
| `usage/scripts/runtime_tests.rs` | 113 | 60 |
| `usage/series.rs` | 193 | 56 |
| `usage/snapshot_scan.rs` | 109 | 63 |
| `usage/source_settings.rs` | 175 | 56 |
| `usage/store.rs` | 221 | 56 |
| `usage/subscriptions/agy.rs` | 428 | 59 |
| `usage/subscriptions/agy_tests.rs` | 250 | 59 |
| `usage/subscriptions/claude.rs` | 390 | 59 |
| `usage/subscriptions/claude_tests.rs` | 385 | 59 |
| `usage/subscriptions/codex.rs` | 412 | 59 |
| `usage/subscriptions/codex_tests.rs` | 361 | 59 |
| `usage/subscriptions/copilot.rs` | 194 | 59 |
| `usage/subscriptions/copilot_tests.rs` | 249 | 59 |
| `usage/subscriptions/gemini.rs` | 341 | 59 |
| `usage/subscriptions/gemini_tests.rs` | 326 | 59 |
| `usage/subscriptions/grok.rs` | 492 | 59 |
| `usage/subscriptions/grok/grpc.rs` | 205 | 59 |
| `usage/subscriptions/grok_tests.rs` | 298 | 59 |
| `usage/subscriptions/kimi.rs` | 145 | 59 |
| `usage/subscriptions/kimi_tests.rs` | 179 | 59 |
| `usage/subscriptions/mod.rs` | 30 | 59 |
| `usage/summary.rs` | 321 | 56 |
| `usage/tokens.rs` | 193 | 55 |
| `workspace_files.rs` | 52 | 37 |
| `workspace_files_tests.rs` | 105 | 37 |
| `workspace_paths.rs` | 267 | 37 |
| `workspace_paths_tests.rs` | 117 | 37 |

## 资源、宏和路径约束

models/presets 全部 11 个 JSON 归任务 9；data_management/fixtures/schema_18.sql 归任务 66。MCP Codex fixture 在 src 外，归任务 40。无 include_bytes!。局部宏随文件所有者移动，不提升成公共宏。#[path] 必须继续指向包内文件，禁止跨包指回原源码。

| 文件:行 | 资源/宏/路径声明 |
| --- | --- |
| `aggregation/codex_subagent.rs:126` | `#[path = "codex_effort_tests.rs"]` |
| `aggregation/daemon_restart.rs:410` | `#[path = "daemon_restart_tests.rs"]` |
| `aggregation/daemon_restart.rs:414` | `#[path = "daemon_restart_desktop_tests.rs"]` |
| `aggregation/membership.rs:4` | `#[path = "default_provider_tests.rs"]` |
| `apps/lifecycle/local.rs:281` | `#[path = "local_node.rs"]` |
| `apps/lifecycle/local.rs:284` | `#[path = "local_node_tests.rs"]` |
| `apps/lifecycle/local.rs:288` | `#[path = "local_agy_tests.rs"]` |
| `apps/lifecycle/local.rs:292` | `#[path = "local_omp_tests.rs"]` |
| `apps/versions/latest.rs:217` | `#[path = "latest_tests.rs"]` |
| `apps/versions/node.rs:264` | `#[path = "node_tests.rs"]` |
| `auth/agy_account_tests.rs:256` | `#[path = "agy_provider_binding_tests.rs"]` |
| `auth/kimi_account_tests.rs:317` | `#[path = "kimi_account/binding_tests.rs"]` |
| `auth/kimi_account_tests.rs:320` | `#[path = "binding_tests.rs"]` |
| `data_management/source_upgrade_tests.rs:4` | `const FIXTURE: &str = include_str!("fixtures/schema_18.sql");` |
| `hermes_dashboard.rs:85` | `#[path = "hermes_dashboard_tests.rs"]` |
| `high_availability/circuit_breaker.rs:303` | `#[path = "circuit_breaker_tests.rs"]` |
| `high_availability/health.rs:127` | `#[path = "health_tests.rs"]` |
| `high_availability/retry.rs:157` | `#[path = "retry_tests.rs"]` |
| `high_availability/selection.rs:94` | `#[path = "selection_tests.rs"]` |
| `mcp/adapters/codex_tests.rs:19` | `include_str!("../../../tests/fixtures/mcp/codex-config.toml");` |
| `mcp/adapters/dsh.rs:457` | `#[path = "dsh_tests.rs"]` |
| `mcp/adapters/opencode.rs:229` | `#[path = "opencode_tests.rs"]` |
| `models/fill_service.rs:126` | `macro_rules! fill {` |
| `models/metadata.rs:314` | `macro_rules! fill_option {` |
| `models/pi_thinking.rs:47` | `let catalog: Catalog = serde_json::from_str(include_str!("presets/pi_thinking.json"))` |
| `models/preset_catalog.rs:22` | `serde_json::from_str(include_str!("presets/claude.json"))` |
| `models/preset_catalog.rs:27` | `serde_json::from_str(include_str!("presets/codex.json")).expect("valid built-in Codex presets")` |
| `models/preset_catalog.rs:31` | `serde_json::from_str(include_str!("presets/gemini_grok.json"))` |
| `models/preset_catalog.rs:36` | `serde_json::from_str(include_str!("presets/hermes_mcode.json"))` |
| `models/preset_catalog.rs:41` | `serde_json::from_str(include_str!("presets/kimi_dsh.json"))` |
| `models/preset_catalog.rs:46` | `serde_json::from_str(include_str!("presets/opencode_openclaw.json"))` |
| `models/preset_catalog.rs:59` | `serde_json::from_str(include_str!("presets/pi.json")).expect("valid built-in Pi presets");` |
| `models/preset_catalog.rs:70` | `serde_json::from_str(include_str!("presets/universal.json"))` |
| `models/preset_search.rs:8` | `serde_json::from_str(include_str!("presets/families.json")).expect("valid preset family data")` |
| `models/preset_search.rs:12` | `serde_json::from_str(include_str!("presets/search_aliases.json"))` |
| `providers/agy.rs:9` | `#[path = "agy_files.rs"]` |
| `providers/claude.rs:9` | `#[path = "claude_form.rs"]` |
| `providers/claude_desktop.rs:415` | `#[path = "claude_desktop/publish_tests.rs"]` |
| `providers/claude_desktop.rs:419` | `#[path = "claude_desktop/project_tests.rs"]` |
| `providers/codex_login_tests.rs:442` | `#[path = "codex_login_policy_tests.rs"]` |
| `providers/codex_login_tests.rs:444` | `#[path = "codex_login_transaction_tests.rs"]` |
| `providers/dsh.rs:392` | `#[path = "dsh_files.rs"]` |
| `providers/dsh.rs:398` | `#[path = "dsh_edit.rs"]` |
| `providers/dsh.rs:401` | `#[path = "dsh_form.rs"]` |
| `providers/gemini.rs:5` | `#[path = "gemini_form.rs"]` |
| `providers/grok.rs:7` | `#[path = "grok_form.rs"]` |
| `providers/import.rs:85` | `macro_rules! declarations {` |
| `providers/kimi.rs:519` | `#[path = "kimi_form.rs"]` |
| `providers/mcode.rs:446` | `#[path = "mcode_form.rs"]` |
| `providers/omp.rs:4` | `#[path = "omp_validation.rs"]` |
| `providers/omp.rs:16` | `#[path = "omp_read_tests.rs"]` |
| `providers/omp.rs:20` | `#[path = "omp_write_tests.rs"]` |
| `providers/openclaw.rs:9` | `#[path = "openclaw_form.rs"]` |
| `providers/opencode.rs:2` | `#[path = "opencode_form.rs"]` |
| `sessions/providers/claude.rs:317` | `#[path = "claude_messages.rs"]` |
| `sessions/providers/claude_blocks.rs:304` | `macro_rules! take {` |
| `sessions/providers/claude_wire.rs:148` | `macro_rules! skip_other_values {` |
| `sessions/providers/claude_wire.rs:172` | `macro_rules! skip_str_values {` |
| `sessions/providers/claude_wire.rs:181` | `macro_rules! skip_seq_values {` |
| `sessions/providers/claude_wire.rs:190` | `macro_rules! skip_map_values {` |
| `sessions/providers/codex_messages.rs:388` | `#[path = "codex_parser_1.rs"]` |
| `sessions/providers/codex_messages.rs:390` | `#[path = "codex_parser_2.rs"]` |
| `sessions/providers/codex_messages.rs:392` | `#[path = "codex_parser_3.rs"]` |
| `sessions/providers/codex_messages.rs:394` | `#[path = "codex_parser_4.rs"]` |
| `sessions/providers/gemini.rs:14` | `#[path = "gemini_document.rs"]` |
| `skills/archive.rs:315` | `#[path = "archive_tests.rs"]` |
| `skills/discovery_cache.rs:141` | `#[path = "discovery_cache_tests.rs"]` |
| `usage/balances/deepseek.rs:153` | `#[path = "deepseek_tests.rs"]` |
| `usage/balances/novita.rs:111` | `#[path = "novita_tests.rs"]` |
| `usage/balances/openrouter.rs:99` | `#[path = "openrouter_tests.rs"]` |
| `usage/balances/siliconflow.rs:148` | `#[path = "siliconflow_tests.rs"]` |
| `usage/balances/stepfun.rs:100` | `#[path = "stepfun_tests.rs"]` |
| `usage/balances/zhipu.rs:69` | `#[path = "zhipu_tests.rs"]` |
| `usage/coding/command_code.rs:372` | `#[path = "command_code_tests.rs"]` |
| `usage/coding/kimi.rs:246` | `#[path = "kimi_tests.rs"]` |
| `usage/coding/minimax.rs:204` | `#[path = "minimax_tests.rs"]` |
| `usage/coding/opencode_go.rs:157` | `#[path = "opencode_go_tests.rs"]` |
| `usage/coding/zenmux.rs:213` | `#[path = "zenmux_tests.rs"]` |
| `usage/coding/zhipu.rs:298` | `#[path = "zhipu_tests.rs"]` |
| `usage/coding/zhipu_team.rs:87` | `#[path = "zhipu_team_tests.rs"]` |
| `usage/scripts/network.rs:188` | `#[path = "network_tests.rs"]` |

## 外部集成测试与 fixture

路径相对 `crates/vw-agent-apps/`；保留现有门面调用的集成测试可留原包，主责任任务负责校正 imports，不能为了移包删除场景。

| 文件 | 主责任任务 |
| --- | ---: |
| `tests/fixtures/mcp/codex-config.toml` | 40 |
| `tests/usage_claude_scan.rs` | 62 |
| `tests/usage_codex_delta.rs` | 61 |
| `tests/usage_dsh.rs` | 62 |
| `tests/usage_exchange.rs` | 55 |
| `tests/usage_gemini.rs` | 62 |
| `tests/usage_grokbuild.rs` | 62 |
| `tests/usage_kimi.rs` | 62 |
| `tests/usage_logs.rs` | 63 |
| `tests/usage_mcode.rs` | 62 |
| `tests/usage_model_names.rs` | 55 |
| `tests/usage_pi.rs` | 62 |
| `tests/usage_pi_children.rs` | 62 |
| `tests/usage_tokens.rs` | 55 |

## 源码边界引用证据

下表记录跨顶层领域的显式 crate 路径（含测试），用于逐项复核而非直接转换为 Cargo 边。super/self、同领域跨任务依赖仍需沿模块树解析；已确认的循环切口见前表。

| 来源 | 被引用路径 |
| --- | --- |
| `aggregation/catalog_ack.rs` | `crate::providers::{     codex_catalog::CodexFiles,     live::{         commit::ProviderTransactions,         write_guard::{FilePrivacy, WriteError}` |
| `aggregation/catalog_ack_tests.rs` | `crate::providers::{     codex_catalog::{CATALOG_FILENAME, CodexFiles}` |
| `aggregation/catalog_generation.rs` | `crate::providers::{     codex_catalog::{CATALOG_FILENAME, CodexFiles}` |
| `aggregation/catalog_generation_tests.rs` | `crate::providers::{     codex_catalog::{CATALOG_FILENAME, CodexFiles}` |
| `aggregation/claude_publish.rs` | `crate::providers::claude_login::configuration_has_pending_login`<br>`crate::providers::{     claude::patch_with_residue,     live::{         commit::*,         stage::FileChange,         write_guard::{FilePrivacy, WriteError}` |
| `aggregation/claude_publish_tests.rs` | `crate::providers::live::write_guard::WriteError`<br>`crate::providers::live::write_guard::WriteError::Conflict`<br>`crate::providers::live::{         commit::{HOOK, Phase}`<br>`crate::providers::{     claude::ClaudeFile, live::commit::*, mode_apply::*, mode_state::StackState, }` |
| `aggregation/claude_refresh_tests.rs` | `crate::providers::{     claude::ClaudeFile, live::commit::*, mode_apply::*, mode_state::StackState, }` |
| `aggregation/codex_effort_tests.rs` | `crate::providers::live::patch_toml::TomlPatch` |
| `aggregation/codex_publish.rs` | `crate::providers::{     codex_catalog::{CATALOG_FILENAME, CodexFiles}` |
| `aggregation/codex_publish_tests.rs` | `crate::providers::{     codex_catalog::{CATALOG_FILENAME, CodexFiles}` |
| `aggregation/codex_settings.rs` | `crate::providers::codex_catalog::CATALOG_FILENAME`<br>`crate::providers::live::{     commit::{AppState, Mode}` |
| `aggregation/daemon_restart.rs` | `crate::providers::{codex_catalog::CodexFiles, live::commit::ProviderTransactions}` |
| `aggregation/daemon_restart_tests.rs` | `crate::providers::live::commit::` |
| `aggregation/default_provider_tests.rs` | `crate::providers::{     live::{         commit::{AppState, ExecutionHostId, Mode, ModeState, PendingTarget, ProviderTransactions}` |
| `aggregation/exit_restore.rs` | `crate::models::codex_projection`<br>`crate::providers::claude_login::configuration_has_pending_login`<br>`crate::providers::codex::project`<br>`crate::providers::live::patch_json::object_value`<br>`crate::providers::{     claude::ClaudeFile,     codex_catalog::{CATALOG_FILENAME, CodexFiles}` |
| `aggregation/exit_restore_tests.rs` | `crate::providers::codex_catalog::CodexFiles::open`<br>`crate::providers::codex_login::prepare`<br>`crate::providers::codex_login::{AuthTarget, LoginInput}`<br>`crate::providers::live::{recovery::RecoveryOutcome, write_guard::WriteError}`<br>`crate::providers::{             codex_catalog::CodexFiles,             codex_login::{AuthTarget, LoginInput}`<br>`crate::providers::{claude::ClaudeFile, live::commit::*, mode_state::StackState}` |
| `aggregation/failover_guard.rs` | `crate::providers::{live::commit::AppState, mode_state::observe}`<br>`crate::routing::supports_app` |
| `aggregation/failover_guard_tests.rs` | `crate::providers::{     live::commit::{AppState, Mode, ModeState}` |
| `aggregation/lifecycle_restore.rs` | `crate::providers::{     claude::ClaudeFile,     codex_catalog::CodexFiles,     codex_login::LoginInput,     live::commit::{Mode, ProviderTransactions}` |
| `aggregation/lifecycle_restore_tests.rs` | `crate::providers::live::write_guard::lock_app`<br>`crate::providers::live::{recovery::RecoveryOutcome, write_guard::WriteError}`<br>`crate::providers::{         codex_catalog::CodexFiles,         codex_login::{AuthTarget, LoginInput}`<br>`crate::providers::{     claude::ClaudeFile, live::commit::*, mode_apply::*, mode_state::StackState, }` |
| `aggregation/member_keys.rs` | `crate::providers::mode_state::StackState` |
| `aggregation/member_keys_tests.rs` | `crate::providers::{     live::{         commit::{ExecutionHostId, PendingTarget, ProviderTransactions}` |
| `aggregation/member_validation.rs` | `crate::providers::live::commit::{AppState, Mode}` |
| `aggregation/member_validation_tests.rs` | `crate::providers::{     live::commit::{AppState, Mode, ModeState}` |
| `aggregation/membership.rs` | `crate::providers::{     live::{         commit::{AppState, Mode, ModeState, PendingTarget, ProviderTransactions}` |
| `aggregation/omo_read.rs` | `crate::providers::{live::patch_json::object_value, paths, snippets, validate_identity}` |
| `aggregation/omo_scope.rs` | `crate::providers::live::write_guard::lock_app`<br>`crate::providers::live::{     commit::{AppState, OmoSelection, PendingTarget, ProviderTransactions}` |
| `aggregation/omo_slim.rs` | `crate::providers::{     live::{         commit::ProviderTransactions,         patch_json::{object_value, replace_object_preserving_trivia}` |
| `aggregation/omo_slim_tests.rs` | `crate::providers::live::{     commit::{ExecutionHostId, HOOK, Phase, ProviderTransactions}` |
| `aggregation/omo_write.rs` | `crate::providers::{     live::{         commit::ProviderTransactions,         patch_json::{JsonPatch, object_value, replace_member_preserving_trivia}` |
| `aggregation/omo_write_tests.rs` | `crate::providers::live::{     commit::{ExecutionHostId, HOOK, Phase, ProviderTransactions}`<br>`crate::providers::{         live::{             stage::{FileChange, WholeFile}` |
| `aggregation/preview.rs` | `crate::providers::desktop_mapping::catalog::{         DesktopMappingMember, DesktopModelCatalog, member_models,     }`<br>`crate::providers::live::commit::AppState`<br>`crate::providers::mode_state::observe` |
| `aggregation/preview_tests.rs` | `crate::providers::live::commit::AppState`<br>`crate::providers::{     live::{         commit::{ExecutionHostId, Mode, ModeState, PendingTarget, ProviderTransactions}` |
| `aggregation/resolve.rs` | `crate::providers::{     desktop_mapping::catalog::{DesktopModel, is_reserved as is_desktop_reserved}` |
| `aggregation/resolve_tests.rs` | `crate::providers::{     desktop_mapping::catalog::{DesktopMappingMember, DesktopModelCatalog}` |
| `aggregation/stale_clients_tests.rs` | `crate::providers::{     codex_catalog::{CATALOG_FILENAME, CodexFiles}` |
| `apps/launch/account.rs` | `crate::auth::codex_refresh::should_adopt_cli_rotation`<br>`crate::auth::credential_file::read_credential`<br>`crate::auth::{credential_file::AccountCredential, credential_lease::CredentialLease}` |
| `apps/launch/account_lifecycle_tests.rs` | `crate::auth::credential_file::{AccountCredential, CredentialFileStore}` |
| `apps/launch/account_tests.rs` | `crate::auth::credential_file::{AccountCredential, CredentialFileStore}` |
| `apps/launch/provider.rs` | `crate::providers::agy::Lease`<br>`crate::providers::validate_profile` |
| `auth/accounts.rs` | `crate::providers::validate_identity` |
| `auth/binding_tests.rs` | `crate::providers::kimi::declarations` |
| `auth/claude_login_files.rs` | `crate::providers::live::write_guard::WriteError` |
| `auth/claude_recover_tests.rs` | `crate::providers::{     claude::ClaudeFile,     claude_login::{LOGIN_CHECKPOINT, LoginHost, LoginPhase}` |
| `auth/claude_rotation.rs` | `crate::providers::live::commit::AppState`<br>`crate::providers::{     claude_login::LoginHost,     live::{commit::ProviderTransactions, write_guard::WriteError}` |
| `auth/claude_rotation_tests.rs` | `crate::providers::{     claude::ClaudeFile,     claude_login::{LoginHost, LoginMaterial}` |
| `auth/claude_snapshot_tests.rs` | `crate::providers::{     claude::ClaudeFile,     claude_login::LoginHost,     live::{         commit::{AppState, ExecutionHostId, ProviderTransactions}` |
| `auth/codex_device.rs` | `crate::providers::{codex_login_material::parse_codex_jwt_claims, validate_identity}` |
| `auth/codex_refresh.rs` | `crate::providers::{codex_login_material::parse_codex_jwt_claims, validate_identity}` |
| `auth/codex_refresh_tests.rs` | `crate::apps::launch::{account::prepare_account_launch, request::LaunchMaterial}` |
| `auth/copilot_token.rs` | `crate::providers::validate_identity` |
| `auth/credential_file.rs` | `crate::providers::claude_login::LoginMaterial`<br>`crate::providers::validate_identity` |
| `auth/credential_file/host.rs` | `crate::providers::paths::open_beneath` |
| `auth/credential_lease.rs` | `crate::providers::live::write_guard::WriteError`<br>`crate::providers::{             claude_login::{LoginMaterial, lock_account}` |
| `auth/dsh_account.rs` | `crate::providers::validate_identity`<br>`crate::providers::{     dsh::{DshCredentials, credential_document}` |
| `auth/dsh_account_tests.rs` | `crate::providers::live::commit::{ExecutionHostId, ProviderTransactions}` |
| `auth/flow.rs` | `crate::providers::validate_identity` |
| `auth/gemini_refresh.rs` | `crate::providers::validate_identity` |
| `auth/kimi.rs` | `crate::providers::validate_identity` |
| `auth/kimi_account.rs` | `crate::providers::validate_identity`<br>`crate::providers::{     kimi::{self, KimiFiles}` |
| `auth/kimi_account/binding.rs` | `crate::providers::live::commit::{AppState, KimiBinding}`<br>`crate::providers::validate_identity` |
| `auth/kimi_account/binding_tests.rs` | `crate::providers::kimi::declarations` |
| `auth/kimi_account_tests.rs` | `crate::providers::live::commit::{ExecutionHostId, ProviderTransactions}` |
| `auth/xai_refresh.rs` | `crate::providers::validate_identity` |
| `daily_memory_files.rs` | `crate::workspace_paths::{PathError, WorkspaceArea, WorkspaceDirectory}` |
| `daily_memory_list.rs` | `crate::workspace_paths::{PathError, WorkspaceArea, WorkspaceDirectory}` |
| `data_management/apply_tests.rs` | `crate::usage::store::initialize_usage_store` |
| `data_management/backup_create_files.rs` | `crate::providers::paths::open_beneath` |
| `data_management/backup_create_tests.rs` | `crate::usage::source_settings::{     UsageSourceSettings, initialize_source_settings, load_source_settings, save_source_settings, }` |
| `data_management/backup_delete_tests.rs` | `crate::usage::source_settings::{initialize_source_settings, load_source_settings}` |
| `data_management/backup_list_tests.rs` | `crate::usage::source_settings::{initialize_source_settings, load_source_settings}` |
| `data_management/backup_rename_tests.rs` | `crate::usage::source_settings::{initialize_source_settings, load_source_settings}` |
| `data_management/backup_restore_tests.rs` | `crate::usage::source_settings::{initialize_source_settings, load_source_settings, save_source_settings, UsageSourceSettings}` |
| `data_management/mcp.rs` | `crate::mcp::{catalog::McpCatalogDraft, validation::parse_native}` |
| `data_management/operations.rs` | `crate::apps::lifecycle::operations::Operation` |
| `data_management/preferences.rs` | `crate::apps::preferences::{HostPlatform, default_user_preferences, normalize_preferences}` |
| `data_management/prompts.rs` | `crate::prompts::listing::PromptEntry`<br>`crate::providers::validate_identity` |
| `data_management/providers.rs` | `crate::providers::claude::import`<br>`crate::providers::claude::settings`<br>`crate::providers::codex_catalog::import`<br>`crate::providers::codex_catalog::settings`<br>`crate::providers::endpoints::update`<br>`crate::providers::gemini::settings`<br>`crate::providers::icons::read`<br>`crate::providers::presentation::read`<br>`crate::providers::presentation::update`<br>`crate::providers::validate_identity`<br>`crate::providers::validation::endpoint` |
| `data_management/retention_tests.rs` | `crate::usage::source_settings::{initialize_source_settings, load_source_settings}` |
| `data_management/safety_snapshot_tests.rs` | `crate::usage::source_settings::{     UsageSourceSettings, initialize_source_settings, load_source_settings, save_source_settings, }` |
| `data_management/skills.rs` | `crate::providers::paths::open_beneath`<br>`crate::skills::archive::MAX_ARCHIVE_ENTRIES`<br>`crate::skills::archive::MAX_ARCHIVE_TOTAL_BYTES`<br>`crate::skills::repos::validate_repo_ref`<br>`crate::skills::{     bulk_sync::SkillSyncTarget,     copy_deploy::{CopyBinding, CopyDeploy, binding_fingerprint}` |
| `data_management/usage.rs` | `crate::usage::import_store::load_imported_usage`<br>`crate::usage::import_store::replace_imported_usage_in_transaction`<br>`crate::usage::store::{self, StoredUsageRecord, UsageSourceIdentity}` |
| `data_management/usage_tests.rs` | `crate::usage::store::initialize_usage_store` |
| `device_sync/skill_update.rs` | `crate::providers::paths::open_beneath`<br>`crate::skills::{     copy_deploy::{binding_fingerprint, CopyBinding, CopyDeploy}` |
| `diagnostics/endpoints_tests.rs` | `crate::providers::endpoints` |
| `hermes_memory.rs` | `crate::workspace_paths::{PathError, WorkspaceArea, WorkspaceDirectory}` |
| `hermes_memory_limits.rs` | `crate::providers::{     hermes::HermesFiles,     live::{         commit::{AppState, ProviderTransactions}` |
| `hermes_memory_tests.rs` | `crate::workspace_paths::PathError` |
| `high_availability/queue_tests.rs` | `crate::providers::failover::{ProviderAuth, ProviderSnapshot}` |
| `mcp/adapters/agy_tests.rs` | `crate::providers::live::{     commit::{ExecutionHostId, ProviderTransactions}` |
| `mcp/adapters/dsh_tests.rs` | `crate::providers::dsh::read` |
| `mcp/adapters/gemini_tests.rs` | `crate::providers::live::{     commit::{ExecutionHostId, ProviderTransactions}` |
| `mcp/adapters/grokbuild_tests.rs` | `crate::providers::{     grok::GrokFiles,     live::{         commit::{ExecutionHostId, ProviderTransactions}` |
| `mcp/adapters/hermes_tests.rs` | `crate::providers::{     live::{         commit::{ExecutionHostId, ProviderTransactions}` |
| `mcp/adapters/kimi_tests.rs` | `crate::providers::{     kimi::KimiFiles,     live::{         commit::{ExecutionHostId, ProviderTransactions}` |
| `mcp/adapters/mcode_tests.rs` | `crate::providers::{     live::{         commit::{ExecutionHostId, ProviderTransactions}` |
| `mcp/adapters/omp_tests.rs` | `crate::providers::{     live::{         commit::{ExecutionHostId, ProviderTransactions}` |
| `mcp/adapters/opencode_tests.rs` | `crate::providers::{     live::{         commit::{ExecutionHostId, ProviderTransactions}` |
| `mcp/adapters/pi_tests.rs` | `crate::providers::{     live::{         commit::{ExecutionHostId, ProviderTransactions}` |
| `mcp/distribution.rs` | `crate::providers::{     dsh::DshFiles,     live::{commit::ProviderTransactions, write_guard::WriteError}` |
| `mcp/native_file.rs` | `crate::providers::live::{     commit::{PendingTarget, ProviderTransactions}` |
| `mcp/native_file_tests.rs` | `crate::providers::live::{     commit::{ExecutionHostId, HOOK, Phase, ProviderTransactions}` |
| `models/codex_template_tests.rs` | `crate::providers::codex_catalog::form` |
| `models/defaults_tests.rs` | `crate::providers::codex` |
| `models/discovery_agy_tests.rs` | `crate::apps::discovery::read::LocalReadEnvironment` |
| `models/discovery_http.rs` | `crate::providers::validation::{header_value, protected_header}` |
| `models/discovery_opencode_tests.rs` | `crate::apps::discovery::read::LocalReadEnvironment` |
| `models/discovery_url.rs` | `crate::providers::validation` |
| `models/edits.rs` | `crate::providers::{validate_identity, validation::endpoint}` |
| `models/fill_hermes_tests.rs` | `crate::providers::hermes::normalize_declaration` |
| `models/fill_kimi_tests.rs` | `crate::providers::kimi` |
| `models/fill_opencode_tests.rs` | `crate::providers::{dsh, mcode, opencode}` |
| `models/fill_pi.rs` | `crate::providers::validation` |
| `models/fill_pi_tests.rs` | `crate::providers::pi::{declarations, patch}` |
| `models/identity.rs` | `crate::providers::validation` |
| `models/mod.rs` | `crate::providers::{validate_identity, validation::endpoint}` |
| `models/preset_catalog.rs` | `crate::providers::{universal_sync::projected_endpoint, validation::endpoint}` |
| `models/preset_catalog_tests.rs` | `crate::providers::{universal, universal_sync, update}` |
| `models/preset_omp_tests.rs` | `crate::providers::omp::validate` |
| `models/user_agent_presets.rs` | `crate::providers::validation::custom_user_agent` |
| `openclaw_catalog.rs` | `crate::providers::live::{     patch_json5::{object_value, replace_member_preserving_trivia}` |
| `openclaw_catalog_tests.rs` | `crate::providers::live::patch_json::object_value` |
| `openclaw_defaults.rs` | `crate::providers::{     live::{         commit::{AppState, ProviderTransactions}` |
| `openclaw_defaults_tests.rs` | `crate::providers::live::patch_json::object_value` |
| `openclaw_health.rs` | `crate::providers::{live::{patch_json5::object_value, write_guard::WriteError}` |
| `openclaw_model.rs` | `crate::providers::{     live::{         commit::{AppState, ProviderTransactions}` |
| `openclaw_tools.rs` | `crate::providers::{     live::{         commit::{AppState, ProviderTransactions}` |
| `openclaw_tools_tests.rs` | `crate::providers::live::patch_json::object_value` |
| `project_profiles/apply_provider.rs` | `crate::providers::validate_identity` |
| `prompts/activation.rs` | `crate::providers::live::{     commit::{PendingTarget, ProviderTransactions}` |
| `prompts/live_file.rs` | `crate::providers::paths` |
| `prompts/mcode.rs` | `crate::providers::live::{     commit::{PendingTarget, ProviderTransactions}` |
| `prompts/omp.rs` | `crate::providers::paths` |
| `prompts/pi_agents.rs` | `crate::providers::live::{         commit::{PendingTarget, ProviderTransactions}` |
| `prompts/pi_system_files.rs` | `crate::providers::live::{     commit::{PendingTarget, ProviderTransactions}` |
| `prompts/sync.rs` | `crate::apps::lifecycle::operations::Operation` |
| `prompts/targets.rs` | `crate::providers::paths` |
| `prompts/template_rename.rs` | `crate::providers::live::{     commit::{PendingTarget, ProviderTransactions}` |
| `prompts/templates.rs` | `crate::providers::live::{     commit::{PendingTarget, ProviderTransactions}` |
| `providers/agy.rs` | `crate::apps::launch::{provider::ProviderLaunch, request::LaunchMaterial}` |
| `providers/agy_tests.rs` | `crate::apps::launch::request::LaunchMaterial` |
| `providers/claude_login.rs` | `crate::auth::claude_rotation::expiry`<br>`crate::auth::{     claude_keychain::ClaudeKeychain,     claude_login_files::{CliLock, Directory}` |
| `providers/claude_login_tests.rs` | `crate::auth::claude_keychain::ClaudeKeychain`<br>`crate::auth::claude_keychain::KeychainError`<br>`crate::auth::claude_keychain::MacOsKeychain` |
| `providers/codex_catalog.rs` | `crate::models::codex_projection` |
| `providers/desktop_mapping/catalog.rs` | `crate::aggregation::model_ids::{Decoded, decode, encode}` |
| `providers/icons.rs` | `crate::icons::validation::is_valid_custom_icon` |
| `providers/icons_tests.rs` | `crate::icons::library` |
| `providers/kimi_form.rs` | `crate::auth::kimi_account::binding_region` |
| `providers/live/recovery.rs` | `crate::apps::catalog::catalog` |
| `providers/operations.rs` | `crate::apps::lifecycle::operations::Operation` |
| `providers/universal.rs` | `crate::icons::validation::is_valid_custom_icon` |
| `providers/validation_tests.rs` | `crate::apps::launch::{provider::prepare_provider_launch, request::LaunchMaterial}` |
| `routing/media_sanitize.rs` | `crate::models::capabilities::resolve_model_capabilities` |
| `routing/override_validate_tests.rs` | `crate::providers::request_options::{update, view}` |
| `routing/target_model.rs` | `crate::providers::desktop_mapping::defaults` |
| `routing/target_model_tests.rs` | `crate::aggregation::model_ids::{Decoded, decode}` |
| `routing/user_agent_tests.rs` | `crate::providers::request_options` |
| `sessions/activity.rs` | `crate::aggregation::stale_clients::process_table` |
| `sessions/compression.rs` | `crate::providers::{     live::{         commit::{PendingTarget, ProviderTransactions}` |
| `sessions/maintenance.rs` | `crate::apps::lifecycle::operations::{Operation, Phase, Progress}` |
| `sessions/migration/provider_refs.rs` | `crate::providers::live::{     commit::{AppState, PendingTarget, ProviderTransactions}` |
| `sessions/migration/rollout.rs` | `crate::providers::validate_identity` |
| `sessions/migration/state_db.rs` | `crate::providers::validate_identity`<br>`crate::providers::{     live::write_guard::{WriteError, lock_directory}` |
| `sessions/resume.rs` | `crate::apps::launch::host::command`<br>`crate::apps::launch::request::LaunchMaterial`<br>`crate::apps::launch::request::validate_launch_request` |
| `sessions/terminal/alacritty.rs` | `crate::apps::launch::{request::LaunchMaterial, terminal_session::TerminalSession}` |
| `sessions/terminal/apple.rs` | `crate::apps::launch::{request::LaunchMaterial, terminal_session::TerminalSession}` |
| `sessions/terminal/custom.rs` | `crate::apps::launch::{request::LaunchMaterial, terminal_session::TerminalSession}` |
| `sessions/terminal/ghostty_kitty.rs` | `crate::apps::launch::{request::LaunchMaterial, terminal_session::TerminalSession}` |
| `sessions/terminal/otty.rs` | `crate::apps::launch::{request::LaunchMaterial, terminal_session::TerminalSession}` |
| `sessions/terminal/warp.rs` | `crate::apps::launch::{request::LaunchMaterial, terminal_session::TerminalSession}` |
| `sessions/terminal/wezterm_kaku.rs` | `crate::apps::launch::{request::LaunchMaterial, terminal_session::TerminalSession}` |
| `skills/app_paths.rs` | `crate::providers::paths` |
| `skills/app_paths_tests.rs` | `crate::apps::catalog::catalog` |
| `skills/backup_catalog.rs` | `crate::providers::paths::open_beneath` |
| `skills/backup_restore.rs` | `crate::providers::paths::open_beneath` |
| `skills/claude_onboarding.rs` | `crate::providers::{     live::{         commit::{PendingTarget, ProviderTransactions}` |
| `skills/claude_onboarding_tests.rs` | `crate::providers::live::{     commit::{ExecutionHostId, ProviderTransactions}` |
| `skills/claude_status.rs` | `crate::providers::{     live::{         commit::ProviderTransactions,         write_guard::{WriteError, lock_directory}` |
| `skills/claude_status_tests.rs` | `crate::providers::live::{     commit::{ExecutionHostId, ProviderTransactions}` |
| `skills/claude_toggle.rs` | `crate::providers::{     live::{         commit::{PendingTarget, ProviderTransactions}` |
| `skills/claude_toggle_tests.rs` | `crate::providers::live::{     commit::{ExecutionHostId, ProviderTransactions}` |
| `skills/copy_deploy.rs` | `crate::providers::paths::open_beneath` |
| `skills/disable_deploy.rs` | `crate::providers::paths::open_beneath` |
| `skills/inventory.rs` | `crate::providers::paths::open_beneath` |
| `skills/local_install.rs` | `crate::providers::paths::open_beneath` |
| `skills/storage_move.rs` | `crate::providers::paths::open_beneath` |
| `skills/uninstall.rs` | `crate::providers::paths::open_beneath` |
| `skills/unmanaged_scan.rs` | `crate::providers::paths::open_beneath` |
| `usage/agy.rs` | `crate::sessions::providers::agy::records`<br>`crate::sessions::providers::agy::timestamp` |
| `usage/agy_native.rs` | `crate::sessions::providers::agy::checked_path` |
| `usage/balances/deepseek.rs` | `crate::providers::validation::header_value` |
| `usage/balances/novita.rs` | `crate::providers::validation::header_value` |
| `usage/balances/openrouter.rs` | `crate::providers::validation::header_value` |
| `usage/balances/stepfun.rs` | `crate::providers::validation::header_value` |
| `usage/binding_tests.rs` | `crate::auth::credential_file::{AccountCredential, CredentialFileStore}`<br>`crate::auth::flow::AuthAccountScope` |
| `usage/claude_scan.rs` | `crate::sessions::source::SourceRegistry` |
| `usage/codex_lineage.rs` | `crate::sessions::rollout` |
| `usage/codex_rebuild.rs` | `crate::apps::lifecycle::operations::Operation` |
| `usage/coding/kimi.rs` | `crate::providers::validation::header_value` |
| `usage/coding/volcengine_sign.rs` | `crate::providers::validation::header_value` |
| `usage/coding/zenmux.rs` | `crate::providers::validation::header_value` |
| `usage/gemini.rs` | `crate::sessions::providers::gemini::{is_session_file, parse_session_document}` |
| `usage/kimi.rs` | `crate::sessions::providers::kimi_requests::{RequestIndex, RequestInfo, timestamp_ms}` |
| `usage/mcode.rs` | `crate::sessions::providers::mcode::open_database` |
| `usage/opencode.rs` | `crate::sessions::providers::opencode_sqlite::open` |
| `usage/pricing.rs` | `crate::models::edits::validate_model_id` |
| `usage/scripts/extract.rs` | `crate::apps::lifecycle::operations::Operation` |
| `usage/scripts/network.rs` | `crate::providers::validation::header_value` |
| `usage/scripts/run.rs` | `crate::apps::lifecycle::operations::Operation` |
| `usage/scripts/run_tests.rs` | `crate::apps::lifecycle::operations::Operation` |
| `usage/scripts/runtime.rs` | `crate::apps::lifecycle::operations::Operation` |
| `usage/subscriptions/claude.rs` | `crate::providers::validation::header_value` |
| `usage/subscriptions/kimi_tests.rs` | `crate::auth::{     copilot_tests::oauth_server,     credential_file::AccountCredential,     kimi::KimiDeviceAuth,     kimi_account_tests::{ACCOUNT, CONFIG, Fixture}` |
| `workspace_files.rs` | `crate::workspace_paths::{PathError, WorkspaceArea, WorkspaceDirectory}` |
| `workspace_paths.rs` | `crate::providers::{     live::write_guard::{FilePrivacy, lock_directory}` |

## 真实外部调用者

以下为 rg 命中的 Rust 调用文件；继续经原门面保持路径，不要求一次改完外部包。

| 文件 |
| --- |
| `crates/vw-agent-apps/tests/usage_pi.rs` |
| `crates/vw-agent-apps/tests/usage_model_names.rs` |
| `crates/vw-agent/src/app_routing/config_apply.rs` |
| `crates/vw-agent-apps/tests/usage_tokens.rs` |
| `crates/vw-agent-apps/tests/usage_codex_delta.rs` |
| `crates/vw-agent/src/app_routing/failover_execution.rs` |
| `crates/vw-agent-apps/tests/usage_claude_scan.rs` |
| `crates/vw-agent/src/provider/model_catalog.rs` |
| `crates/vw-agent/src/device_sync/webdav_settings.rs` |
| `crates/vw-agent/src/app_providers/kimi_binding.rs` |
| `crates/vw-agent/src/usage_bridge/relay.rs` |
| `crates/vw-agent/src/app_routing/shutdown_tests.rs` |
| `crates/vw-agent-apps/tests/usage_pi_children.rs` |
| `crates/vw-agent/src/app_routing/override_apply.rs` |
| `crates/vw-agent-apps/tests/usage_gemini.rs` |
| `crates/vw-agent/src/agent_apps/prompt_toggle_store.rs` |
| `crates/vw-agent-apps/tests/usage_grokbuild.rs` |
| `crates/vw-agent/src/agent_apps/shared_snapshot.rs` |
| `crates/vw-agent/src/model_relay/responses_chat_stream.rs` |
| `crates/vw-agent/src/device_sync/projection.rs` |
| `crates/vw-agent/src/device_sync/s3_upload.rs` |
| `crates/vw-agent/src/app_providers/basic.rs` |
| `crates/vw-agent/src/app_routing/capacity_tests.rs` |
| `crates/vw-agent/src/app_routing/listener_tests.rs` |
| `crates/vw-agent-apps/tests/usage_mcode.rs` |
| `crates/vw-agent/src/agent_apps/auth_accounts_tests.rs` |
| `crates/vw-agent-apps/tests/usage_kimi.rs` |
| `crates/vw-agent/src/agent_apps/prompts_bootstrap.rs` |
| `crates/vw-agent/src/app_providers/agy_binding.rs` |
| `crates/vw-agent/src/usage_bridge/events.rs` |
| `crates/vw-agent/src/model_relay/responses_compaction_tests.rs` |
| `crates/vw-agent/src/agent_apps/store.rs` |
| `crates/vw-agent/src/app_routing/failover_execution_tests.rs` |
| `crates/vw-agent/src/device_sync/s3_settings.rs` |
| `crates/vw-agent/src/device_sync/s3_download.rs` |
| `crates/vw-agent/src/app_routing/override_apply_tests.rs` |
| `crates/vw-agent-apps/tests/usage_dsh.rs` |
| `crates/vw-agent-apps/tests/usage_exchange.rs` |
| `crates/vw-agent/src/app_providers/kimi_binding_tests.rs` |
| `crates/vw-agent/src/agent_apps/preferences_import.rs` |
| `crates/vw-agent-apps/tests/usage_logs.rs` |
| `crates/vw-agent/src/agent_apps/auth_accounts/defaults.rs` |
| `crates/vw-agent/src/usage_bridge/native.rs` |
| `crates/vw-agent/src/model_relay/responses_codex_chat.rs` |
| `crates/vw-agent/src/app_routing/provider_publication.rs` |
| `crates/vw-agent/src/app_routing/route_auth.rs` |
| `crates/vw-agent/src/app_providers/snippets.rs` |
| `crates/vw-agent/src/device_sync/apply.rs` |
| `crates/vw-agent/src/device_sync/export.rs` |
| `crates/vw-agent/src/device_sync/skill_receive.rs` |
| `crates/vw-agent/src/agent_apps/prompt_sync_store.rs` |
| `crates/vw-agent/src/device_sync/webdav_upload.rs` |
| `crates/vw-agent/src/device_sync/webdav_download.rs` |
| `crates/vw-agent/src/agent_apps/auth_accounts/read.rs` |
| `crates/vw-agent/src/app_routing/listener.rs` |
| `crates/vw-agent/src/app_providers/desktop_mapping.rs` |
| `crates/vw-agent/src/app_routing/failover_routing_tests.rs` |
| `crates/vw-agent/src/app_providers/record_publication.rs` |
| `crates/vw-agent/src/model_relay/compaction_v2_tests.rs` |
| `crates/vw-agent/src/agent_apps/prompt_activation_store.rs` |
| `crates/vw-agent/src/app_providers/codex_binding_tests.rs` |
| `crates/vw-agent/src/agent_apps/usage_settings.rs` |
| `crates/vw-agent/src/agent_apps/auth_accounts/gemini.rs` |
| `crates/vw-agent/src/app_routing/compaction_tests.rs` |
| `crates/vw-agent/src/agent_apps/prompt_data_migration.rs` |
| `crates/vw-agent/src/agent_apps/prompt_deactivation_store.rs` |
| `crates/vw-agent/src/agent_apps/shared_snapshot_import.rs` |
| `crates/vw-agent/src/app_routing/aggregation_routing_tests.rs` |
| `crates/vw-agent/src/agent_apps/profile_prompt_store.rs` |
| `crates/vw-agent/src/agent_apps/auth_accounts.rs` |
| `crates/vw-agent/src/agent_apps/auth_accounts/logout.rs` |
| `crates/vw-agent/src/app_providers/grok.rs` |
| `crates/vw-agent/src/agent_apps/auth_accounts/removal.rs` |
| `crates/vw-agent/src/agent_apps/prompts_store.rs` |
| `crates/vw-agent/src/app_routing/protocol_errors.rs` |
| `crates/vw-agent/src/model_relay/signature_rectify.rs` |
| `crates/vw-agent/src/agent_apps/data_import.rs` |
| `crates/vw-agent/src/app_providers/universal_sync.rs` |
| `crates/vw-agent/src/app_routing/desktop_routing.rs` |
| `crates/vw-agent/src/app_providers/failover_tests.rs` |
| `crates/vw-agent/src/agent_apps/mcp/import_tests.rs` |
| `crates/vw-agent/src/app_providers/account_removal.rs` |
| `crates/vw-agent/src/agent_apps/prompt_data_migration_tests.rs` |
| `crates/vw-agent/src/app_providers/claude_binding.rs` |
| `crates/vw-agent/src/app_providers/record_publication_tests.rs` |
| `crates/vw-agent/src/app_providers/agy_form.rs` |
| `crates/vw-agent/src/model_relay/messages_gemini_request.rs` |
| `crates/vw-agent/src/model_relay/responses_anthropic_request.rs` |
| `crates/vw-agent/src/app_providers/official_launch.rs` |
| `crates/vw-agent/src/app_providers.rs` |
| `crates/vw-agent/src/app_routing/failover_routing.rs` |
| `crates/vw-agent/src/app_providers/claude_desktop.rs` |
| `crates/vw-agent/src/app_providers/agy.rs` |
| `crates/vw-agent/src/agent_apps/mcp/data_migration.rs` |
| `crates/vw-agent/src/app_providers/gemini.rs` |
| `crates/vw-agent/src/app_providers/generation_target.rs` |
| `crates/vw-agent/src/app_providers/claude.rs` |
| `crates/vw-agent/src/app_providers/import.rs` |
| `crates/vw-agent/src/app_providers/request_options.rs` |
| `crates/vw-agent/src/app_routing/aggregation_routing.rs` |
| `crates/vw-agent/src/app_providers/failover.rs` |
| `crates/vw-agent/src/agent_apps/mcp/probe_jobs.rs` |
| `crates/vw-agent/src/app_providers/codex_binding.rs` |
| `crates/vw-agent/src/agent_apps/mcp/resync.rs` |
| `crates/vw-agent/src/app_providers/universal.rs` |
| `crates/vw-agent/src/app_providers/model_edits.rs` |
| `crates/vw-agent/src/app_routing/adapter_selection_tests.rs` |
| `crates/vw-agent/src/model_relay/inline_think.rs` |
| `crates/vw-agent/src/gateway/agent_apps_high_availability_store.rs` |
| `crates/vw-agent/src/agent_apps/mcp/resync_tests.rs` |
| `crates/vw-agent/src/app_providers/codex_catalog.rs` |
| `crates/vw-agent/src/agent_apps/mcp/binding_tests.rs` |
| `crates/vw-agent/src/app_providers/data_import.rs` |
| `crates/vw-agent/src/agent_apps/mcp/network_probe.rs` |
| `crates/vw-agent/src/app_routing/dispatch.rs` |
| `crates/vw-agent/src/app_routing/adapter_selection.rs` |
| `crates/vw-agent/src/gateway/agent_apps_diagnostic_jobs.rs` |
| `crates/vw-agent/src/model_catalog.rs` |
| `crates/vw-agent/src/agent_apps/mcp/import.rs` |
| `crates/vw-agent/src/gateway/aggregation_lifecycle.rs` |
| `crates/vw-agent/src/agent_apps/mcp/credentials_tests.rs` |
| `crates/vw-agent/src/acp/catalog_mounts_tests.rs` |
| `crates/vw-agent/src/gateway/aggregation_resync.rs` |
| `crates/vw-agent/src/gateway/agent_apps_failover_runtime.rs` |
| `crates/vw-agent/src/model_relay/compaction_v2.rs` |
| `crates/vw-agent/src/agent_apps/mcp/bulk_tests.rs` |
| `crates/vw-agent/src/gateway/agent_apps_diagnostic_jobs/endpoints.rs` |
| `crates/vw-agent/src/gateway/agent_apps_skills_store_tests.rs` |
| `crates/vw-agent/src/gateway/aggregation_desktop_record.rs` |
| `crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/reachability.rs` |
| `crates/vw-agent/src/agent_apps/mcp/credentials.rs` |
| `crates/vw-agent/src/gateway/agent_apps_skill_migration.rs` |
| `crates/vw-agent/src/agent_apps/mcp/security_tests.rs` |
| `crates/vw-agent/src/gateway/agent_apps/profile_operations.rs` |
| `crates/vw-agent/src/model_relay/transport.rs` |
| `crates/vw-agent/src/gateway/agent_apps/profile_store.rs` |
| `crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/reachability_tests.rs` |
| `crates/vw-agent/src/agent_apps/mcp/connectivity_tests.rs` |
| `crates/vw-agent/src/gateway/agent_apps/profile_skills.rs` |
| `crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/model_check.rs` |
| `crates/vw-agent/src/agent_apps/mcp/catalog_create_tests.rs` |
| `crates/vw-agent/src/gateway/agent_apps_high_availability_service_tests.rs` |
| `crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/stream_check.rs` |
| `crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/speedtest.rs` |
| `crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/model_check_tests.rs` |
| `crates/vw-agent/src/gateway/agent_apps_high_availability_service.rs` |
| `crates/vw-agent/src/gateway/agent_apps_skills_store.rs` |
| `crates/vw-agent/src/agent_apps/mcp/binding.rs` |
| `crates/vw-agent/src/model_relay/responses_stream.rs` |
| `crates/vw-agent/src/gateway/api/handlers/mcp_management_update_tests.rs` |
| `crates/vw-agent/src/model_relay/native_gemini.rs` |
| `crates/vw-agent/src/gateway/api/handlers/mcp_management_resync_tests.rs` |
| `crates/vw-agent/src/model_relay/generation.rs` |
| `crates/vw-agent/src/agent_apps/mcp/profile_slot.rs` |
| `crates/vw-agent/src/gateway/api/handlers/mcp_management_read.rs` |
| `crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/model_check/target.rs` |
| `crates/vw-agent/src/gateway/aggregation_resync/exit.rs` |
| `crates/vw-agent/src/gateway/agent_apps_high_availability_store_tests.rs` |
| `crates/vw-agent/src/gateway/agent_apps_failover_runtime_tests.rs` |
| `crates/vw-agent/src/gateway/agent_apps_failover_timeouts_tests.rs` |
| `crates/vw-agent/src/model_relay/messages_stream.rs` |
| `crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/stream_check_tests.rs` |
| `crates/vw-agent/src/agent_apps/mcp/runtime_tests.rs` |
| `crates/vw-agent/src/agent_apps/mcp/catalog_create.rs` |
| `crates/vw-agent/src/gateway/api/handlers/workspace_memory_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/workspace_directory.rs` |
| `crates/vw-agent/src/gateway/api/handlers/hermes_memory.rs` |
| `crates/vw-agent/src/gateway/aggregation_resync/codex.rs` |
| `crates/vw-agent/src/gateway/aggregation_resync/publish.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_read.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_restart.rs` |
| `crates/vw-agent/src/gateway/aggregation_resync/route.rs` |
| `crates/vw-agent/src/agent_apps/mcp/data_migration_tests.rs` |
| `crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/model_check/response.rs` |
| `crates/vw-agent/src/agent_apps/mcp/host_scope_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_exit.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_resync_codex_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_exit_operation_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_integration_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_redaction_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/model_catalog.rs` |
| `crates/vw-agent/src/gateway/api/handlers/device_sync_download.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_members_ordinary_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_poll_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/hermes_memory_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_clients.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_desktop_write.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_default_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_security_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_high_availability_health.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_codex_settings_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/mcp_management_resync.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_codex_settings.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_high_availability_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_read_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_logout_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/model_catalog/presets.rs` |
| `crates/vw-agent/src/gateway/api/handlers/mcp_management_bindings.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_start.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/uninstall_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_read_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_default.rs` |
| `crates/vw-agent/src/gateway/api/handlers/device_sync_probe.rs` |
| `crates/vw-agent/src/gateway/api/handlers/model_discovery/source.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_diagnostics_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/backup_inventory.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_pi.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_omp_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/workspace_memory.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/discovery_api_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/search_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_binding.rs` |
| `crates/vw-agent/src/gateway/api/handlers/device_sync_config.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_desktop.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_publish.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_clients_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/launch_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/model_catalog_edits.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/recovery_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/aggregation_lifecycle_access.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/remote_tests_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_read_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_refresh_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/backups_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/inventory_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/macos_api_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/mod.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/install_api_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/launch.rs` |
| `crates/vw-agent/src/gateway/api/handlers/device_sync_jobs.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_native.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_codex.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/read.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/apps_usage_script.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/bindings_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_omo_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/mcp_management_delete_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/claude_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/import_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/apps_usage_refresh.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_binding_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/usage/scan.rs` |
| `crates/vw-agent/src/gateway/api/handlers/usage/read.rs` |
| `crates/vw-agent/src/gateway/api/handlers/usage/prices.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_resync_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/macos_files_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/discovery_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/storage_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/updates_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_remove.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_publish_client_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_restart_operation_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/device_sync_projection.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_pi_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_resync.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_cancel_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/mcp_management_import_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_publish_operation_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_omo.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_desktop_routing_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/apps_usage_refresh_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/mcp_management_probe_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_start/native.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/mode_apply.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/claude_login.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/jobs_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/preferences.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_read_snapshot.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/install_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_files.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_routing/config_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/backups_api_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/project_profiles_maintenance.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_remove_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_routing/lifecycle_api_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/mode_state.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/claude_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_routing/outbound_scan.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/access.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_data/backup_acceptance_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_routing/outbound_probe.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_routing/lifecycle_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/repos_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/pi.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_ha_acceptance_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_diagnostics.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_publish_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_diagnostics_acceptance_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/opencode.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/codex_catalog_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_refresh.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/openclaw_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/mcp_management_create.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_omp.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_data/backup_capture.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/claude_desktop.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_app_prompts_read.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_members_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/opencode_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/external_sessions/content.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/operations.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/claude.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_read.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/grok.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/env_diagnose.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/gemini.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/desktop_mapping_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/hermes.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_error_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_data/migration_acceptance_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/external_sessions/catalog.rs` |
| `crates/vw-agent/src/gateway/api/handlers/external_sessions/messages.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/snippets.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/kimi_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/mode_apply_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/gemini_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/external_sessions/compression.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/kimi.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/dsh.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/claude_login_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_app_prompts_write.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_data/backup_restore_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/external_sessions/delete.rs` |
| `crates/vw-agent/src/gateway/api/handlers/external_sessions/resume.rs` |
| `crates/vw-agent/src/gateway/api/handlers/external_sessions/scan.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/omp.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/codex_catalog.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/editor_merge_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/claude_form.rs` |
| `crates/vw-agent/src/gateway/api/handlers/external_sessions/maintenance.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/preview.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/env_restore.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_desktop_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/paths.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_data/migration_write_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/external_sessions/markdown.rs` |
| `crates/vw-agent/src/gateway/api/handlers/external_sessions/usage_scan.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/editor_merge.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/pi_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/agy.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/catalog.rs` |
| `crates/vw-agent/src/gateway/api/handlers/external_sessions/stream.rs` |
| `crates/vw-agent/src/gateway/api/handlers/external_sessions/images.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/desktop_mapping.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/preview_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/mcode.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_templates.rs` |
| `crates/vw-agent/src/gateway/api/handlers/external_sessions/maintenance/host.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/openclaw.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_data/periodic.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/env_clean.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/agy_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/mode_state_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/claude_managed_binding_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/project_profiles.rs` |
| `crates/vw-agent/src/gateway/api/handlers/mcp_management_delete.rs` |
| `crates/vw-agent/src/gateway/api/handlers/app_providers/claude_desktop_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_codex_settings_fixture.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_data/backup_crud_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_data/migration_read_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_data/data_aux_api_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/mcp_management_read_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_restart_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/mcp_management_import.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_members.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_app_prompts_activation.rs` |
| `crates/vw-agent/src/gateway/api/handlers/hermes_dashboard.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_data/data_aux_api.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_logout.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_shared_import.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_read.rs` |
| `crates/vw-agent/src/gateway/api/handlers/mcp_management_bindings_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/project_profiles_execution.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_sync.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_add_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/openclaw_settings.rs` |
| `crates/vw-agent/src/gateway/api/handlers/aggregation_members_client_tests.rs` |
| `crates/vw-agent/src/gateway/api/handlers/model_discovery.rs` |
| `crates/vw-agent/src/gateway/api/handlers/agent_apps_high_availability.rs` |

## 清单与平台约束

用已安装的 Python 3.12 标准库 tomllib 读取根 workspace 与 crates 下所有 Cargo.toml（默认 Python 3.9 无 tomllib，未安装任何依赖）：

- 现有包名 35 个；计划 70 个目标名冲突：无。
- workspace members 35 项；path 依赖记录 68 条，路径不存在 0 条（工作区即时快照，包含其他任务未提交变更）。
- 新包生产依赖不得反向指向 vw-agent-apps。任务 72 基于最终清单检查生产图 SCC，自环与缺失路径；不能把当前计划目录关系当作最终图。
- 原包 reqwest 关闭默认特性，form/json/rustls；serde_json 保留 arbitrary_precision；rusqlite bundled/hooks/limits。tokio 普通 macros/rt/sync/time，macOS 补 io-util/process/time，dev 补 io-util/net/time。按实际使用拆分直接依赖。
- 非 wasm 依赖 httparse/vw-provider-resolver；macOS 依赖 rquickjs/security-framework 与原生文件/钥匙串实现；unix libc。lib.rs 中 workspace、memory、OpenClaw 原生模块保持 macOS cfg，不能扩大支持面。
- .cargo/config.toml 含平台链接/wasm getrandom 配置与 Relax 别名；不改缓存策略，不执行别名。
- test.yml 当前覆盖 Linux/macOS/Windows、workspace nextest、desktop 测试和 wasm reusable workflow。它不是本轮执行授权；本轮不触发 CI。

## 静态审阅记录与待验收

已执行 rg --files 与 crate/private/include 路径检索；逐文件读取形成唯一所有权表，解析全部现有 TOML 包名及路径。RustCodeGraph status 可用；explore 返回大量 upstream 快照且结果截断，未覆盖当前边界，因此以当前 src 原始源码核实。未新增包、搬业务或改变协议。

后续需统一授权验证：最终 workspace/path/锁文件一致性、新库与真实调用方编译、原独立测试与集成测试、宏私有可见性、资源相对路径、三平台 cfg 和原生授权/回滚行为。本任务不声称这些已通过。

## 可见性审阅补充

文本出现次数（仅作线索）：`super::` 1295，`pub(crate)` 196，`pub(super)` 532，`pub(in crate::` 50。

`pub(in crate::auth)` 集中在 credential_file/host、agy_account、kimi_account 等认证子模块，随内聚模块迁移后重定向到新包内祖先模块，不得全部公开。解析器的私有 blocks、JSONL、tree 与 messages 同包迁移。跨包真实使用的 Lease、LoginHost、JWT 解析和 StackState 仅对实际消费者开放；测试访问优先保持同包独立文件。

`super::universal_sync::projected_endpoint` 所在文件还含 update 分发；任务 4 只接收纯端点规则，任务 11 保留同步编排。`mode_apply::{observe_claude_route,ModeApplyError,ClientEndpoint}` 接收任务 29，依赖任务 12 的事务及任务 24 的 ClaudeFile；不能随写入叶子下沉整个模式服务。

## 系统任务 28 补充审计（2026-11-15）

本次仅补齐实施计划，不表示应用代码拆分完成。原逐文件表为任务 1 历史基线；当前归属以本节覆盖，未覆盖行继续沿用原表。原 plan.md 保持只读；唯一入口仍为 `.plans/2026-11-14-应用底层全面拆分/prompt.md`。

RustCodeGraph status 有索引；explore 超时取消，无结果，不依赖旧索引作结论。已通过 rg 枚举包文件、全部模块声明、crate 引用和 include 资源，按当前工作树复核。工作区有大量其它会话变更，本次不提交或覆盖它们。

### 完成证据与现有契约

任务 1—9 卡已删除：1=489091f28、2=e76d80a70、3=15a0e4a4f、4=98bc27aaa、5=81168c6de、6=cd64526aa、7=833b86ef6、8=2c9adf1cf、9=fa17fb142。对应架构清单及 app-icons/config-paths/provider-validation/config-patch/catalog/availability-core/routing-policy/model-catalog 产物存在，原 Cargo.toml 已直接引用。不得重建这些任务。已核实导出：paths 全量 re-export；patch_{dotenv,json,json5,toml,yaml}；catalog/acp_projection/preferences/wsl；circuit_breaker/health/retry；routing::supports_app；models::validate/codex_projection。任务 10—74 的新目标均为计划依赖，非已存在接口。

### 本次具体缺口与唯一归属覆盖

| 来源 | 唯一接收任务 | 原因 |
| --- | --- | --- |
| workspace_paths.rs、workspace_paths_tests.rs | 73 app-workspace-paths | 安全描述符 IO 边界应早于记忆业务；需要 3 的 paths 与 12 的目录锁 |
| openclaw_catalog.rs、openclaw_catalog_tests.rs | 74 app-openclaw-config-rules | 无 IO 的 JSON5 投影，复用 5 与 12，不需原生适配 |
| openclaw_defaults.rs | 74 纯类型/解析/prepare/form；38 set_agents_defaults | 纯规则与事务发布拆符号，不按品牌整体搬包 |
| openclaw_defaults_tests.rs | 74 | 三个现有测试均测试纯规则 |
| models/preset_omp_tests.rs | 25 | 9 保留的环境集成断言 |
| models/codex_template_tests.rs | 24 | 9 保留的 prepare_provider 集成断言 |
| models/preset_catalog_tests.rs | 11 | 9 保留的 native_projection 集成断言 |

任务 37/38、13 与 prompt 已同步，执行顺序 10→11→12→73→74→13…72。plan.md 的数字顺序由此具体补充覆盖。共享 workspace/Cargo 清单及架构文档必须串行写。

当前 src 共 788 文件，按原表加上述覆盖归属，遗漏 0。所有超过 500 行文件继续按该任务的内聚子模块边界拆分，不以文件大小创建库；根模块/跨域文件的符号切口见“具体拆边与延后接收”。非源码根文件包括 Cargo.toml（72 清单）、LICENSE 与来源/隔离验收说明、来源文件.json、包外引用.json（72 保留发布溯源），无 build.rs、无显式 features 段或自定义 lib/bin 入口；lib.rs 为真实入口，upstream 不在模块树中。

### 候选包的实际外依赖线索与直接消费者

源路径及职责沿用上表，目标均为 crates/libs/<目标名>。下表从当前源路径限定匹配直接引用；这是依赖审计证据，不是可复制的 Cargo 清单。保留原 target cfg/features，执行时读取限定符别名和 super 引用闭合实际导出。外部 crate 仅列当前清单中有源码命中的项。

| 任务 | 外依赖源码命中 | 门面内直接消费者（领域） |
| --- | --- | --- |
| 4 | serde_json, vw-api-app-identity, vw-api-app-provider-base, vw-api-types | aggregation, apps, auth, data_management, device_sync, diagnostics, hermes_memory_limits, high_availability, mcp, models, openclaw_catalog, openclaw_catalog_tests, openclaw_defaults, openclaw_defaults_tests, openclaw_health, openclaw_model, openclaw_tools, openclaw_tools_tests, project_profiles, prompts, routing, sessions, skills, usage, workspace_paths |
| 8 | app-model-catalog, app-routing-policy, serde_json, vw-api-app-identity | aggregation |
| 10 | reqwest, serde_json, vw-api-app-identity, vw-api-app-provider-base, vw-api-transport, vw-api-types | aggregation, apps, auth, data_management, device_sync, diagnostics, hermes_memory_limits, high_availability, mcp, models, openclaw_catalog, openclaw_catalog_tests, openclaw_defaults, openclaw_defaults_tests, openclaw_health, openclaw_model, openclaw_tools, openclaw_tools_tests, project_profiles, prompts, routing, sessions, skills, usage, workspace_paths |
| 11 | app-provider-validation, reqwest, serde_json, vw-api-app-identity, vw-api-app-provider-base, vw-api-transport, vw-api-types | aggregation, apps, auth, data_management, device_sync, diagnostics, hermes_memory_limits, high_availability, mcp, models, openclaw_catalog, openclaw_catalog_tests, openclaw_defaults, openclaw_defaults_tests, openclaw_health, openclaw_model, openclaw_tools, openclaw_tools_tests, project_profiles, prompts, providers, routing, sessions, skills, usage, workspace_paths |
| 12 | app-config-patch, serde, serde_json, vw-api-app-auth, vw-api-app-identity, vw-api-transport, vw-api-types | aggregation, apps, auth, data_management, device_sync, diagnostics, hermes_memory_limits, high_availability, mcp, models, openclaw_catalog, openclaw_catalog_tests, openclaw_defaults, openclaw_defaults_tests, openclaw_health, openclaw_model, openclaw_tools, openclaw_tools_tests, project_profiles, prompts, routing, sessions, skills, usage, workspace_paths |
| 13 | reqwest, serde, serde_json, tokio, uuid, vw-api-app-auth, vw-api-app-identity, vw-api-transport | apps, providers, usage |
| 14 | chrono, reqwest, serde, serde_json, tokio, vw-api-app-auth, vw-api-app-identity, vw-api-transport | apps, providers, usage |
| 15 | reqwest, serde_json, tokio, uuid, vw-api-app-auth, vw-api-app-identity, vw-api-transport | apps, providers, usage |
| 16 | reqwest, serde, serde_json, tokio, vw-api-app-auth, vw-api-transport | apps, providers, usage |
| 17 | reqwest, serde, serde_json, tokio, vw-api-app-auth, vw-api-transport | apps, providers, usage |
| 18 | reqwest, serde_json, tokio, uuid, vw-api-app-auth, vw-api-app-identity, vw-api-transport | apps, providers, usage |
| 19 | chrono, reqwest, serde, serde_json, tokio, uuid, vw-api-app-auth, vw-api-transport | apps, providers, usage |
| 20 | reqwest, serde, serde_json, tokio, uuid, vw-api-app-auth, vw-api-app-identity, vw-api-transport | apps, providers, usage |
| 21 | reqwest, serde, serde_json, tokio, vw-api-app-auth, vw-api-app-identity, vw-api-transport | apps, providers, usage |
| 22 | reqwest, serde_json, tokio, vw-api-app-auth, vw-api-app-identity, vw-api-transport | apps, providers, usage |
| 23 | serde, serde_json, vw-api-app-identity, vw-api-transport | aggregation, apps, auth, data_management, device_sync, diagnostics, hermes_memory_limits, high_availability, mcp, models, openclaw_catalog, openclaw_catalog_tests, openclaw_defaults, openclaw_defaults_tests, openclaw_health, openclaw_model, openclaw_tools, openclaw_tools_tests, project_profiles, prompts, routing, sessions, skills, usage, workspace_paths |
| 24 | reqwest, serde_json, vw-api-app-auth, vw-api-app-identity, vw-api-app-provider-base, vw-api-transport, vw-api-types | aggregation, apps, auth, data_management, device_sync, diagnostics, hermes_memory_limits, high_availability, mcp, models, openclaw_catalog, openclaw_catalog_tests, openclaw_defaults, openclaw_defaults_tests, openclaw_health, openclaw_model, openclaw_tools, openclaw_tools_tests, project_profiles, prompts, providers, routing, sessions, skills, usage, workspace_paths |
| 25 | serde, serde_json, vw-api-app-identity, vw-api-types | aggregation, apps, auth, data_management, device_sync, diagnostics, hermes_memory_limits, high_availability, mcp, models, openclaw_catalog, openclaw_catalog_tests, openclaw_defaults, openclaw_defaults_tests, openclaw_health, openclaw_model, openclaw_tools, openclaw_tools_tests, project_profiles, prompts, providers, routing, sessions, skills, usage, workspace_paths |
| 26 | app-model-catalog, model-reasoning, provider-model-cache, provider-model-catalog, provider-model-discovery, reqwest, serde, serde_json, tokio, vw-api-app-identity, vw-api-app-provider-base, vw-api-model-catalog, vw-api-transport, vw-api-types | aggregation, providers, usage |
| 27 | serde, serde_json, vw-api-app-auth, vw-api-app-identity, vw-api-chat-identity, vw-api-model-catalog, vw-api-transport, vw-api-types | providers, routing, sessions |
| 28 | serde, serde_json, tokio, vw-api-app-auth, vw-api-app-identity, vw-api-chat-identity, vw-api-model-catalog, vw-api-transport, vw-api-types | providers, routing, sessions |
| 29 | app-provider-validation, reqwest, serde, serde_json, tokio, vw-api-app-auth, vw-api-app-identity, vw-api-app-provider-base, vw-api-model-catalog, vw-api-task, vw-api-transport, vw-api-types | aggregation, apps, auth, data_management, device_sync, diagnostics, hermes_memory_limits, high_availability, mcp, models, openclaw_catalog, openclaw_catalog_tests, openclaw_defaults, openclaw_defaults_tests, openclaw_health, openclaw_model, openclaw_tools, openclaw_tools_tests, project_profiles, prompts, routing, sessions, skills, usage, workspace_paths |
| 30 | app-availability-core, vw-api-app-identity, vw-api-app-provider-base, vw-api-transport, vw-api-types | 同域 super/self 或门面导出 |
| 31 | reqwest, serde_json, tokio, vw-api-app-identity, vw-api-types | auth, data_management, models, prompts, providers, sessions, skills, usage |
| 32 | app-catalog, vw-api-app-identity, vw-api-types | auth, data_management, models, prompts, providers, sessions, skills, usage |
| 33 | reqwest, serde_json, tokio, vw-api-app-identity, vw-api-task, vw-api-types | auth, data_management, models, prompts, providers, sessions, skills, usage |
| 34 | serde_json, tokio, vw-api-app-identity, vw-api-types | auth, data_management, models, prompts, providers, sessions, skills, usage |
| 35 | serde_json, vw-api-app-identity, vw-api-app-provider-base, vw-api-transport, vw-api-types | auth, data_management, models, prompts, providers, sessions, skills, usage |
| 36 | app-catalog | auth, data_management, models, prompts, providers, sessions, skills, usage |
| 37 | reqwest, tokio, vw-api-transport, vw-api-types | 同域 super/self 或门面导出 |
| 38 | serde, serde_json, vw-api-transport | 同域 super/self 或门面导出 |
| 39 | reqwest, serde_json, vw-api-types | data_management |
| 40 | serde, serde_json, vw-api-app-auth, vw-api-app-identity, vw-api-transport, vw-api-types | data_management |
| 41 | serde, serde_json, uuid, vw-api-app-identity, vw-api-transport, vw-api-types | data_management |
| 42 | reqwest, serde, serde_json, tokio, uuid, vw-api-app-identity, vw-api-transport, vw-api-types, zip | data_management, device_sync |
| 43 | reqwest, serde, serde_json, tokio, vw-api-app-identity, vw-api-transport, vw-api-types, zip | data_management, device_sync |
| 44 | serde_json, vw-api-transport, vw-api-types | usage |
| 45 | chrono, rusqlite, serde, serde_json, uuid, vw-api-types | usage |
| 46 | chrono, serde, serde_json, vw-api-types | usage |
| 47 | serde_json, vw-api-types | usage |
| 48 | chrono, serde, serde_json, vw-api-types | usage |
| 49 | chrono, rusqlite, serde_json, vw-api-types | usage |
| 50 | chrono, reqwest, rusqlite, serde, serde_json, vw-api-types | usage |
| 51 | vw-api-types | usage |
| 52 | rusqlite, serde, serde_json, vw-api-app-identity, vw-api-transport, vw-api-types | usage |
| 53 | reqwest, rusqlite, serde, serde_json, vw-api-transport, vw-api-types | usage |
| 54 | rusqlite, serde_json, tokio, uuid, vw-api-app-identity, vw-api-transport, vw-api-types | usage |
| 55 | chrono, reqwest, rusqlite, rust_decimal, serde, serde_json, vw-api-app-auth, vw-api-app-identity, vw-api-transport, vw-api-types | data_management |
| 56 | chrono, rusqlite, rust_decimal, serde, serde_json, tokio, vw-api-app-auth, vw-api-app-identity, vw-api-transport, vw-api-types | data_management |
| 57 | provider-model-discovery, reqwest, serde, serde_json, tokio, vw-api-types | data_management |
| 58 | chrono, provider-model-discovery, reqwest, serde, serde_json, tokio, vw-api-app-auth, vw-api-transport, vw-api-types | data_management |
| 59 | chrono, reqwest, serde, serde_json, tokio, vw-api-app-auth, vw-api-app-identity, vw-api-transport, vw-api-types | data_management |
| 60 | reqwest, rquickjs, serde, serde_json, tokio, vw-api-types | data_management |
| 61 | chrono, rusqlite, serde_json, uuid, vw-api-transport, vw-api-types | data_management |
| 62 | chrono, rusqlite, rust_decimal, serde, serde_json, uuid, vw-api-transport, vw-api-types | data_management |
| 63 | chrono, provider-model-cache, provider-model-catalog, rusqlite, serde, serde_json, tokio, uuid, vw-api-transport, vw-api-types | data_management |
| 64 | chrono, reqwest, serde, serde_json, zip | 同域 super/self 或门面导出 |
| 65 | chrono, reqwest, tokio | 同域 super/self 或门面导出 |
| 66 | reqwest, rusqlite, serde, serde_json, uuid, vw-api-app-identity, vw-api-app-provider-base, vw-api-transport | 同域 super/self 或门面导出 |
| 67 | rusqlite, serde, serde_json, uuid, vw-api-transport, vw-api-types, zip | 同域 super/self 或门面导出 |
| 68 | rusqlite, rust_decimal, serde, serde_json, uuid, vw-api-app-identity, vw-api-app-provider-base, vw-api-transport, vw-api-types | 同域 super/self 或门面导出 |
| 69 | rusqlite, serde_json, vw-api-task, vw-api-transport, vw-api-types | 同域 super/self 或门面导出 |
| 70 | uuid, vw-api-transport, vw-api-types | 同域 super/self 或门面导出 |
| 71 | serde_json, vw-api-app-identity, vw-api-types | 同域 super/self 或门面导出 |
| 72 | std / 同包规则；以实现时签名核实 | 同域 super/self 或门面导出 |
| 73 | vw-api-app-identity | daily_memory_files, daily_memory_list, hermes_memory, hermes_memory_tests, workspace_files |
| 74 | serde, serde_json | 同域 super/self 或门面导出 |

### 依赖方向、排序和共享状态

箭头为消费者→依赖。显式 crate 路径的领域图如下（含测试，保守上界；super/self 的同域边及符号环切口继续见前表，不将此图误作最终 Cargo 图）。

```text
aggregation → models, providers, routing
apps → auth, providers
auth → apps, providers
daily_memory_files → workspace_paths
daily_memory_list → workspace_paths
data_management → apps, mcp, prompts, providers, skills, usage
device_sync → providers, skills
diagnostics → providers
hermes_memory → workspace_paths
hermes_memory_limits → providers
hermes_memory_tests → workspace_paths
high_availability → providers
mcp → providers
models → apps, providers
openclaw_catalog → providers
openclaw_catalog_tests → providers
openclaw_defaults → providers
openclaw_defaults_tests → providers
openclaw_health → providers
openclaw_model → providers
openclaw_tools → providers
openclaw_tools_tests → providers
project_profiles → providers
prompts → apps, providers
providers → aggregation, apps, auth, icons, models
routing → aggregation, providers
sessions → aggregation, apps, providers
skills → apps, providers
usage → apps, auth, models, providers, sessions
workspace_files → workspace_paths
workspace_paths → providers
```

按依赖深度先规则（2—9 已实施），再状态/投影（10—12），再安全 IO/纯 OpenClaw 规则（73/74），再认证协议/原生适配，最后事务发布和扫描编排。无领域运行时依赖的 JSON/类型规则优先；同层按上述实际外依赖集合数量审查后推进，不按行数排序。provider/auth、apps discovery/lifecycle/uninstall、aggregation/provider 均有环：前表逐符号切口是强制前置，不能把目录图直接搬成 crate 图。73/74 深度为 paths/patch→write_guard→规则与安全 IO，38 的发布还依赖 24 原生实现。

共享可变状态：providers/live/write_guard.rs 的 OnceLock<(Mutex<HashSet<LockKey>>,Condvar)> 必须仅留任务 12；73、38 与所有原生写入复用同一锁表。auth/kimi.rs 的异步 StartSlot 与 grant 锁随 18 内聚迁移，跨账户编排由 22 接收。claude_login_files.rs 的 AtomicU64 临时名和 AtomicBool 停止信号随 15 内聚，不能拆出第二份状态。高可用 circuit_breaker/health 已同属 7，30 依赖同一实例；设备传输配置/快照由 64 契约→65 传输，数据恢复由 66 格式→67/68→69，不能反依赖编排。

### 后续验收与保留原因

37 保留记忆与 dashboard 业务，38 保留 OpenClaw 发布及 model/env/tools/health，原门面只保留 cfg 和兼容导出（现有调用路径）；不允许以门面继续拉入全部依赖而声称消费者隔离完成，72 逐调用方改用职责库。每领域现有卡末节的目标编译、原单测与平台行为仍未验证。重点包括安装/卸载权限、认证过期与轮换、路由载荷、会话格式和迁移、MCP/技能原生落盘、账本幂等、设备归档验签与传输、HA 状态共享、恢复回滚；本轮均不执行。

静态完成条件：全部当前源文件有归属，新增两卡与既有 10—72 入口存在，已删除 1—9 有提交与产物；既有资源表涵盖 presets/fixture/include，迁移时按实际相对路径复核，不复制资源。没有修改业务源码、Cargo.toml、Cargo.lock 或 plan.md。

同深度的外依赖数量排序证据（源码命中数，不能替代 Cargo 图）：72=0；36=1；51=1；73=1；47=2；74=2；32=3；38=3；39=3；44=3；65=3；70=3；71=3；4=4；8=4；23=4；25=4；34=4；37=4；46=4；48=4；49=4；30=5；31=5；35=5；64=5；69=5；10=6；16=6；17=6；22=6；33=6；40=6；41=6；45=6；50=6；52=6；53=6；57=6；60=6；61=6；11=7；12=7；15=7；18=7；21=7；24=7；54=7；67=7；13=8；14=8；19=8；20=8；27=8；43=8；62=8；66=8；28=9；42=9；58=9；59=9；68=9；55=10；56=10；63=10；29=12；26=14。依赖方向优先于该次序，串行入口以 prompt 为准。


## 任务 10 迁移结果（2026-11-15）

静态拆分完成；未运行 Cargo、单元及实机测试，统一验收待授权。

- `providers/projection/{mod,codex,codex_route,fields,grok,json}.rs`、`providers/codex.rs`、`providers/codex_tests.rs`、`providers/opencode_schema.rs` 已移入 `app-provider-projection/src`，原 providers 模块重导出。Projected、Error、fields、codex_route 以及真实跨包调用的投影入口公开；TomlPatch 直接来自 app-config-patch，校验来自 app-provider-validation，无 live 提交依赖。
- Codex 独立测试保留 cfg(test) 注册；OpenCode schema 保留 macOS 条件。迁移文件无 include 资源。preview、codex_catalog/form、claude/gemini/grok 仍从原门面调用同一实现；无副本。
- 本节覆盖历史逐文件所有权表：`claude.rs/claude_form.rs/claude_tests.rs`、`gemini.rs/gemini_form.rs/gemini_tests.rs`、`grok.rs/grok_tests.rs`、`dsh_form.rs/kimi_form.rs/mcode_form.rs/openclaw_form.rs/opencode_form.rs/omp_validation.rs` 暂留原位置，任务 24 接收。原因：WriteError/AppWriteGuard/ProviderTransactions/AppState 归任务 12，表单又调用 basic（任务 11）和具体适配器 validate/official；kimi_form 依赖任务 18 的 binding_region。不能令纯投影包反向依赖服务或 live。
- `grok_form.rs` 调用 preview::editor_category，依历史符号切口由任务 24 接纯叶子、任务 29 收剩余表单编排；`desktop_mapping.rs/desktop_mapping/catalog.rs/desktop_mapping_tests.rs` 依赖 claude_desktop::settings 和 aggregation::model_ids，任务 24 梳理、任务 28 在任务 27 完成后接收整个映射边界。相邻测试原注册保留，随最终所有者迁移。
- 原包 serde_json/toml/toml_edit/tempfile 仍有其他真实使用者，不能删除。新包直接依赖仅 patch、validation、preview 契约、serde_json（arbitrary_precision）、toml、toml_edit；tempfile 仅测试。
- 静态证据：差异空白检查、移动内容/路径与导出审阅、TOML 路径审阅、新包无旧门面引用；后续统一验收须覆盖新包和 preview/codex_catalog/原生适配器编译、迁移测试及三平台条件。静态审阅不代表行为通过。


## 任务 11 迁移结果（2026-11-15）

静态拆分完成；未运行 Cargo、单元及实机测试，统一验收待授权。

- 本节覆盖历史所有权表：basic/create/duplicate/icons/presentation/snippets/universal/update 的实现移入 app-provider-core/src；create/duplicate/icons/presentation/universal/update 的独立测试同移，editor_merge 的 merge/MergeError/at/placeholder 及整个独立测试同移。snippets 的四个纯规则测试同移，macOS preview 集成测试继续独立注册在门面，任务 29 接收。所有既有测试保留，未新增或执行。
- failover::ProviderAuth/ProviderSnapshot 下沉并由原模块重导出；snapshot/classify/codex_auth/grok_auth/has_saved_key 仍依赖 claude/gemini/grok/codex_catalog，任务 29 接收分类编排。高可用 queue/selection 沿用同一重导出类型。
- universal_sync::validate_request/child_id 下沉；project/object_field 依赖 Claude 原生 patch/settings，保留原位置及 universal_sync_tests，任务 29 接收。models/preset_catalog_tests 的 native_projection 集成断言随之由任务 29 接收，不引入 core 对 native 的反依赖。
- catalog::{read,observe,read_file}、delete::ensure_unreferenced 及各自测试保留原位，任务 29 接收（依赖任务 24 原生适配器）。editor_merge::ClaudeFile 原生访问/提交由任务 24 接收；project/display 依赖 preview 分发，由任务 29 接收。其余叶子已单独下沉，未复制业务实现。
- 公共入口仍由 providers 模块逐项重导出，lib.rs 的 providers 注册无需改动；snippets::sensitive 仅因 aggregation/omo_read 的真实调用而公开。图标规则直接依赖 app-icons，JSON 编辑依赖 app-config-patch，字段分类依赖 app-provider-projection；无旧门面回依赖，无资源 include 路径。原包已有依赖仍被其它模块使用，未删除。
- 静态证据：TOML 标准库解析、所有直接路径存在、workspace 注册、模块和独立测试注册、源码引用与迁移函数集合对照、git diff --check。Cargo.lock 未触碰；新包目标编译、迁移单测、原生服务集成及三平台条件留待授权统一验收，静态结果不表示行为通过。
