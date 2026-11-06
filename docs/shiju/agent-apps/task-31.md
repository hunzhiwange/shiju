# 任务31：供应商API凭据启动

2026-11-06，macOS 27.0 (26A428)、arm64；本任务不宣称个人账号或各CLI版本的实机验收，也不标记母任务3/4整体完成。

## 依赖与最小前置

任务30终端与任务28执行主机输入边界已交付。初次核查发现正式应用供应商记录缺失，维护者随后明确授权补齐母任务4的最小租户记录读写/投影，以真实临时数据库和非敏感供应商夹具验收，不自动导入全局auth、个人upstream库或主机环境密钥。只读plan.md、来源与接口.md及供应商来源映射均保持不变。

- `vw-api-types::app_providers`：写入专用API profile（名称、协议、API key、endpoint、model）、脱敏summary、显式provider_id + LaunchRequest；启动请求不接收tenant或密钥。
- `vw-agent-apps::providers`：应用供应商验证/投影能力所有者；API协议按应用显式匹配，拒绝未知字段、空密钥、控制字符、错误协议和含认证信息/query/fragment的endpoint；endpoint仅HTTPS/loopback HTTP。
- `vw-agent::app_providers::AppProviderStore`：正式租户应用供应商记录所有者，复用GatewaySettingsStore及既有SQLite/PostgreSQL数据库后端；固定namespace `app_providers.records`，key为序列化AppId与provider_id组合，TenantScope始终由认证调用方传入。最小upsert不激活、不改CLI live配置；没有记录时拒绝，无全局凭据回退。后续母任务4应扩展同一所有者，而非再建凭据存储。
- 生产路由：`PUT /v1/app-providers/{app}/{id}`写入、`GET /v1/app-providers/{app}/{id}`只返回summary；挂入既有handler_router，继承认证和租户中间件。SDK为`app_provider_save/get`。这两个端点不执行程序，也不是任意键值或命令RPC。
- `AppProviderStore::prepare_launch`：按租户/AppId/provider_id读取真实记录，调用原有`validate_launch_request`校验host、installation、cwd roots及权限跳过授权，再接`prepare_provider_launch`。任务36负责最终授权主机dispatch与终端入口，不在此绕过主机认证开放启动API。

当前profile是API启动所需最小强类型投影，不是完整上游settings_config编辑器，不代表供应商CRUD、live切换、多路由或账号能力全部已迁移。

## 单次进程行为

`ProviderLaunch`不支持Debug或序列化，消费一次后不能重复使用。子进程env_clear，仅保留OS运行所需的PATH、HOME/USERPROFILE、系统临时目录、locale与终端类型；不修改父进程环境或HOME。供应商密钥只通过该子进程环境设置，命令行和临时配置均不含key。输出不回传响应/日志，运行接口仅返回固定结果。SDK现有log_request只记录body是否存在，不序列化正文；write DTO的Debug固定脱敏；API错误丢弃原始输入、SQL及serde错误详情。

随机临时目录创建时Unix权限0700，文件create_new且0600。材料未执行就丢弃、准备失败、spawn失败、非零退出和正常退出均清理；进程运行期间保留目录，回收子进程后显式close，清理失败返回固定错误。调用方不能把该运行接口当作终端已交接的证据；任务36需保留同一材料生命周期接线终端。

| AppId | 检查结果 |
| --- | --- |
| claude | 实际observed claude程序；私有CLAUDE_CONFIG_DIR、settings.json、--settings-sources空值、--settings及--model；ANTHROPIC_API_KEY/BASE_URL仅子进程 |
| codex | 实际observed codex程序；私有CODEX_HOME/config.toml；独立shiju_session responses路由，env_key引用SHIJU_PROVIDER_API_KEY；不继承OPENAI_API_KEY或官方auth |
| gemini | 实际observed gemini程序；私有GEMINI_CLI_HOME/.gemini/settings.json，gemini-api-key auth、ignoreLocalEnv与--model；GEMINI_API_KEY及GOOGLE_GEMINI_BASE_URL仅子进程 |
| agy | 实际observed agy程序；私有--gemini_dir/settings.json及--model；Gemini key/base仅子进程；不切换官方账号 |
| claude-desktop | 桌面应用，无终端能力，拒绝 |
| grokbuild | 已检查来源Grok配置领域；单次进程投影未迁移，明确拒绝，绝不回退claude |
| opencode | 已检查opencode_config增量provider来源；单次进程投影未迁移，明确拒绝 |
| openclaw | 已检查openclaw_config配置来源；单次进程投影未迁移，明确拒绝 |
| hermes | 已检查hermes_config/HERMES_HOME；单次进程投影未迁移，明确拒绝 |
| pi | 已检查来源应用目录及增量成员契约；单次进程投影未迁移，明确拒绝 |
| mcode | 已检查mcode_config provider来源；单次进程投影未迁移，明确拒绝 |
| kimi | 已检查kimi_config provider/API key环境来源；单次进程投影未迁移，明确拒绝 |
| dsh | 已检查dsh_config profile/refs来源；单次进程投影未迁移，明确拒绝 |
| omp | 已检查omp_config/PI_CODING_AGENT_DIR；单次进程投影未迁移，明确拒绝 |

上表“未迁移”是当前投影能力缺口，不把应用标成产品层不支持，不假报其余9个CLI的供应商启动已实现。所有AppId在能力校验中逐项匹配；当前可写启动profile仅支持前述4个CLI。来源misc.rs::open_provider_terminal的通用claude回退没有保留。其余投影须随母任务4各应用契约补齐。

Claude/Codex/Agy权限跳过仅由已授权host决定后添加应用对应flag；Gemini拒绝。Codex/Gemini发现可能改写provider选择的项目配置明确拒绝，避免静默用错供应商；项目搜索在.git根停止。原生Windows .cmd/.bat（含大小写变体）因需要审查的host adapter而拒绝；Windows/WSL installation不能在macOS被当作本地路径执行。不宣称Windows/WSL实机通过。

## 来源核对

本地只读快照：misc.rs::open_provider_terminal、extract_env_vars_from_config、ProviderLaunch、claude_terminal_launch、codex_terminal_launch与agy_terminal_launch；services/provider/codex_terminal.rs、claude_official.rs；各应用配置来源及供应商来源映射。未改upstream。

Gemini私有目录与环境加载另核对[官方configuration文档](https://github.com/google-gemini/gemini-cli/blob/main/docs/reference/configuration.md)及[Storage源码](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/config/storage.ts)，使用GEMINI_CLI_HOME作为home root，目录中仍有.gemini/settings.json；私有profile禁用local env加载。Claude设置来源参数核对[官方设置文档](https://code.claude.com/docs/en/settings)。这些是配置规则依据，不是已安装版本的实机运行证明。

## 验证

状态：已完成本任务当前投影与拒绝边界验收；后续CLI适配、终端dispatch和各CLI实机验收保持后续任务边界。

所有Cargo阶段均从仓库根原子mkdir依次领取共享槽位，owner记录计划/任务/会话与持有shell PID；shell EXIT/INT/TERM清理自己的记录和锁，缓存保留。实际使用slot-2：

- `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`
- `CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`

后者覆盖仓库build.build-dir，Cargo及子进程均继承，未绕过target-dir。没有使用任务独立构建目录。

- 红灯：`cargo test -p vw-agent-apps --lib cc_apps_provider_launch_isolation -- --nocapture`，退出101，1有效测试、1失败。最小可编译签名返回未实现，在真实进程预期行为断言处失败；不是缺工具、缺模块或无关编译错误。
- 实现迭代发现并修复：tempfile默认目录0755，改为创建时0700；Agent API测试最初放在根测试模块无法访问Gateway私有middleware，移到handler相邻独立文件；SDK完整链路测试初始误配Basic/Bearer，改为生产Bearer解析器+PairingGuard。以上均为本任务问题，没有归为既有错误。

未运行全量cargo test、fmt检查、wasm检查、Makefile。无UI改动。

最终聚焦验证（先运行`cargo fmt --all`，退出0；审查无无关格式化差异）：

| 确切命令 | 退出码 | 有效测试 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc_apps_provider_launch_isolation -- --nocapture` | 0 | 8通过、0失败、140过滤；env污染边界另启动同一测试二进制子进程执行1个有效子用例，不重复计数 |
| `cargo test -p vw-api-types --lib cc_apps_provider_launch_isolation -- --nocapture` | 0 | 1通过、0失败、76过滤 |
| `cargo test -p vw-gateway-client --lib cc_apps_provider_launch_isolation -- --nocapture` | 0 | 1通过、0失败、278过滤 |
| `cargo test -p vw-agent --lib cc_apps_provider_launch_isolation -- --nocapture` | 0 | 3通过、0失败、11441过滤 |

共13个顶层有效测试；真实子进程记录并核对argv、cwd、env和私有配置存在性/删除结果。Agent用例实际通过SDK→loopback HTTP→认证与租户中间件→SQLite记录→AppProviderStore读取/投影→授权主机实例校验→子进程执行→配置删除。覆盖相同ID的不同租户分别取自己的key、跨平台/公司和AppId不串读、未授权请求拒绝、记录缺失拒绝、错误host/权限跳过拒绝、DTO未知tenant/secret/command拒绝、4种CLI实际程序及9种未迁移CLI/desktop逐项拒绝、协议错误、控制字符/endpoint认证材料校验、私有权限、准备/启动/非零退出/未运行材料丢弃清理、继承凭据清除且父进程不变、批处理拒绝与macOS拒绝外部Windows/WSL installation。

Agent聚焦测试链接阶段有平台链接器提示：`ld: __eh_frame section too large (max 16MB) to encode dwarf unwind offsets in compact unwind table, performance of exception handling might be affected`。测试仍退出0；没有修改仓库链接/异常处理设置来隐藏提示。该提示单列，不能当作Rust源代码或Clippy诊断已通过的替代证据。

- 审查补充非法输入副作用用例后重新运行`cargo fmt --all`，退出0；重新运行`cargo test -p vw-agent-apps --lib cc_apps_provider_launch_isolation -- --nocapture`，退出0，8通过、0失败、140过滤（上表使用最新数量）。其余三个crate的通过证据未因该独立测试变更失效。
- 必须检查：`cargo clippy --all-targets --all-features -- -D warnings`，退出0；测试补充后同命令再次执行，退出0，无Clippy诊断。
- `git diff --check`，退出0；差异审查仅本次模块声明、供应商类型/存储/API/SDK、投影和独立测试及本文档，无无关格式化差异；plan.md/upstream保持只读，既有编号任务删除排除。
- 全部Cargo与使用槽位产物的测试子进程已结束，自己的slot-2记录/锁已释放；slot-1与slot-9属于其他持有者，未操作；编译缓存保留。

下一步：任务32账号材料和任务36授权执行主机/终端启动接线；母任务4其余CLI投影继续在此正式所有者扩展，各CLI/macOS版本与个人账号按后续验收任务验证。不要把本任务的进程夹具当作所有CLI个人账号实机通过。
