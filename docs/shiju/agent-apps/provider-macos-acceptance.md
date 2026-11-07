# 供应商 macOS 验收证据

## 任务 1：锁定供应商领域实施契约

2026-11-06；状态：已完成。产物：[provider-contracts.md](provider-contracts.md)。本项为文档契约审查，不是供应商行为、API 请求或 Iced 运行验收。

### 开工裁定

原供应商计划等待母任务 2 完整验收，而底座正式交付要求供应商等领域交接落地后才评估母任务 2 完成，形成循环。继续执行指令已裁定以底座子计划 1–93 的实际验收作为开工门槛，逐项消费真实接口；母任务 2 完整领域迁移仍未收口，不修改其状态或只读 plan.md。

母任务 1 来源快照已验收；母任务 3 当前已完成，正式 AppId/catalog、授权主机和隔离证据存在。底座提交 `1d7f1b192` 汇总双库无窗口 6 项通过与真实图标 Iced 窗口证据。本轮仅审查历史证据，没有重跑或将图标通过推及供应商切换。

原调度会话 `01a1ab42-fb8e-7af1-86f1-41f7a5eda2a2` 和任务 93 会话 `01a1ace2-4bbf-7d80-9d7c-04db07691247` 均已完成、归档，没有仍执行的前置收口会话。本轮查询非归档历史8页及归档历史15页，并读取两会话最新完成轮；原底座计划目录清理不作为能力验收证据。

### 来源、实现与人工清单

来源 `/Users/xiangmin/worker/code/cc-switch`，HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，工作区干净。主仓审查基线 `4706d3601`。

| 审查面 | 本轮读取及结论 |
| --- | --- |
| 来源函数与调用者 | provider.rs 的 Provider/ProviderMeta；commands/provider.rs 的 get_providers/get_current_provider/add_provider/update_provider/delete_provider；mode/current.rs 的 Purpose/provider_for/is_referenced。核对 lib.rs 注册、前端 providers.ts 调用、ProviderService 与 mode/controller 消费；相邻 provider_service/provider_commands 和 current 测试只读，未运行 |
| 业务与存储 | providers/mod.rs 的 validate_identity/validate_profile、apps/catalog.rs、AppProviderStore、GlobalDatabase/settings 与图标事务；确认14身份、租户键、整条 upsert、不激活、不复制数据库 |
| API / SDK | handlers/app_providers.rs 的 router/read/save，router.rs/bootstrap.rs 两处注册；DTO app_providers.rs；SDK app_providers_api.rs 的 save/get。相邻 API、SDK、store 测试断言涵盖认证、脱敏、租户/应用隔离和路径；本轮只读，不声称重跑通过 |
| Iced | components/mod.rs、state.rs、state/app_state.rs、message/mod.rs、message/settings/mod.rs、ui_types.rs 和 system_settings.rs 的真实注册位置；本计划供应商编辑 UI 尚未交付，本轮未启动或截图 |
| 逐项覆盖 | 应用能力、读写权限、秘密写入/脱敏、错误/HTTP、状态/路由、14应用格式/路径/不支持操作均有来源；任务1–77拟文件与父模块已登记；单文件与同名目录接线冲突已说明 |

完整 CRUD/当前状态、秘密 Keep/Replace/Clear、主机配置写权限、物理文件锁/pending/恢复、原生应用适配、模式/代理/聚合交叉、环境恢复、进度取消与供应商 Iced/macOS 交叉验收仍待后续任务实现，责任见契约；不能因本项完成而标为通过。母任务 2 完整迁移及母 6/8/10 完整领域验收仍未收口。

### 本轮验证

| 确切命令或检查 | 退出码 | 结果 |
| --- | --- | --- |
| `git -C /Users/xiangmin/worker/code/cc-switch rev-parse HEAD` | 0 | 来源提交一致 |
| `git -C /Users/xiangmin/worker/code/cc-switch status --short` | 0 | 无输出，干净 |
| `/Users/xiangmin/.rustcodegraph/bin/rustcodegraph status` | 0 | 索引可用，6353文件 |
| `/Users/xiangmin/.rustcodegraph/bin/rustcodegraph explore 'agent_apps providers ProviderStore ExecutionHostPolicy'` | 0 | 定位调用者，截断部分精确 node 跟进 |
| `/Users/xiangmin/.rustcodegraph/bin/rustcodegraph node --file crates/vw-agent-apps/src/providers/mod.rs --offset 1 --limit 120` | 0 | 现有校验与启动消费者 |
| `sw_vers`、`uname -m` | 0 / 0 | macOS 27.0（26A428）、arm64 |

任务要求的关键项检查实际命令：

```bash
python3 - <<'PY'
from pathlib import Path
p=Path('docs/shiju/agent-apps/provider-contracts.md')
s=p.read_text()
assert s.strip()
for key in ['macOS', 'API', 'SDK', 'Iced']:
    assert key in s, key
print('文档关键项检查通过；仍需人工逐项审查')
PY
```

链接与登记矩阵检查实际命令：

```bash
python3 - <<'PY'
from pathlib import Path
import re
root=Path('docs/shiju/agent-apps')
for name in ['provider-contracts.md', 'provider-macos-acceptance.md']:
    p=root/name
    for target in re.findall(r'\]\(([^)]+)\)', p.read_text()):
        assert (p.parent/target).exists(), (name, target)
s=(root/'provider-contracts.md').read_text()
ids=['claude','claude-desktop','codex','gemini','grokbuild','opencode','openclaw','hermes','pi','mcode','kimi','dsh','agy','omp']
for app in ids:
    assert re.search(r'^\| '+re.escape(app)+r' /', s, re.M), app
for n in range(1,78):
    assert re.search(r'^\| '+str(n)+r' \|', s, re.M), n
print('链接可达；14应用、77任务落点登记完整')
PY
```

以上两块 Python 实际执行退出均为 0，人工对照来源和注册入口完成语义审查；字符串或行数检查不等于行为验收。定向 `git diff --check`、`git diff --cached --check` 退出 0，仅暂存两份正式文档。初次通过 shell 写验收文档时嵌套 heredoc 分隔符冲突，shell 语法解析退出1、未写入文件；改用 apply_patch 后重新验证，不将该失败记为检查通过。

本轮仅文档，不改 Rust/配置/测试；TDD、有效行为测试计数、Cargo槽位不适用，不调用 Cargo/Clippy/全量测试。未验收 Windows/Linux/其他架构。任务52–77仍须供应商真实窗口和应用证据。

## 任务 2：供应商列表与当前状态

2026-11-06；状态：已完成（按继续执行指令确认的最小只读范围）。

### 范围与真实读取链

继续执行指令授权在本任务增加必要的最小只读原生读取，不提前实现配置写入、事务、模式切换或第二套状态存储。任务 1 的底座开工裁定沿用；母任务 2 完整迁移不视为已完成。

- 来源 `services/provider/mod.rs::ProviderService::list/current`、`mode/current.rs::provider_for`、`kimi_config.rs::read/current_provider_in`、`opencode_config.rs::resolve_config_path`、`app_config.rs::is_additive_mode`。Kimi 从 `default_model → models[alias].provider → providers[id]` 读取，不能拆分 alias 猜 ID；OpenCode JSONC 优先于 JSON，累加应用没有唯一 current。
- 业务唯一所有者：`vw-agent-apps/src/providers/catalog.rs::read`；租户档案仍通过 `AppProviderStore::list` 读取既有 SQLite/PostgreSQL 配置存储，本轮使用真实隔离 SQLite 验证，没有新增数据库或写入状态。
- DTO：`vw-api-types/src/app_providers/catalog.rs`。`direct` 表示托管直连指针，尚无读取能力时为 `unavailable/not_implemented`；`inUse` 仅为可证明的原生配置选择，不承诺进程正在使用或代理实际流量。Kimi 可以读取原生选择，不能据此宣布托管直连指针已存在。
- `rows[].nativeMember` 分别为 present/absent/unknown；原生读取成功才断言成员存在/缺失。原生行只投影 ID，显示名用 ID，不返回嵌套 name、apiKey、端点、模型、凭据或原始解析错误。档案行沿用既有脱敏摘要；原生行 configured=true 仅表示配置中存在该成员，不代表凭据完整、应用可启动或请求成功。
- 当前支持 macOS 授权 HOME 默认路径下的 Kimi `.kimi-code/config.toml` 和 OpenCode `.config/opencode/opencode.json`。Kimi 非空 `KIMI_CODE_HOME` 返回 custom_path_not_supported，OpenCode JSONC 存在时返回 unsupported_format，均不默默回退；其他原生适配返回 not_implemented，其他平台返回 unsupported_platform。自定义路径和更多格式留给所属后续任务。
- 未配置、非法配置、不可读取、尚未实现分别表达，不能将读取失败当成未配置。OpenCode/OpenClaw/Hermes/Pi/Mcode/Dsh/OMP 的唯一 current 为 not_applicable，成员读取未支持时仍明确 unknown 和 nativeRead 原因。
- 配置只读使用授权 HOME 的目录描述符逐层 openat，拒绝叶子及中间符号链接、非常规文件和超过 1 MiB 的输入；不读取客户端 HOME，不导入 CC 数据库，不写用户配置。

### API / SDK / 权限

- `GET /v1/app-providers/{app}` 返回当前租户与应用的档案摘要；SDK `app_provider_list`。沿用已有网关认证与 TenantScope，不将公司请求头单独解释为公司成员认证。
- `GET /v1/app-providers/{app}/catalog/state?targetHostId=...` 返回授权主机的目录和状态；SDK `app_provider_catalog` 使用原生 HTTP 传输、敏感标记的 `x-agent-apps-token`、既有网关认证与租户上下文。路径不占用已有合法供应商 ID 的单条读取 URL。
- 扩充现有管理员 policy grant 的 `provider_config_read: true`，缺省 false。普通应用目录授权、launch 或 preferences_write 不授予原生配置读取。复用既有 policy 私有权限、token 摘要校验、platform/company/host 绑定；配置授权先于数据库/文件读取。离线 503、remote 501，拒绝回退客户端。
- 原生读取错误以 DTO 的 nativeRead/unavailable 返回；授权失败保留 401/403/404/501/503，不转换为空目录。SDK 保留 HTTP 状态且不回显原始错误正文。
- 现有 app_providers 根 router 被 handler_router 和 bootstrap 的 stateful 路径复用，无新通用命令入口。Iced 入口属于后续任务，本任务未新增或验收供应商 UI。

### 验证记录

macOS 27.0（26A428）、arm64，`sw_vers`、`uname -m` 退出 0。应用版本未验证：本轮只在临时 HOME 构造来源 schema 的 Kimi/OpenCode 配置，不启动或读取真实用户安装，不声称 CLI、代理流量或实机切换成功；Windows/Linux 与 PostgreSQL 本轮未验证。

红阶段动态领取槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。红/绿阶段使用相同的四条确切聚焦命令：

```bash
cargo test -p vw-agent-apps --lib cc04_catalog_ -- --nocapture
cargo test -p vw-api-types --lib cc04_catalog_ -- --nocapture
cargo test -p vw-agent --lib cc04_catalog_ -- --nocapture
cargo test -p vw-gateway-client --lib cc04_catalog_ -- --nocapture
```

红阶段结果：

| crate | 红阶段退出码 | 有效测试与结果 |
| --- | --- | --- |
| vw-agent-apps | 101 | 3 项业务断言失败：跨应用行未过滤、真实 Kimi 选择未读取、OpenCode 成员未读取 |
| vw-api-types | 0 | 1 项 DTO 测试通过 |
| vw-agent | 101 | 1 项业务断言失败：列表路由返回 404，预期 200 |
| vw-gateway-client | 0 | 2 项 HTTP 线协议测试通过 |

红阶段虽已设置槽位 CARGO_TARGET_DIR，仓库另有 `build.build-dir=target`，测试日志显示部分中间产物/测试可执行文件位于 `target/debug/deps`；发现后，绿阶段同时设置 `CARGO_BUILD_BUILD_DIR=$CARGO_TARGET_DIR`，最终验证的编译和测试执行均从领取的绝对槽位路径完成。未修改共享 Cargo 配置、其他锁或缓存。

最终绿阶段仍动态领取槽位 1，绝对 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，同时设置相同绝对路径的 `CARGO_BUILD_BUILD_DIR`。先 `cargo fmt --all`（退出 0），再上述四条聚焦命令，最后执行额外授权回归和强制 Clippy：

| 确切命令 | 退出码 | 有效测试数量 / 结果 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_catalog_ -- --nocapture` | 0 | 7 passed，含另起子进程验证自定义 HOME 不回退；子进程的 1 项不重复计入 7 |
| `cargo test -p vw-api-types --lib cc04_catalog_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent --lib cc04_catalog_ -- --nocapture` | 0 | 3 passed，含正式 handler_router 的 Basic 认证、真实 SQLite 租户/应用隔离、独立配置读 grant、真实本地 HTTP/SDK 和文件结果 |
| `cargo test -p vw-gateway-client --lib cc04_catalog_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-agent --lib cc_apps_read_api_authorization -- --nocapture` | 0 | 8 passed，既有目录/主机授权未被放宽；其中子进程的 1 项不重复计数 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全仓全部 targets/features 通过，无新增诊断；非测试命令，不计行为测试 |

13 项本任务测试及 8 项相邻授权回归均为实际运行，未忽略；首次绿阶段为 5/1/3/2 项通过，补充累加应用和自定义 HOME 后重新执行 fmt、7/1/3/2 项聚焦、8 项授权回归及 Clippy，以上表格是最终证据。测试二进制从 `target/rust-slot-1/debug/deps` 执行。Agent 链接仍有历史 `__eh_frame section too large` 提示（既有 task-33 等证据已记录），测试退出 0；不修改链接配置隐藏提示。

各阶段槽位均由原子 mkdir 动态领取；命令、测试及子进程结束后，shell trap 删除自身 owner 并 rmdir 锁目录。最终实际检查槽位 1 的锁已不存在，缓存保留。定向 diff 检查与暂存差异检查退出 0；只读 plan.md 无差异。仅提交本任务代码、独立测试、清单及本条正式证据，不纳入母任务状态和其他编号文件删除。未执行全量 cargo test、fmt check、wasm 检查或 Makefile。

真实边界：业务测试直接读临时 TOML/JSON、损坏文件、JSONC 优先级和符号链接；Gateway 测试使用真实租户存储及管理员 policy；本地回环 HTTP 通过正式 SDK 查询真实 handler，断言 Kimi 选择、独立 direct 状态、损坏/缺失语义、秘密不出站以及配置内容未改变。SDK crate 的独立线协议测试替代远端 HTTP 响应，不将其当成原生文件读取证据。凭据全部为 fixture。

## 任务 3：新增供应商但不激活

2026-11-06；状态：已完成。

### 范围、来源与接线

沿用任务 1 的底座开工裁定；批次 2 提交 `b20f8f5c7` 已提供目录/状态验收。母任务 1、3 当前已完成，母任务 2 完整领域迁移仍未收口，不将底座裁定误报为母任务 2 完成。本轮不改只读 plan.md。

来源仓库 HEAD 为 `ca58c0313360eae7c851fbd7553c1614c7af03e7`，`git rev-parse HEAD` 与 `git status --short` 均退出 0，工作区干净。读取 `ProviderService::add/add_from_editor`、`commands/provider.rs::add_provider`、前端 `providersApi.add` 及相邻 provider_service/provider_commands 测试。来源 command 的 addToLive 缺省 true，部分首个供应商/编辑路径有自动写 live 行为；本任务提供独立显式不激活操作，不将全部来源新增或旧 PUT 改为强制不激活。

- DTO：`vw-api-types/src/app_providers/create.rs::AppProviderCreate/ProviderSecretWrite`；addToLive 必填且只接受 false，apiKey 必须显式 `{action: "replace", value: "..."}`。新记录没有可保留或清除的必需凭据，Keep/Clear 返回 400；空密钥同样拒绝，Debug 脱敏。不修改旧 AppProviderWrite 或旧 PUT。
- 业务：`vw-agent-apps/src/providers/create.rs::create` 校验标识、显式意图及已有 profile 契约，再调用真实租户存储适配。没有 HOME、路径、主机、原生配置或激活调用；不新增数据库所有者或通用命令入口。当前 profile 支持范围沿用既有启动档案校验，其余应用仍明确不支持，完整应用配置由后续适配交付。
- 持久化：`vw-agent/src/app_providers.rs::AppProviderStore::create` 复用原 namespace、租户与 app/id 键，接 `GatewaySettingsStore::insert_if_absent`。SQLite/PostgreSQL 使用完整复合主键的 `INSERT ... ON CONFLICT (...) DO NOTHING`，重复 ID 返回冲突，不以 get+upsert 模拟原子新增；payload 和 updated_at_ms 均不更新。底层两适配的最小改动只服务本行为，既有 save/upsert 不变。
- API：`POST /v1/app-providers/{app}/{id}`，成功 201，仅返回脱敏 AppProviderSummary；重复 409、非法/激活请求 400、存储故障 503。`configured=true` 只代表档案存在，不表示应用已激活。沿用既有 app_providers 根、16 KiB 限制、handler_router/bootstrap 的认证及 TenantScope；只写租户档案，不读取/写入主机配置，不授予 host live-config 权限。Basic 无凭据测试返回 401 且无记录；公司头仍只代表存储作用域，不独立证明成员身份，沿用任务 1 权限边界。
- SDK：`vw-gateway-client/src/client/app_providers/create.rs::app_provider_create` 使用 POST 和现有请求认证/租户上下文。失败只返回受控错误与 HTTP 状态，不包含服务器原始正文、写入请求或凭据。SDK/DTO/handler 子模块由既有父模块注册，两处根 router 共用注册，无需改共享根入口。
- UI：供应商 Iced 新增/编辑入口归任务 53，本轮未新增或验收窗口。后续默认 addToLive 行为由所属应用适配显式实现；本接口不承诺首次激活或 live 成功。

### 红/绿验证

所有阶段通过原子 mkdir 动态领取槽位 **1**；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，同时 `CARGO_BUILD_BUILD_DIR` 设置为同一绝对路径，覆盖仓库 build-dir 配置。owner 记录计划、任务、会话标识和 shell PID；EXIT/INT/TERM 清理自身记录和锁，缓存保留。

红阶段先接到现有 save/upsert 并让测试编译，实际命令如下，均使用默认 features：

```bash
cargo test -p vw-agent-apps --lib cc04_create_ -- --nocapture
cargo test -p vw-api-types --lib cc04_create_ -- --nocapture
cargo test -p vw-agent --lib cc04_create_ -- --nocapture
cargo test -p vw-gateway-client --lib cc04_create_ -- --nocapture
```

| crate | 红阶段退出码 | 有效测试和断言 |
| --- | --- | --- |
| vw-agent-apps | 101 | 3 项，2 passed/1 failed；cc04_create_duplicate_is_atomic 在重复应报错的业务断言失败 |
| vw-api-types | 0 | 1 passed；首次为 crate 自引用导入错误、退出 101，修正测试接线后用相同命令重跑通过；编译错误不算红证据 |
| vw-agent | 101 | 2 项全部业务失败；重复返回 201 而非 409，真实 SDK 重复请求也成功 |
| vw-gateway-client | 0 | 1 passed，真实本地 HTTP 线协议请求 |

绿阶段先 `cargo fmt --all`，退出 0，再运行上述相同四条命令。最终结果：

| 确切命令 | 退出码 | 有效测试数量 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_create_ -- --nocapture` | 0 | 3 passed |
| `cargo test -p vw-api-types --lib cc04_create_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent --lib cc04_create_ -- --nocapture` | 0 | 4 passed |
| `cargo test -p vw-gateway-client --lib cc04_create_ -- --nocapture` | 0 | 2 passed |

共 10 项有效测试，无忽略或零测试替代。测试二进制均来自领取槽位的 `debug/deps`。Agent 链接有既有 `__eh_frame section too large` 提示，退出 0；没有改链接配置隐藏诊断。

业务测试通过真实临时文件的 create_new 存储边界证明不写 native 和重复不改持久化；真实 SQLite/时间戳/并发证据在 Agent 独立测试，未把文件回调当 SQLite。Gateway 测试通过正式 handler_router 验证 Basic 拒绝、tenant/app 隔离、重复后的整表逻辑记录及 updated_at_ms 不变、8 个并发创建恰好一个成功、非法秘密/激活请求无记录。SDK → 本地回环 HTTP → 正式 handler → 隔离 SQLite → 脱敏响应/读取闭环，断言临时 `.claude/settings.json` 字节及目录成员未改变；不调用主机配置写权限或假装激活。SDK crate 独立测试仅替代远端响应，证明 POST/tenant/显式意图以及 409/403 错误不带原始正文，不将其算原生应用行为。全部凭据为 fixture。

macOS 27.0（26A428）、arm64；`sw_vers`、`uname -m` 退出 0。应用版本未验证：只使用隔离临时配置，没有启动或读取真实用户安装。未验证 Iced、应用切换/运行、Windows/Linux、其他架构或 PostgreSQL 实际服务；PostgreSQL 适配参与编译检查，不宣称运行验收。

最终强制命令 `cargo clippy --all-targets --all-features -- -D warnings` 退出 0，全部 targets/features 检查通过，不计行为测试。最终检查 `target/rust-slot-locks/slot-1.lock` 已不存在；红阶段、DTO 接线重跑及绿阶段均正常释放自身锁。`git diff --check` 与 `git diff --cached --check` 退出 0；仅提交本任务代码、测试与本条证据，不纳入其他母任务状态或任务 1/2 的删除；plan.md 无差异。未运行全量 cargo test、fmt check、wasm 或 Makefile。

## 任务 4：更新供应商及字段保留

2026-11-06；状态：已完成。

### 依赖、来源与范围

批次 3 提交 `c5e0fb215` 和本文件任务 3 的验收证据已核对。母任务 1、3 当前状态为已完成；母任务 2 完整领域迁移尚未收口，本次沿用任务 1 已记录的底座开工裁定，不将其改报为完整完成。只读 plan.md 未修改。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，`git rev-parse HEAD`、`git status --short` 均退出 0，工作区干净。读取 `ProviderService::update/update_from_editor`、`Provider/ProviderMeta`、`commands/provider.rs::update_provider`、前端 `providersApi.update` 与相邻 `update_preserves_usage_credentials_that_only_match_previous_config` 测试。来源更新混合目录保存与当前应用 live 同步；本任务仅实现目录保存，不迁入 live、改 ID、OAuth 或应用适配。

- 业务所有者：`vw-agent-apps/src/providers/update.rs::update/profile`，沿用既有标识、应用和 profile 校验。仅修改明确提供的 name/protocol/baseUrl/model/apiKey；未提供的字段及存储中已有的 opaque 元数据完整保留。未引入可任意写入 metadata 的 API，也未新建存储所有者。
- DTO：`vw-api-types/src/app_providers/update.rs::AppProviderUpdate`。字段缺省保留；显式 null、所有权字段、未知字段和 addToLive 拒绝。复用任务 3 的 `ProviderSecretWrite`：遗漏或 Keep 保留凭据，Replace 显式替换；`[redacted]`、`<redacted>`、纯 `*`/`•` 掩码拒绝，空密钥由既有校验拒绝，不从空值推断保留或清除。当前 profile 要求凭据，Clear 返回 400。Debug 中的秘密仍脱敏。
- 存储：`AppProviderStore::update` 复用原租户/应用/ID 键和 namespace。`GatewaySettingsStore::compare_exchange` 及 SQLite/PostgreSQL 条件 UPDATE 同时比较完整作用域和读取的原始 payload；不存在不新增，过期写入不覆盖新字段，返回受控 409。元数据由既有档案读取路径投影后继续可读，响应不输出元数据或凭据。旧 PUT/save 的整条 upsert 行为不变。
- API：`PATCH /v1/app-providers/{app}/{id}`，成功 200 返回脱敏 AppProviderSummary；缺失 404，非法请求 400，写入冲突 409，存储不可用 503。沿用现有分域 router、16 KiB 限制、Basic 认证及 TenantScope；handler_router/bootstrap 已共享根注册，仅新增直接子模块与方法路由。租户头仍只表达存储作用域，不额外证明公司成员身份；未授予 host live-config 权限。
- SDK：`vw-gateway-client/src/client/app_providers/update.rs::app_provider_update` 使用 PATCH、既有认证/租户请求上下文；HTTP 错误仅保留状态，不回显服务器原始正文或请求秘密。DTO、业务、handler、SDK 均使用指定的独立 update_tests.rs 注册。
- UI：供应商 Iced 编辑入口属于任务 53，本轮未新增或验收 UI。各应用当前供应商 live 重投影仍由后续应用任务接入同一更新入口；不声称 live 已成功。

### 红/绿与平台证据

所有验证阶段均以原子 mkdir 动态领取槽位 **1**：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，同时将 `CARGO_BUILD_BUILD_DIR` 设为同一绝对路径，避免仓库 build-dir 配置将中间产物放回公共目录。锁 owner 记录计划、任务、会话标识和持有 shell PID；各阶段 EXIT/INT/TERM 清理自身记录及锁，保留缓存。

红/绿阶段使用相同的四条默认 features 聚焦命令：

```bash
cargo test -p vw-agent-apps --lib cc04_update_ -- --nocapture
cargo test -p vw-api-types --lib cc04_update_ -- --nocapture
cargo test -p vw-agent --lib cc04_update_ -- --nocapture
cargo test -p vw-gateway-client --lib cc04_update_ -- --nocapture
```

| crate | 红阶段退出码 / 有效结果 | 最终绿阶段退出码 / 有效结果 |
| --- | --- | --- |
| vw-agent-apps | 101；3 项，1 passed/2 failed：丢失未编辑 metadata，接受占位符替换 | 0；3 passed |
| vw-api-types | 0；1 passed | 0；1 passed |
| vw-agent | 101；2 项业务失败：元数据丢失，占位符请求返回 200 而非 400 | 0；3 passed |
| vw-gateway-client | 0；2 passed | 0；2 passed |

红阶段所有测试均成功编译并执行，失败来自业务断言；无零测试替代。绿阶段执行 `cargo fmt --all`（退出 0），再运行上述四条命令，最后 `cargo clippy --all-targets --all-features -- -D warnings`（退出 0）。首次绿阶段全部通过后补齐公开 DTO 的密钥语义说明，并对追加的集成测试重新格式化，再完整执行相同聚焦命令及 Clippy；表中记录最终结果。最终有效测试共 **9** 项，未忽略；Clippy 是全仓全部 targets/features 检查，不计行为测试。Agent 链接有既有 `__eh_frame section too large` 提示，测试退出 0；不修改链接配置隐藏提示。

业务测试使用真实临时 JSON 文件核对更新后所有未编辑字段、嵌套元数据和创建时间；另验证 Keep、显式 Replace、Clear/非法端点/非法 ID 拒绝。Gateway 测试使用真实隔离 SQLite、正式 handler_router，断言 Basic 401、跨租户/应用与缺失记录 404、非法请求 400 且档案未变；条件写入测试证明跨平台/公司/应用不命中、过期 payload 不覆盖新值且不改变时间戳，之后局部更新仍保留新字段及元数据。

SDK → 本地回环 HTTP → 正式 handler → 隔离 SQLite → 脱敏响应/GET 闭环，证明改名、显式凭据/模型替换及默认 Keep 后目录值正确，临时 `.claude/settings.json` 字节和目录成员未变。此集成服务只替代远端传输部署，Basic 拒绝由单独正式中间件测试覆盖；不把此临时服务当真实外部部署或公司成员认证验收。SDK crate 独立 HTTP 测试只替代远端响应，证明 PATCH/租户/缺省字段线协议及 409/403 错误不泄漏原始正文。凭据全部为 fixture，没有读取或修改真实用户配置。

`sw_vers`、`uname -m` 退出均为 0：macOS 27.0（26A428）、arm64。应用版本未验证：本任务为目录保存，只构造隔离文件，没有读取或启动真实应用。Iced、live 重投影、应用运行、Windows/Linux、其他架构与 PostgreSQL 实际服务均未验收；PostgreSQL 适配参与编译/Clippy，不宣称运行验收。

最终定向 `git diff --check`、`git diff --cached --check` 退出 0；只读 plan.md 无差异，槽位 1 自身锁已释放，缓存保留。仅提交本任务实际代码、测试及本条证据，不纳入母任务状态或其他编号文件删除；完成后的编号任务文件按要求删除且不纳入提交。未执行全量 cargo test、fmt check、wasm 检查或 Makefile。

## 任务 5：删除供应商的引用检查

2026-11-06；状态：已完成（按继续执行指令限定的可靠读取范围）。

### 范围、依赖及引用事实

批次 4 提交 `d856c8e72` 及本文件任务 1–4 证据已核对；母任务 1、3 完成，母任务 2 沿用任务 1 的底座开工裁定，不改报完整迁移完成。最初因托管引用入口缺失登记阻塞；继续执行指令明确授权补最小只读检查，并允许对无法可靠判断的应用拒绝删除，现按此边界实施。

来源函数：`mode/current.rs::is_referenced/provider_for`、`services/provider/mod.rs::ProviderService::delete`、`commands/provider.rs::delete_provider`，相邻 `provider_service_delete_current_provider_returns_error`。本任务采用更保守的成员引用保护：原生成员存在即拒绝，不沿用上游删除时同时移除 live 成员的行为。

引用不存在的判据不是 `NotImplemented`：检查构建根 `vw-agent-apps/src/lib.rs` 及 providers 模块树，upstream 不参与构建；追踪全部 `AppProviderStore` 调用者，当前写入仅 save/create/update 目录档案，launch 只读档案生成会话材料，Kimi/OpenCode 的会话投影也明确不支持。实际持久化路径只有 `app_providers.records` 命名空间，未迁移 CC DB/current/settings 或 mode 状态，也没有托管直连/代理路由状态写入者。真实隔离 SQLite 集成断言此阶段的命名空间集合只有该目录；此断言与代码调用审查共同构成当前版本不存在托管引用的证据，不将单一 fixture 空表推及所有未来版本。任务 45/46 或应用适配加入状态写入时必须接入本删除检查；当前不引入第二套指针、路由或状态所有者。

- `providers/delete.rs::ensure_unreferenced` 仅放行 macOS Kimi/OpenCode 的可靠原生读取，直接复用 `catalog::observe`，不从 DTO 的 unavailable 推断未引用。Kimi 检查真实 providers/default_model→models→provider，OpenCode 检查真实 provider 成员；任意成员或选择引用均拒绝，只有明确未引用才可删除目录记录。
- `AppProviderStore::delete` 先按租户/应用/ID 确认记录，执行只读检查后使用既有 settings 删除同一作用域的记录；不读取 profile 凭据，不更改 live、不导入 CC 数据库。Kimi/OpenCode 的会话 profile 创建/激活仍属后续应用任务，本轮通过既有存储接口准备完整结构 fixture，不宣称这些应用已能从创建 API 保存或激活。
- 其他应用、非 macOS、JSONC、自定义 Kimi HOME、损坏/不可读/符号链接配置明确拒绝，不当作空配置；缺省文件可可靠确认无成员。其余应用的原生读取/适配归任务 26–43，完整模式状态归任务 45/46。
- `DELETE /v1/app-providers/{app}/{id}` 接入现有分域 router（handler_router/bootstrap 共享），16 KiB 限制；DTO 只收 targetHostId，不接受客户端引用事实、路径、租户或 force。删除目录仍走既有网关认证/TenantScope，并额外使用既有独立 provider_config_read grant 读取指定执行主机；此 grant 不授予修改配置文件权限。拒绝在读取存储/配置之前完成，不拓宽主机授权。
- SDK `app_provider_delete` 发 DELETE、既有认证与租户上下文、敏感标记 x-agent-apps-token，返回强类型 deleted；错误保留 HTTP 状态但不输出原始正文。API 未找到 404、仍引用/无法确认 409、输入错误 400、存储不可用 503；原有主机授权状态保留。

删除检查是当前授权目标主机的即时配置观察，不证明外部进程未来不会改写配置、也不声称任意其他主机没有引用。无 live 写入及状态写入者的当前实现不建立新事务/跨进程锁；后续拥有真实指针写入的事务必须将读取/删除纳入同一互斥边界。Iced 入口属于任务 53，本轮未新增或验收 UI。

### 红/绿、平台与差异证据

红/绿阶段均通过原子 mkdir 动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 为同一绝对路径；owner 记录计划、任务、会话标识与 shell PID，EXIT/INT/TERM trap 清理自己记录及锁。阶段结束后锁不存在、缓存保留。

两阶段先执行 `cargo fmt --all`（退出 0），再使用默认 features 的同一组命令：

| 确切命令 | 红阶段退出码 / 有效结果 | 绿阶段退出码 / 有效结果 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_delete_ -- --nocapture` | 101；3 项，1 passed/2 failed：引用成员与未知应用被放行 | 0；3 passed |
| `cargo test -p vw-api-types --lib cc04_delete_ -- --nocapture` | 0；1 passed | 0；1 passed |
| `cargo test -p vw-agent --lib cc04_delete_ -- --nocapture` | 101；2 项业务失败：仍引用删除成功、未知应用返回 200 而非 409 | 0；4 passed（补租户/应用隔离、正式路由注册） |
| `cargo test -p vw-gateway-client --lib cc04_delete_ -- --nocapture` | 0；1 passed | 0；1 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 不适用 | 0；全仓全部 targets/features 通过 |

最终有效测试共 **9** 项，未忽略，无零测试代替验收。红阶段先使调用链编译，用尚未加检查的删除路径证明业务断言失败，再实现最小引用检查；不是以缺模块/类型当红证据。绿阶段 Agent 链接有既有 `__eh_frame section too large` 提示，测试仍退出 0；Clippy 无诊断，不修改链接配置。

业务独立测试读取真实临时 Kimi TOML/OpenCode JSON，证明成员拒绝、unused 可删且 live 不变；另覆盖未知应用、损坏文件、缺省文件、JSONC 优先级和符号链接拒绝。Gateway 使用真实隔离 SQLite、既有管理员 policy 和租户中间件；SDK→本地回环 HTTP→正式 delete handler→SQLite→强类型响应闭环，Kimi/OpenCode 的 used 均 409、unused 均成功、重复删除 404，其他目录行与两份 live 文件字节保持不变。测试只有远端部署传输被本地服务替代，无 mock 引用列表或假数据库；所有凭据都是 fixture。

本地 SDK 集成服务未启用 Basic（SDK 当前只支持 Bearer），Basic 的 401/成功认证后主机 token 401、公司/主机 grant 403 和 unknown-config 409 由单独测试覆盖。正式 handler_router 的 DELETE 注册使用真实路由另测 400 且记录不变；独立 SDK HTTP 测试覆盖方法、URL、租户、主机 DTO/token 与 409 正文脱敏。存储额外证明同 ID 的跨平台、公司、应用隔离。未将目录租户头解释为公司成员身份，不声称真实部署认证验收。

`sw_vers`、`uname -m` 均退出 0：macOS 27.0（26A428）、arm64。应用版本未验证：只使用来源 schema 的隔离文件，不读取/启动真实用户应用，不声称 live 切换、代理流量或应用运行成功。Windows/Linux、其他架构、其他 12 应用删除、Iced 与 PostgreSQL 实际服务未验收；PostgreSQL 复用现有 delete 后端，参与编译/Clippy。完整模式状态/互斥仍归任务 45/46。

定向 `git diff --check` 与 `git diff --cached --check` 退出 0，审查所有本任务代码/测试/文档；只读 plan.md 无差异。只提交本任务变更，不纳入其他任务状态/删除；本编号任务文件完成后删除且不纳入提交。未执行全量 cargo test、fmt check、wasm 检查或 Makefile。

## 任务 6：复制供应商到新标识

2026-11-06；状态：已完成。

### 依赖、来源与实现边界

核对批次 5 提交 `f30992935` 及本文件任务 1–5 的验收记录；母任务 1、3 已完成，母任务 2 完整迁移尚未收口，沿用任务 1 的底座开工裁定，不将其改报完整完成。只读 plan.md 不修改。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`；`git rev-parse HEAD`、`git status --short` 均退出 0，来源工作区干净。读取 `ProviderList → ProviderCard → App.tsx::handleDuplicateProvider → useProviderActions::addProvider`，相邻 `tests/integration/App.test.tsx` 的复制与 live-only ID 用例，以及 `ProviderService::list/add_without_switching/add`。来源在前端深复制、生成 ID 并可能移动排序；本任务将现有档案能力的复制放到执行端，不迁入其他应用适配、排序调整或 live 同步。

- 唯一业务实现：`vw-agent-apps/src/providers/duplicate.rs::duplicate`。复用既有 `update::profile` 和身份/profile 校验，支持范围与已有 profile 契约相同；不能由本任务推及其余尚未实现的应用配置。配置复制 protocol/apiKey/baseUrl/model，名称由请求显式提供。API key 是既有可编辑供应商配置，不复制 OAuth 登录材料或授权账号。
- 元数据采用白名单：仅复制字符串 websiteUrl/category/notes/icon/iconColor，以及既有 opaque meta 中的字符串 notes。没有把整块 meta、嵌套对象或未知字段带入新记录；authBinding/githubAccountId/liveConfigManaged/current/login/inFailoverQueue/createdAt/sortIndex 等均不复制。排序、端点管理、扩展元数据与应用适配仍由所属任务定义，未来字段不会自动获得复制权限。图标只复制关联值，不复制图标库所有者。
- 存储接线：`vw-agent/src/app_providers.rs::AppProviderStore::duplicate` 只从同 TenantScope、同 AppId 读取源快照，通过原 namespace 与复合键的 `GatewaySettingsStore::insert_if_absent` 原子插入目标。不存在返回 404；目标冲突或同 ID 返回 409，既有记录及时间戳不变。复用 SQLite/PostgreSQL 既有插入实现，不新建数据库、指针或永久业务所有者。
- DTO：`vw-api-types/src/app_providers/duplicate.rs::AppProviderDuplicate` 仅含 newId/name，拒绝未知字段、秘密、租户、路径、绑定和激活意图。`POST /v1/app-providers/{app}/{id}/duplicate` 返回 201 脱敏摘要，非法 400、缺失 404、碰撞 409、存储故障 503；沿用既有目录认证、TenantScope 和 16 KiB 限制。无原生主机文件操作，权限与已有目录 create/update 相同；不增加主机配置授权。公司头仅表达存储作用域，不作为公司成员身份认证。
- SDK：`vw-gateway-client/src/client/app_providers/duplicate.rs::app_provider_duplicate` 使用 POST、既有认证/租户上下文，失败保留 HTTP 状态，不回显服务器原始正文、源秘密或绑定。四层子模块在直接父模块登记，根 handler_router/bootstrap 沿用既有分域注册。
- UI 属任务 53，本轮未新增或验收 Iced 窗口；configured=true 只表示目录档案存在，不表示当前供应商、原生激活或应用运行成功。

### 红/绿验证与平台证据

两阶段均以原子 mkdir 动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 也设为相同绝对路径，覆盖仓库中间产物 build-dir。owner 记录计划、任务、会话标识与 shell PID；EXIT/INT/TERM trap 清理自身记录及锁，保留缓存。

红阶段接线完成后先执行 `cargo fmt --all`（退出 0），使用尚未过滤字段的整条复制跑以下默认 features 命令；全部成功编译，业务断言失败而非缺少模块/类型。绿阶段实现白名单并补齐并发/非法字段验证，再执行 `cargo fmt --all`（退出 0），然后运行相同命令及强制 Clippy：

| 确切命令 | 红阶段退出码 / 有效结果 | 绿阶段退出码 / 有效结果 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_duplicate_ -- --nocapture` | 101；2 项，1 passed/1 failed：整条复制带入绑定、登录和运行态 | 0；3 passed |
| `cargo test -p vw-api-types --lib cc04_duplicate_ -- --nocapture` | 0；1 passed | 0；1 passed |
| `cargo test -p vw-agent --lib cc04_duplicate_ -- --nocapture` | 101；2 项业务断言失败：meta 中登录绑定和 live 状态被复制 | 0；3 passed |
| `cargo test -p vw-gateway-client --lib cc04_duplicate_ -- --nocapture` | 0；1 passed | 0；1 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 不适用 | 0；全部 targets/features 检查通过 |

最终共 **8** 项有效测试，未忽略，无零测试替代验收。Agent 测试链接出现既有 `__eh_frame section too large` 提示，退出 0；未修改链接配置隐藏诊断。业务用真实临时文件断言复制结果完整白名单、源字节不变，create_new 拒绝碰撞且旧文件不变；非法 ID/名称、登录式缺失凭据拒绝，名为展示字段的嵌套对象也不复制。

Gateway 使用真实隔离 SQLite 和正式 handler_router，验证 Basic 401、跨租户/应用 404、同 ID/碰撞 409、非法输入 400，无副作用时全表 payload 与 updated_at_ms 保持一致；源记录在成功复制后也不变。6 个线程同时复制到同一标识恰好一个成功，其他均明确冲突。

SDK → 本地回环 HTTP → 正式 handler → 真实 SQLite → 201 脱敏摘要/GET 闭环，验证目标配置和可复制描述字段、绑定剔除、重复请求拒绝、源目录及隔离 native fixture 字节不变。该本地服务只替代远端部署传输，未启用 Basic（SDK 当前只支持 Bearer），Basic 拒绝由单独正式中间件测试覆盖，不声称真实部署或公司成员认证已验收。SDK crate 独立 HTTP fixture 证明方法/URL/请求体/租户及 409/403 正文脱敏，不冒充执行端业务。凭据均为 fixture，没有读取/修改真实用户配置。

`sw_vers`、`uname -m` 均退出 0：macOS 27.0（26A428）、arm64。应用版本未验证：本任务为目录复制，没有读取或启动真实应用。Iced、live 切换/运行、其他应用适配、Windows/Linux、其他架构与 PostgreSQL 实际服务均未验收；PostgreSQL 复用既有原子新增适配并参与编译/Clippy，不宣称运行验收。

最终定向 `git diff --check` 与 `git diff --cached --check` 退出 0，审查本任务全部代码/测试/文档；plan.md 无差异，自身槽位锁已释放。只提交本任务变更，不纳入母任务状态或其他编号文件删除；本任务完成后删除编号文件且不纳入提交。未执行全量 cargo test、fmt check、wasm 检查或 Makefile。

## 任务 7：排序分类与备注

2026-11-06；状态：已完成。

### 依赖、来源与实现

已核对批次 6 提交 `5235bae51` 和本文件任务 1–6 证据；母任务 1、3 已完成，母任务 2 沿用任务 1 已记录的底座开工裁定，完整领域迁移仍未收口。没有修改只读 plan.md。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，工作区干净（rev-parse/status 均退出 0）。读取 `provider.rs::Provider` 的 sortIndex/category/notes、`ProviderService::update_sort_order/update`、`database/dao/providers.rs::get_all_providers/update_provider_sort_indexes`、commands/provider.rs 的排序调用，以及 ProviderList/presentation.ts 和相邻 providerPresentation.test.ts 的排序并列用例。来源队列移动涉及路由队列，不在本任务迁入；本任务提供目录展示字段的持久化读写。

- `vw-agent-apps/src/providers/presentation.rs::read/update` 是字段投影与校验实现：仅替换 sortIndex/category/notes，保留凭据、配置、绑定、创建时间及其他未知元数据。category 作为分类/分组键，最长 128 字节且拒绝控制字符；notes 最长 8192 字节，允许换行、回车、制表，拒绝其他控制字符；排序为非负 u32。非对象存储、错误字段类型明确失败。
- `vw-api-types/src/app_providers/presentation.rs` 提供强类型 AppProviderPresentation/Entry。PUT 为完整替换这三个展示字段，null 或遗漏清空该字段，空字符串作为空串保存；拒绝未知字段、秘密、租户、路径、绑定、current/addToLive 等运行意图，不接受任意档案补丁。
- `AppProviderStore::set_presentation/list_presentation` 复用 `app_providers.records`、TenantScope/AppId/ID 复合键与既有 compare_exchange。不存在返回 404，并发旧快照返回 409，不新增记录或覆盖新配置。列表从同一真实存储读取，显式索引升序、同索引按 ID、无索引最后；最大 u32 与无索引严格区分。分类仅作本接口展示数据，不赋予官方账号身份或任何运行权限。没有新建存储、数据库迁移或整表排序事务接口。
- API `PUT /v1/app-providers/{app}/{id}/presentation` 返回保存后的三字段；`GET /v1/app-providers/{app}/presentation/entries` 返回按顺序的 `{id,presentation}` 列表，不返回 profile 或凭据。使用独立 entries 后缀避免遮蔽既有 ID 为 presentation 的供应商 GET，并有真实路由回归断言。沿用分域 router、Basic 认证、TenantScope 和 16 KiB 写入限制；非法 400、缺失 404、冲突 409、存储不可用 503。公司头仅表示存储作用域，不证明成员身份；没有授予或调用主机 live-config 权限。
- SDK `app_provider_presentation_set/list` 经既有请求上下文发送 PUT/GET；失败只报告 HTTP 状态，传输/解析错误固定文案，不回显服务器正文或凭据。四层独立 presentation_tests.rs 已接直接父模块；bootstrap/handler_router 沿用已有分域注册。
- UI 归任务 53，本轮未新增或验收 Iced。没有调用 native writer、修改 current/路由/队列状态、切换应用或同步 live 配置；目录排序不代表路由优先级切换成功。

### 红/绿、平台与审查证据

红阶段与两轮绿阶段均用原子 mkdir 动态领取共享槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`；`CARGO_BUILD_BUILD_DIR` 同一绝对路径，覆盖仓库 build-dir。owner 记录本计划、任务、会话及 shell PID，EXIT/INT/TERM trap 清理自身 owner/锁，全部子进程结束后释放，保留编译缓存；未创建其他 target 目录。最终槽位 1 锁不存在。

各阶段先执行 `cargo fmt --all`（退出 0），默认 features 使用以下完全相同的过滤命令：

| 确切命令 | 红阶段退出码 / 有效结果 | 最终绿阶段退出码 / 有效结果 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_presentation_ -- --nocapture` | 101；2 failed：排序仍为旧值，分类/备注未保存 | 0；3 passed |
| `cargo test -p vw-api-types --lib cc04_presentation_ -- --nocapture` | 0；1 passed | 0；1 passed |
| `cargo test -p vw-agent --lib cc04_presentation_ -- --nocapture` | 101；1 passed/1 failed：SDK→API 返回未保存的空展示字段 | 0；3 passed |
| `cargo test -p vw-gateway-client --lib cc04_presentation_ -- --nocapture` | 0；1 passed | 0；1 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 不适用 | 0；全部 targets/features 通过 |

红阶段成功编译，失败发生在业务断言；不是缺模块/类型或零测试。最终 **8** 项有效测试，无忽略。首轮绿阶段 8 项通过，Clippy 退出 101 指出本次两个嵌套 if 可合并；修正为 let 条件链后，重新 fmt、上述四条聚焦命令及完整 Clippy 全通过。Agent 链接有既有 `__eh_frame section too large` 提示，测试退出 0，未改链接设置隐藏提示。

业务测试通过真实临时 JSON 文件保存/重读证明 `cc04_presentation_order_survives_reload`；`cc04_presentation_metadata_does_not_change_live` 证明元数据正确、其他字段保持、清空语义和 native fixture 字节不变。额外覆盖非法文本/损坏存储。Gateway 用真实隔离 SQLite 和正式路由，断言 Basic 401、缺失 404、运行态字段 400，拒绝后原记录及时间戳不变；最大索引、并列和清空顺序确定。

SDK→回环 HTTP→正式 handler→SQLite→强类型响应/GET→重开数据库闭环证明排序、中文分类和备注可持久化；同 ID 的跨 platform/company/application 档案不变，其他原记录、凭据和绑定保留，隔离 native fixture 字节不变。数据库及文件均为临时目录，凭据全为 fixture，无真实用户配置访问。此本地服务只替代远端部署传输，未开启 Basic（SDK 当前支持 Bearer），Basic 拒绝由独立正式中间件测试覆盖；独立 SDK HTTP fixture 仅替代远端响应，验证方法/URL/公司头/请求体及 403/409 正文脱敏。未将该测试称作真实外部部署或公司成员认证验收。

`sw_vers`、`uname -m` 均退出 0：macOS 27.0（26A428）、arm64。应用版本未验证：本任务只读写隔离目录档案，没有启动实际应用。Iced、live 切换/应用运行、Windows/Linux、其他架构、PostgreSQL 实际服务未验收；PostgreSQL 复用已有条件写入适配并参与 Clippy，不宣称运行验收。

定向 `git diff --check`、`git diff --cached --check` 退出 0，本任务实现/测试/正式证据已审查；plan.md 无差异。仅提交本任务变更，不纳入原有母任务状态和任务 1–6 删除。完成后的任务 7 文件删除不纳入提交。未运行全量 cargo test、fmt check、wasm 检查或 Makefile。

## 任务 8：自定义端点维护

2026-11-06；状态：已完成。

### 依赖、来源与实现

核对批次 7 提交 `fc70aa789` 与本文件任务 1–7 的验收证据。母任务 1、3 当前状态已完成；母任务 2 完整领域迁移仍未收口，沿用任务 1 和 provider-contracts.md 记录的底座子计划 1–93 开工裁定，不伪称母任务 2 已完成。只读 plan.md 未修改。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 均退出 0，来源工作区干净。读取 `services/provider/endpoints.rs::{get_custom_endpoints,add_custom_endpoint,remove_custom_endpoint,update_endpoint_last_used}`、commands/provider.rs 的四个直接调用者和 database/dao/providers.rs 的增删实现；前端读取 `useBaseUrlState`、ProviderForm 引用、FullUrlToggle 及相邻 `ProviderFormSettingsConfigSnapshot.test.tsx`，核对当前快照不能覆盖其他配置与完整请求 URL 不追加路径的语义。来源中的测速、代理请求和旧 CC 数据库未迁入。

- `vw-agent-apps/src/providers/endpoints.rs::{read,update}` 为唯一地址簿业务实现。端点保存在既有供应商记录的 `customEndpoints`，不新增数据库或永久业务所有者。保留 apiKey/baseUrl/model、未知字段、绑定和运行状态。列表按 addedAt 降序、同时间按 URL 排序；身份由 URL 与 kind 共同组成。重复新增、删除不存在项幂等；新增不隐式标记使用，markUsed 对不存在项明确失败。addedAt/lastUsed 为执行端毫秒时间；lastUsed 不因时钟回退而倒退。显式使用记录不证明请求成功或应用当前正在使用该地址。
- DTO `vw-api-types/src/app_providers/endpoints.rs` 显式要求 `kind: baseUrl|fullUrl`。baseUrl 去外层空白及末尾斜杠，可含版本路径；fullUrl 要求非空请求路径，只去外层空白，完整保留路径与末尾斜杠。不猜测/拼接路径，不修改供应商的当前 baseUrl 或 isFullUrl，不声称已经接入直连/代理适配。两类均要求 HTTPS，或 localhost/127.0.0.1/[::1] 的 HTTP；拒绝相对地址、非 HTTP(S)、用户凭据、查询参数、片段、内部空白/控制字符和反斜杠。单 URL 最长 2048 字节，最多 64 项；损坏/重复存储明确失败，不能作为空列表返回。
- `AppProviderStore::{change_endpoint,list_endpoints}` 复用既有 `app_providers.records`、TenantScope/AppId/ID 键以及 compare_exchange 条件更新，不创建缺失供应商，不覆盖并发更新的配置。SQLite/PostgreSQL 使用既有 settings 后端，无迁移或任意文件 API。
- API `GET/POST /v1/app-providers/{app}/{id}/endpoints`；POST 接受 `action: add|remove|markUsed` 与 url/kind，拒绝客户端时间戳、秘密、路径、租户、current 等字段。沿用正式分域 router（handler_router/bootstrap 共享）、网关认证、TenantScope 和 16 KiB 限制。供应商缺失 404、并发冲突 409、非法输入/不存在端点的 markUsed 400、损坏存储/不可用 503。只读写目录地址簿，权限与已有目录维护相同，不授予主机配置写入权限。公司头仍仅表示存储作用域，不证明公司成员身份。
- SDK `app_provider_endpoint_change/app_provider_endpoints` 使用既有认证与租户上下文、强类型请求/响应；错误保留 HTTP 状态，不回显服务端正文、敏感载荷或传输原始错误。四层直接父模块接线完成，未另开通用命令入口。
- UI 归任务 68，本轮未新增或验收 Iced；测速归母任务 9。没有调用 native writer、激活供应商、修改 live 文件、创建托管指针或变更路由/队列。

### 红/绿、平台与审查证据

红/绿阶段均通过原子 mkdir 动态领取共享槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同一绝对路径以覆盖仓库 build-dir。owner 记录计划、任务、会话标识与 shell PID，EXIT/INT/TERM trap 清理自身记录及锁；阶段全部子进程结束后释放，保留缓存。最终槽位 1 锁不存在。

各阶段先执行 `cargo fmt --all`（退出 0），使用默认 features 的以下相同过滤命令：

| 确切命令 | 红阶段退出码 / 有效结果 | 绿阶段退出码 / 有效结果 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_endpoints_ -- --nocapture` | 101；2 failed：新增后列表仍空、非法 URL 被接受 | 0；3 passed |
| `cargo test -p vw-api-types --lib cc04_endpoints_ -- --nocapture` | 0；1 passed | 0；1 passed |
| `cargo test -p vw-agent --lib cc04_endpoints_ -- --nocapture` | 101；2 failed：非法 URL 返回 200 而非 400、SDK 新增后列表仍空 | 0；2 passed |
| `cargo test -p vw-gateway-client --lib cc04_endpoints_ -- --nocapture` | 0；1 passed | 0；1 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 不适用 | 0；全部 targets/features 通过 |

红阶段测试先接线编译，以尚未修改记录的实现证明业务断言失败，不以缺少模块/类型或零测试代替红证据。最终 **7** 项有效测试，无忽略。Agent 链接出现既有 `__eh_frame section too large` 提示但测试退出 0，未改链接设置隐藏提示。

业务独立测试在临时目录写入/重读 JSON，验证 `cc04_endpoints_add_remove_and_last_used` 的增删、去重、排序、时间戳及完整路径末尾斜杠保留；`cc04_endpoints_invalid_url_does_not_mutate` 验证非法 URL 拒绝且输入不变；第三项验证容量、损坏存储、不存在项使用和时钟回退。DTO 独立测试证明类型往返及拒绝客户端时间戳/运行字段/隐式 kind。

Gateway 使用真实隔离 SQLite、正式 handler_router 和 Basic 认证中间件，证明 401、404、400 拒绝后记录及更新时间不变。SDK→本地回环 HTTP→正式 POST/GET handler→SQLite→强类型响应→重开数据库闭环，验证 base/full 两种地址增删、显式使用时间与落盘重读；相同 ID 的跨 platform/company/application 档案不变，原记录凭据/绑定/运行字段保留，临时 native fixture 字节不变。所有文件和凭据均为 fixture，没有访问真实用户配置。

本地服务只替代远端部署传输，没有启用 Basic（SDK 当前支持 Bearer）；Basic 拒绝由独立正式路由测试覆盖，不将本地 HTTP 当作真实外部部署或公司成员认证。SDK crate 的独立 HTTP fixture 验证方法、URL、租户、请求体及 403/409 正文脱敏，不冒充执行端业务。

`sw_vers`、`uname -m` 均退出 0：macOS 27.0（26A428）、arm64。应用版本未验证：本任务仅为隔离目录地址簿维护，没有启动实际应用。Iced、测速、live 切换/应用运行、Windows/Linux、其他架构与 PostgreSQL 实际服务未验收；PostgreSQL 复用既有条件更新后端并参与 Clippy，不宣称运行验收。

本任务代码/测试/正式证据已审查，定向 `git diff --check`、`git diff --cached --check` 退出 0；plan.md 无差异。只提交本任务变更，排除原有母任务状态及其他编号文件删除。本任务完成后删除编号任务文件且不纳入提交。未执行全量 cargo test、fmt check、wasm 检查或 Makefile。

## 任务 9：供应商图标库

2026-11-06；状态：已完成。

### 依赖、来源与接线

核对批次 8 提交 `19ca94f39` 及任务 1–8 正式证据；母任务 1、3 已完成。母任务 2 完整领域迁移仍未收口，沿用任务 1 与 provider-contracts.md 的底座子计划 1–93 开工裁定，不冒充母任务 2 完成。只读 plan.md 未修改。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 退出 0，来源干净。读取 `services/provider/icon.rs::{validate_icon,is_valid_custom_icon,list_library_icons,add_library_icon,rename_library_icon,delete_library_icon}`、相邻完整 PNG/容量/去重/改名/删除测试、commands/custom_icon.rs、lib.rs 注册、customIcons API/query 与 IconPicker 调用者。来源选择 `icon.data` 写入供应商，库 ID 仅用于管理库条目；删除库条目不应删除供应商的图片副本。

- 复用 `vw-agent-apps/src/icons/{validation,name,library}.rs`、`vw-agent/src/agent_apps/store.rs::IconLibraryStore` 及 SQLite/PostgreSQL 原子事务；唯一库仍为 `agent_apps.custom_icons/default`。没有第二份库、存储迁移或新增依赖。
- 保存/改名/删除继续使用原 `POST/GET /v1/agent-apps/custom-icons`、`PATCH/DELETE /v1/agent-apps/custom-icons/{id}` 和 SDK `custom_icon_add/custom_icons_list/custom_icon_rename/custom_icon_delete`。完整解码 PNG（含块校验与像素）、data URI 不超过 64 KiB、尺寸不超过 256×256、解码内存上限 1 MiB、名称 1–64 Unicode 字符、库最多 100 项；重复数据保留原 ID/名称/时间，改名不改图片，删除不存在项幂等。
- `vw-agent-apps/src/providers/icons.rs::{read,update}` 只负责供应商消费共享库。选择时校验 ID、引用存在性/唯一性及完整 PNG，然后复制图片；clear 清除 icon/iconColor，其余秘密、未知字段与配置保留。库改名/删除不会产生悬空引用。原生配置、应用激活、直连指针、代理路由均不改变。
- DTO `app_providers/icons.rs` 显式 `action: select` + `iconId` 或 `action: clear`，拒绝任意路径、data、租户和运行字段。新增 `GET/PUT /v1/app-providers/{app}/{id}/icon`，返回 `{data: PNG data URI|null}`；读入非法图片、路径或外链返回受控 503。这个接口仅投影自定义 PNG，既有其他 icon 字符串不被解释为可访问路径。
- `AppProviderStore::{get_icon,set_icon}` 沿用现有租户/应用/供应商键与 compare_exchange，缺失供应商/库引用 404、输入非法 400、并发修改 409、存储不可用 503。SDK `app_provider_icon/app_provider_icon_change` 保留状态码，不回显服务端正文或传输错误细节。四层直接父模块登记到现有分域 router，handler_router/bootstrap 共用；无通用命令入口。沿用目录维护权限、网关认证和 TenantScope，不授予主机配置写权限；公司头不独立证明公司成员身份。
- 共享存储的 `read_icons` 与 `rename_icon` 对将返回的图片增加完整 PNG 校验，阻断历史导入数据中的路径/外链经列表或改名响应返回。沿用原有损坏数组返回空、坏形状条目过滤的行为，不声称已经统一为严格解析；本轮安全校验针对可反序列化条目的图片载荷。相邻存储测试的非图片占位值换为真实 PNG，并新增危险图片拒绝断言。

### 红/绿、平台与审查证据

各阶段原子 mkdir 动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同一绝对路径覆盖仓库 build-dir。owner 记录计划、任务、会话与 PID，EXIT/INT/TERM 清理自身 owner 与锁，全部子进程结束才释放，缓存保留。

红阶段先用可编译的未消费图片实现接线，四条命令均为默认 features，先 `cargo fmt --all`（退出 0）：

| 确切命令 | 红阶段退出码 / 有效结果 | 最终绿阶段退出码 / 有效结果 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_icons_ -- --nocapture` | 101；2 failed：选择后图片仍为空、非法引用被接受 | 0；2 passed |
| `cargo test -p vw-api-types --lib cc04_icons_ -- --nocapture` | 0；1 passed | 0；1 passed |
| `cargo test -p vw-agent --lib cc04_icons_ -- --nocapture` | 101；2 failed：非法选择返回 200、SDK 选择图片仍为空 | 0；2 passed |
| `cargo test -p vw-gateway-client --lib cc04_icons_ -- --nocapture` | 0；1 passed | 0；1 passed |

最终补充跨租户引用拒绝和真实 PNG 存储夹具后，重新运行 `cargo fmt --all`（0）、上述四条命令（共 **6** 项有效测试），再执行以下相邻回归及强制检查：

| 确切命令 | 退出码 | 有效数量 / 结果 |
| --- | --- | --- |
| `cargo test -p vw-agent --lib cc_base_icon_store_read_sqlite_reopen_isolation_and_corruption -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent --lib cc_base_icon_api_add_sqlite_read_and_deduplicate -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent --lib cc_base_icon_api_read_scoped_sqlite -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent --lib cc_base_icon_api_rename_sqlite_boundaries_and_scope -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent --lib cc_base_icon_api_delete_sqlite_idempotent_and_scoped -- --nocapture` | 0 | 1 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 targets/features 通过 |

共 **11** 项有效测试，无忽略、零测试或编译代替行为验收。测试二进制来自领取槽位 `debug/deps`。Agent 链接仍有既有 `__eh_frame section too large` 提示，测试退出 0；未改链接设置。

业务测试使用真实编码 PNG 与临时文件重读，验证保存、去重、改名、删除后的副本、clear、缺失/非法引用及 SVG/外链/路径/过大/超尺寸/截断图片拒绝。Gateway 通过正式路由与真实隔离 SQLite，SDK→本地回环 HTTP→API→共享库原子事务/供应商条件更新→重开数据库，验证去重、改名、删除、选择与清除、跨 platform/company/app 记录保留，以及跨租户引用 404。Basic 无凭据 401 且不改记录，非法字段 400、损坏图片读取/改名 503 且不含路径或秘密；临时 native fixture 字节保持不变。

本地 HTTP 只替代远端传输，未启用 Basic（SDK 支持 Bearer）；Basic 拒绝由独立正式路由测试覆盖，不冒充真实外部部署认证。SDK crate 的 HTTP fixture 只证明线协议、租户与错误正文脱敏。使用假凭据和临时目录，没有读取或修改真实用户应用配置。

`sw_vers`、`uname -m` 退出 0：macOS 27.0（26A428）、arm64。应用版本未验证：本轮未启动应用。Iced 由后续 UI 任务验收，本轮没有窗口/渲染变更；应用 live 切换、Windows/Linux、其他架构、PostgreSQL 实际服务未验收，不能从 Clippy 或 SQLite 推及这些场景。

定向差异与暂存检查退出 0，最终槽位 1 锁不存在、缓存保留。只提交本任务代码、独立测试、受影响的存储夹具和本条正式证据，排除原有任务状态/删除及其他会话变更；编号任务文件完成后删除且不纳入提交。未执行全量 cargo test、fmt check、wasm 或 Makefile。

## 任务 10：统一供应商记录维护

2026-11-06；状态：已完成。

### 依赖、来源与实现

批次 9 提交 `609f2770a` 及本文件任务 1–9 证据已核对；母任务 1、3 为已完成。母任务 2 完整领域迁移仍未收口，沿用任务 1 与 provider-contracts.md 记录的底座子计划 1–93 开工裁定，不将母任务 2 写成已完成。只读 plan.md 未修改。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 退出 0，来源干净。读取 database/dao/universal_providers.rs 的 get_all/get/save/delete、services/provider/mod.rs 的 list_universal/get_universal/upsert_universal/delete_universal，provider.rs 的 UniversalProvider/Apps/Models 和转换实现、相邻统一供应商图标校验测试；直接调用者 commands/provider.rs 的四个 CRUD command、UNIVERSAL_LOCK/universal_mutation、前端 universalProvidersApi 和 UniversalProviderFormModal 保存逻辑均已核对。

- `vw-agent-apps/src/providers/universal.rs::{save,summary}` 是记录校验与安全读取的业务实现。保存 Claude 主/Haiku/Sonnet/Opus 模型、Codex 模型/推理强度、Gemini 模型、应用启用标记和展示字段；不从内建预设自动补知识。config 是完整编辑配置，缺失的可选字段清空；内部未知根字段/meta 保留，不接受公共任意 JSON/meta 写入，不回传旧元数据中的秘密。
- DTO `vw-api-types/src/app_providers/universal.rs` 将 UniversalConfig、各应用模型与 UniversalProviderWrite/Summary 分开。apiKey 复用显式 Keep/Replace/Clear：新建只接受 Replace；已有记录 Keep 保留、Replace 校验并替换，凭据必填所以 Clear 明确拒绝；拒绝脱敏占位值。读响应只有 ID、强类型 config 与 configured，不包含 apiKey 或任意旧 meta；写请求 Debug 对凭据脱敏。非法身份、字段长度/控制字符、不安全端点、带凭据的 URL、路径/外链/SVG/损坏 PNG 图标均拒绝；内联 PNG 沿用完整解码边界，内建图标只接受惰性标识符。
- `vw-agent/src/app_providers/universal.rs` 是 AppProviderStore 的直接子模块，复用同一 GatewaySettingsStore/GlobalDatabase，在租户作用域 `app_providers.universal` 中每个 ID 一条记录；没有第二份数据库、旧 settings 整映射双写或迁移。新建用 insert_if_absent，更新用 compare_exchange，过期写入返回 409；列表按 ID，损坏记录返回 503。删除仅移除此保存的统一记录，返回实际是否存在；不会级联删除应用档案或修改当前/路由/live 配置。来源中生成子档案的同步/清理与应用投影本轮不迁入，任务 11 接应用投影。
- API `GET /v1/universal-providers`、`GET/PUT/DELETE /v1/universal-providers/{id}` 接现有 app-providers 分域 router，handler_router/bootstrap 沿用共享注册。沿用网关认证和 TenantScope 目录维护权限，PUT 限制 128 KiB 以容纳有界 PNG；无任意文件/命令入口，不授予主机 live-config 写权限。公司头只提供作用域，不证明公司成员身份。非法 400、缺失读取 404、竞争写入 409、损坏/不可用 503，错误不回显路径、载荷或底层错误链。
- SDK `universal_provider_save/get/delete`、`universal_providers_list` 发送强类型请求并使用既有租户/认证上下文；错误仅保留状态与固定文案，忽略服务端敏感正文和原始传输错误。业务/DTO/Gateway/SDK 独立 universal_tests.rs 已注册到直接父模块。UI 属任务 70，本轮没有 Iced 或 live 激活变更。

### 红/绿、平台与审查证据

红/绿阶段均用原子 mkdir 动态领取共享槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`；`CARGO_BUILD_BUILD_DIR` 同一绝对路径覆盖仓库 build-dir。owner 记录计划、任务、会话与持有 shell PID，EXIT/INT/TERM trap 负责自身记录/锁清理；全部子进程结束后释放，缓存保留。

两阶段先运行 `cargo fmt --all`，退出 0；默认 features 使用如下相同命令：

| 确切命令 | 红阶段退出码 / 有效结果 | 最终绿阶段退出码 / 有效结果 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_universal_ -- --nocapture` | 101；1 passed/1 failed：保存后 Haiku 字段为 null | 0；3 passed |
| `cargo test -p vw-api-types --lib cc04_universal_ -- --nocapture` | 0；1 passed | 0；1 passed |
| `cargo test -p vw-agent --lib cc04_universal_ -- --nocapture` | 101；1 passed/1 failed：SDK 保存没有可读取配置，API 返回 503 | 0；2 passed |
| `cargo test -p vw-gateway-client --lib cc04_universal_ -- --nocapture` | 0；1 passed | 0；1 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 不适用 | 0；全部 targets/features 通过 |

红阶段是可编译但未保存配置的业务实现，失败发生在真实字段/API 返回断言，不是缺类型或零测试。绿阶段共 **7** 项有效测试，无忽略；测试二进制来自上述槽位 debug/deps。Agent 链接有既有 `__eh_frame section too large` 提示，但测试退出 0，未改变链接设置。

业务临时 JSON 文件保存/重读证明 cc04_universal_roundtrip_preserves_app_fields；cc04_universal_read_hides_credentials 验证普通读取/序列化与 Debug 脱敏。第三项证明 Keep 新建、Clear、占位凭据、非法 URL/图标和损坏记录明确拒绝、输入原记录不变，并接受完整 PNG/惰性图标标识。DTO 拒绝租户、路径、current、meta、隐式字符串凭据和应用模型中夹带秘密。

Gateway 使用真实隔离 SQLite、正式 handler_router 与 Basic 认证中间件，证明未认证 PUT/DELETE 为 401、非法字段 400、缺失 404、损坏读取/列表 503，拒绝后原载荷不变，所有响应无测试秘密。SDK→本地回环 HTTP→正式 PUT/GET/list/DELETE handler→SQLite→重开数据库闭环，证明增改查删、Keep/Replace/Clear、三应用模型保留、未知内部元数据保留及跨 platform/company 隔离；同 ID 应用子档案与临时 native fixture 字节不变。

本地服务仅替代远端传输，没有开启 Basic（SDK 支持 Bearer），Basic 由独立正式中间件测试覆盖，不冒充真实外部部署/公司成员认证。SDK 独立 HTTP fixture 验证方法、路径、公司头、强类型请求体和 403/409 正文脱敏，仅替代远端响应。所有数据库/文件均为临时目录、凭据为 fixture，不读取或修改真实用户配置。

`sw_vers`、`uname -m` 均退出 0：macOS 27.0（26A428）、arm64。应用版本未验证，本轮没有启动实际应用。Iced、应用投影/live 激活、Windows/Linux、其他架构与 PostgreSQL 实际服务未验收；PostgreSQL 复用既有后端且参与 Clippy，不能推及运行验收。

定向 git diff --check、git diff --cached --check 退出 0；本任务代码/独立测试/正式证据已审查，plan.md 无差异。最终槽位 1 锁不存在、缓存保留。仅提交本任务变更，排除原有母任务状态、任务文件删除和桌面终端等其他会话变更；完成后删除任务 10 文件且不纳入提交。未执行全量 cargo test、fmt check、wasm 或 Makefile。

## 任务 11：统一供应商向应用投影

2026-11-06；状态：已完成（目录投影，live 未接通）。

### 依赖与实现范围

批次 10 提交 `cc0bc917d` 及本文件任务 1–10 证据已核对；母任务 1、3 已完成，母任务 2 完整领域迁移未收口，沿用任务 1 正式记录的底座子计划 1–93 开工裁定，不宣称母任务 2 已完成。只读 plan.md 不修改。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 均退出 0、来源干净。读取 `ProviderService::sync_universal_to_apps/project_universal_child_to_live`、相邻 `sync_universal_to_apps_preserves_child_metadata`，`UniversalProvider::to_claude_provider/to_codex_provider/to_gemini_provider` 和直接 command `sync_universal_provider`；同时读取 useTemplateValues.ts 的按占位路径合并/现读配置规则、ProviderForm 调用和相邻 ProviderFormSettingsConfigSnapshot 测试（只读，未运行来源测试）。

- 业务唯一投影入口 `vw-agent-apps/src/providers/universal_sync.rs::{validate_request,child_id,project}`。请求明确选择应用；仅处理所选应用，未选应用不创建、不更新、不删除；选择为空/重复直接拒绝。Claude/Codex/Gemini 必须同时在父记录启用；其余 AppId 返回逐项 unsupported_app，禁用返回 app_disabled，不执行来源的隐式删除。
- 生成 `universal-{app}-{id}` 子档案，使用已有 AppProviderWrite 的协议/凭据/模型字段，同时保存对应 settingsConfig；模型缺省值沿用冻结来源转换，不冒充动态模型知识。Claude 保留主/Haiku/Sonnet/Opus 映射，Gemini 保留模型；Codex 仅纯 origin 补 `/v1`，自定义前缀不补，推理强度保留。Codex 使用现有 toml 库按字段合并和序列化，保留应用独有 TOML 值、其它 provider、未知 custom 字段，避免字符串插值；目录内 TOML 格式/注释不承诺保留，原生文件补丁属于后续任务。
- 已有子档案保留 meta、排序、端点、未知根字段以及 settingsConfig/env/auth 独有键；统一管理的配置和展示字段更新。新档案继承父排序，已有排序不覆盖。不复制父任意元数据或授权字段。内部 universalProviderId 绑定匹配才允许更新，名字碰撞返回 target_conflict，不覆盖普通档案；目标身份过长明确失败。损坏配置返回 invalid_profile，不静默覆盖。
- `vw-agent/src/app_providers/universal_sync.rs` 为既有 AppProviderStore 的存储适配，仍用同租户 GatewaySettingsStore/GlobalDatabase 的 app_providers.records，不另建数据库。新档案 insert_if_absent，更新 compare_exchange；存储/竞争/配置失败为逐项强类型原因，失败不影响其他所选应用，每次操作使用读取到的父快照。
- DTO：`vw-api-types/src/app_providers/universal_sync.rs`。API `POST /v1/universal-providers/{id}/sync`，请求 `{ "apps": ["claude", "codex"] }`，响应数组逐项包含 appId/providerId/catalog/live；catalog 为 updated 或 failed + 固定 code，live 始终 not_attempted。部分失败仍 HTTP 200、必须检查逐项 catalog。输入 400、缺父档案 404、损坏父档案/存储不可用 503。请求不接受 tenant/path/activate 等字段，4 KiB body 上限。响应不包含凭据、原始配置或底层错误。
- handler 接既有 app-providers 分域 router，handler_router/bootstrap 复用既有注册与网关认证/TenantScope 目录维护权限；未授予主机文件或进程能力。公司头只选作用域，不等于公司成员认证。SDK `GatewayClient::universal_provider_sync` 复用统一记录的路径检查和脱敏传输，使用既有租户上下文。Iced 属后续任务，本轮未新增/验收 UI。

### 红绿验证与边界

红/绿阶段均通过原子 mkdir 动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同一绝对路径覆盖仓库 build-dir；owner 记录计划、任务、会话标识与 PID，EXIT/INT/TERM trap 清理自身锁，子进程全部结束后释放、缓存保留。

两阶段先运行 `cargo fmt --all`，退出 0；四个 crate 均使用默认 features：

| 确切命令 | 红阶段退出码 / 有效结果 | 绿阶段退出码 / 有效结果 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_universal_sync_ -- --nocapture` | 101；0 passed/2 failed | 0；2 passed |
| `cargo test -p vw-api-types --lib cc04_universal_sync_ -- --nocapture` | 0；1 passed | 0；1 passed |
| `cargo test -p vw-agent --lib cc04_universal_sync_ -- --nocapture` | 101；1 passed/1 failed | 0；2 passed |
| `cargo test -p vw-gateway-client --lib cc04_universal_sync_ -- --nocapture` | 0；1 passed | 0；1 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 不适用 | 0；全部 targets/features 通过 |

红阶段模块/测试已接线编译，投影尚返回 invalid_profile；业务生成子档案与禁用原因断言失败，Gateway 的 SDK→API 目录结果预期 updated 而得到 failed，均为业务断言失败，不是编译失败或零测试。绿阶段共 **6** 项有效测试、无忽略；业务测试在补充 Codex 原生形状断言后也验证保留 sandbox_mode 和其它 model_provider。测试二进制来自槽位 debug/deps。Agent 链接仍有既有 `__eh_frame section too large` 提示，测试退出 0，未改链接设置。

业务临时 JSON 文件与结构断言证明仅选择 Claude/Codex、不创建 Gemini；覆盖 Gemini 投影、Codex origin/前缀、Haiku、元数据/排序/端点及局部配置保留，禁用/碰撞/过长身份明确错误。DTO 拒绝隐式激活、任意路径、跨租户字段和未知应用，序列化明确 catalog/live 边界。

Gateway 使用真实隔离 SQLite 和正式 handler_router。SDK→本地回环 HTTP→正式 POST handler→SQLite→重开数据库验证落盘的 apiKey/元数据/排序/env 独有键；Kimi 不支持与 Codex 子档案损坏均逐项报告，其余应用继续持久化。未选 Gemini 没有记录，跨 company/platform 缺失父档案返回 404且没有子记录。父档案和临时 native fixture 字节保持不变，所有结果不含 fixture 凭据。正式 Basic 中间件测试未认证 401、空/重复选择与 activate 字段 400，拒绝后目录无写入。

本地 HTTP 仅替代远端传输，Basic 在独立正式中间件测试覆盖，SDK 回环服务未启用 Basic；SDK 独立 fixture 仅替代远端响应，证明 POST/路径/公司头/选择请求及 403 正文脱敏，不冒充部署授权验收。全部凭据、文件、数据库为临时 fixture，不接触真实用户配置。

`sw_vers`、`uname -m` 退出均为 0：macOS 27.0（26A428）、arm64。应用版本未验证，未启动实际 CLI/Iced；live 写入、激活、代理流量、Windows/Linux、其他架构、PostgreSQL 实际服务未验收。

定向 diff 检查与暂存检查退出 0；仅提交本任务实现、独立测试、父注册和本条证据，排除其他母任务状态/任务删除及桌面终端变更。只读 plan.md 无差异。最终槽位 1 锁不存在、缓存保留。未执行全量 cargo test、fmt check、wasm 或 Makefile；任务文件在完成报告后删除且不纳入提交。

## 任务 12：macOS 配置路径与覆盖

2026-11-06；状态：已完成（只读配置路径及访问状态）。

### 依赖与实现范围

核对批次 11 提交 `e13f452d7` 和本文件任务 1–11 的正式证据；母任务 1、3 已完成，母任务 2 沿用任务 1 的底座子计划 1–93 开工裁定，完整领域迁移仍未收口，不将其改报为完成。只读 plan.md 未修改。

来源仓库 HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 退出 0、来源干净。阅读 `commands/config.rs::get_config_status`、打开/选择目录直接调用者，`settings.rs::resolve_override_path/get_*_override_dir`、设置界面 useSettings/useDirectorySettings 引用与相邻 override 测试；`omp_config.rs::normalize_profile/native_agent_dir/data_dir/get_omp_models_path` 及相邻 native_profile_and_config_root_resolution、xdg_data_root_follows_existing_profile_and_agent_override、yaml_extension_follows_native_precedence 测试。同时核对正式契约的 14 应用默认路径和相关路径函数，未执行来源测试。

- 业务唯一解析入口 `vw-agent-apps/src/providers/paths.rs::read`；`execution_environment` 只捕获执行进程相关变量，HTTP 不接受 HOME/cwd/environment/profile/tenant。HOME 来自管理员 policy 的授权 host；不回退客户端路径，不创建目录或读取文件内容。返回 14 应用固定配置文件的路径与 present/absent/inaccessible，存在不等于配置有效、供应商激活或应用可运行。
- Claude 优先 settings.json、已有 claude.json 回退；OpenCode JSONC 优先、无文件默认 JSON；OMP models.yml/config.yml 优先于对应 yaml。Codex/Gemini/Pi 返回相关多文件状态；DSH 返回四个标准 profile 的只读状态，不将其当写入目标清单；Desktop 只列两个固定配置文件，configLibrary 的动态 profile/登录处理归应用任务。
- 显式 directoryOverride 只支持来源已有覆盖的九个应用，`~`/`~/`/`~\\` 根据执行主机 HOME 展开。覆盖只用于本次状态查询，不保存应用设置或改变 catalog/native reader、启动参数。绝对路径须位于授权 HOME；相对路径、其它用户 tilde、父目录穿越、越界环境目录、不可遍历/链接覆盖目录明确 400。管理员当前仅授权 HOME，HOME 外目录没有授权依据，直接拒绝。Hermes 环境目录 trim 后按原样解释、不展开 tilde；Pi 环境目录沿用 tilde 展开；MCode MINIMAX 优先于 MAVIS。
- OMP_PROFILE 优先于 PI_PROFILE（包括显式空值/default），名称沿用原生长度、字符与保留设备名校验；PI_CONFIG_DIR 是 HOME 下目录名；命名 profile 优先于 PI_CODING_AGENT_DIR；显式 default 忽略从 PI_PROFILE 继承的命名 agent 目录；自定义绝对 agent 路径保留。已有 XDG_DATA_HOME/omp[/profiles/name] 仅在使用默认 agent 目录时成为 dataDirectory。环境相对 cwd 路径及 HOME 外 XDG/agent 目录安全拒绝，不能借原生宽松路径语义越权。
- descriptor-relative openat + O_NOFOLLOW/O_DIRECTORY/O_NONBLOCK 遍历授权 HOME 下组件；读取 metadata 不读配置载荷，文件/目录类型不符、链接、权限错误均为 inaccessible，不伪装缺失。显式覆盖目录不可访问时拒绝，默认配置不可访问时返回状态。返回路径仅为观察结果，不能作为后续写授权或防竞争凭证；任务 17/应用适配仍须独立写权限与物理锁。
- DTO `vw-api-types/src/app_providers/paths.rs`；API `POST /v1/app-providers/{app}/config/paths`，4 KiB 上限，请求 `{targetHostId,directoryOverride?}`；SDK `GatewayClient::app_provider_paths` 走 native dynamic_host_client、租户上下文和敏感 token header。handler 在任何路径探测前复用独立 provider_config_read 授权：无 token 401、无 grant/跨租户 403、离线 503、remote 501，不扩大 preferences_write/launch 权限。输入/路径错误受控 400，SDK 不回显错误正文。两个正式路由根通过已有 app-providers router 自动注册；无新通用文件/命令 API。Iced 配置路径界面属于任务 75，本轮未验收 UI/live 写入。

### 红绿验证与边界

正式红阶段及最终验证阶段均原子 mkdir 动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`；`CARGO_BUILD_BUILD_DIR` 设置为同一绝对路径，覆盖仓库另设的 build-dir。owner 记录计划、任务、会话及持有进程 PID，阶段退出清理自身锁、保留缓存。首次接线发现 SDK AppId 缺少导入，已修正；首次探索运行仅设置 target-dir 导致仓库 build-dir 仍生效，未将该轮作为正式槽位证据，随后显式设置两者并完整重跑红测。

各阶段先 `cargo fmt --all`，退出 0；默认 features：

| 确切命令 | 正式红阶段退出码 / 有效结果 | 最终绿阶段退出码 / 有效结果 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_paths_ -- --nocapture` | 101；1 passed / 2 failed | 0；6 passed |
| `cargo test -p vw-api-types --lib cc04_paths_ -- --nocapture` | 0；1 passed | 0；1 passed |
| `cargo test -p vw-agent --lib cc04_paths_ -- --nocapture` | 101；0 passed / 1 failed | 0；2 passed |
| `cargo test -p vw-gateway-client --lib cc04_paths_ -- --nocapture` | 0；1 passed | 0；1 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 不适用 | 0；全部 targets/features 检查通过 |

红阶段业务解析函数显式返回 unavailable；覆盖路径/profile 测试在 unwrap 业务结果处失败，Gateway SDK 预期成功而得到 400，均非编译错误或零测试。绿阶段新增默认路径/环境/访问失败/XDG 等断言，总计 **10** 项有效测试、无忽略；macOS 行为测试按目标平台注册。第一次 Clippy 退出 101，只指出本次 XDG 分支的 collapsible_if，已合并后重新格式化、验证。测试二进制位于槽位 debug/deps；Agent 链接仍提示既有 `__eh_frame section too large`，测试退出 0，未调整链接配置。

SDK→真实回环 HTTP→正式 paths handler→管理员 policy HOME 下临时 models.yaml，返回正确绝对路径和 present；原始 fixture 内容不出现在响应，查询与拒绝后字节保持不变。缺少独立 grant、未知 host、跨 company 均 403，无效 token 401、离线 503、remote 501、HOME 外覆盖 400。另用正式 handler_router + Basic 中间件验证未认证 401。回环服务替代远端传输，独立 SDK fixture 替代远端返回、验证 POST/body/租户头与错误正文脱敏；这些不冒充实际远程主机运行验收。临时文件与 policy 使用假凭据，未读取真实用户配置、未使用钥匙串或外部应用进程。

`sw_vers`、`uname -m` 退出 0：macOS 27.0（26A428）、arm64。应用版本未验证，未启动实际 CLI/Iced；本条仅验收主机路径解析与只读元数据，live 激活、写权限、持久覆盖设置、动态 Desktop profile、Windows/Linux、其它架构和 UI 均未验收。后续应用任务须接通原生文件动作，不将本接口当作写成功证据。

定向 diff 检查及暂存检查均退出 0；仅提交本任务代码、独立测试、父模块注册及本条正式证据。现有其他任务状态/删除与桌面改动不纳入提交；只读 plan.md 无差异。最终锁已释放、缓存保留。未执行全量 cargo test、fmt check、wasm 或 Makefile。完成报告后删除任务 12 文件，不纳入提交，正式证据保留。

## 任务 13：JSON 与 JSONC 定点补丁

2026-11-06；状态：已完成（内存字节补丁原语，live 写入未接通）。

### 依赖、来源与范围

核对批次 12 提交 `11d1d83e5` 及本文件任务 1–12 正式证据；母任务 1、3 已完成，母任务 2 沿用任务 1 记录的底座子计划 1–93 开工裁定，完整领域迁移仍未收口，不将其改报为完成。只读 plan.md 未修改。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 退出 0、来源干净。阅读 `live/patch/json.rs::{JsonPatch::apply_to,LivePatch::apply,parse,serialize}` 及相邻测试，直接消费方 `live/project/claude.rs::direct_patch`、`live/project/gemini.rs::settings_patch`、`services/provider/claude_direct.rs::write/run`、`services/provider/gemini_editor.rs`；同时阅读 `opencode_config.rs::OpenCodeDocument::{load,save}/try_edit_config/set_provider_with_format/set_default_model`、`jsonc_document.rs` 文本编辑机制、直接调用者 `services/provider/live.rs` 与 `commands/provider.rs::set_opencode_default_model`，以及相邻配置/服务 JSONC、损坏文件、Unicode 注释测试。来源测试只读，未运行。

RustCodeGraph status/explore 退出 0，索引含 6443 文件；运行时代码和 Cargo 清单检索确认保注释编辑器仅存在冻结的 upstream 源码中，未连接原生 crate，依赖 JSON5/json-five。当前原生 crate 只有 serde_json，未新增依赖或接入 Tauri。业务唯一落点 `vw-agent-apps/src/providers/live/patch_json.rs::JsonPatch::apply`，通过父模块 `providers::live` 注册，独立测试 `live/patch_json_tests.rs`。

- 提供点名对象键的 set/remove/seed；set 胜过相同路径 remove，seed 只写缺失键，已有 null 保留，缺失对象祖先可创建；路径按字符串片段解释，支持中文、转义引号与 escaped key，不提供数组索引或根节点覆盖。
- 使用 serde_json 验证语义和嵌套限制，在临时副本屏蔽 JSONC 注释/BOM/尾逗号以取得同偏移文本范围；实际只改原文的目标值、删除目标成员及必要逗号、新增成员。未触及字段保留原始数字、Unicode 转义、键顺序、缩进、CRLF、BOM 和注释；语义相同目标保持原字节。被明确替换/删除的目标值内部原文属于修改范围，不承诺保留其内部注释。
- 支持标准 JSON 和带行/块注释、尾逗号的 JSONC；不扩展为 JSON5 标识符/单引号。拒绝非对象根、非对象祖先、重复解码键、非法 UTF-8、损坏/空现有文档、非法空路径及超限输入。文件缺失才从 `{}` 起步；输入/输出上限 1 MiB，路径最多 64 段。错误只有固定枚举，不回显配置或凭据。
- 全程在私有输出副本操作，返回前再次验证，不修改输入、不接触文件系统；任意操作失败不返回部分结果。调用方负责目标选择、权限、物理锁、快照与原子提交，本任务没有引入任意路径/文件 API。
- API/SDK/UI 无新增入口：当前原生调用链尚无可授权的 live 写入事务；本项只交付内部补丁原语，测试直接使用同一公开业务函数。后续事务与应用适配任务接入此所有者；目录创建/编辑、首次激活、实际 native 文件提交和 UI 未验收，不能将本项称为切换成功。

### 红绿验证与平台边界

所有阶段均通过原子 mkdir 动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`；`CARGO_BUILD_BUILD_DIR` 同一绝对路径覆盖仓库另设的 build-dir。owner 记录计划/任务/会话/PID，EXIT/INT/TERM trap 清理自己的 owner/锁；所有子进程结束后释放，缓存保留。测试二进制实际位于槽位 debug/deps。

| 确切命令 | 红阶段退出码 / 有效结果 | 最终绿阶段退出码 / 有效结果 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 0 |
| `cargo test -p vw-agent-apps --lib cc04_patch_json_ -- --nocapture` | 101；0 passed / 2 failed | 0；5 passed / 0 ignored |
| `cargo clippy --all-targets --all-features -- -D warnings` | 不适用 | 0；全部 targets/features 通过 |

红阶段模块/类型/测试接线可编译：补丁返回原字节时目标 token 更新断言失败、数组根被错误接受的业务拒绝断言失败，非编译失败或零测试。中间绿测 4 passed/1 failed，因删除单键的预期字符串误多写一个空格，修正 fixture 后 5 passed；第一次 Clippy 退出 101，发现本次 helper 多余引用，已修正并重跑格式化、全部 5 项聚焦及强制 Clippy，表格为最终结果。

5 项有效行为测试覆盖精确字节保留（中文/emoji 注释、EOF 注释、BOM/CRLF、数字与 Unicode 转义）、重复写入稳定、首/中/尾/唯一键删除、尾逗号、新增嵌套键与转义键、set/remove 优先级及 seed、损坏语法/形状/重复键/非法 UTF-8、批次后段失败。隔离临时文件实测解析失败前后内容不变；其余测试用真实内存文本和 serde_json 值，无 mock/网络/钥匙串/进程替代，无真实用户配置读取或写入。

`sw_vers`、`uname -m` 退出 0：macOS 27.0（26A428）、arm64。应用版本未验证，未启动实际 CLI/Iced；没有 SDK→API→原生文件链的成功声明。Windows/Linux、其他架构及实际应用读写运行未验收。

定向 diff 检查和暂存差异检查退出 0；仅提交本任务补丁、独立测试、两个父注册及本条证据。既有母任务状态/编号文件删除和桌面变更排除；只读 plan.md 无差异。最终实际检查槽位 1 锁已释放、缓存保留。未执行全量 cargo test、fmt check、wasm 或 Makefile。完成报告后删除任务 13 文件，不纳入提交，正式证据保留。

## 任务 14：TOML 定点补丁

2026-11-06；状态：已完成（内存字节补丁原语，live 写入未接通）。

### 依赖、来源与实现范围

核对批次 13 提交 `1a1376a99` 及任务 1–13 的正式证据；母任务 1、3 已完成，母任务 2 沿用任务 1 的底座子计划 1–93 开工裁定，完整领域迁移仍未收口，不将其改报为完成。只读 plan.md 未修改。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 退出 0、来源干净。读取 `live/patch/toml.rs::{TomlPatch::apply_to,parse,keep_layout,table_at_mut,ensure_table_mut}` 及其相邻 5 项测试；读取 `services/provider/editor_toml.rs::{insert_at,remove_at,TomlEdits::apply_to}`、直接消费者 Codex/Grok 的 direct/editor 及 Codex 编辑器相邻全局字段、三方冲突测试；来源测试未执行。RustCodeGraph status/explore 退出 0，索引 6446 文件，确认现有实现仅在冻结 upstream 中，没有接到原生运行包的同职责所有者。

- 业务唯一落点 `vw-agent-apps/src/providers/live/patch_toml.rs::TomlPatch::apply`；直接父模块注册及独立 `patch_toml_tests.rs`。新增直接依赖 `toml_edit = "0.22"`，复用锁文件已有 0.22.27 及其依赖，没有新增锁定包版本；现有 toml 反序列化库不能保留注释布局。
- 仅提供点名 set/remove/seed，路径为字面键片段，带点号键不拆分；set 胜过同路径 remove，seed 不覆盖已有值；缺失标准祖先创建隐式表，内联祖先内新增内联表，写入的标准表转换为内联值。支持点名整表替换/删除，不引入编辑器三方比较、业务 floor 谓词或步骤 trait。
- 原位替换值保留其空白/行尾注释，替换标准表沿用位置和表装饰；未修改的表、键写法、数字写法和注释保留。被点名替换/删除的条目内部内容属于修改范围，不承诺保留其内部注释。没有对 CRLF/BOM 或所有布局作额外保字节承诺。
- 私有解析文档上操作，错误不返回部分结果，不包含路径、配置载荷或凭据；拒绝非法 UTF-8/TOML、非表祖先、空/超过 64 段路径、无值 Item；输入/输出上限 1 MiB，输出再次解析。缺失文件从空文档开始，空文档是合法 TOML。数组可作为值整体替换，不提供数组索引导航。
- API/SDK/UI 无新增入口：沿用任务 13 已验收的纯内部补丁边界，当前原生事务写入链尚未交付，后续事务及应用适配消费本所有者。本项不增加任意路径/文件 API、不改写目录存储投影、不宣称供应商编辑、首次激活或 live 切换成功。

### 红绿验证与平台边界

红/绿阶段均通过原子 mkdir 动态领取槽位 **1**；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同一绝对路径覆盖仓库 build-dir。owner 记录计划/任务/会话/PID，EXIT/INT/TERM trap 清理自己的 owner 与锁；所有子进程结束后释放、缓存保留。测试二进制实际位于槽位 debug/deps。

| 确切命令 | 红阶段退出码 / 有效结果 | 最终绿阶段退出码 / 有效结果 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 0 |
| `cargo test -p vw-agent-apps --lib cc04_patch_toml_ -- --nocapture` | 101；0 passed / 4 failed | 0；4 passed / 0 ignored |
| `cargo clippy --all-targets --all-features -- -D warnings` | 不适用 | 0；全部 targets/features 通过 |

红阶段模块、类型及测试接线均可编译，补丁仅返回原字节时，目标更新、非法文档拒绝、新增子表及 remove/set 优先级均在业务断言失败，非编译失败或零测试。绿阶段同一过滤命令全部 4 项通过，使用默认 features。

4 项测试验证中文/emoji 注释、行尾注释、带点号引用键与 dotted key、内联表、未修改表及十六进制字面量的精确预期文本，重复应用字节稳定；新增普通/内联嵌套表、整表删除、set/remove 优先级、seed 不覆盖、损坏 TOML/UTF-8、非法路径、标量祖先及批次后段失败。隔离临时文件的非法文档解析前后字节不变；其余为真实内存 TOML，无 mock/网络/钥匙串/进程替代，不读取或修改真实用户配置。

`sw_vers`、`uname -m` 退出均为 0：macOS 27.0（26A428）、arm64。应用版本未验证，未启动实际 CLI/Iced；没有 SDK→API→原生文件写入成功证据，Windows/Linux、其他架构和实际应用运行未验收。

定向 diff 检查及暂存检查只涵盖本任务实现、独立测试、父注册、依赖清单/锁文件和本条证据；现有母任务状态/编号文件删除及其他会话变更排除。未运行全量 cargo test、fmt check、wasm 或 Makefile。完成后删除任务 14 文件，不纳入提交，正式证据保留。

本轮定向 `git diff --check` 退出 0；最终槽位 1 锁目录已不存在，缓存目录保留。只读 plan.md 无差异。

## 任务 15：YAML 供应商节点补丁

2026-11-06；状态：已完成（内存字节补丁原语，live 写入未接通）。

### 依赖、来源与实现范围

核对批次 14 提交 `6bab4e8cf` 及本文件任务 1–14 的正式证据；母任务 1、3 已完成，母任务 2 沿用任务 1 已记录的底座子计划 1–93 开工裁定，完整领域迁移尚未收口，不将其改报为完成。只读 plan.md 未修改。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 均退出 0、来源干净。读取 `omp_config.rs::{edit_provider,emit_provider,verify_edit,mapping_key,reject_yaml_tokens,has_yaml_reference_tokens,change_and_commit}`、相邻字节保留/不安全结构/普通标量/空节点测试和 `services/provider/omp.rs` 的 add/update 消费；读取 `mcode_config.rs::{write_entry,set_provider,add_provider,remove_provider}` 及相邻节点保留、锁/默认模型测试，确认原整文档序列化不能承诺字节保留。读取 `yamlNode.ts::{yamlNodeToText,yamlNodeFromText}` 和 PiProviderForm/McodeProviderForm 的节点编辑直接调用；没有找到独立 yamlNode 相邻测试。来源测试本轮未运行。RustCodeGraph status/explore 退出 0，索引 6448 文件，确认已有 OMP 编辑器仅在冻结 upstream，原生 providers/live 尚无 YAML 所有者。

- 唯一原生补丁落点 `vw-agent-apps/src/providers/live/patch_yaml.rs::patch_provider`，父模块注册及独立 `patch_yaml_tests.rs`。沿用 OMP 的块映射文本定位/拼接方式，通过强类型 ProviderSection 限定为 `providers` 或 `custom_provider`。输入为字节、字面供应商 ID、可选 JSON 对象；Some 替换/新增目标节点，None 删除目标节点，缺失目标删除保持原字节。供应商业务 schema、MCode 原生账号/默认模型保护仍由后续所属应用适配负责。
- 仅目标节点重新序列化，尾部空行/注释属于外围并保留；相邻供应商、根字段、中文/emoji 注释及 CRLF 外围字节保留。缺失 section 可追加，null section 仅规范化其头部，删除最后一家供应商保留空映射。目标内部注释和格式属于替换范围，新增目标使用 LF，不承诺重排或通用 YAML 格式化。
- 保留 OMP 显式拒绝边界：flow section、tab、重复键、非映射根/section、非字符串供应商键、多文档/结束标记及无法可靠定位的布局返回 ManualEdit；文档全局扫描锚点/别名/标签/merge 指示符，拒绝可能跨节点引用的结构。普通标量中的 `Search & Fetch * x` 等文本允许。错误不携带配置载荷；非法 UTF-8、空 ID/非对象替换、超过 1 MiB 的输入/输出拒绝，不返回部分结果。
- 输出重新解析，与完整预期文档比较，包含 section 外节点语义校验；不将解析成功代替字节保留断言。补强上游 mapping_key 对冒号的识别：仅尾部或后接空白的冒号是分隔符，避免 `target:key` 的重复编辑失败。
- 新增直接依赖 `serde_yaml = "0.9"`，复用 Cargo.lock 既有 `0.9.34+deprecated` 和其依赖；锁文件仅新增本 crate 的依赖边，没有新增包版本。此依赖只解析及发射目标节点。
- API/SDK/UI 没有新增入口：沿用任务 13/14 的内部字节原语边界，原生事务及 OMP/MCode 应用适配尚未交付，后续消费同一补丁所有者。本项未新增任意文件/命令 API，不宣布供应商编辑、首次激活或 live 切换成功。

### 红绿验证与平台边界

每阶段均原子 mkdir 动态领取共享槽位，owner 记录计划/任务/会话/PID，EXIT/INT/TERM trap 清理自身记录和锁，命令及子进程退出后释放、缓存保留。全部 Cargo 子进程同时继承 `CARGO_BUILD_BUILD_DIR=$CARGO_TARGET_DIR`，覆盖仓库 build-dir，测试二进制实际位于对应槽位 debug/deps。

| 阶段 | 实际槽位 | 绝对 CARGO_TARGET_DIR | 确切命令 | 退出码 / 有效结果 |
| --- | --- | --- | --- | --- |
| 红 | 2 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-2` | `cargo fmt --all` | 0 |
| 红 | 2 | 同上 | `cargo test -p vw-agent-apps --lib cc04_patch_yaml_ -- --nocapture` | 101；0 passed / 2 failed |
| 首次绿 | 1 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` | `cargo fmt --all` | 0 |
| 首次绿 | 1 | 同上 | `cargo test -p vw-agent-apps --lib cc04_patch_yaml_ -- --nocapture` | 101；2 passed / 2 failed，冒号键重复编辑定位失败 |
| 最终绿 | 1 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` | `cargo fmt --all` | 0 |
| 最终绿 | 1 | 同上 | `cargo test -p vw-agent-apps --lib cc04_patch_yaml_ -- --nocapture` | 0；4 passed / 0 ignored |
| 最终绿 | 1 | 同上 | `cargo clippy --all-targets --all-features -- -D warnings` | 0；全部 targets/features 通过，不计行为测试 |

红阶段模块/类型/测试均接线可编译，原样返回输入的占位实现分别在“目标节点未更新”和“不安全结构被接受”业务断言失败，非编译失败或零测试。修正冒号定位后，同一过滤命令的 4 项真实行为测试全部通过，采用默认 features。

4 项覆盖 OMP/MCode 两种 section、目标替换/增删、中文/emoji、不同缩进、带冒号/点号/引号的字面 ID、外围逐字节保留、重复应用稳定、CRLF 外围、null/空 section/最后移除、缺失目标无副作用，以及 17 种损坏/不安全结构、非对象替换、UTF-8/输入输出大小限制。拒绝测试读取隔离临时文件并检查原文件字节未变，其余使用真实内存 YAML；没有 mock/网络/钥匙串/不可控进程替代，不读取或改写真实用户配置。

`sw_vers`、`uname -m` 均退出 0：macOS 27.0（26A428）、arm64。应用版本、个人账号、CLI/Iced 实际运行未验证，没有 SDK→API→执行端文件写入证据；Windows/Linux、其他架构与实机切换未验收。

定向差异与暂存检查仅涵盖本任务代码、独立测试、父注册、依赖清单/锁文件及本条证据，其他会话的终端改动、母任务状态和既有任务文件删除均排除。只读 plan.md 无差异；最终阶段自身槽位 1 锁目录已释放，缓存保留。不执行全量 cargo test、fmt check、wasm 或 Makefile。任务 15 完成后删除编号文件且不纳入提交，正式证据保留。

## 任务 16：dotenv 定点补丁

2026-11-07；状态：已完成（内存字节补丁原语，live 写入未接通）。

### 依赖、来源与实现范围

核对批次 15 提交 `87e51035c` 及本文件任务 1–15 正式证据；母任务 1、3 已完成，母任务 2 沿用任务 1 的底座子计划 1–93 开工裁定，完整领域迁移仍未收口，不将其改报完成。只读 plan.md 未修改。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 退出 0、来源干净。读取 `live/patch/dotenv.rs::{DotenvPatch::apply,entries,parse_key,value_of,unquote}` 及四项相邻测试；读取 `services/provider/gemini_editor.rs::{view,env_changes,EnvWrite::apply,plan_save,write_live}`、三方冲突相邻测试及 provider/mod.rs 编辑器读写调用，核对 `gemini_direct::run_with_edits`、`GeminiProjection::env_patch` 直接消费。来源测试本轮未运行。RustCodeGraph status/explore 退出 0，确认原生 live 目录已有 JSON/TOML/YAML 原语，dotenv 原所有者仍位于冻结 upstream。

- 新增唯一原生所有者 `crates/vw-agent-apps/src/providers/live/patch_dotenv.rs::{DotenvPatch::apply,entries}`，父注册和独立 `patch_dotenv_tests.rs`。延续任务 13–15 内存补丁边界，不执行文件读写。保留来源 set/remove/remove_if/clear 操作；set 优先于删除，第一处原位替换并保留 export 前缀，同名重复行删除，缺失键追加。clear 仅接受执行端函数指针，不新增客户端谓词或配置接口。
- 非目标变量、注释、空行、无法识别的行和顺序逐字节保留，额外保留每行各自 LF/CRLF，混合换行不被整体规范化。追加分隔符沿用来源 CRLF 优先与末尾换行规则。目标整行按来源规则重写，不承诺保留目标行自身的缩进、空格或内联注释。
- 值为原始 dotenv 字面文本，不自动加引号、不做 shell 求值或变量展开；调用者提供需要的引号。entries 保留引号、重复键取最后值但保持首次位置；remove_if 只剥离首尾配对单双引号后比较，不将此接口冒充完整 shell/dotenv 解析器。
- 拒绝非法 UTF-8、NUL、空/非法点名键、set/remove_if 值中的 CR/LF/NUL 和超过 1 MiB 的输入/输出。载荷不进入错误；失败不返回部分结果。沿用来源 ASCII 字母数字/下划线键规则。
- API/SDK/UI 未新增入口：原生事务与 Gemini 适配尚待后续任务消费此原语；不新增任意文件/命令 API，不宣称供应商编辑、首次激活或 live 切换已成功。

### 红绿验证与平台边界

每阶段原子 mkdir 动态领取槽位 2，owner 记录计划/任务/会话/PID，EXIT/INT/TERM trap 在全部子进程结束后清理自身记录和锁、保留缓存。所有 Cargo 同时继承 `CARGO_BUILD_BUILD_DIR=$CARGO_TARGET_DIR`，覆盖仓库 build-dir。

全部阶段实际 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，默认 features；测试二进制位于该槽位 debug/deps。

| 阶段 | 确切命令 | 退出码 / 有效结果 |
| --- | --- | --- |
| 红 | `cargo fmt --all` | 0 |
| 红 | `cargo test -p vw-agent-apps --lib cc04_patch_dotenv_ -- --nocapture` | 101；0 passed / 2 failed |
| 首次绿 | `cargo fmt --all` | 0 |
| 首次绿 | `cargo test -p vw-agent-apps --lib cc04_patch_dotenv_ -- --nocapture` | 0；4 passed / 0 ignored |
| 首次绿 | `cargo clippy --all-targets --all-features -- -D warnings` | 101；本次代码 collapsible_if 一处，已修复 |
| 最终绿 | `cargo fmt --all` | 0 |
| 最终绿 | `cargo test -p vw-agent-apps --lib cc04_patch_dotenv_ -- --nocapture` | 0；4 passed / 0 ignored |
| 最终绿 | `cargo clippy --all-targets --all-features -- -D warnings` | 0；全部 targets/features 通过，不计行为测试 |

红阶段类型/模块/测试可编译，原样返回输入的占位实现分别在“目标值未改动”和“非法输入被接受”业务断言失败，不是编译失败或零测试。最终四项行为测试覆盖目标替换、删除、条件删除、重复行、追加、set 优先、clear、原始引号/中文/emoji/混合换行保留、重复应用稳定、非法输入与大小限制。拒绝测试使用隔离临时文件并检查文件未变；其余使用真实内存文本。没有网络/钥匙串/进程 mock，不读写真实用户配置。

`sw_vers` / `uname -m` 退出 0：macOS 27.0（26A428）、arm64。应用版本、真实账号、CLI/Iced 运行及 SDK→API→执行端写入未验证；Windows/Linux 与其他架构未验收。

定向差异及暂存检查退出 0，只提交本任务实现、独立测试、父注册及本条正式证据；既有终端改动、母任务状态、其他编号任务删除不纳入提交。所有阶段自身槽位 2 锁已释放、缓存保留。未执行全量 cargo test、fmt check、wasm 或 Makefile。完成后删除任务 16 文件且不纳入提交，正式证据保留。

## 任务 17：应用写锁与私有文件权限

2026-11-07；状态：已完成（内部原语边界，原生事务写入口尚未接通）。

### 依赖、范围与来源

核对批次 16 提交 `24076e1e2` 及本文件任务 1–16 证据。母任务 1、3 已完成，母任务 2 沿用任务 1 的底座子计划 1–93 开工裁定，完整领域迁移仍未收口。首次因不存在原生写调用者而登记阻塞；继续执行指令明确按任务 13–16 的内部原语边界交付，解除该阻塞，不把尚不存在的请求链当作已接通。只读 plan.md 无修改。

来源 HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，工作区干净；读取 `live/engine.rs::{lock_app,AppWriteGuard,stage}` 和相邻并发/0600 测试，`config.rs::{atomic_write_private,stage_write,StagedWrite::commit}` 及原子替换相邻测试；读取直接消费者 `mode/operation.rs::{settle,AppWrite::begin}`、`claude_desktop_config.rs::write_desktop_files`。来源测试未运行。RustCodeGraph status/explore 退出 0，6452 文件；核对原生 providers/live、创建/更新、配置路径路由和 read grant，没有可复用的原生供应商文件提交所有者。安装生命周期锁、账号 lease 与启动临时文件有各自职责，本任务未迁移或复制这些业务。

- 原生唯一落点 `crates/vw-agent-apps/src/providers/live/write_guard.rs::{lock_app,AppWriteGuard::write}`，独立 `write_guard_tests.rs`，仅直接父模块注册；无新依赖。模块仅 macOS 编译，不冒充其他平台支持。
- 两类键在同一进程级静态表中原子取得：实际执行进程的 `AppId` 和打开目录的 `(dev, ino)`。不接受租户或客户端 host ID 作为隔离键，因此同一应用的不同目录、不同应用共享的同一物理目录均串行，路径别名也不能绕过资源锁；不同应用且不同目录可以继续。进程本身是本地执行主机边界，不声称跨进程/远程执行/外部应用写入参与该锁。
- 锁不可重入，阻塞调用应放在 blocking worker；RAII 在正常返回、错误和 panic unwind 时释放。调用者须在锁前完成执行主机与目录授权，并在读取、预演、提交全过程持锁。目录必须已存在，末级 symlink 拒绝；父路径授权/安全遍历仍归执行端入口，原语不是权限凭证。
- 写方法要求 `&mut AppWriteGuard`，防止共享同一 guard 的引用并发调用写入。持有目录描述符，使用 `fstatat(AT_SYMLINK_NOFOLLOW)`、`openat(O_EXCL|O_NOFOLLOW|O_CLOEXEC)`、`renameat` 和 `unlinkat` 定位直接子文件，不创建目录，不接受绝对/多段/空/NUL 文件名，不跟随目标 symlink，拒绝目录等非普通目标。目录路径被改名/替换后仍写原已打开目录。
- 同目录独占临时文件初始权限 0600，写入、设置权限并 fsync 成功才 rename。私有文件新增/替换均准确 0600；共享文件保留既有 rwx mode（不保留 setuid/setgid/sticky），缺失时从 0600 起步，不扩大权限；共享文件原权限只读也不扩大。固定错误枚举不携带路径、输入或凭据。失败准备/发布不提交目标，并尝试清理自己的暂存文件；不承诺崩溃恢复、目录 fsync、多文件原子性、ACL/扩展属性保留或外部进程竞争检测。
- 无公共 API/SDK/UI 新行为，故“对外行为附 SDK→API→执行端”不适用。本项仅供 18–21 及应用适配消费；未来事务入口必须使用同一锁并持有完整事务，不能绕过锁直接写文件。真实请求、目录写授权、编辑/首次激活、pending/恢复和全链验收归相应任务及 76；本项不宣称 live 切换成功。

### 红绿与平台证据

红阶段、首次绿及修正可变参数后的绿阶段通过原子 mkdir 动态领取共享槽位 **2**；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，`CARGO_BUILD_BUILD_DIR` 同值覆盖仓库 build-dir。owner 记录计划/任务/会话/PID，EXIT/INT/TERM trap 在所有 Cargo/子进程结束后清理自身记录和锁，缓存保留。测试二进制实际在槽位 debug/deps，默认 features。

| 阶段 | 确切命令 | 退出码 / 有效结果 |
| --- | --- | --- |
| 红 | `cargo fmt --all` | 0 |
| 红 | `cargo test -p vw-agent-apps --lib cc04_write_guard_ -- --nocapture` | 101；0 passed / 2 failed |
| 首次绿 | `cargo fmt --all` | 0 |
| 首次绿 | `cargo test -p vw-agent-apps --lib cc04_write_guard_ -- --nocapture` | 101；本次 openat 可变参数用了 u16 mode_t，macOS 要求整数提升；无有效测试 |
| 首次绿 | `cargo clippy --all-targets --all-features -- -D warnings` | 101；同一编译错误，已改为 c_uint |
| 修正可变参数后 | `cargo fmt --all` | 0 |
| 修正可变参数后 | `cargo test -p vw-agent-apps --lib cc04_write_guard_ -- --nocapture` | 0；5 passed / 0 ignored |
| 修正可变参数后 | `cargo clippy --all-targets --all-features -- -D warnings` | 0；全部 targets/features |

最终审查将写方法收紧为可变持有 guard，并补充成功写入无暂存残留断言；因此重新动态领取槽位 **1**，最终 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同值。以下为最终代码验证，不沿用上次结果：

| 最终确切命令 | 退出码 / 有效结果 |
| --- | --- |
| `cargo fmt --all` | 0 |
| `cargo test -p vw-agent-apps --lib cc04_write_guard_ -- --nocapture` | 0；5 passed / 0 ignored |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0；全部 targets/features 通过，不计行为测试 |

红阶段两项可编译的业务断言失败：第二并发调用未等待、已有私有文件仍是 0644；不是缺模块/类型或零测试。最终 5 项真实行为测试覆盖两个线程同应用/同目录的顺序写入与最终字节、跨 app 的目录物理归属、同 app 跨目录锁、无关 app/资源可继续、目录别名、目录路径替换后的描述符锚定、panic unwind 后释放、新增/替换私有文件 0600、共享文件 0400/0600/0640/0644 保留和新文件 0600、非法文件名/穿越/NUL/symlink/非普通目标拒绝、拒绝后原文件/外部文件/目录条目不变及无暂存残留。unwind 测试中的 panic 被 catch_unwind 捕获，日志出现预期 panic 不代表测试失败。测试使用真实隔离临时目录、字节和 Unix metadata，没有网络/钥匙串/进程 mock，不读取或写入真实用户配置。

`sw_vers`、`uname -m` 均退出 0：macOS 27.0（26A428）、arm64；应用版本、个人账号、CLI/Iced 运行、SDK/API 全链未验证；Windows/Linux、其他架构和跨进程写锁未验收。原生请求链仍需后续任务证明。

定向 diff 检查及暂存检查退出 0，仅提交本任务实现、独立测试、父注册及本条证据；母任务状态、其他编号文件删除、桌面及其他会话改动排除。只读 plan.md 无差异。各阶段自身槽位锁均已释放、缓存保留；未运行全量 cargo test、fmt check、wasm 或 Makefile。任务状态标记已完成后删除编号文件，不纳入提交，正式证据保留。

## 任务 18：多文件预演与暂存

2026-11-07；状态：已完成（内部原语边界，公共事务请求链未接通）。

### 依赖、来源与连线

批次 17 提交 `e1046e3f2` 与任务 1–17 正式证据已核对。母任务 1、3 已完成；母任务 2 沿用任务 1 的底座子计划 1–93 开工裁定，完整领域迁移未收口。最初因单文件写无法分离暂存、无原生事务调用者而登记阻塞；继续执行指令明确授权按内部原语交付并最小调整 write_guard 所有者，本轮按该裁定解除阻塞。不修改只读 plan.md。

来源 HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，工作区干净。读取 `live/engine.rs::{plan,plan_from,stage,ensure_first_write_backup}`、`mode/operation.rs::{run,AppWrite::begin,AppWrite::run}`、直接消费者 `services/provider/claude_direct.rs::run`、`config.rs::{stage_write,StagedWrite::commit}` 及相邻预演、无操作、暂存失败/崩溃和正常发布测试。来源测试未运行。RustCodeGraph status/explore 退出 0（6454 文件）；确认原生锁当前仅有独立测试调用，没有把冻结 upstream 当成可执行原生请求链。

- 新增 `providers/live/stage.rs::{FileChange,StagedBatch,AppWriteGuard::stage}`、独立 `stage_tests.rs` 与直接父模块注册，仅 macOS 编译。FileChange 使用执行端纯解析/补丁闭包，不接受客户端文件操作。全部文件先读取和应用补丁、验证输出大小，精确字节相同的文件排除；然后逐一 fsync 暂存，全部成功才返回句柄。重复目标先拒绝。单目录内多文件适用，跨目录事务尚未交付；这里只准备写入，不新增删除协议。
- 最小调整 `write_guard.rs`：原有 openat/O_EXCL/O_NOFOLLOW、权限、fsync 与 renameat 仍只有一个所有者；拆出内部 `PreparedWrite`，已有 `write` 改为同一 stage_one + commit。新增描述符锚定读取，O_NONBLOCK 防止特殊文件阻塞，拒绝 symlink、非常规文件、路径穿越/NUL 和超过 1 MiB 的输入/输出。保留临时命名空间 `.shiju-live-`，拒绝作为目标，避免批次目标与暂存名交叉。错误不携带路径、字节或凭据，无新依赖。
- StagedBatch 的生命周期保持对 guard 的独占借用；准备失败或正常丢弃均 RAII 清理已创建暂存文件，不修改目标。noop 不写文件、不规范化权限，保留原始字节、inode、mode 和 mtime。私有写入仍为 0600，共享写入保留原权限。原生指针没有写入者，本原语完全不接触状态；测试中旁置 pointer 文件只证明未触碰，不能冒充真实指针事务验收。
- 显式 publish 复用底层逐文件原子替换，调用方须在发布前持久化 pending；本任务不实现 pending、崩溃恢复、发布冲突重算或多文件原子回滚。发布过程中失败的持久恢复仍需任务 19–21 扩展消费，此处仅保证发布前的准备边界。外部进程不参与本进程锁，补丁闭包必须是执行端纯逻辑；测试只在明确的外部竞争场景注入文件系统改变。
- 首写备份本轮不执行、不在目标目录留备份。后续事务必须从已授权执行主机的私有数据根解析 `backups/live-first-write`，禁止客户端路径/HOME 回退；沿用底座 `CredentialFileStore` 的执行端根来源与私有权限模式（目录 0700、文件 0600），不借用其账号命名空间或复制凭据所有者。该后续契约不等于当前已有供应商 DeviceStore/pending；事务接线时须提供实际私有根及权限验证证据，并复用本写入所有者。
- 无新增 API/SDK/UI 行为，不另造任意路径/命令接口。本项不声称供应商编辑、首次激活或 live 切换成功；真实请求与应用适配全链归后续任务。

### 红绿与平台证据

红、绿阶段均通过原子 mkdir 动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，同时设置同值 `CARGO_BUILD_BUILD_DIR` 覆盖仓库 build-dir。owner 记录计划、任务、会话和 PID；shell EXIT/INT/TERM trap 在全部命令和子进程结束后清理自身锁，缓存保留。测试二进制实际从该槽位 debug/deps 运行，使用默认 features。

| 阶段 | 确切命令 | 退出码 / 有效结果 |
| --- | --- | --- |
| 红 | `cargo fmt --all` | 0 |
| 红 | `cargo test -p vw-agent-apps --lib cc04_stage_ -- --nocapture` | 101；0 passed / 2 failed |
| 绿 | `cargo fmt --all` | 0 |
| 绿 | `cargo test -p vw-agent-apps --lib cc04_stage_ -- --nocapture` | 0；5 passed / 0 ignored |
| 绿 | `cargo test -p vw-agent-apps --lib cc04_write_guard_ -- --nocapture` | 0；5 passed / 0 ignored |
| 绿 | `cargo clippy --all-targets --all-features -- -D warnings` | 0；全部 targets/features 通过，不计行为测试 |

红阶段模块和类型可编译，使用会立即发布每个文件的基线实现：第二文件失败后第一文件已被替换，noop 的 inode/mode/mtime 发生变化，两项均在业务断言失败，非编译失败或零测试。红阶段一处测试 unused import 已在新增后续测试时消除。

5 项暂存行为覆盖解析失败不发布、真实后续暂存失败清理第一份暂存、全部成功前目标不变/新文件不存在、显式发布、丢弃清理、字节 noop 的 metadata 保留、私有/共享权限、symlink/非法目标/重复目标/超大输出拒绝。真实 staging 失败通过在第二文件读取后创建同名目录模拟外部竞争，保留该外部目录，断言其他目标不变且无暂存残留；解析使用真实 serde_json。5 项既有写锁回归继续证明串行、物理目录归属、目录描述符锚定、unwind 释放与私有权限；其中预期 panic 被 catch_unwind 捕获，全部通过。测试使用隔离真实临时目录，没有网络、钥匙串或不可控进程 mock，不读取或写入真实用户配置。

`sw_vers`、`uname -m` 退出 0：macOS 27.0（26A428）、arm64。应用版本、个人账号、CLI/Iced 运行与 SDK→API→执行端全链未验证；Windows/Linux、其他架构、跨进程锁和跨目录批次未验收。

定向差异和暂存检查退出 0，只提交本任务 stage 实现、独立测试、write_guard 必要底层连线、父注册及本条证据。母任务状态、其他编号文件删除和其他会话桌面改动均排除。只读 plan.md 无差异；红绿阶段自身槽位 1 锁已释放、缓存保留。未执行全量 cargo test、fmt check、wasm 或 Makefile；任务 18 标记完成后删除，不纳入提交，正式证据保留。

## 任务 19：pending 提交与状态落定

2026-11-07；状态：已完成（授权的内部事务边界，公共请求与应用适配尚未接通）。

### 依赖、来源与边界裁定

已核对批次 18 提交 `5c88e0e0f` 及本文件任务 1–18 的验收证据。母任务 1、3 已完成；母任务 2 沿用任务 1 的底座子计划 1–93 开工裁定，完整领域迁移仍未收口。本任务首次因缺原生主机状态所有者和调用者登记阻塞；继续指令明确授权本任务建立唯一供应商主机事务状态所有者、按内部事务边界验证，解除阻塞。不修改只读 plan.md，不提前实现应用适配、公共任意文件 API 或任务 20 的恢复算法。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，来源工作区干净。已读 `mode/operation.rs::{run,AppWrite::begin,AppWrite::run,commit_target}`、`mode/state.rs::{Pending,PendingFile,PendingTarget,set_pending,update}`、直接消费者 `services/provider/claude_direct.rs` 和相邻 clean-run/intent/crash/target 测试。来源测试未运行。RustCodeGraph status/explore 与原生 src 定向搜索确认此前仅有 write_guard/stage，没有供应商 pending 或指针/模式持久化所有者，冻结 upstream 不是原生运行链。

### 实现与实际连线

- 唯一所有者为 `vw-agent-apps/src/providers/live/commit.rs::ProviderTransactions`，macOS 条件下经现有 live 父模块注册。初始化工厂 `new(private_root, ExecutionHostId)` 只接受显式绝对、当前执行用户持有且无组/其他权限的现有私有目录；末级 symlink 拒绝。根来源沿用实际执行主机 `Host.credential_root`（`gateway/api/handlers/agent_apps/access.rs`）与 `CredentialFileStore::new` 的受信初始化契约，没有新增配置、HOME/客户端路径回退或凭据账号命名空间复用。该内部工厂尚无公共 handler 调用者，不宣称已把真实请求接到工厂。
- 在受信根下以 mkdirat/openat(O_DIRECTORY|O_NOFOLLOW) 打开 `provider-live/<base64url(host)>`，目录 0700 且检查 uid；之后所有操作锚定保留描述符。按强类型 AppId 生成文件名，每个主机/应用只有一份 pending 和状态文件，不按租户另建物理状态。ExecutionHostId 显式校验；TenantScope 随 pending 和已落定指针/模式归属持久化，避免把同名租户供应商误作另一租户。原生租户档案 AppProviderStore/GlobalDatabase 不承担主机状态，没有复制数据库。
- `commit(tenant, &mut AppWriteGuard, changes, target)` 是真实内部事务入口：复用现有物理应用/目录锁进行读取、补丁、全部暂存；新增 per-app flock 串行同一主机状态根的提交，文件描述符 Drop 释放。只要有未完成 pending，任何租户的新提交均拒绝，留给任务 20 恢复，不会覆盖旧意图。状态损坏、symlink、非法身份和不安全根均明确报错，不当成缺省状态。
- `stage.rs` 在真实预演时捕获修改前/后的 SHA-256（base64url，无 padding），不重复读取或记录原始配置载荷。pending 包含版本、主机、应用、租户、目录 dev/ino、目标名、暂存名、前后摘要及目标状态。复用 write_guard 的描述符读取/准备/rename 唯一所有者，最小提取内部 `read_from/prepare`，没有另写一套文件替换逻辑；已有 write/stage 仍走同一底层。
- 顺序为：fsync 全部暂存和所在目录 → 原子私有写 pending(published=false) 并同步状态目录 → 原子写 published=true 并同步目录 → 逐文件 rename/fsync 目录 → 原子写指针/模式并同步状态目录 → unlink pending/fsync。published 在第一个 rename 之前持久化。无实际文件变更时沿用来源的直接落定状态路径，不制造一次文件发布。
- 指针持有租户与供应商 ID；模式整体替换 mode/attached/proxy_route/contract 及其租户归属。缺省部分保持原值，代理路由独立于直连指针。重复提交同一目标得到相同持久状态。新主机状态文件是当前唯一供应商状态，不镜像不存在的代理配置/数据库旧列，不冒充代理运行成功。
- pending 持久化成功后，暂存所有权转交持久事务；错误及 unwind 不删除未发布暂存，部分发布失败保留 pending/剩余暂存，状态失败保留 published pending；状态已落定而清理前中断也保留意图。恢复、外部写冲突重算、备份、跨目录适配和跨文件系统原子回滚不在本项实现范围。flock 仅保护此状态根，不能声称跨进程客户端配置锁已完整交付。
- 无新增 API/SDK/UI，故 SDK→API→执行端公共请求证据不适用；后续应用适配和最终交叉验收负责真实编辑、首次激活与 live 切换。本项只证明内部入口对真实临时目录、状态和 stage 的行为，不以纯 trait/闭包 mock 代替存储落定。新增 serde derive 使用仓库已有版本，仅增加直接依赖登记，没有重型依赖。

### 红绿、槽位与平台证据

每一阶段均原子 mkdir 从 1–10 动态领取槽位，记录任务/计划/会话/PID；设置绝对 CARGO_TARGET_DIR，并以同值 CARGO_BUILD_BUILD_DIR 覆盖仓库 build-dir。各阶段全部 Cargo/子进程退出后清理自身 owner/锁，保留缓存。默认 features，无额外开关；测试二进制位于对应槽位 debug/deps。

- 首次接线阶段槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`：`cargo fmt --all` 退出 0；聚焦测试退出 101，AppId 无 as_str 和 sha2 0.11 摘要不实现 LowerHex，未运行有效测试。修正为既有 serde 枚举值和 base64url 摘要。
- 下一接线阶段槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`：格式化 0；聚焦测试 101，两个测试因临时根未设 0700 而在 fixture 初始化失败，不计业务红证据。修正隔离 fixture 权限，不削弱生产检查。
- 有效红阶段动态领取槽位 **1**，同一绝对路径：格式化 0；聚焦测试 101，0 passed / 2 failed。基线可以编译，但在 `durable intent must exist before publication` 和 `failed settlement retains durable intent` 两个业务断言失败；不是缺类型、零测试或初始化失败。
- 首次绿阶段槽位 **3**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`：格式化 0；事务 6/6、暂存 5/5、写锁 5/5 通过；强制 Clippy 101，只报本次测试钩子类型过于复杂。已提取 cfg(test) 类型别名，并补齐暂存目录 fsync，然后重新验证最终代码。

最终阶段实际槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，`CARGO_BUILD_BUILD_DIR` 同值。

| 最终确切命令 | 退出码 / 有效结果 |
| --- | --- |
| `cargo fmt --all` | 0 |
| `cargo test -p vw-agent-apps --lib cc04_commit_ -- --nocapture` | 0；6 passed / 0 ignored |
| `cargo test -p vw-agent-apps --lib cc04_stage_ -- --nocapture` | 0；5 passed / 0 ignored |
| `cargo test -p vw-agent-apps --lib cc04_write_guard_ -- --nocapture` | 0；5 passed / 0 ignored |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0；全部 targets/features 通过，不计行为测试 |

6 项事务测试覆盖首次发布前真实磁盘 pending/标记、目标文件仍旧值、发布后状态持久化并清理、真实状态路径被外部目录占据导致落定失败而保留 pending(0600)、意图/标记/落定后中断的真实持久状态、剩余暂存字节与已发布文件、同租户/跨租户 pending 拒绝、部分发布失败保留第二文件暂存、重复/noop 落定、指针/代理路由独立和应用/主机隔离、非私有/相对/符号链接根及非法 ID 拒绝。中断由 cfg(test) 阶段钩子返回固定错误模拟进程边界；全部文件发布、目录冲突、状态存储及读取均为真实文件系统，无通用业务 mock，无网络/钥匙串替代，不触碰真实用户配置。写锁回归中的 panic 由既有 catch_unwind 捕获，属于预期验证，5 项全部通过。

`sw_vers`、`uname -m` 退出 0：macOS 27.0（26A428）、arm64。应用版本、个人账号、CLI/Iced 实际运行与公共 SDK/API 请求链未验证；Windows/Linux、其他架构、真实应用激活、跨进程配置写入和崩溃后自动恢复未验收。

最终定向 diff 检查与暂存检查通过；只提交本任务 commit/独立测试、必要 stage/write_guard 底层连线、父注册、Cargo 依赖/锁文件及本条正式证据。排除其他任务状态/删除和桌面并行改动。只读 plan.md 无差异；最终自身槽位锁释放、缓存保留。未执行全量 cargo test、fmt check、wasm 或 Makefile。任务19状态标记完成后删除编号文件，不纳入提交；正式证据保留。

## 任务 20：崩溃后前滚恢复

### 依赖与范围

核对批次 19 `1132125d7` 与任务 1–19 正式证据；母任务 1、3 已完成，母任务 2 继续沿用任务 1 的底座子计划 1–93 开工裁定，完整领域迁移仍未收口。首次因缺原生启动消费链及资源重开契约登记阻塞；继续执行指令明确授权最小受信资源登记、执行端工厂与初始化调用者，按此解除阻塞。只读 plan.md 不修改。

来源 HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`；已读 `mode/operation.rs::{recover,recover_all,recover_on_startup,recover_before_write,AppWrite::begin,run}`、`mode/state.rs::{pending,apps_with_pending,set_pending}`、`services/provider/claude_direct.rs` 的写前入口及 `lib.rs` 的启动入口和相邻 crash/marked/published/target 测试。来源测试未运行。RustCodeGraph status/explore 与原生定向搜索确认现有唯一 pending 所有者为 ProviderTransactions，冻结 upstream 恢复链不作为原生消费证据。

### 原生所有者与自动入口

- `vw-agent-apps/src/providers/live/recovery.rs` 扩展同一 `ProviderTransactions`，未发布 pending 丢弃并清理暂存；published=true 时按 pre/planned 摘要完成剩余暂存，外部变化、目标不可读或暂存缺失/摘要不符均保留当前目标，返回 `RolledForwardExcept { names }`；状态仍按原 pending 租户落定。已完成文件不重写，状态/目录同步失败保留 pending，重试可幂等完成。没有第二份指针数据库。
- 受信资源登记 `*.resource.json` 与 pending/state 同处主机私有状态目录，沿用原子 0600 写入和目录同步。仅 commit 从已授权、已加锁 guard 经 F_GETPATH 获取物理路径，核验重开身份后登记 host/app/path/dev/ino/uid。启动按所有者中的登记逐组件 openat(O_NOFOLLOW|O_DIRECTORY) 重开，再核验主机、应用、设备、inode、uid；目录替换/符号链接、非法 pending 子文件名或重复记录直接拒绝。dev/ino 仅用于身份核验，不单独授予写权限；不从请求路径或 HOME 推断 live 资源。
- `commit.rs::commit` 在同一物理 guard 和主机 flock 下先恢复旧事务；有恢复时返回 `WriteError::Recovered`，本次预演/目标不执行，要求调用者重读状态后重算。注册写入只发生在没有未完成意图时，不能用新请求替换旧 pending 的资源归属。原提交测试按此新行为更新；另加逐文件发布后的明确失败注入点，复用原任务 19 阶段钩子。
- 真实执行端入口：`gateway/bootstrap.rs::run_gateway` 在绑定监听器前 spawn_blocking 调用 `agent_apps::recover_providers_on_startup`；按既有管理员 access 策略（显式策略优先，否则已有本地租户策略）消费 online/local host.id 与 credential_root。无 root/无历史事务不创建资源授权。`access.rs::recover_provider_policy` 构造同一 ProviderTransactions 并执行 recover_on_startup；按应用返回结果，日志仅包含 AppId、结果与跳过数量，不记录路径、配置字节或凭据。恢复失败保留 pending 并记录告警。
- write_guard 仅最小提取 descriptor-relative `lock_directory` 与暂存读取，共用既有读取边界、PreparedWrite::publish、私有文件权限和物理锁；未复制文件替换所有者。
- 无新增公共 API/SDK/UI：底层执行端启动与 commit 消费真实恢复实现，未交付的应用供应商编辑、首次激活及写授权 handler 仍归所属适配任务；不能把本项通过推及真实应用 live 切换。SDK→API→应用文件请求链和 Iced 页面不在本项新增行为中，未验证。

### TDD 与验证

最初两项具名测试以可编译的恢复空实现接线，均在 None 与预期恢复结果的业务断言处失败（退出 101，2 失败）。检查发现仓库 build.build-dir 固定为共享 target 后，停止该验证阶段，不采用其构建隔离证据；后续同时设置 CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 为同一动态槽位路径，未改仓库 Cargo 配置。

隔离复核红阶段：动态槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同值；临时将公开 recover 入口还原为空恢复后执行 `cargo test -p vw-agent-apps --lib cc04_recovery_ -- --nocapture`，退出 101，8 项有效行为测试中 7 失败、1 通过，两项要求的具名断言均失败。阶段退出自动恢复完整实现并释放自身锁，产物路径实际为 target/rust-slot-1/debug/deps。

最终绿阶段动态领取槽位 2：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，`CARGO_BUILD_BUILD_DIR` 同值；本阶段下列命令全部继承相同环境，测试二进制实际位于该槽位 debug/deps。Gateway 初次用例在事务开始前制造状态目录冲突，未产生 pending 而失败（退出 101、1 失败）；将故障移到补丁执行阶段后，本轮完整通过。

| 确切命令 | 退出码 | 有效行为测试 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 不计测试 |
| `cargo test -p vw-agent-apps --lib cc04_recovery_ -- --nocapture` | 0 | 8 |
| `cargo test -p vw-agent-apps --lib cc04_commit_ -- --nocapture` | 0 | 6 |
| `cargo test -p vw-agent-apps --lib cc04_stage_ -- --nocapture` | 0 | 5 |
| `cargo test -p vw-agent-apps --lib cc04_write_ -- --nocapture` | 0 | 5 |
| `cargo test -p vw-agent --lib cc04_recovery_ -- --nocapture` | 0 | 1 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 不计测试 |

共 25 项有效定向测试通过。写锁测试中的既有 catch_unwind panic 为预期场景。Gateway lib test 链接器提示 `__eh_frame` 超过 compact unwind 表 16MB 编码范围，测试退出 0；未调整 unwind/构建安全配置，最终强制 Clippy 退出 0。

8 项隔离真实文件恢复测试覆盖 pending/marked/逐文件 published/全部 published/target 后中断、未发布丢弃、外部已改文件保留及其余文件前滚、部分结果列表、暂存缺失、启动重新构造所有者、目录替换/符号链接拒绝与恢复条件、恶意 pending 名拒绝且零副作用、落定失败保留和幂等重试、跨租户下一次 commit 先恢复原租户并要求重算。中断使用 cfg(test) 明确阶段注入点；全部目标、暂存、登记、状态及权限操作均为隔离真实文件系统，无网络/钥匙串或业务 mock，不碰真实用户配置。

Gateway 定向用例用真实私有策略文件、host.id/credential_root、原生 commit 和真实状态路径被目录占据造成的落定失败产生 pending；重建执行端后调用实际启动策略消费者，验证外部文件保留、原租户指针落定、pending 清理、状态 0600，并验证 0644 策略拒绝且没有目标副作用。

平台命令 sw_vers / uname -m 退出 0：macOS 27.0（26A428）、arm64；本机原生 Rust crate 版本 0.6.0。未验证个人账号、已安装应用版本、真实 CLI/Iced 启动、公开 SDK 请求、Windows/Linux 或其他架构。本项隔离执行端恢复通过不代表应用适配和全链 live 切换验收。


定向差异和暂存检查通过，本任务暂存范围为实现、独立测试、必要父注册/写入底层与 Gateway 启动连线及本条正式证据；母任务状态和既有编号文件删除未暂存。plan.md 无差异；各阶段结束后清理自己的 owner/锁目录，最终实际确认本任务槽位 2 锁不存在，保留缓存。未执行全量 cargo test、fmt check、wasm 或 Makefile。本任务标记完成后删除编号文件，不纳入提交，正式证据保留。

提交隔离核对：并行终端会话随后将其提交修正为仅终端文件，并恢复本任务暂存。恢复实现及本条正式证据独立提交；未纳入其他未提交桌面改动、母任务状态或编号文件删除。

## 任务 21：发布前并发变更检测

2026-11-07；状态：已完成（内部执行端事务边界，未验收真实应用 live 切换）。

### 依赖、来源与实现

已核对批次 20 恢复实现提交 `248d0ce48`、正式证据及任务 1–20 记录。母任务 1、3 已完成；母任务 2 沿用任务 1 已裁定的底座子计划 1–93 开工门槛，完整领域迁移仍未收口。本项不改变其状态，不修改只读 plan.md。

来源 `/Users/xiangmin/worker/code/cc-switch/src-tauri/src/live/patch/mod.rs::{Guarded,LivePatch}`、`mode/operation.rs::{run,AppWrite::begin,AppWrite::run}`；核对 `live/engine.rs`、`services/provider/claude_direct.rs`、`mode/controller.rs` 调用关系及 operation 相邻并发编辑/持续编辑/发布失败/恢复测试。来源 MAX_REPLANS 为 3；来源测试本轮未运行。RustCodeGraph status/explore 与定向搜索确认原生唯一事务所有者为 ProviderTransactions，冻结 upstream 不作为原生实现或运行证据。

- `providers/live/concurrency.rs::publish_checked` 接入既有 `ProviderTransactions::commit` 的逐文件发布循环；每次发布前在原有目录描述符上重读并校验 SHA256 摘要，缺失文件用 None 区分。外部内容改变后，用原执行端键补丁重算，最多 3 次，第四次变化显式返回 `WriteError::Conflict`。
- `Guarded::new/apply` 捕获计划快照摘要并通过既有 FileChange 补丁闭包消费；整文件替换遇到登录轮换、文件新增或消失直接冲突。未增加任意文件 API、动态命令接口、新特性或第二套业务状态所有者。
- 重算后的暂存文件和目录先同步，再更新持久 pending 的 pre/planned/staged，成功后丢弃旧暂存。pending 保存若在 rename 后 fsync 失败，保留两候选暂存以免 durable intent 指向缺失文件。失败或崩溃可能留下不被当前 pending 引用的暂存；本项不新增全目录垃圾清理以免删除其他所有者资源。
- 未发布任何文件时的可返回错误先持久化 published=false，再清理 pending 和自己的暂存，禁止失败目标日后被恢复落定。部分发布后冲突保留 pending，由既有恢复实现保留外部编辑并前滚其余文件与状态。真实 unwind 跳过返回错误的清理，保留已持久化重算意图，重新构造所有者可恢复。
- 重算结果等于外部文件时不 rename，保留外部字节与权限并清理未使用暂存。读取拒绝符号链接和非普通目标，摘要及错误不包含配置字节、凭据或主机路径。测试钩子复用既有 checkpoint，仅增加 BeforePublish 阶段。
- AppWriteGuard / StagedBatch 仍是受信执行端的底层写入及预演原语；持久事务入口统一在 commit 消费本行为。应用编辑、首次激活、公共写授权 handler 与 UI 仍由所属适配任务接入。API/SDK/UI 无本项新增公共入口，未验证 SDK→API→真实应用文件链路，不声称应用 live 已成功。
- 外部程序不参与本进程锁，最终重读与 rename 之间仍存在文件系统竞争窗口；本项提供来源约定的有界检测，不声称跨进程 CAS 或多文件原子事务。

### 红绿验证

各验证阶段按 mkdir 原子竞争依次领取槽位；实际均获得槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。由于仓库 `.cargo/config.toml` 另设 build.build-dir，所有 Cargo 命令同时继承 `CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，未改仓库配置或绕过槽位。脚本记录计划、任务、会话和持有 PID，并用退出/中断 trap 释放自身 owner 与锁目录，缓存保留。

红阶段先注册可编译的 Guarded 空校验及现有发布路径，`cargo fmt --all` 退出 0；执行 `cargo test -p vw-agent-apps --lib cc04_concurrency_ -- --nocapture` 退出 101，2 个有效测试均在业务断言处失败：键补丁覆盖并发新增 user=42；Guarded 返回 Ok 而非 Conflict。不是模块缺失或零测试。

最小实现后相同聚焦命令退出 0，2 项通过。扩展验证阶段并发 7 项、提交 6 项、恢复 8 项全部通过且 Clippy 退出 0；差异审查补齐重算变为 noop 的暂存清理后，最终阶段重新执行下表全部命令。

| 最终确切命令 | 退出码 | 有效行为测试数量 / 结果 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化完成，无无关格式差异 |
| `cargo test -p vw-agent-apps --lib cc04_concurrency_ -- --nocapture` | 0 | 8 通过、0 失败 |
| `cargo test -p vw-agent-apps --lib cc04_commit_ -- --nocapture` | 0 | 6 通过、0 失败 |
| `cargo test -p vw-agent-apps --lib cc04_recovery_ -- --nocapture` | 0 | 8 通过、0 失败 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 主工作区全部目标及 features，无警告 |

8 个并发行为测试涵盖两项指定测试 `cc04_concurrency_replans_key_patch`、`cc04_concurrency_guarded_write_rejects_rotated_auth`，以及 3 次重算耗尽后保留第四轮外部编辑且不写其他文件/状态、部分发布后登录轮换与恢复、重算 pending 摘要及暂存持久化后的真实 unwind/重建恢复、Guarded 缺失/新增/轮换快照、符号链接轮换拒绝且外部目标不变、重算 noop 的权限/字节保留与暂存清理。模拟崩溃的 panic 由 catch_unwind 捕获，预期 panic 输出不代表测试失败。全部使用隔离真实临时目录、文件、权限及状态；假登录字节不含真实凭据，无网络、钥匙串或进程 mock，不读取或修改真实用户配置。

### 平台及差异边界

`sw_vers`、`uname -m` 均退出 0：macOS 27.0（26A428）、arm64；原生 crate 版本 0.6.0。安装应用版本、账号、CLI/Iced 实际运行与公共 SDK 请求未验证；Windows/Linux、其他架构和真实应用激活未验收。

定向 diff 与暂存检查只包含本任务 concurrency 实现/独立测试、commit 接线、WriteError 冲突枚举、父模块注册及本条正式证据；现有桌面变更、母任务状态和其他编号任务删除均不提交。未执行全量 cargo test、fmt check、wasm 或 Makefile。槽位释放并保留缓存；本任务标记已完成后删除编号文件且不纳入提交，正式证据保留。

## 任务 22：编辑草稿投影与预览

2026-11-07；状态：已完成（四应用只读编辑预览；未验收应用 live 切换）。

### 依赖、来源及授权边界

核对批次 21 提交 `25b2d7d27` 与任务 21 正式证据；母任务 1、3 已完成，母任务 2 沿用任务 1 的底座子计划 1–93 开工裁定，未改报其完整迁移完成。首次缺原生应用投影而登记阻塞；继续执行指令明确授权本任务建立四应用最小纯投影、必要只读观察及工厂，作为任务 26–32 后续复用的唯一所有者，依此解除阻塞。没有修改只读 plan.md。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，本轮来源工作区干净。阅读 `ProviderService::{editor_view_with_meta,editor_category}`、`commands/provider.rs::get_provider_editor_view`，四个 `*_editor::{view,inactive_fields}`、`live/project/{claude,codex,gemini,grok}`、`live/floor.rs`、`live/residue.rs`，以及 Codex 登录预测、配置校验与模型目录依赖。直接 TS 消费者 `useDraftEditorProjection`、Add/EditProviderDialog、`providersApi.getEditorView` 与相邻 hook/Claude inactive/Codex 元数据测试已核对；来源测试未运行。RustCodeGraph 状态与检索、定向来源读取用于导航，冻结 upstream 不加入原生运行链。

### 实现与明确限制

- `providers/projection/` 为四应用唯一原生纯投影所有者；`providers/preview.rs::read` 从授权主机 HOME 与任务 12 路径解析结果读取真实配置，复用 catalog 的 descriptor-relative no-follow 有界读取。缺文件在内存中视为空，不创建目录、锁、暂存、pending 或配置；不调用 writer、commit 或 recovery。后续适配可复用本领域模块，未新建第二数据库、通用文件/命令 API、特性开关或 UI。
- Claude：只替换连接/鉴权/模型/协议字段；独有字段按前所有者原值相等才清除，保留外部改值；冻结窗口残留按值清理，Bedrock 顶层 apiKey 转到专用 env。用户 hooks、非供应商功能开关与超时保留；失效字段排除内部旧字段。
- Gemini：读取 `.env` 与 `settings.json`，只替换来源指定的 env 关键字段、`security.auth.selectedType`、`model.name`，其余全局配置保留。官方卡只预览 oauth-personal 配置选择，不证明已登录；非官方卡不会沿用行里残留的 OAuth 类型。
- Codex：复用来源纯路由解析规则，支持第三方自定义/保留 ID/旧顶层地址归一到 custom、keyless 草稿的内存占位、ollama/lmstudio/Bedrock 内置及只改模型的 Default；嵌套关键字段单独清理，前所有者独有值匹配清理；保留 profile 引用，拒绝有效 profile 覆盖。凭据差异先按真实投影计算，再从编辑展示移除重复 bearer 字段，不能把两次 `[redacted]` 当相等。官方卡、现有原生登录材料、非 file 凭据存储、任何非空 Codex meta（含尚未分类的元数据）均明确不可用；非空 modelCatalog.models 依赖模型目录所有者，明确 `model_catalog` 不可用，不猜测账号保留策略、钥匙串、托管路由或生成目录。
- Grok：models.default 与所选整张模型表投影，保留所有 vendor 扩展字段；按前所有者及代理占位表清理退休表，session_summary 按来源条件跟随目标，保留用户模型表。官方卡仅预览去除托管模型指针，不证明 xAI 登录成功。无有效唯一模型表或形状校验失败时显示脱敏草稿并显式 `invalid_draft`，不伪装成 live 投影；语法错误返回无载荷的解析位置。
- 其他应用明确 `unsupported_application`。所有预览只是观察，不保证随后写入时的 revision，也不宣称多文件读快照原子性或应用 live 已成功；应用写事务、UI、模式切换仍归后续任务。

### API、秘密及执行端状态

公开 `POST /v1/app-providers/{app}/config/preview`，DTO 为 `ProviderPreviewRequest/ProviderPreview`，SDK 为 `GatewayClient::app_provider_preview`。沿用既有 app-providers 父 router，标准与 bootstrap 两入口均由原 merge 连通；网关认证与目标主机 `provider_config_read` 授权独立，权限检查在解析目标路径与记录读取前执行。256 KiB 请求上限、128 KiB 草稿上限；不接受任意文件参数，override 沿用 HOME 内边界，符号链接拒绝。

请求可提供草稿，或以 providerId 加空 settings 在当前租户执行端读取已有原生 settingsConfig，无需客户端读取明文 Key。仅含旧 launch profile、缺 settingsConfig 的记录明确 422；不猜测原生配置。现有卡官方类别按来源 legacy 名称/partner/fixed-ID 规则判定；显式空草稿 meta 覆盖保存的 meta，未提供时消费保存的 meta。存储异常/缺记录用安全 HTTP 状态，不返回 SQL、路径、配置正文或错误链。SDK 丢弃失败原始响应体，request Debug 整体脱敏。

响应包含结构化 settings、变更路径/类型、失效字段路径、受控 unavailable 或逻辑文件角色+行列号。**所有字符串值都遮蔽**（包括模型名、URL、命令及未知插件字符串），敏感字段子树连数值/布尔值也遮蔽；这是一份保守的结构展示，不提供可保存的明文编辑底。差异在脱敏前计算；原始基底及摘要只用于执行端本次调用、随后释放，没有返回客户端，也未增加无消费者的持久缓存或三方合并接口。

执行端读取既有 ProviderTransactions 状态：`open_existing` 只打开私有现有目录，不创建、不 fsync、不恢复；pending 存在时返回 `pending_transaction`。跨租户物理配置所有者不返回他租户记录，返回 ownership；同租户指针从唯一 tenant store 取原生 settings，附着契约只消费既有独有字段。缺真实所有者数据明确不可用。现有记录查询采用 `GatewaySettingsStore::read_existing`，SQLite 只读打开、PostgreSQL 仅 SELECT，避免原构造器的 schema 锁与 DDL；预览不回退初始化全局 HOME 数据库。PostgreSQL 本轮未实测。

### 红绿及系统证据

各阶段按原子 mkdir 动态竞争，实际均获得槽位 **1**：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。因仓库另配置 build.build-dir，所有 Cargo 同时继承 `CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。锁目录记录计划、任务、会话标识与 PID；阶段结束 trap 清理自身 owner/锁，缓存保留，未删其他锁。

红阶段先接通可编译空投影：`cargo fmt --all` 退出 0；以下四个同名过滤测试分别退出 **101 / 0 / 101 / 0**，业务包 2 项均在 hooks/失效字段断言失败，真实本地 SDK→API 1 项在投影结果断言失败；DTO 1 项与 SDK 1 项传输测试通过。失败不是缺模块/类型或零测试。

| 最终确切命令 | 退出码 | 有效测试 / 结果 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化完成，无无关格式差异 |
| `cargo test -p vw-agent-apps --lib cc04_preview_ -- --nocapture` | 0 | 12 通过，0 失败 |
| `cargo test -p vw-api-types --lib cc04_preview_ -- --nocapture` | 0 | 1 通过，0 失败 |
| `cargo test -p vw-agent --lib cc04_preview_ -- --nocapture` | 0 | 3 通过，0 失败 |
| `cargo test -p vw-gateway-client --lib cc04_preview_ -- --nocapture` | 0 | 1 通过，0 失败 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 工作区全 targets/features 检查通过 |

12 项业务测试包含指定的 `cc04_preview_never_writes_files`、`cc04_preview_reports_inactive_fields_and_parse_location`，以及四应用规则、凭据同值/轮换先比较后遮蔽、非字符串凭据遮蔽、官方/托管/模型目录/歧义草稿不可用、源类别判定、profile 冲突、symlink 拒绝、只读事务工厂。隔离真实目录快照比较字节、权限、修改时间及目录成员，未创建缺失配置或事务目录。

Gateway 的 3 项有效测试通过真实 loopback HTTP 与正式 SDK 验证四应用草稿→API→执行端投影及失效字段，比较原文件字节和目录成员；另以真实 SQLite 和既有事务状态验证 providerId 读取、数据库/供应商行/状态不变、没有 pending 新增、pending 观察不触发恢复、他租户所有者受控不可用。401、403、503、501 与网关认证拒绝均有真实断言。全用假凭据，无网络上游、钥匙串或进程 mock，不读写真实用户配置。

`sw_vers`、`uname -m` 退出 0：macOS 27.0（26A428）、arm64；原生 crate 0.6.0。真实已安装 CLI 应用/版本、账号、钥匙串、Iced 窗口、正式登录/激活及 Windows/Linux 未验收；不将临时配置与本地 HTTP 等同于真实应用切换。

定向差异/暂存检查覆盖本任务实现、独立测试、最少注册与只读支持，排除其他会话桌面终端、母任务状态及既有编号文件删除；只读 plan.md 无差异。自身槽位锁释放、缓存保留。未运行全量 cargo test、fmt check、wasm 或 Makefile。任务标记完成后删除编号文件且不纳入提交，正式证据保留。

## 任务 23：公共字段与请求校验原生化

2026-11-07；状态：已完成（公共字段存储与校验；不代表原生应用 live/代理执行已验收）。

### 依赖与来源

核对批次 22 提交 `08b6d3c0d` 及正式证据；母任务 1、3 已完成，母任务 2 沿用任务 1 底座子计划 1–93 开工裁定，完整领域迁移仍未收口。本任务继续执行指令明确授权最小扩展公共字段 DTO 和既有 save/create/update 保存链，未把缺失输入契约当作外部阻塞，也未改只读 plan.md。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，本轮 `rev-parse HEAD`、`status --short` 退出 0，工作区干净。读取 `ProviderForm.tsx::handleSubmit`、meta 构造及 key 字段状态，`helpers/requestHeaders.ts::{normalizeRequestHeaders,findRequestHeaderValue}`、`schemas/provider.ts::providerSchema`、`lib/requestOverrides.ts` 和相邻 `tests/lib/requestOverrides.test.ts`；核对普通头的 Kimi/Pi/OpenCode/Mcode 调用位置和原生入口的 DTO、业务规则、存储、Gateway/SDK 直接调用者。来源测试未运行。RustCodeGraph status 可用；explore 无输出后终止本会话进程，改用定向 rg/读取当前文件，没有重建索引。

### 唯一所有者与输入语义

- `vw-api-types/src/app_providers/validation.rs` 定义 `ProviderRequestOptions`、`ProviderRequestOverrides`、`ProviderApiKeyField`；现有 `AppProviderWrite`、`AppProviderCreate`、`AppProviderUpdate` 和 `ProviderPreviewRequest` 增加可省略的 `requestOptions`，不新增独立 validation/任意配置写 API。字段为 `requestHeaders`、`apiKeyField`、`localProxyRequestOverrides`，沿用来源成员名，统一封装为原生公共字段域；不扩展协议或新增 features/dependencies。
- 旧 JSON 省略 requestOptions 仍按原 profile 工作；PUT/PATCH 均保留省略的公共成员，普通头 map 整体替换、空 map 清除，代理覆盖对象整体替换、空对象清除。显式 null、未知公共字段、非法 key 选择、非字符串头、非对象 body、完全同名重复 JSON 头拒绝。key 仅接受 ANTHROPIC_AUTH_TOKEN / ANTHROPIC_API_KEY（显式 AuthToken 恢复来源默认选择），Claude/Codex 可保存选择及代理覆盖，其余应用拒绝不支持的成员；原 profile 的协议/应用能力边界不扩大。
- `vw-agent-apps/src/providers/validation.rs` 为公共地址、请求头及覆盖规则的唯一 Rust 所有者。地址要求 HTTPS 或 loopback HTTP，无用户信息/query/fragment，拒绝空值、超长、空白、控制符、反斜杠及缺 `://`；档案 validate_profile 与 endpoints 地址簿共用此规则。头名/值使用现有 reqwest 的 HTTP parser，拒绝控制字符、draft-header、按 trim/大小写归一后的重复名，保存为 trim 后小写。单组头最多 64 个、名称 256 字节、值 8192 字节，公共选项有 128 KiB 内部上限；HTTP 保存入口继续受既有 16 KiB 请求限制约束。
- 普通请求头允许显式 Authorization 等自定义鉴权头；代理覆盖头独立应用来源完整受保护集合，包括鉴权、路由、传输、代理及追踪头，大小写变化不能绕过。代理 body 保留供应商扩展字段，但禁止存在 stream（即使 false/null），不迁入模型/能力知识或 14 套应用特殊 schema。
- create 在插入前校验/归一；update 在唯一档案上合并后校验；`AppProviderStore::save` 改为保留既有未知/应用独有字段的合并写，复用既有 compare_exchange/insert_if_absent，竞争返回既有 409，不覆盖并发更新。不创建第二数据库或存储所有者。duplicate/universal sync/启动的既有 validate_profile 调用同样覆盖新公共字段。读取保存档案时只增加受控 requestOptions，不把 opaque settingsConfig/meta 反序列化为任意写 DTO。
- 预览接入相同类型与校验：providerId 读取时从执行端档案合并已保存公共选项，读取权限在路径/档案访问前检查；来源 meta 中已存在的公共字段也校验，未知应用元数据保留。Claude/Gemini 已识别的草稿端点、Codex 选中路由的 base_url/http_headers、Grok 选中模型表的 base_url/extra_headers 复用公共规则；仅校验已有投影所有者选出的连接字段，不迁移其余 schema。原有官方/托管/登录不可用边界保留，合法来源 Codex 非空 meta 也不因此被宣称已支持。

### API、秘密与执行结果

沿用现有 `POST/PUT/PATCH /v1/app-providers/{app}/{id}` 及 `POST /v1/app-providers/{app}/config/preview`，父 router、标准/bootstrap 注册及权限边界均未新增授权。SDK 继续使用 app_provider_create/save/update/preview；不另加无保存调用者的校验端点。write-only options/overrides 的 Debug 整体脱敏；SDK PUT 从通用带载荷日志 helper 改为与 create/update 一致的领域请求，仅报告固定错误和状态，丢弃失败原始响应体。响应仍只有既有脱敏 summary 或脱敏 preview，不返回请求头、body 或秘密。

真实 loopback SDK→正式 handler_router→隔离 SQLite 验证合法 Authorization 普通头、key 选择及代理扩展 body 持久保存，头名归一；受保护代理头经 POST/PUT/PATCH 均 400，记录值和成员数不变。随后通过真实 SDK 局部编辑、旧式 PUT、显式清除，重读临时数据库确认应用 settingsConfig、未知嵌套 meta、未编辑公共成员保留。Basic 认证拒绝 401 且无记录；类型/未知字段/控制字符绕过均 400，响应无假秘密。预览由真实策略文件/HTTP 验证无效 token 401、合法 reader 的非法选项 400，私有目录与策略字节不变、无配置/pending/锁新增。

SDK transport 测试替代 HTTP 对端，仅检查传输形状、固定状态错误和丢弃原响应；另用真实 tracing Subscriber 的正向 capture-ready 信号确认捕获有效，POST/PUT/PATCH 的 key、普通头、代理任意 body 不进入领域日志。业务测试使用真实临时目录和 JSON/本地配置，未替代公共校验函数；所有凭据均为 fixture。

保存公共字段不等于已在 CLI/代理执行这些选项。现有隔离启动器对非空公共请求选项明确报“requires a native application adapter”，在创建临时配置前拒绝，避免静默忽略；省略、空选项及已清空的头/覆盖保留原启动行为。源脚本启动回归只验收已有隔离启动机制，不代表真实安装应用/登录成功。应用适配、代理消费、编辑器 UI 仍归后续任务。

### 红绿验证与 macOS

红、首次绿、最终绿/回归阶段均通过原子 mkdir 动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`；仓库 build.build-dir 另配置 target，故所有 Cargo 同时继承 `CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。锁记录计划/任务/会话/PID，阶段结束 trap 删除自身 owner 后 rmdir，缓存保留；未操作其他会话槽位或进程。

红阶段 `cargo fmt --all` 退出 0，四条同名过滤测试的退出码分别为 **101 / 0 / 101 / 0**，有效测试数 **2 / 1 / 1 / 1**。业务两项在非法头被放行、局部更新丢失公共字段断言失败；Gateway 真 SDK/HTTP 在受保护代理 Authorization 被创建成功处失败。不是模块缺失、编译错误或零测试。首次绿四包 **4 / 2 / 3 / 2** 项通过，强制 Clippy 退出 0；审查后补原生预览连接字段、日志捕获及空选项启动验证，以下为最终结果：

| 最终确切命令 | 退出码 | 有效测试数 / 结果 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 无无关格式差异 |
| `cargo test -p vw-agent-apps --lib cc04_validation_ -- --nocapture` | 0 | 6 通过 |
| `cargo test -p vw-api-types --lib cc04_validation_ -- --nocapture` | 0 | 2 通过 |
| `cargo test -p vw-agent --lib cc04_validation_ -- --nocapture` | 0 | 3 通过 |
| `cargo test -p vw-gateway-client --lib cc04_validation_ -- --nocapture` | 0 | 3 通过 |
| `cargo test -p vw-agent-apps --lib providers:: -- --nocapture` | 0 | 105 通过 |
| `cargo test -p vw-agent-apps --lib cc_apps_provider_launch_isolation_ -- --nocapture` | 0 | 9 通过 |
| `cargo test -p vw-agent --lib app_providers -- --nocapture` | 0 | 40 通过 |
| `cargo test -p vw-gateway-client --lib app_providers -- --nocapture` | 0 | 19 通过 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 targets/features 通过 |

回归过滤包含部分同名聚焦测试，不叠加为独立行为数量；providers/启动回归中的子进程输出不重复计入父测试数。业务 6 项包括两项指定具名断言、普通/代理头区分与清除、预览源 meta/公共选项拒绝、四应用既有原生请求字段拒绝、未实现启动选项无文件副作用及空选项兼容。Gateway 3 项包括真实 SQLite 写入链、认证/坏请求无副作用、独立预览授权；SDK 3 项包括 wire/error、PUT 原始错误丢弃和真实 tracing 日志捕获。测试链接阶段出现 macOS ld 的既有大 __eh_frame 提示，但命令成功；强制 Clippy 无警告退出 0。最终槽位 1 锁已不存在，缓存保留。

`sw_vers` / `uname -m` 退出 0：macOS 27.0（26A428）、arm64；原生 crate 0.6.0。未验收真实安装 CLI/桌面应用版本、账号/钥匙串、Iced 窗口、真实代理/应用激活或 Windows/Linux；这些限制不被临时存储/HTTP/脚本测试替代。

定向 git diff --check、暂存差异审查通过；只提交本任务公共 DTO、校验、保存/预览/SDK/启动必要连线、独立测试、既有 literal 的兼容默认值和本条证据。未纳入母任务状态、此前编号任务删除及其他会话差异；只读 plan.md 无差异。任务完成后删除编号文件且不提交其删除，正式证据保留。未运行全量 cargo test、fmt check、wasm 或 Makefile。

## 任务 24：编辑保存三方冲突

2026-11-07；状态：已完成（Claude 自定义档案的全局键编辑与非当前卡目录草稿；不代表关键字段切换、其他应用适配或 Iced 已验收）。

### 依赖、来源与继续执行裁定

批次 23 提交 `0744c6231` 及本文件任务 23 证据已核对；母任务 1、3 已完成，母任务 2 继续沿用任务 1 的底座子计划 1–93 开工裁定，完整领域迁移仍未收口。最初缺可信基底与保存适配而登记阻塞；继续执行指令明确授权本任务补最小服务端基底、DTO、存储/工厂和真实保存适配，依此解除，不修改只读 plan.md。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`。已阅读 `claude_editor.rs::{EditorSave,ConflictPolicy::resolve,JsonChange::conflicts_with,plan_save,global_changes,write_live}` 及相邻冲突测试、`editor_toml.rs::TomlEdits::{between,apply_to}`、`LiveEditConflictDialog.tsx::useLiveEditConflict`、`EditProviderDialog.tsx` 与 provider/mod.rs 的编辑保存直接调用者。复用“当前值既不同于基底也不同于目标才冲突；明确保留外部时跳过冲突键，其余用户修改照常提交”的语义；不复制来源应用写入/数据库所有者。来源测试未运行。RustCodeGraph status/explore 已运行；图内冻结 upstream 命中与当前原生文件逐项区分，新文件采用定向读取与差异审查。

### 实现与支持边界

- `vw-api-types/src/app_providers/editor_merge.rs` 定义 `EditorOpen/EditorView`、`EditorSave/EditorEdit/EditorValue`、`EditorChoice/EditorResult`。草稿以显式键路径的 Set/Remove 表达用户修改；缺键、显式 null 和删除不同。没有客户端 base/current、任意文件路径、命令或 force 输入；未知字段拒绝，写入 DTO 的 Debug 遮蔽全部载荷。
- `vw-agent/src/gateway/api/handlers/app_providers/editor_merge.rs` 在执行端保留未遮蔽基底和完整档案/事务状态快照；随机不透明上下文绑定租户、应用、供应商、主机、管理员 HOME/private root、当前配置目录设备/inode 及基底内容。最多 128 个上下文，15 分钟过期，重启丢弃，成功后单次消费；不建立第二持久存储。显示继续复用任务 22 的全字符串/敏感键遮蔽，显示结果不能回送为基底。
- 冲突响应只包含键路径和随机 resolution。选择绑定当前文件完整字节与本次 edits，保存重试重新读文件、档案与事务状态；外部再次修改会重新报告冲突并签发新选择上下文。无有效选择时不能直接 KeepMine/KeepTheirs；基底档案、主机或状态失配明确 409，需重新打开。服务端只取显式用户差异，基底与目标相同的编辑不会恢复外部旧值，外部已经写成目标不算冲突。
- `vw-agent-apps/src/providers/editor_merge.rs::merge` 复用唯一 `projection::fields` 字段分类和任务 13 的 `JsonPatch`；支持嵌套对象键与整值/数组替换，拒绝重叠/重复路径、超限路径、损坏/非对象配置及遮蔽占位值。当前 Claude 卡只支持全局键，连接/鉴权关键字段、供应商独有 env、整个 env 替换与内部协议字段一律明确拒绝。供应商关键/独有字段的当前卡写入由后续应用切换适配实现，不通过本接口伪支持。
- `ClaudeFile::open/commit` 是本行为最小原生保存适配：在管理员授权 HOME 下 descriptor-relative 打开固定 `.claude/settings.json`，拒绝链接、缺失及不安全资源；保留应用/目录物理 guard，复用任务 19–21 唯一 `ProviderTransactions::commit`、pending、恢复与发布前变更检测。提交补丁内部在事务锁下再次校验状态与批准文件字节，外部并发变化返回冲突；不修改直连指针或模式。目标为 0600，未编辑字节由定点补丁保留；无文件变化时不重写事务状态。外部进程不参与锁，仍沿用任务 21 已注明的最终检测至 rename 之间竞争窗口，不宣称文件系统 CAS。
- 当前卡全局编辑只更新 live，目录档案不变；非当前卡以 `AppProviderStore::editor_save_record` 的同一真实存储 compare_exchange 更新 settingsConfig、保留其他档案字段，不读写 live、不创建事务资源或首次激活。没有历史事务时视为空状态，目录保存不会 provision provider-live。缺少 settingsConfig 的普通 profile、官方/非空托管元数据、代理 attached 状态、跨租户所有者、pending、无法投影的配置均明确拒绝；本轮不负责档案/live 双写、首次激活或代理重同步。

### API、SDK 与权限

- `POST /v1/app-providers/{app}/{id}/editor/open` 与 `/editor/save` 接入既有分域 router 和 Gateway 身份认证；请求体限制 128 KiB。SDK 为 `GatewayClient::{app_provider_editor_open,app_provider_editor_save}`，原生 transport，沿用租户头与敏感 `x-agent-apps-token`，非成功错误体丢弃以防配置载荷泄漏。WASM 明确拒绝。
- open 复用管理员 `provider_config_read`；save 必须具备独立 `provider_config_write`，默认 false，没有从读取/启动/偏好写权限推导授权，也不自动生成新授权。每次保存重新鉴权后才读基底/数据库/主机文件。执行端工厂只消费既有管理员 HOME 与 credential_root；生产未注入数据库时复用现有 GlobalDatabase 工厂，不另建数据库。
- 200 + saved=false 表示有键级冲突，调用者须展示明确选择；200 + saved=true 才是已保存。无有效/过期/失配上下文与状态变化返回 409；无效输入/投影返回 400/422，未授权 401/403。除 Claude 外应用返回 501；非 macOS 未实现。Iced 弹窗/编辑器未新增、未验收，归后续 UI 任务。

### 红绿证据与实际环境

红、绿阶段均从仓库根通过原子 mkdir 动态领取槽位 **1**：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，并将 `CARGO_BUILD_BUILD_DIR` 设置为同一绝对路径，避免仓库 build-dir 配置绕过槽位。每阶段写入自己的计划/任务/会话/PID owner，所有 Cargo 及子进程结束后由 shell trap 清理 owner/rmdir 锁，缓存保留；最终实际确认本任务 slot-1.lock 不存在。

红阶段 `cargo fmt --all` 退出 0；下表四条聚焦命令退出码依次 **101 / 0 / 101 / 0**，有效测试数 **2 / 1 / 2 / 1**。业务断言在 x 未更新、冲突未报告处失败；Gateway 真实 SDK/HTTP 在冲突被当作成功保存、目录用户值未写入处失败，均已编译接线，不是模块缺失或零测试。随后最小实现合并及边界处理，绿阶段 `cargo fmt --all` 退出 0，最终结果：

| 确切命令 | 退出码 | 有效通过数量 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_editor_merge_ -- --nocapture` | 0 | 4 |
| `cargo test -p vw-api-types --lib cc04_editor_merge_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-agent --lib cc04_editor_merge_ -- --nocapture` | 0 | 6 |
| `cargo test -p vw-gateway-client --lib cc04_editor_merge_ -- --nocapture` | 0 | 1 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

两项指定测试 `cc04_editor_merge_keeps_external_unedited_fields`、`cc04_editor_merge_conflict_requires_explicit_choice` 全部通过；另覆盖 null/删除、嵌套无关键、外部同目标、未改值、重叠/关键字段/占位/损坏输入拒绝。Gateway 六项有效测试使用隔离真实 SQLite、真实管理员策略/事务状态 fixture、真实临时 macOS 文件和本地回环 HTTP，经 SDK→API→执行端读写证明：冲突拒绝不改文件，明确 KeepMine/KeepTheirs 后只提交用户差异，外部无关值保留，第二次外部修改使旧选择失效，成功上下文重放失败；非当前卡只更新目录、无历史事务时不创建资源；原档案和状态变化拒绝、投影失败拒绝、符号链接拒绝、读取授权不能保存、正式路由 Gateway Basic 拒绝，以及显示/错误脱敏。凭据与 provider state 均为隔离 fixture；没有上游网络、钥匙串或进程 mock，不触碰真实用户配置。

实际运行平台 `sw_vers`：macOS **27.0 (26A428)**；`uname -m`：**arm64**。本机原生文件/数据库/SDK/API 行为已验证；没有启动或查询已安装 Claude 版本，没有真实账号、真实 CLI 消费配置、Iced 渲染或其他应用适配证据，这些明确未验证，不能把 fixture 指针/档案视为真实应用激活成功。

定向 diff 与暂存审查仅包含本任务 DTO、业务合并/最小适配、必要存储与授权工厂、Gateway/SDK 注册、独立测试及本条证据；未纳入母任务状态、其他编号任务删除或终端计划变更。只读 plan.md 无差异。未执行全量 cargo test、fmt check、wasm 或 Makefile。任务标记完成后删除编号任务文件且不提交其删除，正式证据保留。

## 任务 25：通用配置片段维护

2026-11-07；状态：已完成（冻结片段存取/提取与内存编辑保留语义，不代表 live 切换或 Iced 已验收）。

### 依赖、来源与语义裁定

核对批次 24 提交 `0d3ce72ba` 及正式证据；母任务 1、3 已完成，母任务 2 沿用任务 1 的底座子计划 1–93 开工裁定，完整领域迁移仍未收口。最初因“仅迁移现有产品”与“应用片段”的含义不明登记阻塞；继续执行指令明确沿用冻结来源：片段存取/提取剔除关键字段，保存片段不改 live，不新增显式 live 应用；以现有内存预览和显式差异编辑保留当前用户值、保存后真实文件字节不变验收，依此解除阻塞。只读 plan.md 未修改。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`。已阅读 `commands/config.rs::{validate_common_config_snippet,get_common_config_snippet,set_common_config_snippet,extract_common_config_snippet}`、`services/provider/mod.rs::{extract_common_config_snippet,extract_common_config_snippet_from_settings,extract_claude_common_config,extract_codex_common_config,extract_gemini_common_config,extract_opencode_common_config,extract_openclaw_common_config,is_sensitive_config_key}` 及相邻字段剔除测试、`services/provider/live.rs::{provider_uses_common_config,remove_common_config_from_settings}` 及旧片段移除测试；核对 `CommonConfigEditor.tsx` 和直接调用者 `ProviderForm.tsx`：新版编辑全局 settingsConfig，三应用片段只存库，不依旧片段重写 live。未运行来源测试，也不迁入来源的历史清理/启动自动迁移。RustCodeGraph status/explore 已执行，冻结 upstream 命中与当前原生模块分开核对；新增文件定向读取审查。

### 实现与支持边界

- `vw-agent-apps/src/providers/snippets.rs::{normalize,extract}` 是片段唯一业务所有者；Claude/Codex/Gemini 复用既有 `projection::fields` 关键字段分类。Claude 剥离关键顶层/env、协议元数据和来源规定的上游窗口字段，保留用户功能开关；Codex 剥离关键顶层/嵌套键、model_providers、MCP 及旧 mcp.servers、web_search=disabled 注入哨兵，保留其它偏好；Gemini 提取 env，去掉关键字段，仅保留非空字符串。OpenCode 剥离 options/settings 中 apiKey/baseURL，OpenClaw 剥离顶层 apiKey/baseUrl，沿用来源片段形状，不创建应用切换适配。其它应用明确拒绝。
- 采用来源凭据命名规则并递归清洗嵌套对象/数组，补充 camelCase accessKey/accessToken/privateKey 等形式；不把所有复数 TOKENS 或普通遥测开关当密钥。非法结构、超出 64 KiB 的输入/规范化结果、超过 64 层结构和遮蔽占位值明确拒绝。JSON 规范化为对象文本；Codex 解析后重编码 TOML，原注释不保留，避免注释带入凭据；空白及无配置项的纯注释 Codex 规范化为空片段，回读稳定。
- `vw-agent/src/app_providers/snippets.rs` 复用同一 GlobalDatabase/GatewaySettingsStore，固定租户命名空间 `app_providers.snippets`，每应用一条原子 upsert 记录；配置片段不按主机另存副本，不另建数据库。写入前清洗，读取时再次清洗，损坏记录返回 503；读取清洗不回写旧数据，不做历史结构迁移。Saved 提取复用同一存储的 preview_record 与租户键，只取已有 settingsConfig；Draft 提取不写存储。
- 内存编辑保留语义复用任务 22 `preview::read` / `projection` 和任务 24 `editor_merge::merge`，没有第二套 merge/apply 所有者。真实预览中旧片段的 hooks/theme/用户 env 不覆盖当前 live 用户字段；显式编辑时未改的旧值被跳过，只提交用户差异。本项没有 live apply API、事务发布、供应商激活、旧快照回填或 UI 入口，不能把只读投影声称为切换成功。

### API、SDK 与权限

- 强类型 DTO 位于 `vw-api-types/src/app_providers/snippets.rs`：SnippetRead/Write/Extract、带标签的 Draft/Saved 来源及 ConfigSnippet。未知字段、路径/命令/force/客户端租户字段和 null 写入拒绝；所有 DTO Debug 遮蔽载荷。
- `GET /v1/app-providers/{app}/config/snippet?targetHostId=...` 读取；`PUT` 同路径保存/清空；`POST .../config/snippet/extract` 提取。128 KiB HTTP 体限制，沿用既有分域 router、Gateway 身份认证和 TenantScope。独立读授权用于读取/提取，独立写授权用于保存；每次在阻塞执行任务内先按管理员策略核对令牌、租户、主机及能力，再访问数据库。写授权不由读授权推导，缺授权默认拒绝。复用已有 access 授权，不要求或创建配置私有事务根，接口不打开 native 配置。
- SDK `GatewayClient::{app_provider_snippet_get,app_provider_snippet_save,app_provider_snippet_extract}` 使用原生 transport 和租户上下文，x-agent-apps-token 标为敏感，错误响应体丢弃；WASM 明确拒绝。保存成功仅代表片段已存储，不表示 live 已变更。非法输入/不支持应用返回 400、档案缺失 404、未认证/未授权 401/403、存储损坏 503。

### 红绿证据、平台与审查

各构建/验证阶段均从仓库根原子 mkdir 动态领取槽位 **1**：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同样设置为该绝对路径，避免仓库 build-dir 配置绕过槽位。每阶段记录计划/任务/会话/PID，全部 Cargo 和产物子进程结束后 shell trap 清理自己的 owner/rmdir 锁，缓存保留。

红阶段 `cargo fmt --all` 退出 0；以下四条聚焦命令退出码为 **101 / 0 / 101 / 0**，有效测试数 **2 / 1 / 2 / 1**。业务包提取结果仍含模型/协议/密钥而失败，Gateway 的真实 SDK/HTTP 提取与脏片段回读同样在未剔除关键字段的业务断言处失败；不是缺类型或零测试。既有内存投影行为已通过，不伪造该行为的失败。首次绿阶段发现本次 Codex 嵌套路径解构的 Borrow<&str> 编译错误，已修复后重新格式化、完整聚焦验证及 Clippy；最终补测空 Codex 回读一致性与 camelCase 凭据，最终结果：

| 确切命令 | 退出码 | 有效通过数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 非行为测试 |
| `cargo test -p vw-agent-apps --lib cc04_snippets_ -- --nocapture` | 0 | 5 |
| `cargo test -p vw-api-types --lib cc04_snippets_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-agent --lib cc04_snippets_ -- --nocapture` | 0 | 3 |
| `cargo test -p vw-gateway-client --lib cc04_snippets_ -- --nocapture` | 0 | 1 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

共 **10** 项有效测试，无忽略。两项指定测试 `cc04_snippets_extract_excludes_provider_keys`、`cc04_snippets_apply_preserves_live_user_fields` 均通过；另验显式差异不会恢复未改快照、非法形状/大小/占位值、嵌套凭据、Codex 用户 web_search/MCP 邻键保留与空片段回读。Gateway 使用隔离真实 SQLite、真实管理员策略、macOS 临时 `.claude/settings.json` 和回环 HTTP，经 SDK→API→存储回读证明：Draft/Saved 提取清洗、存储不含已剔除的凭据、重开数据库后读取一致、平台/公司隔离、读取脏记录时清洗且损坏拒绝、空白清空、非法输入及未知路径字段无写入副作用、读权限不能保存、无授权作用域拒绝、正式 Gateway Basic 未认证拒绝；整个链路 live 文件字节不变且无额外配置文件。SDK 另验证方法/路径/请求体与错误载荷不泄漏。均为假凭据，未替代上游网络、钥匙串或应用进程，不触碰真实用户配置。

实际平台 macOS **27.0 (26A428)**、**arm64**；本仓库 crate 版本 **0.6.0**。已验证本机原生临时文件/数据库/SDK/API 行为；未启动或查询已安装 Claude/Codex/Gemini/OpenCode/OpenClaw 版本，没有真实账号、CLI 消费配置或 Iced 渲染证据，这些明确未验证。Agent lib test 链接仍有既有 `__eh_frame section too large` 提示（此前任务 10/11/12/20/23 已记录），测试退出 0；未改变链接设置，最终强制 Clippy 退出 0。

定向 diff/暂存审查仅包含本任务业务/存储、DTO、SDK、权限 handler 与必要父注册、独立测试及本条正式证据；未纳入母任务状态、此前编号文件删除和其他计划变更。plan.md 无差异，自身槽位锁释放、缓存保留。未执行全量 cargo test、fmt check、wasm 或 Makefile。任务标记完成后删除编号文件且不提交其删除，正式证据保留。


## 任务 26：Claude 关键字段切换

2026-11-07；状态：已完成（macOS 隔离文件、真实数据库及 SDK/API 事务链；真实账号、CLI 消费与 Iced 未验收）。

### 依赖与来源

已核对批次 25 提交 `6d8d5247c` 和本文件任务 25；母任务 1、3 已完成，母任务 2 继续沿用任务 1 的底座子计划 1–93 开工裁定，完整领域迁移尚未收口。首次缺少四类输入及激活契约登记阻塞；继续指令明确授权本任务建立强类型配置、扩展现有保存及统一同步的 Claude 分支，并接通六入口，依此解除。没有修改只读 plan.md。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`。已读 `services/provider/claude_direct.rs::{switch_to,reapply,write,run}`、`live/project/claude.rs::{ClaudeProjection::of,direct_patch,project_onto}`、`tests/golden/claude_key_fields.rs`，核对 `services/provider/mod.rs` 首卡、switch_claude_direct 和 `live.rs` reapply 调用，以及 `claude_official::{bound_account,strip_from_patch,with_login_switch}` 的登录边界。未运行冻结来源测试、未链接 upstream 成为运行所有者。RustCodeGraph status/query/node 与定向源码搜索用于核对现有调用者。

### 实现与事务边界

- `vw-api-types/src/app_providers/claude.rs` 定义官方、第三方、Bedrock、Vertex 配置；官方无需 key/URL，Bedrock 支持默认凭据链、Bearer、显式 AWS key/session 或 profile，Vertex 为项目/区域及可选凭据文件引用。引用路径只写入配置，不由本接口读取文件或执行凭据命令。敏感类型 Debug 遮蔽，未知字段/客户端根路径拒绝；没有伪造非空 key。
- `vw-agent-apps/src/providers/claude.rs::{settings,patch,import,ClaudeFile::open,ClaudeFile::apply}` 为应用适配。字段所有权继续由任务 22 的 `projection::claude`/`fields` 决定；字节修改复用 `JsonPatch`，最小开放同一 JSON/JSONC 验证器的内部 object_value 读取，保留注释、空白和未修改字段，拒绝重复键及损坏配置。清除上一家所有关键字段、仅按原值清除独有字段，保留用户外部改值及功能开关；遗留 Bedrock 顶层 apiKey 沿唯一投影转换到 AWS_BEARER_TOKEN_BEDROCK。
- `vw-agent/src/app_providers/claude.rs` 是既有 AppProviderStore 的子模块；复用 `app_providers.records` 和相同租户 key、insert_if_absent/compare_exchange，不建立第二档案库。typed claude 与 settingsConfig 同行保存，保留元数据/全局配置，替换时清除旧顶层凭据。列表/摘要支持四类，响应不返回秘密。typed PUT/PATCH 是明确的完整 ClaudeProfile 替换；既有 legacy 输入及默认仅存目录行为保持。
- 在管理员授权 HOME 下以 descriptor-relative no-follow 打开固定 `.claude`，settings.json 优先、已有 claude.json 回退，明确激活时可创建缺失目录/文件；0600 发布。保留物理应用 guard，接任务 19–21 唯一 ProviderTransactions、发布前重算、pending 及启动恢复；补丁执行时重核对主机状态和目标档案。attached 模式、跨租户当前指针、未完成 pending 明确拒绝，不覆盖旧意图；旧事务由既有启动恢复所有者处理。外部编辑仍有任务 21 已注明的最终检查至 rename 窗口，不宣称文件系统 CAS。
- 已发布独有字段契约使用既有 ModeState.contract 随同一 pending/状态落定，直连状态保留已保存 proxy_route；后续目录编辑不会丢失清理依据，重投影能清掉旧独有字段。没有新建旁路快照或第二事务状态。
- 目录先 CAS 保存，之后 live 失败不会静默回滚目录。ClaudeResult 分别返回 catalogSaved 与 live=not_attempted/applied/failed/recovery_required；只有 applied 表示本次 live 已完成。发布开始后的失败保留 pending 供既有恢复，后续请求不会覆盖它；未发布的配置错误保留原文件和指针，可修复后 reapply。没有跨数据库/文件系统原子回滚承诺。

### 六入口、API 与 SDK

- 既有 `POST/PUT/PATCH /v1/app-providers/{app}/{id}` 在 Claude 分支接受 ClaudeWrite（targetHostId、claude 完整配置、activation），旧请求仍走原保存链；POST 成功 201。activation 缺省 catalog_only；first 只在无当前指针时激活，if_current 仅重投影当前卡，switch 显式切换。非当前编辑仅目录，不创建 native 文件或事务状态。
- `POST /v1/app-providers/claude/{id}/switch`、`.../reapply` 读取同一租户档案后走同一适配；`.../import` 读取授权真实配置，导入可表达的四类强类型档案，不改 live/指针，要求独立读、写授权。不能表达的多鉴权、helper 或其它关键字段明确 422，不静默丢弃。没有原始秘密读取 API。
- `POST /v1/universal-providers/{id}/sync/claude` 先消费既有 universal_sync 投影/存储所有者，再按显式 activation 应用到 live。原通用 sync 默认仍只写目录。最小扩展纯投影 Claude 分支以接受编辑成云配置的原统一子卡，重新同步时清除云凭据/选择器，并撤掉过时 typed claude 形状；其它应用算法未迁移。
- SDK `claude_provider_{create,save,update,switch,reapply,import,sync}` 走原生 transport、租户上下文，主机令牌 HeaderValue 标为 sensitive，错误响应体丢弃；WASM 明确拒绝。沿既有分域 router 和 Gateway 认证注册；在数据库/主机访问前按已有执行端策略校验 token、租户、主机与写能力，import 再核对读能力，客户端不能指定 HOME/private root。未认证/未授权 401/403，非法输入 400/422，缺失 404，竞争/待恢复状态 409，存储/运行边界失败不回传敏感内容。
- 本任务没有新增 Iced 或登录管理；带托管 authBinding/账号关联的档案不伪装登录切换成功。官方类型仅使用客户端已有登录；账号关联与真实登录处理不在本次验收内。

### 红绿与平台证据

各阶段从仓库根原子 mkdir 动态领取共享槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`；`CARGO_BUILD_BUILD_DIR` 同设此绝对路径，防止仓库 build-dir 绕过槽位。锁内记录计划/任务/会话/PID，每阶段 shell trap 在 Cargo/子进程结束后清理 owner 并 rmdir，自身锁已释放，编译缓存保留。

初始红阶段先 `cargo fmt --all`（0）：`cargo test -p vw-agent-apps --lib cc04_claude_ -- --nocapture` 退出 **101**，2 个有效测试分别在旧凭据未清除、新凭据未写入的业务断言失败；同过滤 DTO 命令退出 **101**，1 项发现 serde 官方 unit 变体接受多余字段的边界缺陷，改成严格空对象变体后通过。初次 Gateway 接线发生 E0282 返回类型推断错误，已修复，不将编译失败计为业务红证据。最终源码完整结果如下，均无忽略/零测试冒充验收：

| 确切命令 | 退出码 | 有效通过数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 非行为测试 |
| `cargo test -p vw-agent-apps --lib cc04_claude_ -- --nocapture` | 0 | 5 |
| `cargo test -p vw-api-types --lib cc04_claude_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-agent --lib cc04_claude_ -- --nocapture` | 0 | 9 |
| `cargo test -p vw-gateway-client --lib cc04_claude_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-agent --lib gateway::api::handlers::app_providers::create_tests -- --nocapture` | 0 | 4 |
| `cargo test -p vw-agent --lib gateway::api::handlers::app_providers::update_tests -- --nocapture` | 0 | 3 |
| `cargo test -p vw-agent-apps --lib cc04_universal_sync_ -- --nocapture` | 0 | 2 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

提交前审查另调整不存在切换目标的检查顺序，避免创建配置/事务目录；统一同步目录已保存后如目标回读失败，仍返回 catalogSaved=true/live=failed。随后再次动态领取槽位 1，在同一上述绝对 CARGO_TARGET_DIR 执行 `cargo fmt --all`（0）、`cargo test -p vw-agent --lib cc04_claude_ -- --nocapture`（0，9 项）及 `cargo clippy --all-targets --all-features -- -D warnings`（0）；缺失目标不创建目录的实际 SDK/HTTP 断言通过，复验不重复累计有效数量。

共 **25** 项通过，其中本任务 16 项、相邻回归 9 项。两个指定测试 cc04_claude_switch_cleans_previous_provider_keys / cc04_claude_switch_preserves_user_fields 均通过。真实回环 HTTP + SDK + 隔离 SQLite/临时主机文件证明四类依次切换及凭据清除、文件/指针结果、当前编辑、非当前目录编辑、读取导入、重投影、统一子卡云配置再同步、默认目录保存、缺文件首激活、legacy 路径、符号链接拒绝、权限/无效输入无写入、正式路由 Basic 认证、发布契约跨目录编辑的独有字段清理，以及目录保存/live 失败后的明确部分结果与修复重投影。全部为假凭据，没有替代上游/钥匙串/应用进程，没有读写真实用户配置。

实际 `sw_vers` 为 macOS **27.0 (26A428)**，`uname -m` 为 **arm64**，仓库 crate 版本 **0.6.0**。没有查询/启动真实 Claude 应用版本、真实账号、CLI 消费配置或 Iced 渲染，这些明确未验证。Agent lib test 仍有既有 `__eh_frame section too large` 链接提示，测试退出 0；未修改链接设置，最终强制 Clippy 退出 0。

定向 diff/暂存检查通过，仅提交本任务实现、独立测试、最小父注册/保存接线和本条正式证据；不纳入母任务状态、此前编号文件删除或其它计划。只读 plan.md 无差异。未运行全量 cargo test、fmt check、wasm 或 Makefile；完成后删除任务 26 编号文件，不提交其删除，正式证据保留。

## 任务 27：Claude 原生登录切换

2026-11-07；状态：已完成（macOS 隔离资源与假 Keychain；真实账号、真实 Keychain 和 UI 未验收）。

### 依赖、来源与授权边界

核对批次 26 提交 `4a23324e1` 与本文件任务 26；母任务 1、3 已完成，母任务 2 仍沿用任务 1 的底座子计划 1–93 开工裁定，不改报其完整迁移完成。首次因默认登录事务与绑定输入缺失登记阻塞；继续执行指令明确授权在现有 auth_accounts/CredentialFileStore/ClaudeAccountDirectory/租约所有者下补齐最小契约，依此解除。没有修改只读 plan.md，没有扩展母任务 6 的 OAuth 或全量账号管理。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，来源工作区干净。已读 `claude_official::{with_login_switch,switch_login,restore_login,recover_login_switch}`、`claude_direct::{write,run}`、provider/mod.rs 与 live.rs 直接调用者，以及 snapshot 的凭据键、资料缓存、Keychain 适配、默认登录迁入/切回相邻测试。使用 RustCodeGraph status 与定向搜索核对当前原生所有者；长时间无输出的 explore 已中断，不将其当作检索证据。冻结 upstream 没有加入原生运行链。

### 实现与调用链

- `vw-api-types/src/app_providers/claude_login.rs::ClaudeLoginBinding` 仅包含 accountId；`ClaudeConfig::Official` 增加可省略的 account，旧官方请求仍使用现有默认登录。未知字段、凭据、HOME/根路径与租户输入拒绝；Debug 遮蔽绑定。账号 ID 沿既有 identity 校验，无新的任意文件/命令 API。
- `AuthAccountStore::{claude_login_account,archive_claude_default}` 复用原元数据命名空间、执行主机与租户隔离的 CredentialFileStore 和原 credential lease。只能解析已登记的目录账号，令牌型记录不伪装为原生登录；租约覆盖登录和文件提交。默认旧账号按 oauthAccount/组织识别，已有账号回写最新材料，首次发现的旧登录也通过同一所有者登记 `native_<identity hash>` 目录，防止成功换号丢失旧登录；不自动授予该身份使用权限，不建立第二账号库。
- `vw-agent-apps/src/providers/claude_login.rs::LoginHost` 在受信 HOME 的固定 `.claude` 目录保存 0600、fsync 的暂态登录日志，原始材料不进入 DTO、错误或日志。保留默认槽位自己的 MCP/插件凭据与 `.claude.json` 全局键，按来源账号键替换登录与 oauthAccount，清理来源账号缓存。账号目录保留登录快照，离开默认账号前通过原所有者归档最新默认材料；没有声明与外部 CLI 同时刷新多份账号材料的一致性保证。
- `auth/claude_login_files.rs` 使用 no-follow、descriptor-relative 的固定子文件读写、原子私有文件替换、进程锁与 CLI 刷新/存储/资料目录锁。锁保持 heartbeat，仅可回收带自身标识且 PID 已退出的遗留锁，未知/活跃 CLI 锁保留并拒绝切换。此处仍不能提供外部文件系统 CAS，外部不遵守锁的写入存在最终检查窗口。
- `auth/claude_keychain.rs::MacOsKeychain` 是唯一可替代的 OS 边界：固定 `/usr/bin/security`，stdin 传送秘密，stderr 丢弃、输出与执行时间有界，写入后回读验证；支持 Keychain 未命中时的私有 `.credentials.json`。默认 Keychain 仅允许当前执行用户 HOME，配置目录覆盖、非 ASCII 目录哈希及超出 interactive 输入缓冲的载荷明确拒绝，不回退到将秘密置于进程参数的路径。未通过真实 Keychain 验证。
- 既有 `ClaudeFile::apply` 保持原调用语义，新增内部同链 `apply_with_login`，在相同 Contract/pending/state 保存登录操作标识。登录失败不提交 settings/指针；首次发布前文件失败恢复原登录与原文件凭据存储形态；已发布 pending 或已落定操作标识使登录跟随目标前滚。后续恢复不会错误回退已发布的配置；恢复失败保留日志并返回 recovery_required。检测到外部新登录/令牌变化时保留它并拒绝覆盖。日志还绑定原执行主机；同 HOME 的别名不能消费或清除其它执行主机的登录意图，启动恢复会延后该别名。
- `gateway/api/handlers/app_providers/claude_login.rs` 是既有 Host 的登录编排，复用 create/save/update/switch/reapply/sync 所用的 Claude 适配。增加绑定时沿原 SDK 请求类型传送 accountId，沿原分域路由返回 ClaudeResult；无需重复添加业务所有者或命令入口。除原 provider_config_write 外，还必须有同租户/主机的 official_account 身份许可；授权在账号/文件与目录写入前完成。catalogSaved/live 的部分成功语义沿任务 26 保留。启动工厂在配置事务恢复后协调同一登录日志；后续操作也检查该日志。
- SDK 沿用 `claude_provider_{create,save,update,switch,reapply,sync}` 的原 transport、敏感令牌头与错误体丢弃契约，新增独立序列化测试登记到真实 app_providers_api 父模块。没有新增 UI，Iced 与真实 CLI 消费明确未验证。

### 红绿与验证

所有阶段均从仓库根动态原子 mkdir 领取共享槽位 **1**；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同设此绝对路径，以覆盖仓库 build-dir。锁记录计划/任务/会话/PID；各阶段 shell trap 在 Cargo/产物进程退出后删除自身 owner 并 rmdir，编译缓存保留。所有下列命令继承此槽位；未运行全量 cargo test、fmt check、wasm 或 Makefile。

先将独立测试接线编译，再运行 `cargo test -p vw-agent-apps --lib cc04_claude_login_ -- --nocapture`：退出 **101**、2 项业务断言失败，分别证明旧实现没有拒绝缺失登录、没有将目标登录应用到默认槽位。初始接线编译失败、SDK 零测试及 Gateway 夹具错误均已修复，不计作业务红证据或验收。最终源码先运行 `cargo fmt --all`（0），结果如下：

| 确切命令 | 退出码 | 有效通过数量 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_claude_login_ -- --nocapture` | 0 | 10 |
| `cargo test -p vw-api-types --lib cc04_claude_login_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-agent --lib cc04_claude_login_ -- --nocapture` | 0 | 2 |
| `cargo test -p vw-gateway-client --lib cc04_claude_login_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-agent-apps --lib cc04_claude_ -- --nocapture` | 0 | 15（含上述 10） |
| `cargo test -p vw-agent --lib cc04_claude_ -- --nocapture` | 0 | 11（含上述 2） |
| `cargo test -p vw-agent --lib agent_apps::auth_accounts::tests -- --nocapture` | 0 | 5 |
| `cargo test -p vw-agent --lib gateway::api::handlers::agent_apps::recovery_tests -- --nocapture` | 0 | 1 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

去重为 **34** 项通过：本任务 14 项、相邻回归 20 项。两个指定行为测试 `cc04_claude_login_failure_leaves_pointer_and_settings`、`cc04_claude_login_write_failure_restores_original` 均通过；另外证明部分 Keychain 写入失败立即恢复、空登录/文件 fallback 恢复、未发布重启回退、发布后失败与统一 pending 前滚、外部新登录保留、活跃 CLI 锁不删除、真实 Keychain 适配拒绝其它 HOME，以及同 HOME 的执行主机别名拒绝消费其它主机日志。

Gateway 使用真实回环 HTTP、原 SDK、临时 SQLite、真实临时主机目录与假账号材料：强类型绑定创建后 settings 清除旧第三方凭据、文件/指针与登录目标一致，旧登录由同一账号库保留；无效 token/无身份许可无目录记录和配置写入；允许但不存在的账号返回 live=failed，settings/指针不变；预先损坏真实 settings 文件，使登录换入后的文件提交失败，返回 catalogSaved=true/live=failed，原登录恢复且无指针发布。SDK 测试还验证真实路由/序列化与错误体脱敏。

仅替代 Keychain 接口；假返回形状为 `{"claudeAiOauth":{"refreshToken":"fake-..."},"organizationUuid":"org"}`，资料为 `.claude.json` 的 oauthAccount.accountUuid/organizationUuid。没有读写真实用户登录/配置、没有执行真实 security 或 OAuth。发布后故障使用既有内部事务 checkpoint，文件/数据库/pending/恢复均走真实资源，未以 mock 次数证明行为。

实际平台 macOS **27.0 (26A428)**、**arm64**，crate 版本 **0.6.0**。未查询/启动已安装 Claude 版本、真实账号、真实 Keychain、CLI 消费配置或 Iced 渲染，这些均未验证。Agent lib test 仍有既有 `__eh_frame section too large` 链接提示，相关测试退出 0；没有修改链接设置，强制 Clippy 退出 0。

定向 diff/check 与暂存审查通过；提交仅含本任务实现、独立测试、最小 DTO/工厂/SDK 注册与本条证据，不纳入母任务状态、此前编号文件删除或其它计划。只读 plan.md 无差异；任务完成后删除编号文件且不提交其删除，正式证据保留。

## 任务 28：Codex 路由表投影

2026-11-07；状态：已完成（授权的纯路由投影边界；没有提交 live）。

### 依赖、来源与边界

核对批次 27 提交 `51725a50e` 及本文件任务 27 的 34 项去重测试/Clippy 证据。母任务 1、3 已完成，母任务 2 继续沿用任务 1 的底座子计划 1–93 开工裁定，完整领域迁移仍未收口。首次因投影与原生提交的边界不清登记阻塞；继续执行指令已明确：28 提供内部原生 TOML 与严格拒绝，29 处理 auth/暂存，30 统一提交；SDK/API 按只读行为证明文件不变，解除原阻塞。不修改只读 plan.md，不提前实现 29/30。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，工作区干净。已核对 `live/project/codex.rs::{CodexProjection::of,third_party_route,custom_route,check_effective_route}` 及路由/自有认证/旧形态相邻测试，`services/provider/codex_direct.rs::{project,plan,run_with_edits}`、`codex_editor` 与 `codex_terminal` 直接调用者，以及 golden `codex_red_lines.rs` 的 CX-01/04/05/06/11/13。原生继续复用任务 22 的路由解析，不接入冻结 upstream 或新增第二个业务所有者。RustCodeGraph status/explore 与定向读取/rg 已执行，未将索引中的 upstream 命中误作当前运行实现。

### 实现与调用链

- 新增 `vw-agent-apps/src/providers/codex.rs::project`：接收真实读取的 live TOML、目标 settings、已知前所有者及登录所有者提供的 `login_on_disk` 事实，产生仅供执行端消费的 `NativeProjection`。它不读写主机文件、选择登录保留策略或发布事务；29/30 可消费 `config()`。`NativeProjection` 没有 Serialize，Debug 固定脱敏，错误是无载荷的强类型枚举。
- 原生路径直接调用唯一 `codex_route::CodexProjection::of`，绕开编辑草稿的 keyless 占位回退；真正结果不剥离必要 bearer。校验 config/auth 形状和有界输入，拒绝所消费的显示掩码、pending key 和错误类型；keyless 官方登录回退返回 MissingCredential，不生成可发布的占位配置。
- `projection/codex.rs` 将已有字段归属、TomlPatch 和最终 profile 检查提取为共用 `apply`，供原生与编辑路径共同消费；没有复制第二套路由/写入规则。key 仅进入归一后的 `model_providers.custom.experimental_bearer_token`，旧顶层 bearer/base URL 清理；非目标用户表、项目、非活跃 profile 保留。env_key、auth/aws、Authorization http_headers/env_http_headers 等来源认可的表级自有认证保留，不注入行 key。来源中 requires_openai_auth=true 的头认证回退规则仍沿唯一解析器处理，不扩大到“所有普通头都替代 key”。
- 活跃 profile 重定向 model_provider、openai_base_url 或 experimental_bearer_token 时整次拒绝，无部分结果。对照来源补正既有预览判定：省略顶层 selector 时内置 openai 与 profile 显式 openai 是同一路由；profile 名/值按来源 trim，空字符串不作重定向。同一目标路由允许，非活跃 profile 不阻止投影。
- 对外复用既有 `ProviderPreviewRequest/ProviderPreview`、`POST /v1/app-providers/codex/config/preview` 和 `GatewayClient::app_provider_preview`，没有新增可返回原生明文 TOML/key 的接口、配置源、文件/命令 API、权限或 UI。编辑路径仍允许缺 key 草稿预览并移除重复 bearer，最终整棵显示结果继续脱敏；原生路径和显示路径不可互换。当前已有登录、官方卡、非 file 凭据存储、托管 metadata、生成模型目录等既有不可用边界未扩展。

### 红绿与验证

所有阶段从仓库根按原子 mkdir 动态领取共享槽位 **1**；实际 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，同时设置 `CARGO_BUILD_BUILD_DIR` 为此绝对路径，覆盖仓库 build-dir。锁记录计划/任务/会话/PID；shell trap 覆盖正常、失败及中断退出，在 Cargo/产物进程结束后释放自身 owner/锁，保留缓存。

先将原生入口接到现有编辑结果，独立失败测试编译后，四条指定命令退出 **101/0/0/0**，有效运行数 **4/1/1/1**：业务包中 2 项失败（原生结果被剥离 key、原生错误放行 keyless fallback），其余通过。profile/表级认证行为本已存在，未伪造失败。额外命令 `cargo test -p vw-agent-apps --lib cc04_codex_native_default_profile_does_not_override_route -- --nocapture` 退出 **101**，有效 1 项，在默认 openai profile 被误拒绝的业务断言失败，随后按来源修正。各阶段先 `cargo fmt --all`，均退出 0；没有以缺模块或零测试当红证据。

最终源码先 `cargo fmt --all`（0），同阶段完整验证如下：

| 确切命令 | 退出码 | 有效通过数量 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_codex_ -- --nocapture` | 0 | 8 |
| `cargo test -p vw-api-types --lib cc04_codex_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-agent --lib cc04_codex_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-gateway-client --lib cc04_codex_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-agent-apps --lib cc04_preview_codex -- --nocapture` | 0 | 2 |
| `cargo test -p vw-agent --lib cc04_preview_ -- --nocapture` | 0 | 3 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

共 **16** 项通过（本任务 11 项、相邻预览回归 5 项），无忽略。指定 `cc04_codex_key_is_in_active_route_table`、`cc04_codex_profile_override_rejects_without_side_effects` 均通过。另验标准/内联表、旧顶层地址、保留 ID 表和嵌入 token 的路由归一、行 key 仅在目标表出现一次、自有认证无注入、登录事实控制 requires_openai_auth、占位/错误形状拒绝，以及默认/同路由/非活跃/带空格 profile 语义。

Gateway 用真实临时 `.codex/config.toml`、`auth.json`、管理员读权限策略及回环 HTTP，经原 SDK→实际 API→执行端读配置：普通投影响应脱敏，三种覆盖和带空格 profile 名均返回 unavailable=ownership、空显示/空差异；metadata-only 主机返回 403。每种场景断言 config/auth 原字节不变、目录仍仅两个文件，没有锁、暂存或 pending。DTO 拒绝 publish/authFile/loginOnDisk/tenant/file 扩权字段；SDK 另验证正式方法/路径/请求体/租户头与错误载荷脱敏。SDK 单测的本地响应夹具为 400、带假 secret 的对象；真实 Gateway 集成没有替代文件/业务投影/鉴权。所有凭据均为假值，没有访问真实用户配置、真实账号、Keychain 或网络上游。

实际平台 macOS **27.0 (26A428)**、**arm64**；crate 版本 **0.6.0**。已验证真实本机隔离文件读入、内存原生 TOML 和只读 API/SDK；未查询/启动已安装 Codex 版本，真实账号、CLI 消费、Iced 渲染和 live 提交均未验证。登录/暂存及统一提交明确归 29/30。Agent lib test 仍出现既有 `__eh_frame section too large` 链接提示，测试退出 0；未改变链接配置，最终强制 Clippy 退出 0。

定向 diff/check 与暂存审查通过；仅提交本任务实现、独立测试、最小父注册和本条证据。保留其他母任务状态和此前编号任务删除；plan.md 无差异。各阶段自身槽位锁已释放、缓存保留。未运行全量 cargo test、fmt check、wasm 检查或 Makefile。完成后删除任务 28 编号文件，不提交其删除；正式证据保留。下一步任务 29 消费原生投影并规划登录事实。

## 任务 29：Codex 登录暂存与轮换

2026-11-07；范围：按继续执行裁定交付内部登录规划/事务准备及统一删除/恢复能力；完整供应商 SDK/API live 切换验收归任务 30，本批没有接通或宣称该闭环。

### 依赖、来源与实现边界

批次 28 提交 `a896ac028` 及本文件任务 28 已交付纯路由投影；母任务 1、3 已完成，母任务 2 继续沿此前底座子计划 1–93 的开工裁定，不改报全部迁移完成。本任务首次登记的 29/30 所有者缺口，已由继续指令明确解除：29 准备登录/暂存和共享事务删除，30 消费并统一提交。不修改只读 plan.md，不提前实现供应商切换 API。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，来源工作区干净。已读 `services/provider/codex_login.rs::{LoginStash,plan,official}` 与相邻测试、`codex_direct.rs::{load_stash,run_with_edits}` 及 provider/editor 调用者、golden `codex_red_lines.rs` 的当前登录/登出保持场景；原生登录分类沿冻结 `codex_config.rs` 的用户身份、凭据、模式和残留分类函数。先前 RustCodeGraph status/explore 与定向源码检查已核对原生与 upstream 的区别，未把冻结快照当运行实现。

- `providers/codex_login.rs::prepare` 只读取已授权且已持锁的 Codex 目录；调用者先恢复 outstanding intent，再传入既有账号所有者/目录的执行端事实。返回不可序列化、Debug 脱敏的 `PreparedLogin`；`changes()` 提供固定 auth/stash 两个 FileChange，`login_on_disk` 供任务 28 的投影消费。没有独立提交方法，没有客户端路径、命令或明文登录响应。
- 唯一规划在 `codex_login_policy.rs`；分类在 `codex_login_material.rs`。按用户与 workspace 区分暂存；CLI 刷新后的 live 优先于行内旧快照；旧行仅在暂存第一次建立或冻结升级规则允许时补种。离开已登出的官方卡使对应旧暂存失效；保留登录的第三方切换不重放旧登录。只删除有已知第三方 key 证据的残留，不误删用户自己的 API key；托管登录不进入暂存。代理仅在明确 restore_last 且 token 未过期时恢复最近登录；官方恢复沿来源规则。JWT 仅用于本地身份/到期分类，不作为授权验证。
- 暂存固定为同一 `.codex` 物理目录的 `shiju-login-stash.json`，0600、设备本地，不写回供应商行、不云同步、不新建永久账号库，也不自动发现/迁移用户的 `.cc-switch` 文件。这样 auth、stash 与 config/catalog 可以消费同一个目录 guard、同一 ProviderTransactions intent。没有读取真实用户配置。
- `live/stage.rs::FilePatch` 明确区分 Some(bytes) 与 None 删除；既有 Vec 返回闭包通过 blanket 实现保持原调用方式。`WholeFile` 绑定准备快照，在 staging 和发布前重读时拒绝外部轮换。`write_guard.rs::PreparedWrite` 通过保留目录描述符的 unlinkat 真正删除 auth，空文件与 `{}` 都不冒充登出。删除先准备私有空暂存作为 intent 资源，提交/恢复沿原所有者 fsync；不在事务外删除 auth。
- `commit/concurrency/recovery` 共同保存、发布、恢复可空的 planned 摘要；含删除的 intent 为版本 2，旧版本执行器明确拒绝。原版本 1 写入 intent 继续可读；缺失 planned 字段拒绝，不隐式当作删除。未发布 intent 丢弃，已标记/部分发布前滚；目标已不存在时删除恢复幂等，外部新登录/不安全目标保留并返回既有 RolledForwardExcept。调用者必须检查恢复结果并重新规划；逐文件发布仍不是文件系统多文件原子 CAS，不承诺外部 CLI 不遵守锁时不存在最终检查窗口。
- 不新增 API、DTO、SDK 或 UI，也不添加虚构的只读端点凑测试数量。按继续裁定，原计划另外三包的 cc04_codex_login_ DTO/Gateway/SDK 命令本批不适用，未运行零测试充数。任务 30 需在真实供应商入口完成鉴权、账号引用/租约、准备后 config 再校验、组合 changes、目录/指针提交与 SDK→API→执行文件验收。当前 helper 对非 file/未知凭据存储、不可解析 auth/stash、未来暂存版本、符号链接等明确拒绝，不伪装支持系统 Keychain。

### 红绿与验证

各阶段从仓库根动态原子 mkdir 领取共享槽位 **1**；实际 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，同时设置 `CARGO_BUILD_BUILD_DIR` 为此绝对路径以覆盖仓库 build-dir。锁记录任务/计划/会话/PID，shell trap 在 Cargo 及子进程结束后清理自己的 owner/锁，保留缓存。

首轮 `cargo fmt --all` 退出 0；将准备输出接到原文件基线后，`cargo test -p vw-agent-apps --lib cc04_codex_login_ -- --nocapture` 退出 **101**，有效 **8** 项（3 通过、5 在业务断言失败）：指定 roundtrip/logout 测试分别证明没有实际删除 auth、旧暂存未作废，另有轮换冲突与恢复断言失败。没有把缺模块/编译失败/零测试当红证据。随后新增 schema 测试曾因夹具调用私有 save 编译失败，已改为写真实隔离 pending；首轮完整 Clippy 发现本次迁入的 collapsible_if，已修复，不归咎既有问题。

最终源码先执行 `cargo fmt --all`（0），同阶段命令如下：

| 确切命令 | 退出码 | 有效通过数量 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_codex_login_ -- --nocapture` | 0 | 18 |
| `cargo test -p vw-agent-apps --lib providers::live:: -- --nocapture` | 0 | 49 |
| `cargo test -p vw-agent-apps --lib cc04_claude_ -- --nocapture` | 0 | 15 |
| `cargo test -p vw-agent-apps --lib cc04_codex_ -- --nocapture` | 0 | 26（含上述 18） |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

共 **90** 项去重通过（本任务 18、相邻回归 72），无忽略。指定 `cc04_codex_login_roundtrip_keeps_latest_login`、`cc04_codex_login_logout_is_not_resurrected` 通过。独立测试还覆盖 Intent/Marked/FilePublished/Published/Settled 五处故障后重建执行器恢复、真实 auth 删除及私有 stash、外部轮换拒绝/恢复保留、workspace 区分与旧暂存升级不复活登出用户、托管与残留不入暂存、过期代理恢复规则、只读准备/脱敏、坏资源拒绝、空文件区别、删除 intent schema 与版本保护、missing 删除无副作用，以及 auth/stash/config 三文件与指针的同一 intent 前滚。组合测试使用内部固定补丁，证明可组合事务能力，不代表已经交付供应商 live 入口。

全部使用真实临时目录/JSON/文件、统一 pending/恢复和假 token；没有替代文件系统或事务所有者，没有访问真实账号、网络上游或 Keychain。实际 macOS **27.0 (26A428)**、**arm64**，业务 crate 版本 **0.6.0**。未查询已安装 Codex 版本，真实账号轮换、真实 CLI 消费、Iced 渲染及供应商 SDK/API 切换均未验证，后者明确由任务 30 验收。未运行全量 cargo test、fmt check、wasm 或 Makefile。

定向 diff/check 与暂存审查通过；提交只包含本任务实现、底层删除/恢复支持、独立测试、最小模块注册与本条正式证据。既有母任务状态、其他计划、此前编号文件删除不纳入提交；plan.md 无差异。各阶段自己的槽位锁已释放，缓存保留。状态：**已完成（本次授权的内部准备边界）**；任务 30 消费本实现，完成后删除任务 29 编号文件而不提交其删除。

## 任务 30：Codex 配置与目录指针提交

2026-11-07；状态：已完成（本任务的保存目录统一提交行为；不代表母任务 5 全部交付）。

### 前置、来源与本次裁定

批次 29 为 `2b802c64c`，28 为 `a896ac028`；母任务 1、3 已完成，母任务 2 沿任务 1 已有的底座子计划 1–93 开工裁定，完整迁移仍未收口。首次检查因母任务 5 尚无原生目录契约登记阻塞；继续指令明确允许在唯一模型目录领域所有者补齐最小保存目录读取/投影与租户持久化，不等待网络发现、预设迁移或 OAuth，因此解除阻塞。已读母任务 5 只读设计与共享契约子任务；未改任何只读 plan.md，也没有将母任务 5 改报完成。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，来源工作区干净。核对 `codex_official_models::{OfficialLogin::of,rows_for_switch}` 的缓存身份/不刷新用户 token/锁外网络规则、`codex_direct::{plan,run_with_edits}` 及 `codex_editor`/`codex_terminal` 调用者、`codex_config::{plan_codex_model_catalog,codex_native_row_is_valid,CODEX_CATALOG_PARSER_REQUIRED_FIELDS}` 与相邻测试，以及 golden `codex_red_lines::model_catalog_pointer_ownership`。网络发现与模板知识留给母任务 5；本项不读取 Codex 的 models_cache.json，也不调用冻结 upstream 作为业务实现。

### 唯一目录所有者与实际调用链

- `vw-api-types/src/model_catalog.rs` 定义本行为实际消费的 `ModelCatalogIdentity`、`SavedCodexCatalog`、`CodexNativeModel`。身份为执行主机/应用/供应商/精确端点，租户来自既有认证请求上下文；模型 slug 保留请求身份。消费已经准备的原生目录行，不从简化 modelCatalog 猜模板、补能力或制造官方列表。两个来源解析必需能力 bool 显式输入，未知 context_window 保留 None；未知输入字段拒绝。
- 领域验证/原生 JSON 投影仅在 `vw-agent-apps/src/models/mod.rs`；持久化适配仅在 `vw-agent/src/model_catalog.rs::ModelCatalogStore`。复用 GatewaySettingsStore 的租户 SQLite/Postgres 门面和固定 `model_catalog.saved` 命名空间，不在 providers 内复制目录存储。保存为 SHA-256 标识的不可变部署修订；provider 行 CAS 绑定 `modelCatalogRevision`，读取必须同时匹配租户、主机、应用、供应商、端点和修订内容。并发保存失败可能保留未引用的目录修订，但不能使 live 混用另一请求的目录。后续母任务 5 扩展上述同一所有者；本项仅实测 SQLite，没有声称验证 PostgreSQL。
- `vw-agent-apps/src/providers/codex_catalog.rs::CodexFiles` 从授权 HOME 的保留描述符打开固定 `.codex` 并持有物理目录 guard，消费任务 28 的唯一原生路由投影和任务 29 的登录准备。`config.toml`、受管目录 `shiju-model-catalog.json`、`auth.json` 与 `shiju-login-stash.json` 进入同一 ProviderTransactions intent，供应商指针及 direct mode 在同一 intent 中落定；需要删除的 auth/目录是真删除。四个文件均按私有权限准备，没有往 TOML 内联私有 modelCatalog。
- 无目录或空列表是合法无目录：清除受管目录指针和受管文件；供应商自己声明的外部指针优先，仅复制 TOML 值，不读取该任意路径。来源按文件名识别受管指针，旧绝对路径中的 `cc-switch-model-catalog.json` 和当前受管文件名也不能在无保存目录时残留；不迁移或删除旧任意目录下的文件。
- 目录/登录准备后重新投影并检查 active profile；WholeFile 绑定 config/auth/stash/catalog 的准备快照。新增有真实调用者的 `ProviderTransactions::commit_checked` 在暂存后和发布标记前复核保存行、目录修订与执行端状态；已有 commit 沿原签名委托空校验。发布开始后仍按冻结 intent 前滚，不承诺文件系统多文件原子 CAS；外部修改保留并返回恢复结果，调用入口要求重新规划。
- direct contract 保存本次已提交的 exclusive 所有权，供当前供应商先编辑目录记录、后 reapply 时清理旧窗口字段；复用原投影的 exclusive 参数，没有复制路由/补丁所有者。
- API DTO 为 `vw-api-types/src/app_providers/codex_catalog.rs`；Gateway 为对应 handler，注册进入既有 app_providers router，由 router.rs/bootstrap.rs 的既有分域注册消费。`PUT /v1/app-providers/codex/{id}/config` 保存及按 catalog_only/first/if_current/switch 决定提交；POST `/switch`、`/reapply` 消费已保存记录。SDK 为 `codex_provider_save/switch/reapply`，沿真实 native HTTP、租户头、敏感令牌头及错误体丢弃契约。
- 写权限在目录/数据库访问前由既有 provider_editor_context 校验；非授权租户/主机或无写权限拒绝；live 提交另拒绝附着/代理状态或其它租户指针。eligible live 操作的 profile 预检查在保存数据前拒绝重定向，错误不含 TOML/凭据。保存目录记录与 live 是显式部分成功语义：catalogSaved=true 仍须检查 live=applied/failed/recovery_required。恢复任何旧 intent 后返回 recovery_required，不把旧输入继续当作本次提交。
- 本项消费任务 28 已支持且能确定端点的直连路由；未提供官方/托管账号切换 API，也不伪造账号引用或租约。已有托管绑定/托管标志、非 file 凭据存储、不能归属端点的配置明确拒绝。实际验证了本机原生登录的保留、暂存和清除，不宣称托管账号、Keychain 或 OAuth 切换通过。没有新增 UI；Iced 编辑入口由后续任务消费本 SDK。

### 红绿与最终验证

各阶段均从仓库根按原子 mkdir 顺序动态领取共享槽位 **1**；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同设该绝对路径，以覆盖仓库 build-dir。锁记录计划/任务/会话/PID；shell trap 覆盖正常、失败与中断，在 Cargo/产物进程退出后清理自身 owner/锁，缓存保留。未使用其它 target-dir 或独占全仓缓存。

红阶段先接通编译、使用尚未消费保存目录的事务基线。四条指定过滤命令的退出码为 **101/0/101/0**，有效数量 **4/1/2/1**；Gateway 在“配置缺少受管目录指针”的业务断言失败。随后业务包 6 项的明确断言同时证明指针 None 与 intent 只有 auth/stash/config 三文件，未包含目录，退出 101；补充来源绝对受管指针用例后 9 项中 1 项在业务断言失败，退出 101。最初 String→静态错误的接线编译错误、私有 guard 读取接口编译错误，以及缺少 alg 的假 JWT 夹具错误均已修复，不作为预期红证据或通过证据；没有把零测试当验收。

最终源码先运行 `cargo fmt --all`（0），以下命令使用同阶段上述槽位：

| 确切命令 | 退出码 | 有效通过数量 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_codex_catalog_ -- --nocapture` | 0 | 9 |
| `cargo test -p vw-api-types --lib cc04_codex_catalog_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-agent --lib cc04_codex_catalog_ -- --nocapture` | 0 | 6 |
| `cargo test -p vw-gateway-client --lib cc04_codex_catalog_ -- --nocapture` | 0 | 2 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

共享事务增加发布前校验，故对相邻消费者执行聚焦回归（重新动态领取同一实际槽位 1，源码未再变化）：

| 确切命令 | 退出码 | 有效通过数量 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_codex_ -- --nocapture` | 0 | 35 |
| `cargo test -p vw-agent-apps --lib providers::live:: -- --nocapture` | 0 | 49 |
| `cargo test -p vw-agent-apps --lib cc04_claude_ -- --nocapture` | 0 | 15 |
| `cargo test -p vw-agent --lib cc04_codex_ -- --nocapture` | 0 | 7 |
| `cargo test -p vw-agent --lib cc04_claude_ -- --nocapture` | 0 | 11 |
| `cargo test -p vw-api-types --lib cc04_codex_ -- --nocapture` | 0 | 2 |
| `cargo test -p vw-gateway-client --lib cc04_codex_ -- --nocapture` | 0 | 3 |

去重 **122** 项通过，其中本任务 **18** 项、相邻回归 **104** 项，无忽略。两个指定行为 `cc04_codex_catalog_pointer_matches_committed_provider`、`cc04_codex_catalog_failure_is_recoverable` 均通过。实际故障覆盖 Intent/Marked/FilePublished/Published/Settled 五处；重建执行器后未发布丢弃、已标记前滚，目录/配置/指针一致。还验证四文件同一 intent、发布标记前输入有效性校验变为失败时拒绝并丢弃（仅此校验回调使用阶段 hook 改变布尔有效性夹具，数据库修订隔离另由真实 SQLite 测试证明）、外部目录修改保留、exclusive 编辑后 reapply、用户目录指针优先及旧受管绝对路径清除。

Gateway 使用真实临时 SQLite、授权策略、回环 HTTP、原 SDK 和真实临时 `.codex` 文件，证明：保存有目录→首次激活→保存无目录→切换清除→切回恢复；配置不含私有 modelCatalog，API 成功/错误不回传 key；非法 token/缺写权限没有记录或文件副作用；active profile 拒绝保持文件不变；保留本机登录与暂存清除；坏 auth 时返回 catalogSaved=true/live=failed 且旧文件/指针保持；部署/租户/修订隔离。API 恢复场景使用真实私有 staged 文件和 Pending schema 表示已有中断意图，SDK 请求先恢复两文件并返回 recovery_required，下一次请求才重新提交；实际运行中的五阶段注入由业务包测试证明，没有把手工 pending 夹具当作自然崩溃或 mock 次数证明。

所有凭据均为假值，JWT 只用于本地身份分类；未替代文件系统、数据库、pending 事务或 Gateway 鉴权，没有访问真实账号、Keychain、网络模型上游或真实用户配置。平台为 macOS **27.0 (26A428)**、**arm64**，crate **0.6.0**。`codex --version` 退出 0，记录本机随应用安装的 **codex-cli 0.166.0-alpha.6**；只查询版本，未运行 CLI 消费本次配置，真实账号/CLI 会话/Iced 渲染未验证。Agent lib test 继续有既有 `__eh_frame section too large` 链接提示，测试退出 0；未改链接设置，强制 Clippy 退出 0。

定向 diff/check、只读 plan.md 无差异检查与暂存审查通过；仅提交本任务代码、独立测试、最小注册及本条正式证据，保留其它母任务状态、此前编号文件删除及其它计划。全部自身槽位锁已释放，缓存保留；未运行全量 cargo test、fmt check、wasm 或 Makefile。任务完成后删除编号任务文件，不提交其删除；正式证据保留。下一步为批次 31。

## 任务 31：Gemini 双文件事务切换

2026-11-07；状态：已完成。本条仅记录 Gemini 官方 Google 登录与第三方 API key 的双文件切换，不代表真实账号登录、CLI 消费或 Iced 验收。

### 前置、来源与实现

核对批次 30 提交 `c1e1c3fef` 及任务 30 的正式验证证据。母任务 1、3 的当前状态为已完成；母任务 2 仍未整体收口，沿用任务 1 与 provider-contracts.md 已记录的底座子计划 1–93 开工裁定，不将其改报完成。只读 plan.md 未修改。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 均退出 0、来源干净。阅读 `gemini_direct::{projection,switch_to,reapply,run_with_edits}`、`gemini_auth::{detect_gemini_auth_type,is_google_official_gemini}`、`live/project/gemini::GeminiProjection::{of,env_patch,settings_patch}` 及相邻测试；直接调用者为 ProviderService 的首次供应商与 switch 分支、`provider/live.rs` 的 reapply 和 `gemini_editor::{view,write_live,plan_save}`。另核对 golden `mcp_bytes::gemini_switch_keeps_mcp_servers` 与 `file_modes`。来源测试只读，未运行。RustCodeGraph status 可用，explore 查询长时间无输出后终止，当前新增模块/未覆盖调用链用定向源码读取核对。

- `vw-api-types/src/app_providers/gemini.rs` 定义 GeminiProfile、显式 Official/ThirdParty 认证、模型、host target 及 catalog_only/first/if_current/switch 激活方式。未知字段、客户端文件路径和官方卡内的 API key 拒绝；秘密 Debug 脱敏。官方登录类型显式选择，不从供应商名称猜认证方式，不导入 OAuth token 或托管账号快照。
- `vw-agent-apps/src/providers/gemini.rs::{settings,patches,GeminiFiles}` 直接消费既有 `projection::gemini` 和 `fields::gemini_floor_env/GEMINI_FLOOR_SETTINGS`，未复制投影规则；使用已有 DotenvPatch/JsonPatch 保留用户行、JSONC 注释、MCP 和设置。只替换受管 env 键、security.auth.selectedType 与 model.name；官方切换清除旧 key、端点/Google 环境和旧模型，保留 GEMINI_SANDBOX、DEBUG、认证/模型对象的其它键及 MCP。新 DTO 的 model 对应 settings.json 的 model.name；其它既有受管 env 会清理，不新增任意 env/文件编辑 API。
- HOME 来自执行主机授权策略；固定 `.gemini` 子目录通过 retained descriptor 与 no-follow 打开并持有既有物理目录锁。`.env`、settings.json 绑定 WholeFile 快照，均以 0600 准备，进入同一 ProviderTransactions intent；当前供应商指针和 direct mode 同次落定。目录状态、保存行及前供应商在发布标记前重新校验。复用已交付的 commit_checked/pending/recovery，无第二事务引擎、无事务外文件写入。
- 发布仍是逐文件 rename，不能当作文件系统多文件原子 CAS。未发布意图丢弃、已标记意图前滚；发生外部修改时保留外部文件并报告恢复结果。实测部分发布后 settings.json 被外部修改时，恢复返回 RolledForwardExcept，目标指针按既有恢复契约落定；调用入口先返回 recovery_required，下一次请求才重新规划，不把保留外部内容后的状态报作本次 live 成功。
- `vw-agent/src/app_providers/gemini.rs` 沿既有 AppProviderStore/租户 namespace 与 CAS 保存目录行，保留元数据，列表/摘要识别显式官方档案。API handler 在 `gateway/api/handlers/app_providers/gemini.rs`，经既有 app_providers/router/bootstrap 分域注册，沿 provider_editor_context 在数据库和目录访问前校验主机写权限。
- SDK `gemini_provider_save/switch/reapply` 对应 `PUT /v1/app-providers/gemini/{id}/config`、`POST /switch` 与 `/reapply`。保存目录、首次激活、当前编辑、显式切换和重新应用均接同一适配器；目录保存与 live 结果分开，调用者须检查 catalogSaved 和 live，不能由目录保存成功推断 live 已成功。跨租户指针、代理/附着状态及已有 authBinding/liveConfigManaged 明确拒绝；未实现托管账号租约或 OAuth 授权。无新增 Iced 页面，后续 UI 使用本 SDK，通用全局编辑器仍不声称支持 Gemini 写入。
- dotenv 新值使用可保留字面量的引号，避免将 JSON 转义误当 dotenv 转义；不能无损表示的值明确拒绝。SDK 令牌头为 sensitive，HTTP 错误体丢弃；成功响应仅返回结果枚举，不返回 key/配置。

### 红绿与验证

从仓库根按原子 mkdir 顺序动态领取共享槽位；格式化与最终验证实际槽位 **1**、红阶段实际槽位 **2**：

- 槽位 1：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。
- 槽位 2：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。

各阶段同时将 CARGO_BUILD_BUILD_DIR 设为对应绝对路径以覆盖仓库 build-dir。锁内记录计划/任务/会话/PID，shell trap 覆盖正常、失败与中断；Cargo/产物进程结束后才清理自身 owner/锁，保留缓存。没有另建任务构建目录或使用其它 target-dir。

红阶段先接通可编译基线，仅提交 env 而 settings 保持旧内容。四条指定过滤命令退出码依次 **101/0/101/0**，有效数量 **4/1/2/2**：业务包 1 通过、3 在业务断言失败，指定双文件测试实际读到 oauth-personal 而非 gemini-api-key；官方切换仍有旧模型，恢复意图未含两个变化文件。Gateway 1 通过、1 失败，当前供应商编辑后的 model.name 为 Null 而非 edited。没有用缺模块、编译失败或零测试代替红证据。

最终源码先执行 `cargo fmt --all`（退出 0），以下四命令同阶段使用槽位 1：

| 确切命令 | 退出码 | 有效通过数量 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_gemini_ -- --nocapture` | 0 | 9 |
| `cargo test -p vw-api-types --lib cc04_gemini_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-agent --lib cc04_gemini_ -- --nocapture` | 0 | 6 |
| `cargo test -p vw-gateway-client --lib cc04_gemini_ -- --nocapture` | 0 | 2 |

共 **18** 项通过，无忽略。指定 `cc04_gemini_switch_commits_both_files` 和 `cc04_gemini_official_switch_removes_only_managed_keys` 通过；另覆盖 Intent/Marked/FilePublished/Published/Settled 五处故障后重建事务执行器恢复、意图确含双文件、发布前保存输入变化拒绝、外部修改保留并报告异常、JSONC/MCP 字节和非受管 env 保留、秘密标点/反斜杠、坏 JSON/密钥/符号链接拒绝及文件私有权限。

Gateway 以真实临时 SQLite、授权策略、回环 HTTP 和原 SDK 验证 SDK 请求→API 响应→执行端临时文件/指针：第三方首次激活、当前档案编辑重投影、官方目录仅保存后显式切换、重新应用；非当前 first/if_current 不改 live。错误 token、缺写权限、错误主机/租户、托管绑定、坏 live JSON 均无目标文件或目录行副作用。恢复 API 测试用真实私有 staged 文件和 Pending schema 构造已有中断意图，先恢复两文件并返回 recovery_required，下一次才应用；不是自然崩溃，实际五阶段故障由业务包 hook 测试证明。

所有凭据均是假值；没有替代文件系统、数据库、事务或 Gateway 鉴权。仅故障点/保存输入有效性通过内部阶段 hook 控制；未访问真实用户配置、真实账号、Keychain 或网络模型上游。平台 macOS **27.0 (26A428)**、**arm64**，业务 crate **0.6.0**；`gemini --version` 退出 0 为 **0.66.0**，仅查询版本。真实 Google 登录、真实 Gemini CLI 会话消费、Iced 渲染和非 macOS 未验证。首轮本次 handler 的 unused variable 警告已修复，最终测试只余既有 `__eh_frame section too large` 链接提示，未改链接配置。

强制 `cargo clippy --all-targets --all-features -- -D warnings` 使用重新动态领取的槽位 1 和相同绝对 CARGO_TARGET_DIR，退出 **0**，不计行为测试。定向 diff/check、暂存差异审查与只读 plan.md 无差异检查均通过；只提交本任务代码、独立测试、最小模块注册和本条正式证据。各阶段自身槽位 1/2 的锁均已释放，缓存保留。原有母任务状态、此前编号文件删除与其它计划保持原状，未纳入提交。未执行全量 cargo test、fmt check、wasm 或 Makefile；完成后删除任务 31 编号文件而不提交其删除。下一步为批次 32。

## 任务 32：Grok 模型表定点切换

2026-11-07；状态：已完成。本项证明隔离 macOS 文件与 API/SDK 链路，不代表真实账号登录、Grok CLI 消费或 Iced 验收。

### 前置、来源与实现

核对批次 31 提交 `e97a9659a` 与本文件任务 31 正式证据。母任务 1、3 已完成；母任务 2 完整领域迁移仍未收口，沿用任务 1 和 provider-contracts.md 已记录的底座子计划 1–93 开工裁定，不将其误报为整体完成。只读 plan.md 未修改。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 退出 0、来源干净。阅读 `grok_direct::{projection,retired_tables,switch_to,reapply,run_with_edits}`、`grok_editor::{view,plan_save,store_into_row,write_live}`、`live/project/grok::{GrokProjection,GrokConfigPatch}`、原生配置 schema 及相邻测试；直接调用者为 ProviderService 的首次激活/switch/editor-save 和 provider/live.rs 的 reapply。来源测试只读，未运行。按 rustcodegraph 技能检查索引，status 可用；explore 长时间无输出后终止，未覆盖调用链使用定向源码读取。

- `vw-agent-apps/src/providers/grok.rs::{settings,patch,GrokFiles}` 为固定 `.grok/config.toml` 的执行端适配；实际调用既有 `providers/projection/grok.rs` 的同一模型表投影。将原投影提取为返回 DocumentMut 的内部函数，原 preview 继续委托它，没有复制模型表规则或调用 upstream 作为业务实现。只替换 models.default、跟随的 session_summary 和供应商整张模型表；保留用户模型、MCP、设置、注释及用户单独选择的摘要模型。整表替换清除旧 env_key，保留新表的嵌套 headers/vendor 选项；供应商行的全局配置不随切换生效。本项不新增全局编辑器，桌面编辑归任务 57。
- 写入表名保存在既有 ModeState.contract.exclusive.grokWrittenTable，contract.key 绑定供应商 ID，官方卡保存明确空表名。文件、当前指针、direct mode 与写入记录进入同一 ProviderTransactions intent；清理只消费已提交记录，不能从 Grok 当前 default 或已编辑供应商行猜旧表。缺少/错版/不匹配写入记录明确拒绝，不做兼容迁移。沿用来源已存在的 PROXY_MANAGED 清理规则。
- 授权 HOME 经 retained descriptor/no-follow 打开固定子目录并持有物理目录锁；config.toml 按 0600 准备，WholeFile 绑定读取快照。沿用 commit_checked，在发布标记前复核目录行、前供应商和 AppState；恢复已有意图后要求调用者重新规划。没有第二套事务引擎或事务外文件写入。
- `vw-api-types/src/app_providers/grok.rs` 为显式 Official/ThirdParty TOML 的强类型 DTO，未知字段/任意文件路径/官方卡载荷拒绝，Debug 脱敏。既有原生 schema 检查模型、名称、端点、api_backend、凭据字段和正整数 context_window；歧义行拒绝，单一模型表沿用既有选择语义。不会创建 OAuth 账号快照或托管租约。
- `vw-agent/src/app_providers/grok.rs` 沿 AppProviderStore/GatewaySettingsStore 的现有租户 namespace 与 CAS 保存行，保留元数据；摘要/列表识别新档案。对应 handler 通过已有 app_providers router 和 bootstrap/router.rs 分域注册消费，权限在目录/数据库访问前沿 provider_editor_context 校验。`PUT /v1/app-providers/grokbuild/{id}/config` 与 `POST /switch`、`/reapply` 对应 SDK `grok_provider_save/switch/reapply`。支持 catalog_only/first/if_current/switch，保存目录与 live 结果分开；附着/代理、异租户状态与托管绑定拒绝。SDK 令牌头 sensitive，HTTP 错误体丢弃，结果不回传配置或 key。无新增 UI；非 macOS 执行明确不支持。

### 红绿与验证

各阶段从仓库根按原子 mkdir 顺序动态领取槽位 **1、2**，未固定编号；实际路径为：

- `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。
- `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。

同设 CARGO_BUILD_BUILD_DIR，覆盖仓库 build-dir；锁记录计划/任务/会话/PID，shell trap 覆盖正常/失败/中断，Cargo 与产物进程退出后清理自身 owner/锁，保留缓存。首次格式化与绿阶段四 crate 使用槽位 1；首轮红阶段 DTO/Gateway/SDK 使用槽位 2，业务包编译接线修正后在槽位 1 重跑；最后格式化、业务包 8 项与 preview 回归 2 项在槽位 2。

红阶段基线写入记录但尚未消费，业务包 **5 项：3 通过、2 在业务断言失败，退出 101**，指定 preserves_unowned_model_tables 读到旧写入表残留；Gateway **2 项：1 通过、1 在当前档案改名后旧表仍存在的断言失败，退出 101**。DTO **1**、SDK **2** 项通过，退出 0。最初投影函数可见性的接线编译错误已修复，不作为红证据；没有用零测试或编译错误代替业务失败。补充外部修改测试首次误按多文件部分发布预期 RolledForwardExcept；检查实际单文件尚未发布契约后，修正为无残留意图/指针未落定/外部内容保留，未改共享事务行为。

最终源码先执行 `cargo fmt --all`（退出 0），最终有效结果：

| 确切命令 | 槽位 | 退出码 | 有效通过数量 |
| --- | --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_grok_ -- --nocapture` | 2 | 0 | 8 |
| `cargo test -p vw-api-types --lib cc04_grok_ -- --nocapture` | 1 | 0 | 1 |
| `cargo test -p vw-agent --lib cc04_grok_ -- --nocapture` | 1 | 0 | 4 |
| `cargo test -p vw-gateway-client --lib cc04_grok_ -- --nocapture` | 1 | 0 | 2 |
| `cargo test -p vw-agent-apps --lib cc04_preview_grok_ -- --nocapture` | 2 | 0 | 2 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 1 | 0 | 非行为测试 |

去重 **17** 项通过：本任务 **15**、相邻回归 **2**，无忽略。指定 `cc04_grok_switch_preserves_unowned_model_tables`、`cc04_grok_editor_schema_rejects_bad_rows` 均通过。业务包以真实临时文件/私有事务目录验证模型表定点替换、用户模型/MCP/注释保留、用户摘要模型保留、旧代理表清理、嵌套 vendor 表、符号链接拒绝、0600 权限，以及 Intent/Marked/FilePublished/Published/Settled 五阶段故障后重建执行器恢复。文件/指针/写入记录一致；发布标记前保存输入变化拒绝，单文件发布前外部修改撤销意图并保留外部文件，不把多文件部分发布的结果套用于此行为。

Gateway 使用真实临时 SQLite、主机授权策略、回环 HTTP 和原 SDK，证明请求→API 响应→临时 `.grok/config.toml`/指针/写入记录：首次激活、当前档案改名、仅保存后 reapply 再改名、官方档案保存后显式 switch 清理；非当前 first/if_current 不改 live。错误 token/缺写权限、错误主机、租户目录隔离、坏 live TOML、托管绑定与缺失写入记录均有实际结果断言，权限拒绝未写文件或目录行。不是用 mock 调用次数证明行为；恢复阶段 hook 仅替代故障边界/输入有效性，其余文件系统、存储、事务和鉴权未替代。

所有凭据均为假值；未访问真实用户配置、真实账号、OAuth/Keychain 或网络模型上游。平台 macOS **27.0 (26A428)**、**arm64**，crate **0.6.0**；`sw_vers`、`uname -m`、`grok --version` 均退出 0，Grok **1.0.77 (50ca579fa996) [stable]**。仅查询版本，真实 CLI 会话消费、真实账号登录、Iced 渲染及非 macOS 未验证。Agent lib test 继续有既有 `__eh_frame section too large` 链接提示，测试退出 0；未改链接设置，最终强制 Clippy 退出 0。

本任务代码/独立测试/正式证据的定向 diff/check、暂存检查及只读 plan.md 无差异检查通过；只提交本任务变更，不纳入既有母任务状态、此前任务文件删除或其它计划。各阶段槽位 1/2 自身锁均释放、缓存保留；未执行全量 cargo test、fmt check、wasm 或 Makefile。完成后删除编号任务文件、不提交其删除；正式证据保留。下一步为批次 33。

## 任务 33：Claude Desktop 直连配置

2026-11-07；状态：已完成。本项验收真实 macOS 临时目录、SQLite 与本地 HTTP/SDK 链路；真实 Desktop 进程、账号与 Iced 未验证。

### 前置、来源与实现

核对批次 32 提交 `ca6819afe` 和本文件任务 32 证据。母任务 1、3 已完成，母任务 2 当前仍未整体完成；按 provider-contracts.md 已有底座子计划 1–93 开工裁定消费真实接口，不伪报母任务 2 完成。只读 plan.md 未修改。多目录支持按本次明确授权在既有唯一事务所有者内补齐。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 退出 0、来源干净。读取 `claude_desktop_config.rs` 的 direct_target/write_direct、write_profile_at_paths、restore_official_at_paths、gateway_profile_patch、MetaPatch、macos_paths_from_home/paths_from_dirs 及相邻用户字段、官方恢复、meta 条目与私有权限测试；真实调用者为 ProviderService::switch_desktop_direct、live::write_live_for_state 和泛导入拒绝分支。表单来源为 ClaudeDesktopProviderForm.tsx、forms/ProviderForm.tsx 的 Desktop 分派及相邻 ClaudeDesktopProviderForm.test.tsx。来源只读、未运行来源测试。RustCodeGraph status 退出 0；explore 无输出后中止（130），其未覆盖的调用链改用定向源码读取。

- `vw-agent-apps/src/providers/claude_desktop.rs::{settings,ClaudeDesktopFiles}` 只由管理员授权的 HOME 固定推导 `Library/Application Support/Claude/claude_desktop_config.json`、`Claude-3p/claude_desktop_config.json`、`Claude-3p/configLibrary/00000000-0000-4000-8000-000000157210.json` 和 `_meta.json`。逐层 mkdirat/openat/O_NOFOLLOW，核验目录所有者及不可组/全局写；不消费客户端路径。保留来源固定 profile ID/名称，未重品牌。
- 官方档案将两个 deploymentMode 恢复为 1p，清除 profile 网关/旧模型/发现字段及 3p enterpriseConfig 的旧网关字段，保留用户字段和非空企业设置；meta 只移除本 profile，必要时选回其余条目。第三方直连仅支持 Anthropic bearer 凭据，复用既有 validation::endpoint（HTTPS 或回环 HTTP，拒绝控制字符和 URL 凭据），按来源裁剪地址和令牌两端空白后写 gateway profile、两个 3p 模式和 appliedId；旧模型字段清除，用户 auto 模式、收紧的出站白名单、部署选择器及其它 meta 条目保留。沿用 JsonPatch 保留非受管 JSON/JSONC 字节；profile 写入按 0600，两个配置与 meta 沿用 Shared 权限策略。
- 多目录扩展限于既有 `live/write_guard.rs`、`stage.rs`、`commit.rs`、`concurrency.rs`、`recovery.rs`：一个应用 guard 一次领取全部物理目录锁；执行端 DirectoryTarget 把固定逻辑键映射至目录索引与直属文件名，两者均经过原 checked_name 校验，未放开相对路径。一次 stage 预演全部文件；同一 pending 包含每文件目录索引、前后摘要和 staged 文件，全部暂存目录 fsync 后才发布意图。跨目录使用 version 3，旧执行器拒绝；单目录仍产生原 version 1/2，省略零目录索引及空 additional，未迁移旧状态。
- 资源登记来自保留描述符的物理路径、device/inode/uid；启动恢复无跟随链接重开全部登记目录，校验身份、权限与索引之后才碰文件。未标记意图丢弃；已标记意图前滚，保留外部改动并报告 RolledForwardExcept。文件、direct 指针与模式落定仍由同一 ProviderTransactions 所有者处理，无第二引擎、分次 commit 或事务外文件写入；只承诺逐文件发布及持久意图恢复，不承诺跨文件系统原子回滚。两个既有 Gateway 恢复 fixture 仅补显式 directory: 0。
- DTO 在 `vw-api-types/src/app_providers/claude_desktop.rs`：Official/ThirdParty、baseUrl/authToken、明确激活意图，拒绝未知字段/任意路径/官方载荷，Debug 脱敏。AppProviderStore 适配沿原租户 namespace/CAS 保存和摘要列表保留元数据。handler 通过既有 app_providers 根路由及 bootstrap/router 注册；权限在数据库和文件操作之前经 provider_editor_context 校验。SDK 在既有 app_providers_api 根注册，无新增通用命令入口。
- `PUT /v1/app-providers/claude-desktop/{id}/config` 和 `POST /switch`、`/reapply` 对应 SDK `claude_desktop_provider_save/switch/reapply`，支持 catalog_only/first/if_current/switch；目录保存与 live 结果分开，不能由 catalogSaved 推断 live 成功。附着/代理、异租户状态、托管绑定和需路由的旧映射/非 Anthropic 档案明确拒绝。模型映射归任务 34，Iced 表单归任务 67；本项没有伪造路由、OAuth 或 UI 支持。
- `POST /v1/app-providers/claude-desktop/live/import` / SDK `claude_desktop_provider_import_live` 经相同授权后明确返回 422 与固定 `claude_desktop.import_unsupported`，不读取 live、不创建事务状态或目录行；不是泛文件 API。SDK 令牌头 sensitive，失败体丢弃，成功响应只返回结果枚举，不回传配置或凭据。

### 红绿与验证

各阶段均从仓库根按原子 mkdir 顺序动态领取共享槽位 **1**，实际：

`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`

同设 CARGO_BUILD_BUILD_DIR 覆盖仓库 build-dir；锁记录计划/任务/会话/PID，shell trap 覆盖退出/失败/中断，Cargo 与产物子进程结束后才移除自身 owner 和锁，保留编译缓存，没有固定分配或其它构建目录。

红阶段为可编译基线：profile 整体替换尚未保用户字段，泛导入尚未拒绝。业务包 **5 项：3 通过、2 在指定行为断言失败，退出 101**；keeps_other_fields 读到 userAutoMode 为 Null 而非 true，generic_import 读到 Ok 而非固定拒绝。DTO **1 项失败，退出 101**，明确 camelCase 凭据字段未接线；Gateway **3 项：1 通过、2 在保字段/泛导入响应断言失败，退出 101**；SDK **2 项通过，退出 0**。不是缺模块、编译错误或零测试充当红证据。审查时新增端点策略测试，命令 cargo test -p vw-agent-apps --lib cc04_claude_desktop_direct_credentials_follow_existing_endpoint_policy -- --nocapture 的 1 项在远端明文 HTTP 被应用的断言失败（退出 101）；随后接回已有端点校验，验证拒绝时四文件和指针不变，以及回环地址和令牌裁剪。

最终源码执行 `cargo fmt --all`，退出 **0**。四条任务聚焦命令默认 features，初次绿阶段全部通过；最后的 Clippy 风格修正后再次执行相同命令。有效数量如下：

| 确切命令 | 退出码 | 有效通过数量 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_claude_desktop_ -- --nocapture` | 0 | 11 |
| `cargo test -p vw-api-types --lib cc04_claude_desktop_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-agent --lib cc04_claude_desktop_ -- --nocapture` | 0 | 4 |
| `cargo test -p vw-gateway-client --lib cc04_claude_desktop_ -- --nocapture` | 0 | 3 |

本任务 **19 项**，无忽略。使用真实临时三目录/四文件/私有事务根验证指定保字段、泛导入拒绝、官方清理、profile 凭据切换、meta 其它条目及 0600；Intent/Marked/FilePublished/Published/Settled 故障后重建执行器并从启动入口恢复；已发布部分文件后的外部改动保留、剩余文件前滚；输入在意图后改变则丢弃且不落指针；目录换 inode/符号链接和未登记目录索引均在写任意文件前拒绝。

Gateway 使用真实临时 SQLite、主机授权策略、回环 HTTP 与原 SDK，证明请求→响应→四文件/指针：首次激活、当前编辑、目录仅保存后重新应用、官方显式切换；first/if_current 非当前不改 live。错误 token、缺写权限、错误主机、坏 live JSON、托管账号及需路由档案无文件或目录行修改。泛导入返回 422，未改临时文件或创建 live 行。仅阶段 hook 替代不可控崩溃/输入变化边界，未替代文件系统、存储、事务或鉴权；所有凭据为假值。

共享设施及单目录应用聚焦回归：

| 确切命令 | 退出码 | 有效通过数量 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib providers::live:: -- --nocapture` | 0 | 49 |
| `cargo test -p vw-agent-apps --lib cc04_gemini_ -- --nocapture` | 0 | 9 |
| `cargo test -p vw-agent-apps --lib cc04_grok_ -- --nocapture` | 0 | 8 |
| `cargo test -p vw-agent-apps --lib cc04_codex_ -- --nocapture` | 0 | 35 |
| `cargo test -p vw-agent-apps --lib providers::claude_tests:: -- --nocapture` | 0 | 5 |

回归 **106 项**，连同本任务去重 **125 项通过**；最后一次源码修正后再跑 live 的 49 项，不重复计数。初轮强制 `cargo clippy --all-targets --all-features -- -D warnings` 退出 101，仅本次 OR 范围模式写法触发 manual_range_patterns；已改为 `1..=3`，未压制 lint。最终相同 Clippy 命令退出 **0**；端点策略修正后已再次运行格式化、四 crate 聚焦测试、live 49 项及 Clippy，全部退出 0。

平台 `sw_vers`、`uname -m` 退出 0：macOS **27.0 (26A428)**、**arm64**，crate **0.6.0**。默认 `/Applications/Claude.app/Contents/Info.plist` 版本查询退出 1、未取得版本；不据此推断其它安装位置。本项没有读取真实用户配置/账号/Keychain、启动 Desktop、访问网络模型上游；真实 Desktop 消费、账号登录、Iced 明暗主题及非 macOS 未验证。Gateway lib test 有既有 `__eh_frame section too large` 链接提示，测试退出 0，未改链接配置。

最终定向差异/暂存检查及只读 plan.md 无差异检查均通过；各阶段自身槽位 1 的锁已释放，缓存保留。仅提交本任务代码、独立测试、必要父注册与本条正式证据；既有母任务状态、之前编号文件删除和其它计划不纳入。未运行全量 cargo test、fmt check、wasm 或 Makefile。完成后删除本任务编号文件且不提交其删除，正式证据保留；下一步为批次 34 与后续真实 Desktop 验收。

## 任务 34：Claude Desktop 模型映射

2026-11-07；状态：已完成。范围为从 Claude 目录导入兼容档案、默认角色路由及目录映射校验/保存；不交付代理运行时或聚合模型目录。真实 macOS 临时目录、SQLite 和 HTTP/SDK 链路已验收；真实 Desktop 进程、账号及 Iced 未验证。

### 前置、来源与实现

已核对批次 33 提交 `363edbc32` 及本文件任务 33 的正式证据。母任务 1、3 已完成；母任务 2 完整领域迁移仍未收口，继续沿用任务 1/provider-contracts.md 记载的底座子计划 1–93 开工裁定，不将其误报为完整完成。只读 plan.md 无差异。

来源 `/Users/xiangmin/worker/code/cc-switch` 的 HEAD 为 `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 均退出 0、来源干净。读取 `claude_desktop_config.rs::{default_proxy_routes,is_compatible_direct_provider,validate_direct_provider,validate_proxy_provider,is_claude_safe_model_id,direct_inference_model_specs,proxy_model_routes}`、`commands/provider.rs::{import_claude_desktop_providers_from_claude_blocking,claude_provider_models_are_claude_safe,suggested_claude_desktop_routes}` 及相邻导入/模型校验测试；来源调用者为 `src/lib/api/providers.ts` 的 Desktop 默认路由/导入调用和 `ClaudeDesktopProviderForm.tsx` 的路由表保存。来源只读，未运行来源测试。按 rustcodegraph 技能检查 status（退出 0），explore 返回来源映射与既有 Desktop 符号；新模块与未覆盖原生链路以定向源码读取核对。

- 执行端唯一规则所有者为 `vw-agent-apps/src/providers/desktop_mapping.rs`。保留默认路由的物理顺序 Sonnet/Opus/Haiku/Fable、routeId/envKey/supports1m；导入按角色 env 生成映射，没有角色模型时回退 ANTHROPIC_MODEL。大小写无关的 `[1M]` 后缀转换为 supports1m，同一目标模型合并；显式 `_NAME` 优先，非 Claude 模型名作为缺省标签。原生 Anthropic bearer、Claude 安全模型可成为 direct；其余有受支持协议/静态凭据和有效模型的档案仅保存 proxy 映射。保留备注与设置，取消导入行的 failover 标记；支持既有通用 AppProviderWrite 和任务 26 的原生 Claude 记录。
- 缺凭据/官方登录、托管绑定/托管 providerType、Bedrock/Vertex、full URL、未知协议、远端明文端点及掩码凭据不导入；不伪造其依赖或 OAuth 运行时。模型/协议字段类型错误、控制字符、非法 routeId、退化角色名、未转换的 `[1M]`、空 proxy 路由及超过 200 路由均拒绝；direct 仅允许同名目标，非法映射在数据库 CAS 之前拒绝。本任务不迁移旧目录、修复非法 routeId 或生成运行时发布目录。
- `vw-agent/src/app_providers/desktop_mapping.rs` 消费原 AppProviderStore/GatewaySettingsStore 租户 namespace。导入跳过已有 Desktop ID，原子 insert-if-absent，不改当前指针；映射保存在已有 `meta.claudeDesktopMode/claudeDesktopModelRoutes`，校验凭据后 CAS 保存，保留档案其它内容。每行独立插入，不宣称整批导入原子性；错误不会写入该非法行。
- 强类型 DTO 在 `vw-api-types/src/app_providers/desktop_mapping.rs`，拒绝未知字段和任意路径。Gateway 沿既有 app_providers 根路由与 gateway/router 注册，API 为 `POST /v1/app-providers/claude-desktop/model-mapping/defaults`、`POST /providers/import-claude`（同一 Desktop 前缀）、`PUT /{id}/model-mapping`（同一前缀）。默认路由要求 read；导入与保存要求目标主机 write，均在数据库访问之前调用 provider_editor_context。SDK 对应 `claude_desktop_default_routes/import_claude/mapping_save`，复用既有敏感令牌头、租户请求上下文、身份校验和丢弃错误载荷的传输。
- 导入仅返回数量，映射保存返回 catalogSaved=true/live=not_attempted；不返回凭据或假报 live 成功。需路由的档案仍由任务 33 的直接切换入口明确拒绝，无主机配置或事务目录写入；导入的兼容原生档案可通过原有 SDK switch→API→ProviderTransactions 写入四文件与 direct 指针。没有新增 UI；Iced 表单归任务 67，真实聚合目录/运行时归母任务 10。

### 红绿与验证

各阶段从仓库根按原子 mkdir 顺序动态领取共享槽位 **1**：

`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`

同时设置 CARGO_BUILD_BUILD_DIR 至同一绝对路径，覆盖仓库 build-dir；owner 记录计划、任务、会话与持有 PID。阶段结束后清理自身 owner/锁，保留编译缓存；未固定按任务编号分配或使用其它 target 目录。

红阶段先接通可编译接口：业务包 **3 项均在业务断言失败，退出 101**，导入返回 None、非法 gpt-5 route 被接受；Gateway **1 项在导入数量 0≠1 断言失败，退出 101**；DTO **1 项**、SDK **2 项**通过，退出 0。不是缺模块/类型、编译错误或零测试代替红测。

最终源码先执行 `cargo fmt --all`，退出 **0**；以下均采用默认 features，相同槽位：

| 确切命令 | 退出码 | 有效通过数量 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_desktop_mapping_ -- --nocapture` | 0 | 4 |
| `cargo test -p vw-api-types --lib cc04_desktop_mapping_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-agent --lib cc04_desktop_mapping_ -- --nocapture` | 0 | 3 |
| `cargo test -p vw-gateway-client --lib cc04_desktop_mapping_ -- --nocapture` | 0 | 2 |
| `cargo test -p vw-agent-apps --lib cc04_claude_desktop_ -- --nocapture` | 0 | 11 |
| `cargo test -p vw-api-types --lib cc04_claude_desktop_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-agent --lib cc04_claude_desktop_ -- --nocapture` | 0 | 4 |
| `cargo test -p vw-gateway-client --lib cc04_claude_desktop_ -- --nocapture` | 0 | 3 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

本任务 **10 项**、相邻回归 **19 项**，去重 **29 项通过**，无忽略；指定 imports_only_compatible_rows/rejects_invalid_model_id 均通过。独立测试覆盖原生/通用档案导入、元数据保留、默认角色顺序、重复目标合并、标签与 marker、非法 ID/字段类型/协议/端点/凭据、直连映射限制、已有 ID 不覆盖。Gateway 使用真实临时 SQLite、管理员策略、回环 HTTP 和原 SDK：非法映射保存后记录原样不变；错误 token/缺写权限不创建目录行、不改临时文件，read-only 仍能读取默认路由；proxy 档案明确拒绝直接激活；native 档案成功沿既有事务写入 profile 凭据、base URL 和 current 指针。测试没有替换文件系统、数据库、事务、鉴权或 HTTP，无 mock 调用次数断言；全部凭据为假值。

`sw_vers`、`uname -m` 退出 0：macOS **27.0 (26A428)**、**arm64**，crate **0.6.0**。默认 `/Applications/Claude.app/Contents/Info.plist` 版本查询退出 1，未取得应用版本；真实 Desktop 消费、账号/Keychain、网络模型上游、Iced 渲染与非 macOS 未验证。未触碰真实用户配置。Gateway lib test 有既有 `__eh_frame section too large` 链接提示，测试退出 0；未改链接设置，最终强制 Clippy 退出 0。

定向差异与暂存检查通过，plan.md 保持只读；仅提交本任务代码、独立测试、必要父注册和本条证据，既有母任务状态、此前编号文件删除与其它计划不纳入。自身槽位 1 锁已释放、缓存保留。未执行全量 cargo test、fmt check、wasm 或 Makefile。验收后删除任务 34 编号文件，正式证据保留；下一步为批次 35，真实 Desktop/UI/路由运行时按其所属任务验收。

## 任务 35：OpenCode 增量供应商

2026-11-07；范围：OpenCode 的点名成员增改/移除、已声明默认模型、只读原生导入及 API/SDK 闭环。状态：已完成。

### 依赖与来源

已核对批次 34 提交 `221402ff2` 和本文件任务 34 的正式证据。母任务 1、3 已完成；母任务 2 仍未整体收口，沿任务 1/provider-contracts.md 已记录的底座子计划 1–93 开工裁定消费真实接口，不将其误报为完整完成。未修改只读 plan.md。

来源快照 `ca58c0313360eae7c851fbd7553c1614c7af03e7`：`opencode_config.rs::{resolve_config_path,get_providers_with_format,native_provider_problem,set_provider_with_format,remove_provider,set_default_model}`，`services/provider/live.rs::{write_live_snapshot,remove_opencode_provider_from_live,import_opencode_providers_from_live}`，相邻 `services/provider/opencode_tests.rs`；表单 helper `opencodeFormUtils.ts::{isNativeOpencodeConfig,hasOpencodeDefinition,getOpencodeDeclaredModels}` 及 ProviderForm/ProviderList/presentation 调用者和相邻测试。导航使用仓库 RustCodeGraph 索引；新增/图未覆盖路径用源码读取核对。

### 实现与接口

- 唯一执行端所有者为 `vw-agent-apps/src/providers/opencode.rs`；V2 已知字段校验在相邻 `opencode_schema.rs`。复用已有 JsonPatch/物理目录 guard/ProviderTransactions。V1 写 `provider.<id>`、V2 写 `providers.<id>`，保留未知声明字段与未点名 provider/MCP/主题；更新按声明内的键差异落补丁，保留嵌套 JSONC 注释。有效 V2 优先且拒绝 V1 覆盖；移除同名有效 V2 与旧 V1，避免旧声明复活；无效 V2 旁支保留。部分 V1 内置覆盖仅允许已有 live 同名成员，移除后不能靠旧成员资格重新加入。
- 固定授权 HOME `.config/opencode` 中 JSONC 优先于 JSON，保留描述符并拒绝符号链接；JSON 选定后 JSONC 新出现会报冲突。默认模型只允许 live 中已声明且未禁用的模型，模型 ID 可含 `/`；只改顶层 `model`，不改 `small_model`。缺失成员移除不创建空配置。
- 所有 live 写进入既有 pending/恢复事务，校验租户、direct/attached 状态、保存行修订、文件前像与路径优先级；不创建唯一 provider 指针。读取/导入不创建配置、锁或事务目录，不读取账号数据库/Keychain、不执行 OpenCode。现有 catalog/delete 引用观察接到同一 JSONC/V2 所有者，live 成员阻止目录删除，显式 remove-live 保留目录行，之后目录删除只删除该行。
- `vw-api-types/src/app_providers/opencode.rs` 提供强类型 profile/format/activation/target/default-model/state/result。原有租户命名空间 `AppProviderStore` 条件保存，保留行元数据；导入跳过已有行，不覆盖目录草稿，后续导入策略归任务 44。敏感 profile Debug 脱敏；SDK 标记凭据 header 为 sensitive，失败响应不保留载荷。权限沿既有 provider_editor_context：state 要读权限，其余操作要写权限；不接收客户端文件路径/命令。

以下路径前缀均为 `/v1/app-providers/opencode`，注册沿既有 app_providers 分域 router 和 SDK 模块：

| API | SDK |
| --- | --- |
| `PUT /{id}/config` | `opencode_provider_save`，catalog_only/add/if_member |
| `POST /{id}/add` | `opencode_provider_add`，消费已保存行 |
| `POST /{id}/remove-live` | `opencode_provider_remove_live`，不删除目录行 |
| `PUT /config/default-model` | `opencode_default_model` |
| `POST /config/state` | `opencode_state`，只返回成员 ID 和默认模型 |
| `POST /config/import` | `opencode_import`，只返回新增数量 |

`catalogSaved` 与 live applied/not_attempted/failed/recovery_required 分开返回。没有新增 UI；任务 58 消费本 API/SDK。自定义路径、真实应用消费、路由/聚合运行时不由本项冒充验收。

### 验证证据

各阶段均从仓库根用原子 mkdir 顺序动态领取槽位 **1**，最终：

`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`

同时设置 `CARGO_BUILD_BUILD_DIR` 为同一绝对路径，覆盖仓库 build-dir。首次红测因遗漏此附加变量使用了仓库 build-dir，未采用其结果；随即在完整槽位环境重跑：业务包 **2 项在业务断言失败，退出 101**（新增成员缺失、默认模型未更新），不是编译错误或零测试。之后四包首次完整聚焦检查为 2/1/4/2 项通过。补充恢复测试的临时事务根最初缺少 0700 权限，被安全检查拒绝；按既有 fixture 设置权限后重新验证。未削弱执行端检查。

最终源码先执行 `cargo fmt --all`，退出 **0**；以下均采用默认 features、同一槽位：

| 确切命令 | 退出码 | 有效通过数量 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_opencode_ -- --nocapture` | 0 | 10 |
| `cargo test -p vw-api-types --lib cc04_opencode_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-agent --lib cc04_opencode_ -- --nocapture` | 0 | 4 |
| `cargo test -p vw-gateway-client --lib cc04_opencode_ -- --nocapture` | 0 | 2 |
| `cargo test -p vw-agent-apps --lib cc04_delete_ -- --nocapture` | 0 | 3 |
| `cargo test -p vw-agent --lib cc04_delete_ -- --nocapture` | 0 | 4 |
| `cargo test -p vw-agent-apps --lib cc04_catalog_ -- --nocapture` | 0 | 7 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

本任务 **17 项**、相邻回归 **14 项**，去重 **31 项通过**，无忽略。指定 `cc04_opencode_membership_preserves_other_providers`、`cc04_opencode_default_model_keeps_small_model` 通过。其余覆盖 V1/V2 优先级与无效旁支、嵌套注释、部分内置覆盖、禁用模型/含斜线模型、非法与重复 JSON key、JSONC 优先级变化、缺失配置只读与移除、符号链接拒绝、文件/保存输入冲突，以及 Intent/Marked/FilePublished/Published/Settled 五处中断后的事务重建恢复与无唯一指针。

Gateway 使用真实临时 SQLite、管理员策略、回环 HTTP 和原 SDK，证明 SDK 保存/编辑 → API applied → 执行端文件包含新值，默认模型写入且 small_model/MCP/注释保留；非法默认模型无文件变化；live 成员阻止目录删除，remove-live 后目录行仍在并可单独删除；导入只读文件字节/权限不变、重复导入 0；假 token、异租户、异主机和缺写权限拒绝且无目录行/配置/事务目录变化；只读授权仍可读取 state。未替代文件系统、数据库、事务、鉴权或 HTTP；仅使用假凭据，没有网络上游或 mock 次数断言。

`sw_vers`/`uname -m`：macOS **27.0 (26A428)**、**arm64**，crate **0.6.0**。`command -v opencode` 未找到可执行文件，`opencode --version` 退出 **127**，应用版本、真实 OpenCode 消费、真实账号/Keychain、网络上游、Iced 渲染及非 macOS **未验证**。未触碰真实用户配置。Gateway lib test 有既有 `__eh_frame section too large` 链接提示，测试退出 0；未修改链接配置。

定向 diff/check 和暂存审查通过；只提交本任务实现、独立测试、必要注册、原生观察接线及本条证据。既有母任务状态、此前编号任务删除和其它计划不纳入；所有 plan.md 保持只读。自身锁释放、缓存保留；未运行全量 cargo test、fmt check、wasm 或 Makefile。完成后删除任务 35 编号文件且不提交该删除，正式证据保留；下一步批次 36，OpenCode Iced 编辑归任务 58。

## 任务 36：OpenClaw 增量供应商

2026-11-07；范围：点名维护 OpenClaw 供应商，设置 primary 时保留默认模型其它字段并移除与新 primary 重复的 fallback。状态：已完成。

### 依赖与实现

核对批次 35 提交 `c9e64fe96` 及上节验收记录。母任务 1、3 已完成；母任务 2 完整迁移仍未收口，沿用任务 1/provider-contracts.md 中底座子计划 1–93 的开工裁定，不将其误报为完整完成。所有 plan.md 保持只读。

来源 HEAD 确认为 `ca58c0313360eae7c851fbd7553c1614c7af03e7`。阅读 `openclaw_config.rs::{set_provider,remove_provider,get_default_model,set_default_model}` 及相邻测试、`commands/openclaw.rs`、`services/provider/live.rs` 的写入/导入/移除调用者和 `services/provider/mod.rs` 的 OpenClaw 导入测试；`useProviderActions.ts::setAsDefaultModel` 是保留扩展字段并过滤重复 fallback 的来源。导航先使用 RustCodeGraph，未覆盖新文件使用源码读取。

- 执行端唯一业务所有者为 `vw-agent-apps/src/providers/openclaw.rs`：固定授权 HOME `.openclaw/openclaw.json`，点名修改 `models.providers.<id>`；新建 models 时补 mode=merge，现有 mode、其他成员、agents/workspace/memory 和未知字段保持。声明内按差异叶节点修改，保留未触及的 JSONC 注释。默认模型仅接受 live 成员已声明的模型（允许模型 ID 内含 `/`），只修改 `agents.defaults.model.primary` 和存在的 fallbacks 数组，过滤所有等于新 primary 的条目，其它 fallback 的顺序、重复项和默认模型扩展字段保留。
- 复用 JsonPatch、描述符目录锁及 ProviderTransactions；不新增事务所有者或唯一供应商指针。文件前像、目录行修订、租户和 direct/attached 状态在提交前检查；中断由既有 pending 恢复。缺失配置读取/导入不建目录，缺失成员移除不创建配置文件；符号链接、损坏结构、重复 key、非法模型拒绝。沿用现有 JSON/JSONC 补丁能力，JSON5 未加引号键/单引号明确拒绝，不将其记为已支持；自定义配置路径未接入。
- `vw-api-types/src/app_providers/openclaw.rs`、`vw-agent/src/app_providers/openclaw.rs`、Gateway/SDK 同名模块沿用现有分域注册、管理员主机策略及租户存储。声明 Debug 脱敏，SDK 敏感 header 与失败载荷丢弃；DTO 拒绝客户端文件路径/命令。目录列表摘要、原生成员观察和删除引用检查接同一所有者；live 成员阻止目录删除，remove-live 保留目录行，目录删除不清理原生默认模型引用。

API 前缀 `/v1/app-providers/openclaw`：

| API | SDK |
| --- | --- |
| `PUT /{id}/config` | `openclaw_provider_save`；catalog_only/add/if_member |
| `POST /{id}/add` | `openclaw_provider_add`；消费已保存声明 |
| `POST /{id}/remove-live` | `openclaw_provider_remove_live` |
| `PUT /config/default-model` | `openclaw_default_model` |
| `POST /config/state` | `openclaw_state`；只投影成员 ID 和 primary |
| `POST /config/import` | `openclaw_import`；只读配置，跳过已有目录行 |

返回 `catalogSaved` 与 live applied/not_attempted/failed/recovery_required，保存目录不等于 live 成功。导入策略延续任务 35 的新增不覆盖边界，完整导入策略归任务 44；Iced 编辑归任务 59，本项没有新增或验收 UI。工作区与记忆仅断言原样保留，没有实现其管理功能。

### 红绿验证与平台边界

各阶段均从仓库根原子 mkdir 动态领取槽位 **1**；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，并设置同路径 `CARGO_BUILD_BUILD_DIR` 覆盖仓库 build-dir。锁中记录计划/任务/会话/PID，shell trap 在退出/中断时清理自身记录与锁，缓存保留。

红测使用下表四个相同命令：业务包 **2 项在行为断言失败，退出 101**（成员未写入、primary/fallback 未更新）；DTO **1 项通过，退出 0**；Gateway **2 通过/1 业务失败，退出 101**（保存编辑未反映到执行端文件）；SDK **2 项通过，退出 0**。不存在用编译失败或零测试充当红测。

最终执行 `cargo fmt --all`，退出 **0**。采用默认 features 的有效结果：

| 确切命令 | 退出码 | 通过数量 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_openclaw_ -- --nocapture` | 0 | 8 |
| `cargo test -p vw-api-types --lib cc04_openclaw_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-agent --lib cc04_openclaw_ -- --nocapture` | 0 | 3 |
| `cargo test -p vw-gateway-client --lib cc04_openclaw_ -- --nocapture` | 0 | 2 |
| `cargo test -p vw-agent-apps --lib cc04_catalog_ -- --nocapture` | 0 | 7 |
| `cargo test -p vw-agent-apps --lib cc04_delete_ -- --nocapture` | 0 | 3 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

本任务 **14 项**、相邻回归 **10 项**通过，无忽略。指定 `cc04_openclaw_membership_keeps_unrelated_config`、`cc04_openclaw_primary_preserves_remaining_fallbacks` 均通过；额外证明新建 merge/已有 replace 保留、嵌套注释、未知默认字段、workspace/memory 保留、非法与未声明模型、含斜线模型、只读元数据脱敏、缺失移除、符号链接拒绝、输入修订/外部文件冲突及 Intent/Marked/FilePublished/Published/Settled 五阶段重建恢复，无唯一指针。

Gateway 使用临时 HOME、真实 SQLite、管理员策略、真实回环 HTTP 和原生 SDK：save/add/if_member → API applied → 文件凭据 fixture 更新；default-model → primary 更新、fallback 过滤且 workspace/注释保留；不存在模型拒绝且文件不变；成员阻止目录删除、remove-live 不删目录行且可单独删除；只读导入重复返回 0 且不创建事务目录；假 token、异租户、异主机、缺写权限无行/文件/事务副作用，读权限仍可获取 state。只有远端部署传输替换为本地回环，凭据均为假值；未替代文件系统、数据库、鉴权或事务，也没有网络上游 mock 次数断言。

`sw_vers`/`uname -m`：macOS **27.0 (26A428)**、**arm64**，crate **0.6.0**。`command -v openclaw` 无结果，`openclaw --version` 退出 **127**；应用版本、真实 OpenClaw 消费、真实账号/Keychain、网络上游、Iced 渲染和非 macOS **未验证**。未读取或修改真实用户配置。Gateway 测试链接器有既有 `__eh_frame section too large` 提示，测试退出 0，不修改链接配置。

定向 diff/check、暂存审查与 plan.md 只读检查通过；只提交本任务实现、独立测试、必要接线和本条证据，排除既有母任务状态、其它计划与此前编号文件删除。自身槽位锁释放、缓存保留；未执行全量 cargo test、fmt check、wasm 或 Makefile。验收后删除编号任务文件，不提交其删除；下一步批次 37。

## 任务 37：Hermes 增量供应商

2026-11-07；范围：Hermes 原生 YAML 成员、关联凭据字段和安全导入。状态：已完成。

### 依赖与实现

核对批次 36 提交 `4001d7173` 及上节验收记录。母任务 1、3 已完成；母任务 2 完整迁移未收口，沿用任务 1/provider-contracts.md 中底座子计划 1–93 的开工裁定，不将其误报为完整完成。未修改任何 plan.md。

来源 HEAD 为 `ca58c0313360eae7c851fbd7553c1614c7af03e7`。阅读 `hermes_config.rs::{get_hermes_dir,get_providers,set_provider,remove_provider,normalize_provider_models_for_write,sanitize_hermes_provider_keys}`、相邻测试、`commands/hermes.rs`、`services/provider/live.rs::{write_live_snapshot,import_hermes_providers_from_live,remove_hermes_provider_from_live}` 及 `tests/hermes_roundtrip.rs`；RustCodeGraph 用于入口和调用链导航，新增文件及未覆盖文件用源码读取。

- 唯一执行端所有者为 `vw-agent-apps/src/providers/hermes.rs`。固定文件集合是 Hermes 目录下 `config.yaml`；关联凭据在成员 `api_key`、`key_env` 字段中，不新增凭据文件、不读取环境密钥、Keychain 或 shell 配置。默认授权 HOME 下 `.hermes`；执行主机 `HERMES_HOME` trim 后按字面路径解析，只允许授权 HOME 下正常组件，不展开 tilde、不允许相对路径/越界/父目录穿越/目录或文件符号链接。未新增客户端路径覆盖接口。
- 只改 `custom_providers` 序列中点名成员，保留其它成员原始字节、根区块/注释及当前成员未知字段、遗漏的凭据字段；支持正常缩进与无缩进列表、CRLF 输入的正常 YAML 解析。表单 `models` 数组转换为原生字典并在未显式提供 model 时取首模型；保留显式原生 model，读取时转换回数组。历史 camelCase 字段归一化为 snake_case，伪脱敏密钥拒绝。`providers` 字典按显式 name/字典 key 读取，列表同名优先；字典独有成员及其 key 禁止编辑、添加覆盖或移除，state 单独返回 readOnlyMembers。每次补丁将输出语义与预期文档比较，损坏配置、重复 key/成员、flow 列表编辑及跨节点引用等不能安全修改的布局明确拒绝，不静默重写整份 YAML。
- 复用目录描述符写锁、私有文件权限、ProviderTransactions、前像校验、租户/目录行修订和 pending 前滚恢复；不新增事务所有者或唯一供应商指针。目录保存、原生发布分别返回结果；当前成员表单编辑走 IfMember，首次添加走 Add。model/agent/MCP/memory 等其它区块保持，不提供其设置入口。
- `vw-api-types/src/app_providers/hermes.rs`、`vw-agent/src/app_providers/hermes.rs`、Gateway/SDK 同名模块沿用既有分域注册、管理员主机读写权限与租户存储。声明 Debug 脱敏，SDK 凭据 header 标记敏感，失败响应载荷丢弃。目录摘要、原生成员观察与删除引用检查接同一所有者；成员存在时阻止目录删除，remove-live 保留目录记录，导入只新增、不覆盖已有目录行、不改源文件。完整导入策略归任务 44；Iced 表单归任务 60，本轮 UI 未新增或验收。

API 前缀 `/v1/app-providers/hermes`：

| API | SDK |
| --- | --- |
| `PUT /{id}/config` | `hermes_provider_save`；catalog_only/add/if_member |
| `POST /{id}/add` | `hermes_provider_add`；消费保存的声明 |
| `POST /{id}/remove-live` | `hermes_provider_remove_live` |
| `POST /config/state` | `hermes_state`；只返回成员及只读成员 ID |
| `POST /config/import` | `hermes_import`；只读源文件、新增不覆盖 |

### 红绿验证与平台边界

红测、首轮绿测、最终验证均从仓库根原子 mkdir 动态领取槽位 **1**；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，并设置同路径 `CARGO_BUILD_BUILD_DIR` 覆盖仓库 build-dir。锁记录计划/任务/会话/PID，shell trap 负责阶段结束及失败/中断时释放自身锁，保留缓存。

红测执行下表四个相同 Hermes 过滤命令：执行端 **2 项业务失败，退出 101**（成员结果缺失）；DTO **1 通过，退出 0**；Gateway **3 项业务失败，退出 101**（成员结果/导入计数缺失）；SDK **2 通过，退出 0**。不存在用编译失败或零测试充当红测。首轮绿测执行端 7 通过/1 失败，定位无缩进列表被误判为根区块；修复后最终结果如下。代码审查另补充部分表单别名/旧凭据合并及显式 model 往返保留场景。

最终 `cargo fmt --all` 退出 **0**。默认 features 有效结果：

| 确切命令 | 退出码 | 通过数量 |
| --- | --- | --- |
| `cargo test -p vw-agent-apps --lib cc04_hermes_ -- --nocapture` | 0 | 9 |
| `cargo test -p vw-api-types --lib cc04_hermes_ -- --nocapture` | 0 | 1 |
| `cargo test -p vw-agent --lib cc04_hermes_ -- --nocapture` | 0 | 5 |
| `cargo test -p vw-gateway-client --lib cc04_hermes_ -- --nocapture` | 0 | 2 |
| `cargo test -p vw-agent-apps --lib cc04_catalog_ -- --nocapture` | 0 | 7 |
| `cargo test -p vw-agent-apps --lib cc04_delete_ -- --nocapture` | 0 | 3 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

本任务 **17 项**、相邻回归 **10 项**通过，无忽略。指定 `cc04_hermes_roundtrip_keeps_unrelated_yaml`、`cc04_hermes_external_edit_is_not_lost` 均通过；另证明部分表单保留凭据/未来字段、model 与模型字典往返、字典只读/列表优先、缺失移除不建文件、非法结构/链接拒绝、授权 HERMES_HOME 组件规则、目录行修订和外部文件冲突不覆盖，以及 Intent/Marked/FilePublished/Published/Settled 五阶段重建恢复且无唯一指针。目录回归相邻断言更新为 Hermes 已可观察，缺失配置的已保存行明确 Absent，不再误报 NotImplemented。

Gateway 使用临时 HOME、真实 SQLite、管理员策略、真实回环 HTTP 与原生 SDK：save/add/if_member → API applied → YAML 中假凭据实际更新；remove-live → 保留目录记录并可随后删除；导入重复返回 0，不创建事务目录且只读源文件不变；导入字典覆盖层可观察，编辑/移除拒绝且原生文件、原目录记录不变；部分目录表单遗漏 key_env/未来字段后再次 add，文件中仍保留这些字段；假 token、异租户、异主机、缺写权限无目录/文件/事务副作用，读权限仍可 state。只将远端部署传输替换为本地回环；凭据均为 fixture，不替代文件系统、SQLite、鉴权或事务，不以 mock 次数作为行为证据。

平台：`sw_vers`/`uname -m` 为 macOS **27.0 (26A428)**、**arm64**，crate **0.6.0**。`command -v hermes` 无结果，`hermes --version` 退出 **127**；Hermes 应用版本、真实应用消费、真实账号/Keychain、网络上游、Iced 渲染和非 macOS **未验证**。未读取或修改真实用户配置。HERMES_HOME 路径规则由纯解析断言验证，真实文件链在默认隔离目录验证；显式设置覆盖和实际自定义目录部署未验收。Gateway 链接器有既有 `__eh_frame section too large` 提示，测试退出 0，不修改链接配置。

定向差异检查、暂存审查和只读 plan.md 检查通过；只提交本任务代码、独立测试、必要接线与本条证据，排除其它计划、母任务状态和此前编号文件删除。自身槽位锁释放、缓存保留；未执行全量 cargo test、fmt check、wasm 或 Makefile。验收后删除本编号任务文件，不提交其删除；下一步批次 38。

## 任务 38：Pi 增量成员管理

2026-11-07；状态：已完成。批次 37 提交 `7fc253fe7` 及其上节验收已核对；母任务 1、3 已完成，母任务 2 完整迁移仍未收口，沿用任务 1/provider-contracts.md 已记录的底座子计划 1–93 开工裁定，不冒称母任务 2 完成。未修改任何 plan.md。

来源 HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，相关来源文件无工作区差异。阅读 `services/provider/pi.rs::{add,update,remove,enable,import_from_live,update_usage_script}`、`pi_config/mod.rs::{get_pi_agent_dir,insert_pi_provider,replace_pi_provider_if_present,remove_pi_provider,validate_provider_node}`、ProviderService 直接分派及相邻测试、`PiProviderForm.tsx::{buildPiSettingsConfig,PiProviderForm}`、ProviderForm 调用者和表单测试。使用 RustCodeGraph 检查既有入口与调用链，新文件/未覆盖来源使用源码读取。

### 实现与边界

- 唯一原生业务所有者为 `vw-agent-apps/src/providers/pi.rs`。固定只读写授权 HOME 下 `.pi/agent/models.json`；执行主机 `PI_CODING_AGENT_DIR` 可指向同一授权 HOME 下的绝对目录，禁止相对路径、越界、父组件及符号链接。不接收客户端文件路径，不读取环境密钥，不打开 settings.json、auth.json 或 Keychain，不运行命令。路径覆盖实际部署未验收，仅默认隔离目录和纯路径解析有证据。
- 使用既有 JSON/JSONC 定点补丁保留其它供应商、根字段与注释原字节；声明保持原生对象、模型数组、独立原生 name 和未来字段。现有共享解析器接受注释/尾逗号，拒绝 JSON5 单引号/裸标识符、重复对象 key、损坏文档或非对象成员，不静默整份改写。成员 key 沿用既有供应商身份契约；不提供改名入口，带 originalId 的请求明确拒绝。
- Add 对不同声明的同键成员拒绝覆盖；相同声明可幂等启用。IfMember 编辑已存在成员，缺失时只保存目录，不自动启用。remove-live 在发布前将最新原生声明保存到现有目录行，保留元数据，随后可重新启用；目录删除仍需成员引用检查。导入只读源文件，只新增缺失目录行，不覆盖已有档案，完整策略归任务 44。
- 原生写入复用目录描述符锁、私有权限、ProviderTransactions、前像/目录修订检查与 pending 前滚恢复；不创建唯一 current 指针。观察目录和删除引用守卫接同一所有者。目录写入与原生发布分别返回 catalogSaved/live，冲突不假报成功。
- 强类型 DTO、执行端存储、Gateway 和 SDK 分别位于各 crate 的 `app_providers/pi.rs`；只补既有父模块接线。权限沿用管理员主机读写、租户隔离和现有分域 API。声明/脚本 Debug 脱敏，SDK 敏感凭据 header 与失败载荷丢弃沿用相邻模式。用量脚本接口只更新目录 `meta.usageScript`，不读取/回写原生快照、不执行脚本；执行属于母任务 7。Iced 表单归任务 61，本任务未新增或验收 UI。

前缀 `/v1/app-providers/pi`：

| API | SDK |
| --- | --- |
| `PUT /{id}/config` | `pi_provider_save`；catalog_only/add/if_member |
| `POST /{id}/add` | `pi_provider_add` |
| `POST /{id}/remove-live` | `pi_provider_remove_live` |
| `POST /config/state` | `pi_state`；仅成员 key |
| `POST /config/import` | `pi_import` |
| `PUT /{id}/usage-script` | `pi_usage_save`；仅目录元数据 |

### 红绿验证

红测执行任务四条相同 `cc04_pi_` 过滤命令：执行端 **2 项业务断言失败，退出 101**（新增成员未出现、同键覆盖未拒绝）；DTO **1 通过，退出 0**；Gateway **2 通过/1 业务断言失败，退出 101**（实际原生文件中编辑凭据未出现）；SDK **2 通过，退出 0**。未以编译失败或零测试作为红测。首绿 14 项通过；首绿包装脚本在所有命令结束后因运行期间调整脚本出现尾部语法错误，槽位已释放，最终阶段完整重跑，不以包装脚本错误替代命令验证。

红测、首绿、最终阶段均从仓库根原子 mkdir 动态领取槽位 **1**；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同值，覆盖仓库 build-dir。自身锁记录计划/任务/会话/PID，正常结束释放；最终脚本中断先等待子进程再退出释放，缓存保留。

最终默认 features：

| 确切命令 | 退出码 | 有效数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-agent-apps --lib cc04_pi_ -- --nocapture` | 0 | 5 passed |
| `cargo test -p vw-api-types --lib cc04_pi_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-agent --lib cc04_pi_ -- --nocapture` | 0 | 5 passed |
| `cargo test -p vw-gateway-client --lib cc04_pi_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-agent-apps --lib cc04_catalog_ -- --nocapture` | 0 | 7 passed |
| `cargo test -p vw-agent-apps --lib cc04_delete_ -- --nocapture` | 0 | 3 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

本任务 **14 项**、相邻回归 **10 项**通过，无忽略。包括指定两项行为、非法结构/路径/链接拒绝、目录修订和外部原生文件冲突，以及 Intent/Marked/FilePublished/Published/Settled 五阶段重建恢复且无唯一指针。

Gateway 使用真实回环 HTTP、原生 SDK、SQLite、临时 HOME 和管理员策略：save/add/if_member → applied 响应 → models.json 中假凭据实际更新；移除保留目录，引用解除后可删除目录行；导入重复计数归零且源文件字节不变；同键碰撞和改名请求不改目录或原生文件；外部更新的凭据/未来字段不被用量元数据或导入覆盖，移除后重新启用仍保持最新声明；假 token、异租户、异主机、缺写权限无目录/文件/事务副作用，读权限可 state。每次检查原生文件时也断言 settings.json、auth.json 字节未变，二者使用故意无法解析的假内容，证明成员管理不依赖默认模型或登录解析。只以回环替代远端传输，凭据均为 fixture，没有替代文件系统、存储、鉴权或事务。

平台：macOS **27.0 (26A428)**、**arm64**，crate **0.6.0**。`command -v pi` 无结果，`pi --version` 退出 **127**；Pi 应用版本、真实应用消费、真实账号/Keychain、网络上游、Iced 渲染及非 macOS **未验证**，未读取或修改真实用户配置。Gateway 链接器仍有既有 `__eh_frame section too large` 提示，测试退出 0，未改链接配置。

定向差异/暂存检查及只读 plan.md 检查通过；只提交本任务实现、独立测试、必要接线和本条证据，排除其它计划与此前编号任务删除。自身槽位已释放，缓存保留；未执行全量 cargo test、fmt check、wasm 或 Makefile。验收后删除本编号任务文件，不提交其删除；下一步批次 39。

## 任务 39：OMP 增量成员管理

2026-11-07；状态：已完成。已核对批次 38 提交 `681ed863d` 和上节验收；母任务 1、3 已完成，母任务 2 的完整领域迁移仍未收口，沿任务 1/provider-contracts.md 已记录的底座子计划 1–93 开工裁定消费真实接口，不冒称母任务 2 已完成。只读 plan.md 无差异。

来源 HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，来源 `services/provider/omp.rs`、`omp_config.rs` 无工作区差异。阅读 `omp::{prepare,list,sync_native,add,update,delete,remove,enable,import_from_live}`、`omp_config::{get_omp_native_agent_dir,get_omp_models_path,validate_provider,edit_provider,change_and_commit}`、ProviderService 分派、`commands/omp.rs::current_state` 和相邻内联测试；表单参考 `PiProviderForm.tsx::{buildPiSettingsConfig,PiProviderForm}` 的 OMP 分支。使用 RustCodeGraph 检查已有调用者，新增/图未覆盖路径用源码读取。

### 实现与边界

- 唯一运行时所有者为 `vw-agent-apps/src/providers/omp.rs`；`omp_validation.rs` 保留来源的 OMP 替换节点规则。沿用现有 YAML 定点补丁，保留其它成员、根字段、注释原字节，不复制永久业务所有者或整份重序列化。只校验本次替换节点，原生其它未来字段/协议不阻断独立编辑；锚点、别名、tag、merge、重复 key、含糊流式分区和损坏文档明确拒绝。
- 固定操作授权执行 HOME 下 `.omp/agent/models.yml`，原生优先级为已有 models.yml → models.yaml，均不存在时新建 models.yml。只有旧 models.json 时显式拒绝，交 OMP 自行迁移，不覆盖旧文件。执行主机 PI_CONFIG_DIR、OMP_PROFILE/PI_PROFILE 和 PI_CODING_AGENT_DIR 按原生优先级解析，目录必须位于授权 HOME，拒绝相对路径、父组件、越界和符号链接；客户端不接受路径、环境变量或命令。覆盖/命名 profile 有纯路径测试，实际部署未验证。
- Add 拒绝不同声明的同键覆盖，相同声明可幂等启用；IfMember 编辑停用档案只保存目录，不自动启用。remove-live 保存最新原生节点后移除成员，保留目录元数据，可再启用；目录删除通过真实成员引用守卫。导入只读文件，只新增缺失目录行，不覆盖已有档案，完整导入策略归任务 44。
- 写入复用目录描述符锁、私有文件、ProviderTransactions、目录修订/文件前像检查和 pending 前滚恢复，不产生唯一 current 指针。发布标记前重新检查 YAML 优先级，新出现的更高优先级文件触发冲突并保留原生文件。原生读取错误显式返回失败，目录观察显示 unavailable，不用保存档案冒充当前原生状态。
- 本行为从不打开/改写 config.yml、config.yaml、agent.db、其它登录文件或 Keychain，不修改原生默认模型，不执行凭据表达式或脚本。强类型 DTO、Gateway、SDK、执行端存储只增加 OMP 局部模块和必要父接线；沿用管理员主机权限、租户隔离、敏感凭据 header 和无载荷错误。没有新增或验收 UI，Iced 编辑归任务 62；不宣称默认模型、登录、代理或真实应用运行闭环已完成。

前缀 `/v1/app-providers/omp`：

| API | SDK |
| --- | --- |
| `PUT /{id}/config` | `omp_provider_save`；catalog_only/add/if_member |
| `POST /{id}/add` | `omp_provider_add` |
| `POST /{id}/remove-live` | `omp_provider_remove_live` |
| `POST /config/state` | `omp_state`；只返回原生成员 key |
| `POST /config/import` | `omp_import`；只读原生文件 |

### 红绿验证

修正测试接线后执行四条相同 `cc04_omp_` 过滤红测：执行端 **1 通过/4 业务断言失败，退出 101**（含指定 roundtrip 与 membership 两项）；DTO **1 通过，退出 0**；Gateway **2 通过/2 业务断言失败，退出 101**（同键覆盖未拒绝、实际文件未包含编辑凭据）；SDK **2 通过，退出 0**。编译错误与零测试未作为有效红测证据。

首轮绿测执行端 6 通过/1 失败：新增不安全 YAML 测试使用 IfMember 且目标不存在，实际正确地不写；改用 Add 验证写入拒绝。首轮 Clippy 指出迁入校验函数的一处 collapsible_if，已修正；以以下最终完整重跑为准。

红测、首绿、最终阶段都从仓库根原子 mkdir 动态领取槽位 **1**；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同值以覆盖仓库 build-dir。自身锁记录计划/任务/会话/PID，阶段结束释放，缓存保留。

最终默认 features：

| 确切命令 | 退出码 | 有效数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-agent-apps --lib cc04_omp_ -- --nocapture` | 0 | 8 passed |
| `cargo test -p vw-api-types --lib cc04_omp_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent --lib cc04_omp_ -- --nocapture` | 0 | 5 passed |
| `cargo test -p vw-gateway-client --lib cc04_omp_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-agent-apps --lib cc04_catalog_ -- --nocapture` | 0 | 7 passed |
| `cargo test -p vw-agent-apps --lib cc04_delete_ -- --nocapture` | 0 | 3 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

本任务 **16 项**、相邻回归 **10 项**通过，无忽略。含保注释增改移除、默认与登录只读、同键冲突、危险 YAML/字段/路径/链接拒绝、旧格式/优先级、目录/原生文件变更拒绝，以及 Intent/Marked/FilePublished/Published/Settled 五阶段重建恢复且无唯一指针。同步调整相邻目录测试对 OMP 已具备原生读取的预期。

Gateway 使用真实回环 HTTP、原生 SDK、SQLite、临时 HOME 和现有管理员策略：save/add/if_member → applied 响应 → models.yml 中假凭据实际更新；停用保留目录并解除引用后可删除；外部轮换的凭据/未来字段在移除后保存并可重新启用；导入重复计数归零、源文件原字节不变。异租户、异主机、假 token、缺写权限均无目录/文件/事务副作用，读权限可查 state；同键碰撞/改名请求拒绝；损坏原生配置的 state/import 失败而保存目录不变。每次读取结果文件均断言 config.yml 和 agent.db 的故意非配置假内容不变。只以回环替代远端传输，均用假凭据；未替代文件系统、存储、鉴权或事务，未读取/写入真实用户配置。

平台：macOS **27.0 (26A428)**、**arm64**，crate **0.6.0**。`command -v omp` 无结果，`omp --version` 退出 **127**；OMP 应用版本、真实应用消费、真实账号/Keychain、网络上游、Iced 渲染和非 macOS **未验证**。Gateway 链接器仍有既有 `__eh_frame section too large` 提示，测试退出 0，未改链接配置。

定向差异/暂存检查及只读 plan.md 检查通过；只提交任务 39 的实现、独立测试、必要接线和本条证据，排除其它计划、母任务状态和此前编号文件删除。自身锁已释放、缓存保留；未执行全量 cargo test、fmt check、wasm 或 Makefile。验收后删除本编号任务文件，不提交其删除；下一步批次 40。

## 任务 40：Mcode 增量供应商

2026-11-07；状态：已完成。已核对批次 39 提交 `000bd531d` 与上节正式验收。母任务 1、3 已完成；母任务 2 完整领域迁移仍未收口，本次沿用任务 1/provider-contracts.md 已记录的底座子计划 1–93 开工裁定，不误报母任务 2 完整完成。未修改任何 plan.md。

来源 HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，来源工作区干净。核对 `mcode_config::{data_dir,get_providers,provider_key_exists,validate_provider,write_entry,add_provider,set_provider,remove_provider}`，`services/provider/mod.rs::{list,save_native_provider}`、live 分派、相邻内联测试和 `tests/mcode_commands.rs`。表单规则来自 `McodeProviderForm.tsx::configSchema` 与 `helpers/yamlNode.ts::yamlNodeFromText`。使用 RustCodeGraph 检查既有模块/调用者，新文件与图未覆盖内容直接读取。

### 实现与边界

- 运行时唯一所有者为 `vw-agent-apps/src/providers/mcode.rs`。沿用 `live/patch_yaml.rs::patch_provider(CustomProvider)`，只发射选定 `custom_provider.<id>` 节点，保留其它供应商、根字段和周围注释原字节，不整份重序列化、不调用冻结上游实现。
- 服务端检查协议（缺省 anthropic-messages）、HTTP(S) 端点、非空真实形状的假密钥、models 非空对象及 model/name/limit/options、headers 类型；拒绝 redacted 凭据、非 custom 替换、非法标识及误放的配置根字段。编辑器入口只接受 `custom_provider.<同一 id>` 一个节点，不允许其它根字段、兄弟节点或改名。未知节点内字段可保留；危险/含糊原生 YAML、重复键、锚点/别名/tag/merge 和损坏文档写入明确失败。
- 默认路径为授权执行 HOME 下 `.minimax/config.yaml`；执行主机 `MINIMAX_DATA_DIR` 优先，空值回退 `MAVIS_DATA_DIR`，再回退默认目录。路径必须位于授权 HOME；相对路径、父组件、越界和符号链接拒绝，客户端没有文件路径/环境变量/命令参数。
- 同时持有保留目录描述符、现有 AppWriteGuard 和 Mcode 原生 `config.yaml.lock` 目录锁。心跳刷新原生锁，发布前检查锁身份；退出只移除自己仍持有的空锁。已有原生锁一律明确冲突，包含陈旧锁，不按时间删除其它进程的锁，需由原生应用恢复。私有文件、前像/目录修订检查、pending 发布与恢复均复用 ProviderTransactions，不产生唯一 current 指针。
- Add 不覆盖不同声明，同声明可幂等启用；IfMember 对非成员只保存目录。remove-live 先保留最新原生节点到目录后移除成员，可再启用；引用守卫禁止直接删除 live 成员。原生非 custom 账号节点不能被目录创建遮蔽或被编辑/移除；`defaultModel` 和 `defaultLightModel` 选中模型不能被删除/停用。默认与账号根字段保持原字节，未实现其选择/登录管理。
- DTO、Gateway、SDK、存储按相邻供应商模式增加局部模块与必要父接线，沿用管理员主机权限、租户隔离、目录 CAS、秘密 Debug 脱敏、敏感凭据 header 与不携带响应载荷的错误。目录观察增加真实原生成员读取。没有新增或验收 UI，Iced 表单归任务 63；不宣称真实应用消费、登录、代理或模型选择闭环通过。

前缀 `/v1/app-providers/mcode`：

| API | SDK |
| --- | --- |
| `PUT /{id}/config` | `mcode_provider_save`；catalog_only/add/if_member |
| `PUT /{id}/editor` | `mcode_provider_editor_save`；服务端 YAML 范围/schema 检查 |
| `POST /{id}/add` | `mcode_provider_add` |
| `POST /{id}/remove-live` | `mcode_provider_remove_live` |
| `POST /config/state` | `mcode_state`；仅原生 custom 成员 key |
| `POST /config/import` | `mcode_import`；只读文件，仅新增缺失目录行 |

### 红绿验证

首轮测试接线/命名编译错误不计红证据，修正后运行四条相同 `cc04_mcode_` 过滤命令：执行端 **0 通过/4 业务断言失败，退出 101**，含两条指定测试；DTO **2 通过，退出 0**；Gateway **2 通过/3 业务断言失败，退出 101**（未写实际节点、碰撞未拒绝、越界 YAML 未拒绝）；SDK **2 通过，退出 0**。没有用编译错误或零测试代替红测。

首绿修正 URL 类型引用（复用现有 reqwest::Url，未新增依赖）；随后 Gateway **6 通过/1 失败**，暴露通用删除守卫仍将 Mcode 当作未支持的问题。在既有唯一 `delete::ensure_unreferenced` 中接入已实现的 Mcode 原生观察，保持 live 引用拒绝；以下最终完整重跑通过。

各阶段均原子 mkdir 动态领取槽位 1；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同值以覆盖仓库 build-dir。自身 owner 记录计划/任务/会话/PID，所有命令与子进程结束后 trap 释放自己的锁，缓存保留。

最终默认 features：

| 确切命令 | 退出码 | 有效数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-agent-apps --lib cc04_mcode_ -- --nocapture` | 0 | 9 passed |
| `cargo test -p vw-api-types --lib cc04_mcode_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-agent --lib cc04_mcode_ -- --nocapture` | 0 | 7 passed |
| `cargo test -p vw-gateway-client --lib cc04_mcode_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-agent-apps --lib cc04_catalog_ -- --nocapture` | 0 | 7 passed；内含自调用隔离子进程，未重复计数 |
| `cargo test -p vw-agent-apps --lib cc04_delete_ -- --nocapture` | 0 | 3 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

本任务 **20 项**、相邻回归 **10 项**去重通过，无忽略。独立测试覆盖单节点增改删与其它字节保留、原生账号/默认模型保护、服务端编辑范围与 schema、路径优先级/越界/符号链接、原生锁占用与释放、外部文件/目录修订冲突，以及 Intent/Marked/FilePublished/Published/Settled 五处故障后重建事务执行器恢复且没有唯一 current 指针。Gateway 测试链接器有既有 `__eh_frame section too large` 提示，测试退出 0，未修改链接配置。

Gateway 使用真实回环 HTTP、原生 SDK、SQLite、临时 HOME 和现有管理员策略；save/editor/add/if_member → API applied → config.yaml 实际假凭据更新，remove-live 保留目录且解除引用后可删除。导入不回写文件、不覆盖已有档案；异租户/异主机/假 token/无写权限/编辑器越界/账号碰撞/原生锁占用均不写原生配置或创建档案。默认轻量模型受保护，原生外部轮换的凭据和未来字段在 remove-live 后保存且可再启用；损坏原生文件显式返回错误。只以回环替代远端传输，未替代鉴权、文件系统、存储或事务，未读取/写入真实用户配置。

平台：macOS **27.0 (26A428)**、**arm64**，crate **0.6.0**。`command -v mcode` 无结果，`mcode --version` 退出 **127**；实际 Mcode 版本/消费、真实账号/Keychain、网络上游、Iced 渲染和非 macOS **未验证**。

定向差异/暂存审查与只读 plan.md 检查通过；仅提交本任务实现、独立测试、必要接线和本条证据，排除母任务状态、其它计划与此前编号文件删除。自身槽位锁释放、缓存保留；未运行无过滤全量 cargo test、fmt check、wasm 或 Makefile。验收后删除本编号文件，不提交该删除；下一步批次 41。

## 任务 41：Kimi 供应商配置

2026-11-07；状态：已完成。核对批次 40 提交 `0f609699c` 与上节验收。母任务 1、3 已完成；母任务 2 整体领域迁移尚未收口，沿用任务 1/provider-contracts.md 已记录的底座子计划 1–93 开工裁定，不误报母任务 2 完整完成。未修改任何 plan.md。

来源快照 `ca58c0313360eae7c851fbd7553c1614c7af03e7`，来源工作区干净。阅读 `kimi_config::{read,get_providers,validate_provider,apply_provider,apply_provider_unchecked,write,current_provider_in,add_and_commit,switch_and_commit}`、相邻内联测试、`tests/kimi_commands.rs`、`services/provider/mod.rs` 的原生读写/官方连接调用者，及 `helpers/kimiToml.ts::{kimiConfigToToml,kimiConfigFromToml}`、`KimiProviderForm.tsx` 调用和 `kimiConfigSchema`。先使用 RustCodeGraph 检查既有适配/事务/调用者，新增文件与来源配置用直接读取。

### 实现与边界

- 唯一运行时所有者为 `vw-agent-apps/src/providers/kimi.rs`，不调用冻结上游。配置按 `{provider, models}` 保存，写入 `[providers.<id>]` 和点名模型表。通过 toml_edit 只替换这一家的节点，保留其它供应商、模型、根字段、登录引用、服务表与周围注释；目录观察复用相同读取/引用检查。
- 服务端验证类型、HTTP(S) 端点、凭据/环境变量/OAuth 引用形状、正数上下文长度、模型别名及归属、能力/overrides/headers 类型和 TOML 可表示性；拒绝掩码和不存在的供应商/模型引用。编辑入口仅接受同一个供应商及其模型的原生 TOML，禁止全局默认、其它供应商、服务表或改名。
- 模型别名不按分隔符猜供应商。默认/次级模型引用不允许悬空，跨供应商别名碰撞拒绝；首次 Add 在没有默认时选定首个模型，已有默认与次级选择保留。Add 不覆盖已存在的原生节点；IfMember 对非成员只存目录。
- 只消费已有官方登录节点：导入和编辑支持 `managed:kimi-code` 一类原生键，SDK 将键作为 URL segment 编码；编辑官方节点时固定原生连接、端点和 OAuth 引用，仅允许模型变化；禁止伪造新的官方连接，不读取、创建或删除 credentials。官方节点移除明确拒绝，服务/账号关联维护留给登录管理领域。没有实现 OAuth，也没有验收真实登录或上游可用性。
- 使用授权执行 HOME 下 `.kimi-code/config.toml`；保留非空 `KIMI_CODE_HOME` 明确不支持的既有边界，不接受客户端路径、环境或命令参数。目录与文件符号链接拒绝。保留目录描述符和 AppWriteGuard，复用 ProviderTransactions 的物理锁、私有文件、前像检查、目录 CAS、pending 及恢复；保留目录元数据，移除前归档最新原生节点。
- API/SDK/存储按相邻模块接线，沿用既有主机读写权限、租户作用域、敏感 header、Debug 脱敏和无响应载荷的错误。只证明 TOML 原生成员和默认引用；不创建托管唯一 current 指针，不声称代理/模式切换或 Iced 闭环通过。UI 留给所属桌面表单任务。

前缀 `/v1/app-providers/kimi`：

| API | SDK |
| --- | --- |
| `PUT /{id}/config` | `kimi_provider_save`：catalog_only/add/if_member |
| `PUT /{id}/editor` | `kimi_provider_editor_save`：服务端 TOML 范围与 schema 检查 |
| `POST /{id}/add` | `kimi_provider_add` |
| `POST /{id}/remove-live` | `kimi_provider_remove_live` |
| `POST /config/state` | `kimi_state`：仅成员标识 |
| `POST /config/import` | `kimi_import`：原生文件只读，仅新增缺失目录行 |

### 红绿验证

红阶段四条默认 features 的 `cc04_kimi_` 过滤命令：执行端 **0 通过/2 业务断言失败，退出 101**，即任务指定两项；DTO **2 通过，退出 0**；Gateway **2 通过/1 业务断言失败，退出 101**（实际原生文件未写入编辑后的假凭据）；SDK **2 通过，退出 0**。编译错误和零测试未作为红测证据。

首轮绿测执行端 7、DTO 2、Gateway 6、SDK 2 通过。随后补充官方节点接收非对象声明的拒绝断言，并在格式化后重跑最终检查。各阶段均由原子 mkdir 动态领取槽位 **1**；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同值以覆盖仓库 build-dir。owner 记录计划/任务/会话/PID，阶段结束 trap 删除自身记录、释放锁，缓存保留。

最终默认 features：

| 确切命令 | 退出码 | 有效数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-agent-apps --lib cc04_kimi_ -- --nocapture` | 0 | 7 passed |
| `cargo test -p vw-api-types --lib cc04_kimi_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-agent --lib cc04_kimi_ -- --nocapture` | 0 | 6 passed |
| `cargo test -p vw-gateway-client --lib cc04_kimi_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-agent-apps --lib cc04_catalog_ -- --nocapture` | 0 | 7 passed |
| `cargo test -p vw-agent-apps --lib cc04_delete_ -- --nocapture` | 0 | 3 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

本任务 **17 项**、相邻回归 **10 项**去重通过，无忽略。执行端包括指定两项、默认/次级引用与别名碰撞、官方连接保护、编辑范围和 schema、0600/符号链接、外部文件/目录修订冲突，以及 Intent/Marked/FilePublished/Published/Settled 五处故障后重建事务执行器恢复，未生成唯一 current 指针。

Gateway 测试使用真实回环 HTTP、正式 SDK、SQLite、临时 HOME 和管理员 policy：SDK save/editor/add/if_member → API applied → 执行端 TOML 假凭据实际更新；首次激活自动补模型归属/默认引用；默认删除、异租户/主机/假 token/无写权限、错误模型归属和伪造官方连接拒绝且无文件/目录记录副作用。官方导入与编辑保留端点/OAuth 引用且不创建凭据文件；导入不写原文件、不覆盖目录；移除保留可复用目录行，live 成员受删除守卫保护。仅替代远端 HTTP 传输，未替代文件系统、存储、鉴权或事务，不读取/写入真实用户配置；所有密钥均为 fixture。

平台 macOS **27.0 (26A428)**、**arm64**，crate **0.6.0**。`command -v kimi` 无结果，`kimi --version` 退出 **127**；实际 Kimi 应用版本/消费、真实账号/Keychain、网络上游、Iced 渲染和非 macOS **未验证**。Gateway 链接器有既有 `__eh_frame section too large` 提示，测试退出 0，未修改链接配置。

定向差异/暂存检查通过，plan.md 无差异；只提交本任务实现、独立测试、必要接线和本条证据，排除其它计划、母任务状态和既有编号文件删除。自身槽位锁已释放、缓存保留；未执行无过滤全量 cargo test、fmt check、wasm 或 Makefile。验收后删除本编号文件且不提交其删除；下一步批次 42。

## 任务 42：Dsh 供应商配置

2026-11-07。批次 41 提交 `56cf7510f` 和上节正式验收已核对。母任务 1、3 已完成；母任务 2 整体领域迁移仍未收口，沿用任务 1/provider-contracts.md 已登记的底座子计划 1–93 开工裁定，不冒称母任务 2 已完成。所有 plan.md 保持只读。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，工作区干净。阅读 `dsh_config::{data_dir,target_paths,read_paths,provider_entry,credential_document_from,get_providers,validate_provider,native_provider,update_provider,save_and_commit,save_paths_and_commit,account_grant}`、相邻供应商/profile/账号测试、`tests/dsh_commands.rs` 生命周期和失败无副作用断言、`services/provider/mod.rs` 列表/增量增删/保存调用者，以及 `DshAccount.tsx` 的共享状态、登录与原生账号卡和 `ProviderList.tsx` 消费。使用仓库 RustCodeGraph 技能检查既有事务、供应商和调用者；新增代码与来源用直接读取。未引入冻结 upstream 的运行依赖。

### 实现和范围

- `vw-agent-apps/src/providers/dsh.rs` 是 Dsh 纯配置投影所有者，`dsh_files.rs` 只管理固定执行路径、目录描述符和锁。只替换 `llm-pi-ai.config.providers.<id>`；其它供应商、config 字段、插件、周围注释和其它插件的 `!!js` 原文保留。目标 config 含动态 YAML/引用或无法安全编辑的布局明确拒绝，不通过全文件重写伪支持。
- 消费与来源一致的 `{api, options:{baseURL,apiKey,headers}, models:{...}}` 供应商声明，转换为原生 models 数组、displayName/baseURL/headers、contextWindow/maxTokens、input 和 reasoningEfforts。校验 ID、保留官方 ID、协议、端点、凭据、模型及正数容量；掩码不作为密钥。纯目录保存、首次 Add、已存在成员编辑分别显式表达；Add 不覆盖原生节点，IfMember 对非成员仅存目录。
- 静态密钥写入共享 `.credentials.yaml` 的专属 `refs.SHIJU_DSH_<ID>`，既有 `records`（含 DeepSeek 账号 grant/device）、其它 refs 与未知字段语义保留；不复制账号、设备记录、会话、技能、AGENTS 或 sdk/sdk-minimal 配置。凭据文档版本非 1 明确拒绝。显式 apiKeyEnv 只能引用执行端凭据文件中的既有非空值，不从进程环境回退；共享引用被其它供应商使用时禁止轮换。
- 目录记录的 `meta.authBinding`/托管标志，以及无可解析凭据的 apiKeyEnv，返回 HTTP 422 与强类型 `DshError { code: invalid_binding }`；API 不保存目录或写入文件，SDK 仅保留白名单错误码，不透传任意失败载荷。此处拒绝无法由供应商目录解析的托管绑定，未实现账号管理或 OAuth。
- 授权 HOME 下默认 `.dsh`，执行端 DSH_HOME 仅允许 HOME 内的绝对目录；客户端不接受路径/环境/命令。逐组件 no-follow 打开，目录不安全或符号链接明确拒绝。Web/Desktop 始终准备，headless/acp 仅在已有 patch 时参与；sdk/sdk-minimal 不参与。不支持链接 profile 别名，不把这项拒绝描述为别名兼容。
- 根目录和所有 profile 的保留描述符交给已有 `lock_directories`/ProviderTransactions，一个 pending 管理凭据和全部 patch；使用 0600、前像检查、目录 CAS、租户/主机身份、发布前校验和前滚恢复，不制造唯一 current。复用 Dsh 原生 exclusive-create `<file>.lock` 协议，未知/活跃/遗留锁不擅自删除，需原生侧解除后重试。各 profile 的既有同名供应商内容不一致或 home 级 llm-pi-ai config 遮盖时拒绝写入。
- 业务存储沿 `AppProviderStore` 的唯一租户命名空间保存并 CAS，元数据保留；通用 catalog 接入本实现只读成员观察。Gateway/SDK 仅补本任务父注册，不另开通用命令入口。目录保存与 live applied/not_attempted/failed/recovery_required 分开报告。

| API | SDK |
| --- | --- |
| `PUT /v1/app-providers/dsh/{id}/config` | `dsh_provider_save`，catalog_only/add/if_member |
| `POST /v1/app-providers/dsh/{id}/add` | `dsh_provider_add`，消费既有目录行 |
| `POST /v1/app-providers/dsh/config/state` | `dsh_state`，只返回原生成员 ID |

本任务未新增原生账号卡、登录动作、导入或移除 API；通用导入策略归任务 44，Iced 表单归任务 65。没有验收 UI、OAuth 或真实 Dsh 消费。

### 红绿验证

红阶段四条默认 features 的 `cc04_dsh_` 命令：执行端 **0 通过/2 业务断言失败，退出 101**，即 `cc04_dsh_provider_edit_keeps_native_account`（没有新增专属凭据引用）和 `cc04_dsh_invalid_binding_is_explicit`（绑定未被拒绝）；DTO **1 通过，退出 0**；Gateway **0 通过/1 业务断言失败，退出 101**（SDK 返回后执行文件仍无供应商）；SDK **2 通过，退出 0**。没有将编译失败或零测试算作红阶段证据。

首轮绿测执行端 **5 通过/1 失败，退出 101**，定位新建 YAML 的 `- config:` 顺序与增量编辑定位不兼容，已兼容两种顶层字段顺序；Gateway 首轮缺少显式校验函数限定路径，退出 101，已修复；DTO 1、SDK 3 通过。随后拆开纯投影和文件持有职责、补共享凭据引用防护及凭据版本检查，再执行最终验证。

所有阶段均通过原子 mkdir 动态领取共享槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，同时令 `CARGO_BUILD_BUILD_DIR` 等于该绝对路径以覆盖仓库 build-dir。owner 记录计划/任务/会话/PID；阶段结束 trap 删除自身记录并释放锁，缓存保留。最终默认 features 验证：

| 确切命令 | 退出码 | 有效数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-agent-apps --lib cc04_dsh_ -- --nocapture` | 0 | 6 passed |
| `cargo test -p vw-api-types --lib cc04_dsh_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent --lib cc04_dsh_ -- --nocapture` | 0 | 3 passed |
| `cargo test -p vw-gateway-client --lib cc04_dsh_ -- --nocapture` | 0 | 3 passed |
| `cargo test -p vw-agent-apps --lib cc04_catalog_ -- --nocapture` | 0 | 7 passed，含子进程断言，去重计数 |
| `cargo test -p vw-agent-apps --lib cc04_delete_ -- --nocapture` | 0 | 3 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

本任务 **13 项**和相邻回归 **10 项**有效测试通过，无忽略。独立执行端测试覆盖指定两项行为、其它账号 records/refs、兄弟节点/插件原文、profile 差异/home 遮盖/失效凭据拒绝、IfMember 未加入无副作用、Intent/Marked/FilePublished/Published/Settled 五处注入失败后重建事务执行器前滚、0600、sdk 不触碰、外部文件/CAS 校验、未知原生锁及符号链接拒绝。故障注入使用正常进程内清理，不冒称验证了杀进程后的原生遗留锁自动回收。

Gateway 采用真实回环 HTTP、正式 SDK、SQLite、管理员授权 policy 和临时 HOME：SDK catalog-only → API not_attempted 且原文件不变；SDK add/save-if-member → API applied → Web/Desktop 中出现专属 apiKeyEnv，凭据 refs 实际轮换且原生 DeepSeek grant、其它 refs 保留；非成员编辑仅存目录；无效目录绑定与缺失凭据引用返回同一可识别 invalid_binding，记录与文件不变；异租户/主机、假 token、无写权限均拒绝且无目录行/文件副作用。只替代不可控外部边界，未替代鉴权、文件系统、存储或事务；全部假密钥，不读取或修改真实用户配置。

平台 macOS **27.0 (26A428)**、**arm64**，crate **0.6.0**。`command -v dsh` 无结果，`dsh --version` 退出 **127**；Dsh 应用版本、实际应用消费、真实账号登录、网络上游、Iced 渲染和非 macOS **未验证**。Gateway 链接器出现既有 `__eh_frame section too large` 提示，测试退出 0；强制 Clippy 退出 0，未改链接配置。

状态：**已完成（本任务的原生配置/API/SDK 行为边界）**。定向 diff/check、暂存审查与只读 plan.md 无差异检查通过；仅提交本任务实现、独立测试、必要注册/目录观察和本条正式证据。既有母任务状态、其它计划和此前编号文件删除不纳入提交。自身槽位锁释放，缓存保留；未执行无过滤全量 cargo test、fmt check、wasm 或 Makefile。验收后删除本编号文件且不提交该删除；下一步批次 43，真实 Dsh/UI 按所属任务验收。

## 任务 43：Agy 供应商配置

2026-11-07。核对批次 42 提交 `007f365ad` 及上节正式验收；母任务 1、3 已完成，母任务 2 完整迁移未收口，沿 provider-contracts.md 底座子计划 1–93 的开工裁定消费真实接口。只读 plan.md 未修改。原阻塞已由本次继续指令明确解除：Agy“切换”指终端启动选择持久 profile，无全局 current；允许必要路径/启动适配，固定共享链接不得替换用户目录。

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 退出 0、来源干净。阅读 `agy_config::{prepare_launch,prepare_key_profile,key_profile_patch,link_shared_config}`、`services/provider/agy_direct::{is_official,live_settings,terminal_launch}`、`ProviderService::switch` 明确拒绝分支、终端直接调用者，以及相邻 cards_never_write_the_real_settings_credentials_or_shell_files / key_launch_prepares_a_private_profile_linked_to_shared_config 测试。使用仓库 RustCodeGraph 技能定位现有启动/事务调用链；来源与未覆盖新模块直接读取，没有链接冻结 upstream 作为运行依赖。

### 实现与接口

- `vw-agent-apps/src/providers/agy.rs` 为 Agy 校验与 profile 准备所有者，`agy_files.rs` 只持有目录/链接核验和进程租约；既有 `providers/paths.rs::agy_profile_directory` 从管理员授权 `credential_root` 固定推导 `agy-profiles/<base64url(host)>/<base64url(sha256(tenant JSON))>/<provider-id>`。租户来自鉴权中间件，主机来自授权 policy，供应商 ID 校验为固定单路径段；客户端不能提供根、文件、链接目标或命令。0700 目录和新写入的 0600 settings；没有照搬 `.cc-switch` 或新增路径配置。
- 私有 profile 使用真实 `antigravity-cli/settings.json`，仅补丁更新 `modelProvider: gemini` 和显示名 `model`；不选模型时删除该字段，保留注释、未知选项及原生会话。`GEMINI_API_KEY` 和可选 `GOOGLE_GEMINI_BASE_URL` 仅进入所选子进程环境，不写 settings、argv、终端包装脚本或共享目录。默认端点时不继承 ambient endpoint，子进程 HOME 使用授权主机 HOME。
- 固定 `config` 链接只指向已存在、逐组件 no-follow 打开的授权 `~/.gemini/config`。链接采用 create-only 准备：既有相同目标复用，异目标链接、真实目录/文件明确拒绝，绝不替换或回滚未知用户条目。失败可以保留空私有目录或有效固定链接，不能由这些准备产物推断启动成功；适配器对共享原生目录只读，不替用户初始化它。
- settings 使用既有唯一 ProviderTransactions 的私有文件补丁、pending、发布前校验和前滚恢复；每个隔离 profile 使用自己的事务注册空间，避免多个 profile 争用同一应用 journal。固定链接和保留目录身份在恢复前、提交校验及启动前核验；通用写守卫仍拒绝任意符号链接，无新文件/命令 API，也无链接替换意图。没有更改共享事务语义。
- profile 根的 0600 flock 租约保留到子进程回收或启动失败；同一 profile 活跃时明确拒绝再次准备/轮换，其它租户、主机和供应商独立。未把互斥限制伪装成同 profile 多进程支持。进程退出、失败或丢弃准备对象均释放租约，但保留会话/profile；继承的任意凭据通过现有 env_clear 移除。
- model-only / 无 key 的卡走原生登录，最多传会话级 `--model=<显示名>`，不创建私有 profile、不修改登录。来源旧记录只允许显式 category=official 或 google-official promotion 选择 native，不按可编辑名称猜测；无法解析的 managed authBinding/liveConfigManaged 明确拒绝。未实现账号切换、Keychain 写入或 OAuth。
- 目录记录沿 `AppProviderStore` 的既有租户命名空间和 CAS 保存；Agy DTO 支持 name、可选 model/apiKey/baseUrl，校验控制字符、错误端点、无 key 的端点与掩码。保存返回 `catalogSaved=true`，不声称 live 已启动；编辑的配置在下一次授权 launch 准备后生效。读取只返回摘要，Debug/SDK 错误不含凭据。通用 catalog 的 Agy current/in-use 为 NotApplicable。

| API | SDK / 结果 |
| --- | --- |
| `PUT /v1/app-providers/agy/{id}/config` | `agy_provider_save`；只存目录，返回 catalogSaved |
| `POST /v1/app-providers/agy/{id}/switch` | `agy_provider_switch`；鉴权后固定 HTTP 422 / agy_switch_unsupported |
| 既有 `POST /v1/agent-apps/launch`，providerId 选择卡片 | 既有 `agent_apps_launch_identity_authorized`；修正真实 dispatch 为持久 Agy profile，返回 session ID 并可查询实际进程状态 |

未新增 UI；Iced 编辑归任务 66。真实 native settings、credentials、shell 文件和全局 current 均不写；非 macOS 的供应商启动明确拒绝，不实现 Windows 链接命令。

### 红绿与平台证据

红阶段先修复测试接线中的字面量编译错误（不计业务失败）；动态槽位 2 下重跑执行端命令，**0 passed / 2 业务断言失败，退出 101**：`cc04_agy_switch_preserves_native_profile` 缺真实 profile 布局，`cc04_agy_failed_apply_keeps_original_binding` 未拒绝既有真实 config 目录。其余红阶段槽位 1：DTO 1、Gateway 1、SDK 1 通过，均退出 0，无零测试充数。首轮实现两项业务测试通过；随后补恢复/隔离/真实 HTTP 场景，修复新 handler 的 Result 类型推导编译错误，再完成验证。

全部 Cargo 阶段原子 mkdir 动态领取共享槽位 **1** 或 **2**，记录计划/任务/会话/PID；`CARGO_TARGET_DIR` 分别为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。同时设置 `CARGO_BUILD_BUILD_DIR` 为同一目录，覆盖仓库 build-dir，未另建缓存。阶段退出释放自身锁、保留缓存。

最终默认 features 验证使用动态领取的槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径：

| 确切命令 | 退出码 | 有效数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-agent-apps --lib cc04_agy_ -- --nocapture` | 0 | 6 passed |
| `cargo test -p vw-api-types --lib cc04_agy_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent --lib cc04_agy_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-gateway-client --lib cc04_agy_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-agent-apps --lib cc_apps_provider_launch_isolation_ -- --nocapture` | 0 | 9 passed，子进程断言不重复计数 |
| `cargo test -p vw-agent-apps --lib cc04_catalog_ -- --nocapture` | 0 | 7 passed |
| `cargo test -p vw-agent-apps --lib cc04_paths_ -- --nocapture` | 0 | 6 passed |
| `cargo test -p vw-agent-apps --lib cc04_write_guard_ -- --nocapture` | 0 | 5 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

本任务 **11** 项及相邻回归 **27** 项，去重 **38** 项通过，无忽略。执行端独立测试覆盖指定两项行为、0600/0700、原生 settings/假登录引用不变、共享链接内容/异链接及真实目录拒绝、目录被替换后拒绝执行、同 profile 活跃租约、不同租户/主机/供应商隔离、失败启动/重复准备保留 session、注释/未知设置保留、模型删除/缺省端点清除、native model-only 无私有目录与密钥、非法输入/权限跳过无副作用，以及 Intent/Marked/FilePublished/Published/Settled 五处注入失败后的重建/前滚与空 current/mode。

Gateway 使用真实回环 HTTP、正式 SDK、SQLite、管理员 policy、应用发现和隔离 HOME；只替代真实终端应用的连接动作（连接既有 TerminalSession 的 Unix socket），实际受控假 agy 进程必须通过 TTY、环境、真实 settings 与链接断言后才写 session/proof。SDK save → HTTP catalogSaved 且无 profile；SDK global switch → HTTP 422 且无 profile；SDK identity launch → API session → 真实私有 settings/config 链接和持久 session。错误 token、未许可 provider 身份、无配置写权限均拒绝且不创建 profile/目录行；异租户记录不可读。没有替代鉴权、事务、文件系统、存储或进程生命周期；所有凭据均为假值。

最后审查将 launch 的 profile 解析改为消费同一份已捕获目录行，发布前复核同一前像，避免双读混用版本；重新动态领取槽位 1，以同一绝对 CARGO_TARGET_DIR 运行 `cargo fmt --all`（退出 0）、`cargo test -p vw-agent --lib cc04_agy_ -- --nocapture`（退出 0，2 passed）及 `cargo clippy --all-targets --all-features -- -D warnings`（退出 0）。此复验不重复计入 38 项去重数量。

平台 macOS **27.0 (26A428)**、**arm64**，crate **0.6.0**。`command -v agy` 无结果、退出 **1**；真实 Agy 应用版本、实际应用消费、原生 Keychain 登录、账号切换、网络上游、Iced 渲染和非 macOS 均 **未验证**。假进程版本仅为发现 fixture，不当成真实应用验收。Gateway 测试有既有链接器 `__eh_frame section too large` 提示，测试退出 0；强制 Clippy 退出 0，没有修改链接参数或降级检查。未运行无过滤全量 cargo test、fmt check、wasm 或 Makefile。

定向差异审查排除 cargo fmt 引起的 Dsh 纯格式差异；不纳入母任务状态、此前编号文件删除或其它计划，所有 plan.md 保持只读。正式证据保留；本编号任务在最终通过后删除且不提交其删除。下一步批次 44；Agy 的真实应用/UI 验收按所属边界另行开展。

状态：**已完成（持久 profile 配置与授权终端选择边界）**。本阶段自身槽位 1 锁已释放，之前槽位 2 锁亦已释放，缓存保留；定向 diff/check、暂存审查和只读 plan.md 无差异检查通过。仅提交本任务代码、独立测试、必要父注册与本条证据。

## 任务 44：从当前配置导入策略

2026-11-07。批次 43 提交 `97767eeac` 与本文件正式验收已核对。母任务 1、3 已完成；母任务 2 完整迁移未收口，沿任务 1/provider-contracts.md 已登记的底座子计划 1–93 开工裁定消费真实接口。最初审查发现缺少部分原生导入器并登记阻塞；继续指令已明确授权在既有领域所有者补齐最小能力，解除阻塞。未修改任何只读 plan.md。

### 来源与实现边界

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD 为 `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 均退出 0，来源干净。已读取 `services/provider/live.rs::{read_live_settings,import_default_config,should_import_default_config_on_startup}`、`commands/provider.rs::import_default_config_internal`、`lib.rs` 的启动导入调用、golden/import_rows.rs 的 first_run_* 测试，以及 Codex/Grok/Agy 原生读取、Grok 官方种子 ID 和 Dsh 的 get_providers/get_providers_from。RustCodeGraph status/explore 退出 0；图包含冻结 upstream，新增原生模块以定向源码读取补查。来源只读，未运行来源测试。

- `vw-agent-apps/src/providers/import.rs` 只拥有分派和首次导入策略；应用解析仍归现有 Claude、Codex catalog、Gemini、Grok、Agy 及各增量应用模块。新增解码器解析原生输入，没有倒用写投影重建输入，没有新增路径/命令 API、第二套档案库或永久业务所有者。
- `AppProviderStore::import_native` 使用同一租户命名空间及现有应用 CAS 保存。首次导入遇到任何既有目录行即跳过；主动导入也保护既有普通目录。无自动官方种子机制的应用不按名称猜测 seed。Grok 保留来源的主动官方例外：原生官方态保存 `grokbuild-official`，启动不重建；该官方条目单独存在不阻止主动导入自定义 default，自定义目录已存在则不覆盖或新增另一个 default。
- OpenCode/OpenClaw/Hermes/Pi/OMP/Mcode/Kimi 分派到已交付的原生读取/declarations，主动导入仅新增缺失成员，已有行及其元数据不覆盖；启动跳过这些增量应用。Pi/OMP 使用各自 JSON/YAML 文件与既有有界路径规则，不经过通用 read_live_settings。
- Codex 读取固定 config.toml/auth.json，只复制 API key 与原生配置，登录令牌不进入档案；preserveLogin 仅表达后续既有登录所有者保留原生登录的请求，没有创建登录绑定或声称已经切换。非 file 凭据存储、坏路由形状和占位凭据明确拒绝。Gemini 读取现有 .env/可选 settings.json，保留受支持凭据和模型；dotenv 不执行环境展开，无法无损解码的双引号转义明确拒绝。Grok 去除供应商外的 MCP 配置，拒绝残缺自定义模型表；Agy 只从既有 settings 的 model 创建原生登录卡，空模型不导入，不读取进程 key、不创建持久 profile。
- Desktop 泛导入返回 HTTP 422 / use_claude_compatible_import，现有“从 Claude 导入兼容供应商”入口保留。Dsh 的来源 get_providers 会合并多个 profile 并解析凭据引用/环境回退，来源没有专用 import_dsh 导入入口，通用 import_default 对其增量模式直接跳过；本任务按继续授权的“有据拒绝”返回 HTTP 422 / dsh_native_profile_import_unsupported，没有把原生成员观察伪装为成功导入或读取任意环境凭据。
- API 为 `POST /v1/app-providers/{app}/catalog/import`，DTO 为 targetHostId/trigger（manual 或 startup），SDK 为 `provider_import`。沿用分域 router、认证中间件与租户作用域；同时要求主机配置读、写权限，鉴权在存储/原生文件打开前完成。成功只返回 imported 数量；非法目标/权限/平台沿既有契约，存储故障 503、并发冲突 409、配置不可导入 422。SDK 敏感 token header 与无载荷错误沿现有模式，不回显档案、凭据或原生配置。
- 真实 gateway bootstrap 既有启动恢复路径接到 `startup_provider_policy` → `access::import_provider_policy` → 同一 import_native。仅消费管理员 policy 中同一 grant 的读写授权、本地在线 host 和显式 tenant；缺失/坏配置延后且仅记录固定消息。导入不写 live、登录、全局 current/mode，不引入新的原生事务。未新增 UI；桌面入口由后续所属任务消费 SDK。

### 验证

红阶段动态槽位 1：`cargo test -p vw-agent-apps --lib cc04_import_ -- --nocapture` 编译接线成功，两个指定测试在业务策略断言失败，**0 passed / 2 failed，退出 101**；`cargo test -p vw-api-types --lib cc04_import_ -- --nocapture` **1 passed，退出 0**。初次 Gateway 验证发现新启动 helper 的 anyhow/String 转换编译错误，已修复；后续完整过滤验证通过。没有将找不到模块、零测试或历史结果计作行为证据。

所有 Cargo 阶段均原子 mkdir 动态领取仓库共享槽位 **1**，记录任务/会话/PID；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径，覆盖仓库 build-dir 而未另建构建空间。最终验证的确切命令和结果如下：

| 确切命令 | 退出码 | 有效数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-agent-apps --lib cc04_import_ -- --nocapture` | 0 | 4 passed |
| `cargo test -p vw-api-types --lib cc04_import_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent --lib cc04_import_ -- --nocapture` | 0 | 7 passed |
| `cargo test -p vw-gateway-client --lib cc04_import_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent-apps --lib cc04_agy_ -- --nocapture` | 0 | 6 passed |
| `cargo test -p vw-agent --lib cc04_agy_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-agent --lib cc04_recovery_ -- --nocapture` | 0 | 1 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 非行为测试 |

本任务 **13** 项、相邻回归 **9** 项，共 **22** 项去重通过，无忽略。

真实回环 HTTP 使用正式 SDK、Gateway handler、鉴权 policy、租户 SQLite 与隔离 HOME；没有替代文件系统、存储、权限或业务所有者。覆盖五个默认导入应用的真实文件→目录、原生文件即时不变、坏原生文件下重复启动仍跳过、已有非 default 原生登录卡、Grok 主动/启动区别、Pi/OMP 专用增量、已有成员不覆盖、Desktop/Dsh 明确拒绝、错误 token/缺读权限/缺写权限无目录与原生副作用，以及异租户隔离。直接启动策略调用证明只有授权的空目录可以首次导入，随后原生变化不能覆盖档案。SDK 独立 HTTP fixture 验证请求路径/trigger/租户及错误敏感载荷丢弃；所有 key/token/login 都是假值。

平台为实际 macOS **27.0 (26A428)**、**arm64**，crate **0.6.0**。已安装应用版本未查询；真实应用消费、真实登录/Keychain、网络上游、Iced 渲染和非 macOS **未验证**，不把隔离文件与 HTTP 证据扩展为真实应用端到端验收。Gateway 测试链接器有既有 `__eh_frame section too large` 提示，行为测试仍退出 0；强制 Clippy 结果见表，没有降低 lint 或修改链接参数。未运行无过滤全量 cargo test、fmt check、wasm 或 Makefile。

状态：**已完成（统一目录导入策略与授权启动边界）**。差异/暂存检查及只读 plan.md 检查通过；仅提交本任务代码、独立测试、必要父注册和本条正式证据，排除母任务状态、此前编号文件删除、其它计划与 cargo fmt 产生的 Dsh 纯格式差异。所有阶段自身槽位 1 锁已释放，缓存保留；完成后删除本编号文件，不提交该删除。下一步批次 45。

## 任务 45：直连路由聚合状态契约

2026-11-07；状态：已完成（状态与客户端契约决策边界）。批次 44 提交 `a85e99b0f` 及正式验收已核对。母任务 1、3 已完成；母任务 2 的完整领域迁移仍未收口，沿 provider-contracts.md 底座子计划 1–93 的既有开工裁定消费真实接口。原阻塞由继续指令解除：45 负责状态、执行端契约计算和只读决策，46 负责真实模式切换、文件免重写及路由服务接线。未修改任何 plan.md。

### 来源、实现与交付边界

- 对照冻结来源 `mode/state.rs::{ModeState,StackState}`、`current.rs::provider_for`、`contract.rs::{claude,gemini,grok,claude_desktop}`、`stack.rs` 及直接调用者 `controller.rs::{write_proxy,switch_route_locked}`。Mode 保持 Direct/Proxy；stack 是独立持久状态，退回 Direct 保留开关、成员和永久 key 登记簿，只有 Proxy 且 enabled 时逻辑激活。attached 独立展示，不把配置附着当作代理运行证据。
- 唯一所有者仍是 `vw-agent-apps/src/providers/live/commit.rs::ProviderTransactions`，AppState/PendingTarget 新增 stack 和独立租户归属；未提供新的存储或任意状态写入 API。普通应用适配只补新增可选字段的默认值，行为不变。拒绝重复成员、非法标识、跨租户更新、无归属非空 stack、移除或改派已登记 key。包含 stack 的文件意图使用版本 4，旧执行器会拒绝；恢复保持版本 3 的目录安全校验，不丢 stack。
- `vw-agent-apps/src/providers/mode_state.rs::{ClientConfiguration,observe,rewrite_decision}` 提供同一算法供 46 消费。客户端投影按 Claude/Gemini/Grok/Desktop/Codex 强类型应用分支表达，摘要递归规范化对象顺序并保留数组顺序，覆盖最终地址、凭据、模型/别名/显示名与独有字段。独有值由已知字段规则保留；客户端契约版本 2 与既有版本 1 的指针/清理记录明确分开，旧记录保守要求重写。相同模式和契约才能 SkipRewrite；Proxy 分离时、缺契约、跨模式或内容变化均要求重写。
- `ClientConfiguration::saved` 只在执行端从已保存行生成 Direct 配置投影，复用已有 Claude、Gemini、Grok 字段投影及官方分类。其余应用的已保存行决策明确返回 422，状态观察不受此限制。内部五类最终投影契约计算不宣称已具备对应代理投影生成器。46 必须传最终客户端投影，不能拿 Direct 行摘要代替代理配置；需持有实际应用锁并在发布前验证输入。
- DTO、Gateway、SDK 位于各 crate 的 `app_providers/mode_state.rs`，沿原分域父模块接线。`GET /v1/app-providers/{app}/mode/state?targetHostId=...` 返回已持久化的 mode、直连指针、逻辑使用者、保留路由、attached、契约存在位与 stack 成员；可选 providerId/mode=direct 返回已保存行的脱敏决策。客户端不能提交 contract/settings/stack/path/tenant，额外字段和第三个 mode 值被拒绝。响应不含契约摘要、exclusive、配置、凭据或主机路径，SDK 凭据头标记敏感且丢弃失败响应载荷。
- 权限检查早于状态/存储读取，沿用管理员主机配置读取授权和租户作用域；缺状态只读返回空态，不创建目录、不恢复 pending。pending 返回 409，外属状态返回 403。路由运行时明确展示 Unavailable；请求 Proxy 候选决策返回 503，不创建空代理。

### 红绿与验证证据

初次接线编译错误（新增字段默认值、未注册的 hex 编码依赖、Debug）已修复，不计业务红证据。仓库 `.cargo/config.toml` 另设 build-dir；有效阶段将 `CARGO_BUILD_BUILD_DIR` 同设为槽位路径，以下红/绿证据均来自槽位中的二进制。

有效红阶段动态领取槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`：四条指定默认 features 过滤命令均实际运行。执行端 **3 passed / 2 failed，退出 101**，两条指定测试分别在逻辑使用者仍为 direct 和相等契约仍要求重写的业务断言失败；DTO **1 passed，退出 0**；Gateway **0 passed / 2 failed，退出 101**，其中持久 stack 未出现是有效业务失败，另一项为测试预期误用 403（现有鉴权应为 401，已修正，不作为业务失败证据）；SDK **1 passed，退出 0**。没有用编译失败或零测试代替红证据。

首绿本任务 14 项、相邻回归 14 项通过；Clippy 发现一项新增 collapsible_if，已修正，并补充无归属 stack 与独有字段不一致拒绝断言。

完整验证阶段重新动态领取槽位 **2**；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，`CARGO_BUILD_BUILD_DIR` 同值；所有命令继承该绝对路径，未指定 --target-dir，完成后仅释放自身锁并保留缓存。

| 确切命令 | 退出码 | 有效数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-agent-apps --lib cc04_mode_state_ -- --nocapture` | 0 | 9 passed |
| `cargo test -p vw-api-types --lib cc04_mode_state_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent --lib cc04_mode_state_ -- --nocapture` | 0 | 4 passed |
| `cargo test -p vw-gateway-client --lib cc04_mode_state_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent-apps --lib cc04_commit_ -- --nocapture` | 0 | 6 passed |
| `cargo test -p vw-agent-apps --lib cc04_recovery_ -- --nocapture` | 0 | 8 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全仓全部 targets/features |

末审补充 Direct/attached 不一致时也要求重写；之后重新动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`（`CARGO_BUILD_BUILD_DIR` 同值），重跑 `cargo fmt --all`、四条相同 `cc04_mode_state_` 过滤命令及 `cargo clippy --all-targets --all-features -- -D warnings`，均退出 0，分别为 9/1/4/1 项；不重复计数。

本任务 **15 项**、相邻回归 **14 项**通过，无忽略。两条指定测试全部通过。实际证明状态重开往返、Direct/Proxy 使用者区分、stack 独立、永久 key 与租户约束、递归对象顺序稳定/数组顺序变化、跨应用与旧版本隔离、分离/模式/独有字段变化决策、版本 4 意图与文件前滚恢复、旧意图版本拒绝、只读缺省不建目录、鉴权/租户/pending/非法请求拒绝。

### macOS / SDK→API→文件证据与后续

本机 macOS **27.0（26A428）/ arm64**，工作区包版本 **0.6.0**，默认 features。真实本地 TCP HTTP、SDK、SQLite 与临时私有根/配置文件；使用假凭据，无替代业务存储或状态算法。SDK 请求取到已保存行的相等契约 SkipRewrite，API 状态重开可见 Direct 指针、保留 route 与 stack；随后仅通过内部事务持久化 Proxy 观察夹具，SDK 逻辑使用者变为 route，仍明确 runtime=Unavailable。观察/决策前后状态文件、SQLite 文件和临时客户端文件字节一致；单独执行端断言还检查临时文件 inode/mtime 不变，读取缺省状态不创建 provider-live。此 Proxy 夹具只证明状态观察，不表示代理实际运行。

Gateway 测试有此前任务已记录的 `__eh_frame section too large` 链接器提示，测试退出 0，未修改链接配置或降级 lint。

Iced UI、真实客户端应用版本/消费、代理运行与 live 文件免重写未验证，明确归 46 或所属 UI 任务。46 应直接消费本契约算法和 ProviderTransactions/PendingTarget.stack，沿明确外部服务边界验证真实可用依赖，再取得模式切换及 live 免重写证据；不得以此次决策断言代替。

定向差异/暂存审查和 plan.md 只读检查通过；仅提交本任务实现、独立测试、必要默认字段/注册和本条证据。排除既有母任务状态、其它计划、此前编号文件删除及 cargo fmt 产生的 Dsh 纯格式差异。未执行全量 cargo test、fmt check、wasm 或 Makefile。验收后删除本编号任务文件，不提交该删除；下一步批次 46。

## 任务 46：模式切换的配置事务

2026-11-07；验收边界为 **Claude 配置事务与显式路由服务接入点**，不代表生产代理协议或其它应用模式迁移。批次 45 提交 `3d407fc05` 与正式验收已核对；母任务 1、3 完成，母任务 2 依 provider-contracts.md 既有底座开工裁定消费真实交付。最初缺少显式服务契约的阻塞，由继续指令明确授权在本任务定义最小契约/执行端注入入口而解除；未修改任何 plan.md。

### 来源、接口与实现

只读来源 `/Users/xiangmin/worker/code/cc-switch` HEAD 为 `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 均退出 0，来源干净。读取 `mode/controller.rs::{write_proxy,enter_locked,switch_route_locked,exit_locked,write_direct_for_exit}`、直接调用者及相邻相同契约免重写/独有字段测试，`mode/operation.rs::{run,recover}` 及相邻发布/外部修改测试，`live/residue.rs` 与相邻测试；继续后对照只读母任务 8、底座《后端边界》《存储与生命周期》及来源 `live/project/claude.rs::proxy_projection`。RustCodeGraph status/explore 成功，未覆盖的新原生模块使用定向读取；未运行或修改来源包。

- 唯一文件/状态所有者仍为 `ProviderTransactions`；`providers/mode_apply.rs::ClaudeMode::apply` 在既有 Claude 固定路径守卫下先提交客户端配置及持久模式/路由，再调用运行时 `publish`。保留直连指针及 stack；Detach 保留逻辑 Proxy/路由，attached=false；Direct 退出保留上次路由。计算最终代理地址、占位凭据、模型与独有字段之后，消费任务 45 的版本 2 客户端契约及 `rewrite_decision`。契约相同的换路由只提交目标，不写客户端文件。
- `RouteRuntime` 为母任务 8 后续运行时的唯一接入接口，只表达 read-only prepare（受信本地客户端地址/当前作用域就绪）与幂等 publish（目标供应商或移除）。`ClientEndpoint` 拒绝远程地址、用户凭据、query/fragment、隐式端口；按解析后的 URL 规范化。继承冻结来源 `PROXY_MANAGED` 占位值，不把档案中的上游密钥写到代理客户端配置。Gateway 通过 `ProviderRouteRuntime(Arc<dyn RouteRuntime>)` Extension 注入，先只读探测，再在应用锁下复核；发布始终持有应用守卫。没有空实现、后台监听器、环境地址回退、协议引擎或第二套持久存储。生产装配未注入运行时，Proxy 在原生/状态目录创建前返回 503；没有将已有 model_relay 误作长期路由。
- 客户端落盘与外部运行时发布不是跨进程原子事务。发布失败返回 `targetPublication=pending`，明确配置已经提交；重复同一操作根据持久目标重新计算契约、跳过文件并重试幂等发布。文件发布中断沿已有 pending 前滚；首次恢复请求返回 409 要求重读，再次请求发布目标。不会以运行时失败回滚已提交文件，也没有承诺自动启动生产服务或后台重试。
- 退出清理复用底层 Claude patch/投影，按持久 contract.exclusive 的已写值精确匹配才删除。新增内部显式 legacy-residue 策略，仅模式事务关闭旧预设猜测清理，既有预览/普通直连维持原行为；外部改成另一冻结预设值也保留。模式操作通过登录所有者提供的 journal 查询拒绝未完成登录，不执行 Keychain/账号迁移。目录行及直连行前像、模式状态在应用守卫与事务发布前复核；外属 state/pending 不恢复、不发布。
- DTO 位于 `vw-api-types/src/app_providers/mode_apply.rs`；`POST /v1/app-providers/{app}/mode/apply` 接受 targetHostId 和明确的 proxy/direct/detach action，SDK 为 `provider_mode_apply`。沿既有分域根注册，供两类 Gateway 根装配复用，不新开通用命令入口。未知字段拒绝，不能提交 path/settings/contract/runtime/tenant/stack。授权沿管理员 host-config write 与租户作用域，在存储/文件动作前完成；400 非法身份、401 错 token、403 无写授权/外属状态、409 并发/已恢复、422 不支持、503 运行时/存储不可用。SDK 凭据 header 标记敏感，错误响应载荷丢弃。
- 本行为沿已交付 Claude 适配接通，不顺带迁移其它应用。其它 AppId、enabled stack、官方/托管登录路径明确拒绝，Claude 进入要求已有可恢复的第三方直连行；原生账号、聚合目录与完整别名/模型发布仍归其唯一领域所有者。未增加 UI，当前供应商编辑/首次激活不被该接口旁路或宣称成功。读取模式的 runtime 观察仍按已有生产未装配事实返回 Unavailable；本次 HTTP fixture 的可用性由 apply 的 Published 和真实请求证明，不外推生产运行状态。

### 红绿与验证

红阶段原子 mkdir 动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同值。先 `cargo fmt --all`（退出 0），再运行四条指定默认 features 过滤命令：业务包的两条指定测试在业务断言失败（未附着/未清理已写值），**0 passed / 2 failed，退出 101**；DTO **1 passed，退出 0**；Gateway 未装配拒绝路径 **1 passed，退出 0**；SDK **1 passed，退出 0**。接线编译成功，没有以缺类型、缺模块或零测试代替红证据。

首绿动态槽位 **1**，相同绝对目录和 build-dir，格式化、四条过滤命令和强制 Clippy 均退出 0，分别 5/1/3/1 项。本轮末审补充未完成登录、写权限、外属状态及非法输入拒绝，并改进创建文件前的只读前置检查。

最终验证动态槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，`CARGO_BUILD_BUILD_DIR` 同值；以下命令均继承该绝对路径，没有使用 --target-dir 或新建专属构建目录：

| 确切命令 | 退出码 | 有效数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-agent-apps --lib cc04_mode_apply_ -- --nocapture` | 0 | 6 passed |
| `cargo test -p vw-api-types --lib cc04_mode_apply_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent --lib cc04_mode_apply_ -- --nocapture` | 0 | 4 passed |
| `cargo test -p vw-gateway-client --lib cc04_mode_apply_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent-apps --lib cc04_mode_state_ -- --nocapture` | 0 | 9 passed |
| `cargo test -p vw-agent-apps --lib cc04_commit_ -- --nocapture` | 0 | 6 passed |
| `cargo test -p vw-agent-apps --lib cc04_recovery_ -- --nocapture` | 0 | 8 passed |
| `cargo test -p vw-agent-apps --lib cc04_claude_ -- --nocapture` | 0 | 26 passed |
| `cargo test -p vw-agent-apps --lib cc04_preview_claude_ -- --nocapture` | 0 | 1 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全仓全部 targets/features |

本任务 **12** 项及相邻回归 **50** 项，去重 **62** 项通过，无忽略。两条指定测试全部通过。未运行无过滤全量 cargo test、fmt check、wasm 或 Makefile。各阶段记录计划/任务/会话/PID，自身占用锁在子进程结束后清理，编译缓存保留。

### SDK→API→执行端文件与 macOS 证据

Gateway 测试使用真实回环 HTTP、正式 SDK、真实权限 policy/租户中间件、SQLite 与隔离 HOME/private 根。只替代母任务 8 尚未交付的不可控路由服务边界：测试服务真实接收并校验作用域、readiness/target 控制请求；在接受 publish 前独立读取实际 `.claude/settings.json`、持久 ModeState、直连指针/pending 完成状态；实际服务状态按请求记录 active target。随后按 SDK 写出的客户端地址和占位鉴权发送 `POST /v1/messages`，校验 model/max_tokens/messages 并返回固定 Anthropic message 形状（content 文本、stop_reason、usage），调用方断言实际响应 `pong`。这不是空实现、布尔假运行状态或调用次数验证，也不是生产协议转换/流式验收。

SDK Proxy → API Published → 客户端本机地址/占位凭据；换到相同契约的另一目录行 → active target 改变，文件字节、inode、mtime 均不变，直连指针保持原行。外部更改独有值/未知设置 → SDK Detach → 写回直连地址、只删除与已写值相同的字段、保留外部值（包括另一旧窗口预设）、Proxy/route 保留且 attached=false；SDK Direct → 持久 mode=Direct。真实 HTTP publish 503 → API Pending 且配置已提交；重试 → Published，原生文件不重写。错误 token、无写权限、非法身份、其它应用、外属状态、未注入/未就绪服务均拒绝，相关断言检查无文件/状态/目标发布副作用。执行端独立测试还覆盖校验失败无写入、发布后注入故障的前滚恢复与外部更改保留、未完成登录 journal 拒绝。

全部凭据为假值。业务包独立测试以受控 RouteRuntime 实现隔离运行时调用边界；真实 HTTP 与 Messages 证据在 Gateway 集成测试取得，没有替代权限、存储、事务或文件系统。

实际平台 macOS **27.0（26A428）/ arm64**，workspace crate **0.6.0**。真实 Claude 应用版本/消费、真实账号或 Keychain、生产代理协议引擎、流式/多模态转换、Iced 渲染及非 macOS 均 **未验证**。没有读取或修改真实用户配置。后续应由母任务 8 将真实服务注入上述唯一接入点，按各自领域验收扩展应用投影/聚合/原生登录；不得将本记录扩写为全应用或生产代理通过。

状态：**已完成（Claude 配置事务与显式服务边界）**。仅提交本任务实现、独立测试、必要父注册/底层策略及本条正式证据；排除既有母任务状态、此前编号文件删除、终端体验变更及 cargo fmt 产生的 Dsh 纯格式差异。所有 plan.md 保持只读；本编号任务验收后删除，不提交该删除。

## 任务 47：macOS 环境冲突诊断

2026-11-07；批次 46 提交 `025f1546a` 及上节正式验收已核对。母任务 1、3 已完成；母任务 2 完整领域迁移仍未收口，沿任务 1/provider-contracts.md 中底座子计划 1–93 的既有开工裁定消费实际接口，不冒称母任务 2 整体完成。所有 plan.md 保持只读。

### 来源与实现边界

只读来源 `/Users/xiangmin/worker/code/cc-switch` 提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 退出 0，来源干净。阅读 `services/env_checker.rs::{check_env_conflicts,get_keywords_for_app,matches_env_keyword,check_shell_configs}`、相邻关键词与 Unix 继承环境测试，`commands/env.rs::check_env_conflicts`，直接调用者 `src/lib/api/env.ts::{checkEnvConflicts,checkAllEnvConflicts}` 与 `src/App.tsx::checkEnvOnSwitch`。RustCodeGraph status/explore 成功；未覆盖的新原生模块定向读取。

- 唯一原生所有者 `vw-agent-apps/src/providers/env_diagnose.rs::diagnose`，仅 macOS，支持来源已有 Claude/Codex/Gemini/GrokBuild 关键词；其它应用/平台明确拒绝。执行端授权 HOME 固定扫描 `.bashrc/.bash_profile/.zshrc/.zprofile/.profile`，系统固定扫描 `/private/etc/profile` 与 `bashrc`（macOS `/etc` 实际目录）。缺省文件忽略，读取失败返回明确错误，不当作空诊断。目录与文件描述符使用 O_NOFOLLOW，openat 固定单组件、O_NONBLOCK，拒绝链接与非普通文件；每文件至多 1 MiB、最多 4096 条诊断。
- 来源使用强类型 UserShell/SystemShell/ProcessInherited 和固定文件枚举、1-based 行号，不返回任意可调用路径。只记录合法有界变量名，DTO/Debug 从未持有或返回变量值、原始行或凭据；没有执行 shell/source、网络调用、配置写入或父进程环境修改。
- 保留上游关键词规则：Claude/Codex/Gemini 为开头匹配，Grok 只精确匹配 XAI_API_KEY/GROK_DEFAULT_MODEL。继承变量单独只读展示、排序去重，不加入可清理 shell 冲突；同名用户/系统/继承来源保留为不同条目，不能据此判断最终生效顺序。
- Shell 条目是静态定义候选，不承诺完整 shell 语义或有效值。只有整份用户文件由空行、注释及单行字面量赋值构成时，才标 `cleanupAvailable=true`；含函数、条件、source、续行、命令替换或 here-document 等复杂语法时整份文件候选均只读，可能包含示例文本。系统定义和进程继承值始终 false。此标记仅为清理资格；任务 48 必须签发并校验服务端诊断标识、基底摘要与实际写权限，不能相信客户端的路径、行号或布尔值。本任务没有清理/备份/恢复入口。
- DTO：`vw-api-types/src/app_providers/env_diagnose.rs`；`POST /v1/app-providers/{app}/environment/diagnose` 只接受 targetHostId，未知字段拒绝。沿母任务 2 既有管理员 host-config read 授权、租户作用域与 app-providers 父注册，两类 Gateway 根复用；SDK 为 `GatewayClient::provider_env_diagnose`。权限检查在文件和环境扫描前；401 错 token/未通过 Gateway Basic、403 无授权/外属租户、503 离线/读取故障、501 远程绑定、422 不支持应用。SDK token header 敏感、错误响应载荷丢弃。没有新增 UI，不宣称编辑/首次激活/live 或后续清理已通过。

### 红绿与验证

红阶段通过原子 mkdir 动态领取 **槽位 2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，`CARGO_BUILD_BUILD_DIR` 同值。`cargo fmt --all` 退出 0；四条规定默认 features 命令均完成接线编译：业务包指定两条测试在来源数量业务断言失败（0 passed/2 failed，退出 101），Gateway 的 SDK→API 来源断言失败（1 passed/1 failed，退出 101），DTO 1 passed/SDK 1 passed（均退出 0）。没有把缺类型、缺模块或零测试当作红证据。

绿阶段重新原子领取 **槽位 2**，上述两个绝对目录设置相同。以下命令均继承该设置，未使用 --target-dir 或任务专属构建目录：

| 确切命令 | 退出码 | 有效数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-agent-apps --lib cc04_env_diagnose_ -- --nocapture` | 0 | 6 passed |
| `cargo test -p vw-api-types --lib cc04_env_diagnose_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent --lib cc04_env_diagnose_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-gateway-client --lib cc04_env_diagnose_ -- --nocapture` | 0 | 1 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全仓全部 targets/features |

Gateway 测试链接阶段有 macOS ld 的 `__eh_frame section too large (max 16MB)` 提示，命令仍退出 0；这是 vw-agent 测试二进制的 unwind 表大小提示，不是 Rust 编译错误，未扩展范围修改链接配置。

本任务去重 **10 项**通过，无忽略；子进程额外执行一遍同名继承环境测试，不重复计数。各阶段记录计划/任务/会话/PID，在所有子进程结束后释放自己的锁，保留缓存；未运行全量 cargo test、fmt check、wasm 或 Makefile。

### SDK→API→执行端与 macOS 证据

真实回环 HTTP/正式 SDK/既有权限与租户中间件/隔离 HOME 文件验证：写入假凭据 `.zshrc` → SDK diagnose → API 返回 UserShell/Zshrc/line=1 和清理资格 → 执行端源文件字节保持原样，未创建事务/配置目录，响应不含假凭据。无读授权、未知主机、离线、远程、错 token、外属租户与不支持应用均拒绝，文件仍原样；正式根注册的无 Basic 请求返回 401。SDK 独立测试核对实际 POST 路径、JSON targetHostId、公司头及 403 错误载荷脱敏；DTO 拒绝 path/line/environment/tenant/value 注入。

执行端隔离测试验证用户 shell 行号、同名用户/系统/继承三来源、系统与继承不可清理、关键词、复杂 shell 不执行且只读、链接/目录/超限源拒绝。真实子进程通过 Command::env 注入假继承变量，调用正式 `diagnose` 并断言 ProcessInherited、cleanup=false、无秘密值、子进程环境保持原值、临时 HOME 无文件；父进程环境不修改。仅在纯来源扫描测试替代固定系统目录与不可控继承变量枚举，真实生产边界另由子进程证明；HTTP 测试未替代权限或执行端文件系统。

实际 macOS **27.0（26A428）/ arm64**，workspace crate **0.6.0**。没有读取或修改真实用户 shell 配置；读取本机固定系统 shell 源只用于正式执行路径的只读验收。真实客户端应用版本/消费、账号、Iced UI、完整 shell 求值、清理/恢复及非 macOS 均未验证，不将静态诊断外推为这些能力通过。

状态：**已完成（macOS 只读诊断与清理资格边界）**。定向差异与暂存检查通过，plan.md 只读；仅提交本任务模块、独立测试、必要父注册和本条证据，排除既有 Dsh 格式差异、母任务状态、此前编号文件删除及其他计划。完成后删除任务 47 编号文件，不提交其删除；下一步批次 48。

## 任务 48：环境冲突选择性清理

2026-11-07；核对批次 47 提交 `44c17f71c` 与上节正式验收，前置诊断已交付。母任务 1、3 已完成；母任务 2 完整领域迁移仍未收口，继续沿任务 1/provider-contracts.md 的底座子计划 1–93 开工裁定消费真实接口，不将其改报完整完成。所有 plan.md 保持只读。

### 来源、实现与边界

只读来源 `/Users/xiangmin/worker/code/cc-switch` HEAD 为 `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 退出 0、来源干净。阅读 `services/env_manager.rs::{delete_env_vars,create_backup,delete_single_env}`、相邻备份目录测试，`services/env_checker.rs::{check_env_conflicts,check_shell_configs}` 及相邻测试、`commands/env.rs`、`src/lib/api/env.ts` 和 `EnvWarningBanner.tsx::handleDelete`。使用 RustCodeGraph status/explore 核对现有调用与写锁，图未覆盖的新文件定向读取；没有运行或修改来源包。

- 执行端唯一业务所有者为 `vw-agent-apps/src/providers/env_clean.rs::{diagnose,clean}`。复用任务 47 的安全读取与静态 shell 解析，不复制第二套解析器，不沿用上游按变量名删除所有来源的行为。只对已诊断、整文件为简单字面量赋值的用户 shell 源提供选择标识；系统源、进程继承及复杂 shell 始终只读。不执行 shell，不修改进程环境。
- 正式 diagnose 响应在条目 `cleanup` 中提供服务签发的 256-bit 随机 `diagnosticId` 与 `baseDigest`；内存表绑定授权租户/目标主机、应用、物理 HOME、固定 UserShellFile 和诊断行号，15 分钟过期、最多 16384 条。进程重启、标识过期或成功消费后需要重新诊断；没有新增配置或持久供应商状态。文件摘要覆盖原字节及 dev/inode、长度、权限、UID、硬链接数、mtime/ctime（纳秒）。客户端不能提交路径、行号、变量值、清理资格或租户。
- 清理先校验全部选择与文件基底，再持久化备份，备份后复核全部基底，逐文件发布前再次复核 HOME 身份/安全权限与文件基底。复用物理目录/App 写锁，并在 HOME 描述符上持有跨进程 advisory flock。拒绝链接、非普通源、硬链接、多用户可写源、替换的 HOME/文件、重复/伪造/外属或旧诊断。按所选诊断行移除定义，保留同名其它行/其它文件、其它 export、独立及行内注释、CRLF 和非选中行末尾无换行；沿既有 atomic writer 保留源文件模式。
- 备份固定落于授权 `credential_root/environment-backups/{opaque-id}.json`。根必须为当前用户所有且私有；固定子目录以 0700 创建并复核，文件通过描述符相对 O_EXCL/O_NOFOLLOW 以 0600 独占创建，文件及目录 sync_all 成功后才允许发布。版本 1 备份包含授权 scope、appId、每个固定文件枚举的 baseDigest/original/replacement。备份失败不修改源；发布前失败或外部变更时保留已持久备份。响应只有不透明 backupId 与实际移除定义数量，原始秘密只存在执行端私有备份。
- 清理是逐文件、带备份与发布前复核的操作，不宣称多文件整体原子事务；发布中断可能已有部分文件更新，备份保留。外部 shell 编辑器不参与 advisory 锁，复核与原子 rename 之间仍有非协作写入竞争窗口，不承诺内核级 compare-and-swap。没有扩展为通用文件/命令 API、恢复或后台事务服务。
- DTO 为 `vw-api-types/src/app_providers/env_clean.rs`；`POST /v1/app-providers/{app}/environment/clean` 与 SDK `GatewayClient::provider_env_clean` 沿既有分域父注册接到两类 Gateway 根。使用既有 host-config write 授权与租户边界，在标识查询、备份及文件动作前检查；400 非法选择/未知字段、401 鉴权失败、403 无写权限/外属租户、409 旧基底/重放/外属诊断、422 不支持应用/平台、503 读取/备份/写入不可用。SDK 凭据 header 标敏感，错误载荷丢弃。没有新增 UI、恢复入口、激活或 live 切换；UI/恢复分别归任务 73/49。

### 红绿与验证

红阶段原子 mkdir 动态领取槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，`CARGO_BUILD_BUILD_DIR` 同值。最初的接线编译问题修正后重新运行四条规定命令；业务包两条指定测试均在业务断言失败（未删除所选定义、备份受阻却未拒绝），**0 passed / 2 failed，退出 101**；DTO/Gateway/SDK 各 **1 passed、退出 0**。`cargo fmt --all` 退出 0，没有把缺类型、缺模块或零测试当作红证据。

首轮绿阶段发现测试临时私有根默认 0755，业务 4 项及 HTTP 成功路径被生产私有权限检查拒绝；参照相邻夹具改为 0700，没有削弱安全检查。同期诊断回归 6/2 项及 Clippy 退出 0。随后完成同字节 inode 替换与备份后 HOME 替换场景，运行下列最终验证。

最终动态领取 **槽位 2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，`CARGO_BUILD_BUILD_DIR` 同值。所有 Cargo 命令及子进程继承该绝对目录，无 --target-dir 或任务独有缓存；所有测试使用默认 features：

| 确切命令 | 退出码 | 有效数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-agent-apps --lib cc04_env_clean_ -- --nocapture` | 0 | 8 passed |
| `cargo test -p vw-api-types --lib cc04_env_clean_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent --lib cc04_env_clean_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-gateway-client --lib cc04_env_clean_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent-apps --lib cc04_env_diagnose_ -- --nocapture` | 0 | 6 passed |
| `cargo test -p vw-agent --lib cc04_env_diagnose_ -- --nocapture` | 0 | 2 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全仓全部 targets/features |

本任务 **12** 项与诊断回归 **8** 项，去重 **20** 项通过，无忽略。Gateway 链接器沿此前任务出现 `__eh_frame section too large` 提示，测试命令仍退出 0，未改变链接配置或 lint。未运行无过滤全量 cargo test、fmt check、wasm 或 Makefile。各阶段在锁目录记录计划/任务/会话/PID，所有子进程结束后清理自身 owner 并 rmdir 释放，缓存保留。

### SDK→API→执行端与 macOS 证据

Gateway 集成测试使用正式 SDK、真实回环 HTTP、真实权限 policy/租户中间件与隔离 HOME/private 根：写入含同名重复定义、注释及其它 export 的 `.zshrc` 和同名 `.profile` → SDK diagnose 获得绑定诊断选择 → SDK clean → API 返回 backupId/removedDefinitions=1 → 独立读取实际源文件，只有选中定义消失，同名其它行、其它文件和其它 export 保留；实际 0600 备份的 original 与完整源字节一致，响应不含假凭据。只读授权、错 token、外属租户、同 HOME 不同目标主机、伪造摘要、不支持应用、重放及诊断后外部修改均拒绝，并断言源文件/备份目录无额外写入。正式根入口未通过 Basic 返回 401。

执行端隔离文件测试还证明：备份位置受阻时源不变；持久备份完成的内部观察点能读取完整备份且源仍原样；该点中断或外部改源后保留源与备份；多文件任一旧基底导致任何发布前拒绝；有效多文件选择只清理各自所选定义；同字节 inode 替换、HOME 替换、源/备份符号链接与不私有备份目录拒绝。DTO 拒绝客户端文件坐标/秘密/权限注入，SDK 独立测试核对真实请求路径/方法/JSON/租户头与错误载荷脱敏。

全部凭据为假值。没有替代权限或文件系统；业务诊断夹具只替代系统扫描根和继承枚举，内部 after_backup 观察点用于确定性模拟持久备份之后、源发布之前的中断/外部修改。正式 HTTP diagnose 仍调用真实执行端诊断路径。实际平台 macOS **27.0（26A428）/ arm64**，workspace crate **0.6.0**。没有读取或修改真实用户 shell 配置；真实客户端应用版本/消费、账号、Iced、恢复、完整 shell 求值及非 macOS **未验证**。

状态：**已完成（macOS 服务端诊断选择、私有备份与选择性清理）**。定向差异/暂存审查通过，plan.md 未修改、自身 Cargo 锁已释放；只提交本任务实现、独立测试、必要父注册/诊断接线和本条正式证据，排除既有 Dsh 格式差异、母任务状态、其它编号文件删除及其它计划。本编号任务验收后删除且不提交删除；下一步批次 49。

## 任务 49：环境备份恢复

2026-11-07。批次 48 提交 `f83de4f3a` 与上节正式验收已核对；母任务 1、3 已完成，母任务 2 完整领域迁移仍未收口，沿任务 1/provider-contracts.md 已记录的底座子计划 1–93 开工裁定消费真实接口，不将其改报完整完成。所有 plan.md 保持只读。

### 来源、实现与边界

只读来源 `/Users/xiangmin/worker/code/cc-switch` HEAD 为 `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 退出 0、来源干净。阅读 `services/env_manager.rs::{restore_from_backup,restore_single_env,create_backup,delete_env_vars}`、相邻备份测试、`commands/env.rs::restore_env_backup` 与直接 TS 包装 `src/lib/api/env.ts::restoreEnvBackup`。来源按请求路径读备份并追加未经转义定义；本实现使用既有执行端备份所有者和固定文件枚举，按原字节恢复。RustCodeGraph status/explore 检查既有清理调用链，图未覆盖的新模块定向读取；没有运行或修改来源包。

- 唯一恢复所有者为 `vw-agent-apps/src/providers/env_restore.rs::restore`；DTO、Gateway、SDK 分别为各 crate 的 `app_providers/env_restore.rs`，只补既有父模块注册。`POST /v1/app-providers/{app}/environment/restore` 和 `GatewayClient::provider_env_restore` 只接受 targetHostId/backupId，拒绝请求路径、文件坐标、秘密、scope 和 force 等注入。沿既有 host-config write 权限，在备份查询和文件动作前完成鉴权/租户检查；401 鉴权失败、403 无写授权/外租户、400 非法标识、409 缺失/外属/不匹配备份、422 不支持应用/平台、503 执行端不可用。逐文件失败通过强类型 items.status 返回，HTTP 成功不代表所有项成功。
- 清理与恢复共用 `env_backup.rs` 的私有备份格式/IO、源身份摘要与安全检查，从原清理模块移出已有实现，没有第二套持久库或解析器。保持 256-bit 不透明标识、固定授权 credential_root/environment-backups 目录、描述符相对读取、O_NOFOLLOW/O_NONBLOCK、0600 文件和私有所有权检查；拒绝链接、硬链接、非普通/过大/损坏备份、未知字段、重复文件和非枚举坐标。
- 清理备份升级为版本 2，增加物理 HOME 的 dev/inode、每文件 cleanedDigest/restoredDigest。每个清理文件发布并 fsync 后，复核预期字节与安全元数据，再原子更新备份 checkpoint；完整 original 始终在源修改前持久。版本 1 缺少恢复授权基底，明确拒绝，无旧备份迁移或猜测恢复。
- 恢复复用物理目录/App 写锁和 HOME advisory flock，绑定租户/目标主机、AppId 与 HOME 身份。逐文件仅在当前字节和 dev/inode、长度、权限、UID、硬链接数及 mtime/ctime 纳秒摘要都等于清理后基底时写回 original，保留模式。外部改写、同内容 inode 替换、源缺失、HOME 替换和不安全来源均不覆盖。成功恢复持久 restoredDigest；重复请求仅在恢复后身份与内容仍相同时返回 already_restored，否则 conflict。
- 逐文件返回 restored/already_restored/not_cleaned/conflict/unavailable，不返回原始秘密或任意主机路径。部分冲突不阻止其它有效项恢复；清理尚未发布且原基底仍匹配时返回 not_cleaned。源发布至 checkpoint 持久之间中断时保留原备份并安全拒绝自动重试，不猜测已发布状态。无多文件整体原子性或内核 CAS 承诺：非协作编辑器仍可在最后复核与 rename 之间写入；沿任务 48 的竞争边界，没有削弱验证。
- 未新增 Iced、任意文件/命令入口、进程环境修改、供应商激活或 live 切换；环境 UI 归后续所属任务。

### 红绿与验证

红阶段动态领取槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，`CARGO_BUILD_BUILD_DIR` 同值。`cargo fmt --all` 退出 0，四条规定过滤命令均运行：业务包两条指定测试在业务断言处失败（未恢复选中定义、未拒绝外属备份），**0 passed / 2 failed，退出 101**；DTO/Gateway/SDK 各 **1 passed、退出 0**。没有用编译失败或零测试替代红证据。

首轮实现验证发现恢复 checkpoint 的不可变/可变借用冲突，修正借用范围后重跑格式化与全部规定验证。最终阶段动态领取 **槽位 2**，所有 Cargo/脚本及子进程继承 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`、`CARGO_BUILD_BUILD_DIR` 同值，无 --target-dir 或任务独有缓存；各 crate 使用默认 features：

| 确切命令 | 退出码 | 有效数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-agent-apps --lib cc04_env_restore_ -- --nocapture` | 0 | 8 passed |
| `cargo test -p vw-api-types --lib cc04_env_restore_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent --lib cc04_env_restore_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-gateway-client --lib cc04_env_restore_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent-apps --lib cc04_env_clean_ -- --nocapture` | 0 | 8 passed |
| `cargo test -p vw-agent --lib cc04_env_clean_ -- --nocapture` | 0 | 2 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全部 targets/features |

本任务 **12** 项、清理回归 **10** 项，去重 **22** 项通过，无忽略。Gateway 链接器沿前序任务提示 `__eh_frame section too large`，命令仍退出 0；未修改链接配置或放宽 lint。未运行无过滤全量 cargo test、fmt check、wasm 或 Makefile。每阶段以原子 mkdir 领取槽位并记录计划/任务/会话/PID；全部子进程结束后 trap 删除自身 owner、rmdir 释放锁，编译缓存保留。

### SDK→API→执行端与 macOS 证据

Gateway 集成使用正式 SDK、真实回环 HTTP、真实授权 policy/租户中间件和隔离 HOME/private 根：SDK diagnose 获取选择 → SDK clean 返回 backupId 并实际清理所选定义 → SDK restore → API items[0].status=restored → 独立读取源文件与完整 original 一致，重复请求 already_restored；恢复后外部编辑返回 conflict 并保留外部字节。只读授权、错 token、外属租户/目标主机、外属 AppId、伪造/路径标识拒绝，源与备份字节不变；正式 handler_router 未通过 Basic 返回 401。SDK 独立测试核对真实路径/方法/JSON/租户头及错误载荷丢弃，凭据 header 标敏感，响应不含假秘密。

执行端独立隔离文件测试证明：CRLF、选中行的引号/注释和未选中定义完整还原，其它文件不动；同字节 inode 替换与 HOME 重新绑定拒绝；源链接/缺失、备份链接/非私有权限/损坏/旧版本/坐标注入/重复文件拒绝且不覆盖；两个文件中的一项冲突时另一项实际恢复，原权限 0640 保留；持久备份后、源发布前中断返回 not_cleaned；发布前 HOME 替换保留新旧目录。仅用内部观察点替代不可控中断时机，诊断夹具替代系统扫描根和继承枚举；权限、文件系统、HTTP、SDK 均真实执行，所有凭据均为假值。

实际平台 macOS **27.0（26A428）/ arm64**，workspace crate **0.6.0**。没有读取/修改真实用户 shell 配置；真实客户端应用版本/消费、账号/Keychain、Iced 渲染、完整 shell 求值及非 macOS **未验证**。

状态：**已完成（macOS 授权环境备份恢复与逐文件冲突结果）**。定向差异/暂存审查通过，plan.md 只读、自身 Cargo 锁已释放；只提交任务 49 实现、独立测试、必要父注册/清理备份接线及本条正式证据，排除既有 Dsh 格式差异、母任务状态、其它编号文件删除与其它计划。本编号任务验收后删除且不提交该删除；下一步批次 50。

## 任务 50：配置操作进度与取消

2026-11-07。批次 49 提交 `c55667052` 与上节完成证据已核对；母任务 1、3 已完成，母任务 2 沿任务 1 的底座子计划 1–93 开工裁定消费实际接口，不将母任务 2 完整迁移改报完成。plan.md 保持只读。

### 来源、实现与边界

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 退出 0、来源干净。只读 `commands/provider.rs::{sync_universal_provider,import_opencode_providers_from_live}`、TS `providers.ts::{sync,importOpenCodeFromLive}`、`mode/operation.rs::{run,AppWrite::run,recover_before_write}`、controller 调用及相邻 provider_commands/operation 测试；没有构建或修改来源包。RustCodeGraph status/explore/node 定位现有 Operation、生产路由和事务所有者；新模块及图未覆盖部分使用定向搜索/读取。

- 业务所有者 `vw-agent-apps/src/providers/operations.rs::ProviderOperation` 复用既有 `apps::lifecycle::operations::Operation::start_action`，只增加供应商阶段、计数与发布屏障，不新增调度器、持久任务库或通用执行 API。现有导入 `AppProviderStore::import_native`、投影 `universal_sync` 与事务 `ProviderTransactions::commit_checked` 调用同一屏障，沿用各自解码、CAS、写锁、暂存、pending 和恢复所有者。
- 取消请求与发布决定通过同一状态锁排序。发布前取消由 worker 确认后返回 cancelled，真实配置和已提交指针保持原样，暂存清理；进入发布临界区后不因取消中止剩余写入，返回 settling/cancellationRequested，再完成为 completed。发布后执行错误返回 failed/settling，绝不改报 cancelled；事务保留原 pending 并由既有前滚恢复继续。目录注册等既有私有恢复元数据不作为用户配置发布。catalog 批次保持既有逐行 CAS/逐项失败语义，不增加整体原子性或跨存储回滚承诺。
- DTO 为 `vw-api-types/src/app_providers/operations.rs`，复用 TaskId/TaskStatus；支持固定 Import(app,trigger) 与 Project(id,apps)，拒绝路径、命令和未知字段。progress 提供 operationId/requestId/status/phase/completed/total/cancellationRequested；计数表示处理项数，包含跳过的既有导入项和投影失败项，不是成功写入数；失败不会返回 completed 状态。
- Gateway `handlers/app_providers/operations.rs` 注册 `POST /v1/app-providers/operations` 与 `GET/DELETE /v1/app-providers/operations/{id}?targetHostId=…`，沿用原 app_providers 分域父注册和正式 handler_router。提交前、排队后工作开始前重做授权；全部操作要求目标主机配置写权限，导入额外要求读权限。读取/取消重新授权并绑定 tenant/host；无 token、外租户、外主机和只读授权拒绝且不修改 catalog/native 配置。校验 400、不支持的通用导入应用 422、认证 401/403、未知/不属于当前作用域 ID 404、请求关联冲突 409、容量 429；执行期间失败通过安全的 task status 返回，不出站原始错误链、路径或秘密。
- SDK `client/app_providers/operations.rs` 提供 provider_operation_start/get/cancel，使用敏感 token header、原有租户上下文、受限 operation ID 和编码 host query；错误载荷丢弃。相同 tenant/host/requestId 与相同请求在保留期复用 operationId，改变请求返回 409，不重新提交写入。宿主内存最多保留 1024 项，满时清理终态；重启/清理后旧 ID 返回 404，关联与进度不承诺跨重启持久重放。断开观察者不停止 owned worker。没有新增 SSE 重放、任意文件/命令入口或进程执行权限。
- 原同步导入/投影继续走同一业务实现；本任务提供新的异步操作入口。供应商编辑/首次激活的异步 UI 接入仍归所属适配任务，未声明这些 live/UI 路径已完成。Iced 未改动、未启动。

### 红绿与验证

红阶段领取槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，`CARGO_BUILD_BUILD_DIR` 同值。`cargo fmt --all` 退出 0；`cargo test -p vw-agent-apps --lib cc04_operations_ -- --nocapture` 两个指定测试分别在“取消未拒绝发布”与“Preparing != Settling”业务断言失败，**0 passed / 2 failed，退出 101**。四条规定过滤命令均执行；DTO/SDK 各 1 passed、退出 0；Gateway 初次因测试误用 TaskId::to_string 编译失败，修正为实际字段后重跑，不将编译失败或零测试计作红证据。实现检查中的测试闭包参数类型与临时根 0700 权限遗漏已修正，没有放松生产私有目录检查。

本轮阶段动态领取过槽位 **1、2、3**，绝对目录分别为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-2`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-3`；纠正后的业务 6 项、DTO 1 项、SDK 2 项与事务 6 项在槽位 3 通过。最终完整验证领取 **槽位 1**，所有以下命令及子进程继承 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`、`CARGO_BUILD_BUILD_DIR` 同值；聚焦测试使用默认 features：

| 确切命令 | 退出码 | 有效数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-agent-apps --lib cc04_operations_ -- --nocapture` | 0 | 6 passed |
| `cargo test -p vw-api-types --lib cc04_operations_ -- --nocapture` | 0 | 1 passed |
| `cargo test -p vw-agent --lib cc04_operations_ -- --nocapture` | 0 | 3 passed |
| `cargo test -p vw-gateway-client --lib cc04_operations_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-agent-apps --lib cc04_commit_ -- --nocapture` | 0 | 6 passed |
| `cargo test -p vw-agent --lib cc04_import_ -- --nocapture` | 0 | 7 passed |
| `cargo test -p vw-agent --lib cc04_universal_sync_ -- --nocapture` | 0 | 2 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全部 targets/features |

暂存审查补齐 SDK 测试模块与相邻模块一致的非 wasm 限定后，在同一槽位 1 再次执行 `cargo fmt --all`、`cargo test -p vw-gateway-client --lib cc04_operations_ -- --nocapture`（2 passed）与 `cargo clippy --all-targets --all-features -- -D warnings`，均退出 0。

本任务 **12** 项、相关回归 **15** 项，去重 **27** 项通过，无忽略。Gateway 测试链接器继续提示既有 `__eh_frame section too large`，测试实际退出 0；没有改变链接设置或放宽 lint。未执行无过滤全量测试、fmt check、wasm 或 Makefile。各阶段原子 mkdir 领取并记录计划/任务/会话/PID，子进程全部结束后 finally 删除自身记录并 rmdir 释放锁；最终脚本处理 SIGINT/SIGTERM 时先结束并等待自身子进程组，再释放，缓存保留。未处理其他会话的锁或缓存。

### SDK→API→执行端与 macOS 证据

真实回环 HTTP、正式 SDK、真实授权 policy/租户中间件与隔离 SQLite/HOME：SDK start 导入 Pi 的两个声明 → 202/operationId/requestId → SDK get 最终 completed、completed=total=2 → 独立读取保存记录实际含两条假凭据，原 models.json 字节保持一致；重复请求复用 ID，改变关联请求返回 409。SDK start 投影统一供应商 → API 完成 2 项 → 独立读取 Claude/Codex 已保存子记录；响应不含假密钥。终态取消返回原终态；外租户/主机、错误 token、撤回写权限后的提交/取消拒绝，记录不变；正式 handler_router 的 Basic 拒绝返回 401。SDK 单独的真实 HTTP 协议夹具证明 GET/DELETE 路径、host 编码、租户头和 settling DTO，403 响应丢弃秘密载荷；该协议夹具不冒充业务执行证据。

执行端 6 项独立行为测试证明：发布前取消不改原文件；实际事务暂存期间取消保留原字节/空指针、无 pending 或暂存残留；持久发布 marker 后取消实际写完文件并落定指针；marker 后中断保留 published pending、failed/settling，现有 recover 实际写完文件与指针并清 pending；无观察者仍完成工作，同一句柄重复提交拒绝。仅用现有内部 checkpoint 替代不可控中断时机，文件系统、授权、数据库及回环 HTTP 均实际执行，凭据均为假值。

实际平台 macOS **27.0（26A428）/ arm64**，workspace crate **0.6.0**。未使用真实用户配置或账号；真实客户端版本/消费、Keychain、Iced 渲染及非 macOS **未验证**。

状态：**已完成（请求关联、进度与安全取消边界）**。本任务定向差异/暂存审查通过，仅提交上述实现、独立测试、必要父注册与本条证据；排除既有 Dsh 格式差异、其他任务/计划与并行桌面变更。自身 Cargo 锁释放，plan.md 只读；编号任务验收后删除且不提交该删除，下一步批次 51。

## 任务 51：供应商 API 权限与脱敏回归

2026-11-07。已核对批次 50 提交 `d527b8522` 与上节正式证据；母任务 1、3 为已完成，母任务 2 整体迁移未收口，沿任务 1/provider-contracts.md 已有底座子计划 1–93 开工裁定消费实际接口，不冒称母任务 2 完成。所有 plan.md 保持只读。

### 来源、审查与最小修复

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，rev-parse/status 退出 0、来源干净。只读 `commands/provider.rs::{get_providers,get_provider_editor_view}`、`commands/config.rs::{get_config_status,get_common_config_snippet,set_common_config_snippet}`、`commands/env.rs::{check_env_conflicts,delete_env_vars,restore_env_backup}`，以及 TS providers/env 调用、provider_commands、config 校验测试和 `ProviderService::is_sensitive_config_key` 相邻测试。来源已将 COOKIE/AUTHORIZATION 视为凭据；没有构建或修改来源。

沿生产 app_providers 父注册审查目录 CRUD/复制、展示/端点/图标、统一供应商/投影、路径/预览/编辑器/片段、14 应用适配/导入/模型映射、模式、环境诊断/清理/恢复及异步操作端点。目录读取使用强类型摘要，原生适配返回状态/数量，片段复用 scrub，错误返回固定安全消息/状态，进度只返回固定 DTO；没有新增任意文件/命令入口。租户目录写入与主机配置写入仍沿既有权限边界，不擅自为目录 CRUD 添加主机权限。

红测试发现 `vw-agent-apps/src/providers/preview.rs::redact` 遗漏 Authorization/Cookie：非字符串认证值、Cookie 子树中的属性名会保留。最小修复仅给既有大小写无关字段识别补充 `authorization`、`cookie`，整个凭据子树被替换为 `[redacted]`。既有所有字符串叶子脱敏策略保持；`preview::read` 与 `editor_merge::display` 复用此所有者，不新增 DTO、SDK、handler 或第二套脱敏实现。独立 Gateway 测试位于 `handlers/app_providers/api_security_tests.rs`，按 macOS test 模块注册。

### 红绿与确切验证

各阶段均原子 mkdir 动态领取 **槽位 1**，记录计划/任务/会话/PID；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同值，全部 Cargo 与子进程继承。首次日志捕获测试误用未依赖的 tracing_subscriber，退出 101；已改用仓库现有 tracing::Subscriber 模式，未加依赖，不将编译失败计作红证据。

纠正接线后，`cargo fmt --all` 退出 0；`cargo test -p vw-agent --lib cc04_api_security_ -- --nocapture` 在响应凭据扫描业务断言失败，**3 passed / 1 failed，退出 101**，失败为 `cc04_api_security_nested_secrets_never_reach_responses`。实现修复后增加只读 grant 拒绝断言，最终同一阶段验证如下（聚焦测试使用默认 features）：

| 确切命令 | 退出码 | 有效数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-agent --lib cc04_api_security_ -- --nocapture` | 0 | 4 passed |
| `cargo test -p vw-agent --lib gateway::api::handlers::app_providers:: -- --nocapture` | 0 | 145 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全部 targets/features |

领域 145 项已包含本任务 4 项，即额外相邻回归 141 项，去重 145 项；无忽略。测试链接器提示既有 `__eh_frame section too large`，实际测试退出 0；未改变链接配置或放宽 lint。没有运行无过滤全量 cargo test、fmt check、wasm 或 Makefile。

### SDK→API→执行端、权限与平台证据

- 正式 SDK `app_provider_preview` → 真实回环 HTTP/生产 preview 子路由/真实 policy 与 tenant middleware → 隔离 HOME 的实际 Claude settings.json。假 API key、混合大小写数值 Authorization、Cookie 子树属性名、嵌套数组密码均不出现在响应。预览前后整个隔离目录的目录清单、文件和 SQLite 字节一致。安装线程作用域 TRACE 捕获器，先确认捕获标记存在，再扫描本请求的 SDK/Gateway 日志，未发现假凭据；不将其外推为宿主所有线程的全局日志审计。
- 14 应用真实 switch/add 路由分别接受有效 target DTO，外租户/外主机返回 403，错误 token 返回 401；读权限存在但写权限撤回后均返回 403。导入/操作提交/环境清理/恢复/编辑保存的外租户请求返回 403。61 个权限拒绝请求后，实际文件/目录清单与数据库字节不变，未写真实用户配置。
- 正式 SDK `provider_operation_start/get` → API → Pi 原生配置导入真实 SQLite。终态 completed，独立读取保存记录确实含假 key，提交/进度 DTO 不含凭据；原生目录/摘要/模式读取响应、404、外租户进度 GET/取消 DELETE 403 及损坏 Claude 配置的 parseError 均扫描通过；parseError 只给位置，不返回源片段。各拒绝前后目录/数据库字节一致。
- 另用正式 `handler_router` 与既有 Basic middleware 对 29 个读写/导入/预览/错误/进度相关请求证明无 Gateway 凭据时先返回 401，隔离数据库及目录无变化。业务 HTTP 测试注入管理员 policy 路径，全部使用生产子路由，没有替换 handler、存储、文件系统或授权逻辑。

实际平台 **macOS 27.0（26A428）/ arm64**，workspace crate **0.6.0**。只使用假凭据和临时 HOME/SQLite；未使用真实用户凭据/账号。真实客户端应用消费、Keychain、Iced/UI 渲染和非 macOS **未验证**；本任务未改 UI，不将安全回归外推为这些路径验收。

状态：**已完成（供应商 API 权限与脱敏回归）**。仅提交本任务共享脱敏修复、独立测试/注册及本条正式证据，排除既有 Dsh 格式差异、母任务状态、其他编号任务删除及其它计划。定向差异与暂存检查通过；各阶段自身锁释放、缓存保留。验收后删除任务 51 编号文件，不提交该删除；下一步批次 52。

## 任务 52：桌面供应商列表与排序

2026-11-07。依赖批次51提交 `07a0d3b9c` 及其正式证据；调度明确 provider-contracts.md 底座子计划1–93开工裁定覆盖本批，不宣称母任务2完整领域迁移完成。只读 plan.md 未修改。

来源：冻结来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 ProviderList/ProviderCard/presentation、App.tsx 调用及 ProviderCardLayout 相邻测试。消费来源的列表、成员状态、分类备注与排序行为；路由/聚合和激活不在本批实现。现有 SDK catalog/presentation 是唯一数据入口，不增加业务存储或API。

实现：`vw-desktop/src/app/{components,state,message}/app_providers/list.rs` 和最小父注册；“应用”卡片的“供应商”按钮复用当前连接的主机/令牌，展示服务直连、原生当前、成员状态、读取不可用原因和凭据配置摘要。分类、排序、备注弹窗使用统一 settings 控件、按钮与 toast，内容区4px滚动条，两侧7px留白，标题/操作固定；中文不用等宽字体。保存成功后SDK重新读取列表，失败保留表单，取消丢弃草稿。应用/主机/网关/租户/项目上下文和单调请求代次共同拒绝过期响应；关闭窗口不会重用代次。

红证据：`cargo test -p vw-desktop --lib cc04_ui_list_ -- --nocapture`，退出101，0 passed/2 failed；两个指定测试分别在过期响应覆盖断言和“排序未通过真实SDK请求保存”断言失败，非编译失败。随后修复返回校验及正式SDK调用，4项行为测试通过。初轮已设 CARGO_TARGET_DIR，但仓库 build-dir=target 仍导致中间产物共用；发现后设置 CARGO_BUILD_BUILD_DIR 同值并重跑最终测试，不以初轮作为槽位隔离证据。

最终隔离验证：所有槽位均原子 mkdir 动态领取，记录任务/会话/PID；CARGO_TARGET_DIR 和 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-N`。桌面聚焦及窗口阶段槽位1；正式API相邻回归槽位4；首次Clippy槽位3。`cargo fmt --all` 退出0；`cargo test -p vw-desktop --lib cc04_ui_list_ -- --nocapture` 退出0，4 passed；`cargo test -p vw-agent --lib cc04_presentation_ -- --nocapture` 退出0，3 passed；`cargo clippy --all-targets --all-features -- -D warnings` 退出0。桌面测试真实HTTP检查PUT路径和sortIndex/category/notes；相邻正式SDK→生产路由→隔离SQLite验证重读、拒绝不写、作用域隔离及原生文件不变，不将HTTP夹具冒充真实Gateway业务。链接器既有 __eh_frame 提示不影响测试退出0，未放宽lint。

macOS 27.0（26A428）/arm64，workspace 0.6.0；显式 `cc04-provider-window`（harness=false/test=false）复用隔离bootstrap和字体加载，未知case/theme直接报错。临时回环HTTP夹具仅替代服务端，使用假token及16条中文摘要，真实组件/Message/SDK均执行。无生产bootstrap、真实网关或用户配置写入。

确切命令 `SHIJU_CC04_WINDOW_CASE=list SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window` 与 light 同形命令均成功构建并启动窗口；原始无bundle进程无法通过CUA定位，主动SIGTERM结束（Cargo退出101），不将启动计通过。复制同一槽位产物到临时 `/tmp/CC04ProviderAcceptance.app` 后分别以相同case/theme运行，CUA实际操作、手动关闭均退出0；临时fixture线程随窗口退出并join。首次真实渲染发现验收base未Fill导致弹窗裁剪，已修正入口容器并重新验收。原生日志有IMK mach-port系统提示，无panic。

操作：明暗主题各打开排序备注弹窗，保存数字排序、中文分类/动态备注，SDK重读后可见；浅色另保存80遍中文长备注，列表正常换行并滚动；notes=fail触发真实HTTP403，toast可见、表单保留可编辑；取消后未保存fail不覆盖此前中文备注。实际截图：[暗色保存](evidence/task-52/dark-saved.png)、[暗色失败](evidence/task-52/dark-failure.png)、[暗色取消](evidence/task-52/dark-cancel.png)、[浅色表单](evidence/task-52/light-form.png)、[浅色长备注](evidence/task-52/light-saved.png)、[浅色失败](evidence/task-52/light-failure.png)。截图均为真实原生绘制，未以Element测试替代。非macOS、真实客户端激活/消费及真实账号未验证。

最终重跑阶段仍动态领取槽位1，两个Cargo目录均指向 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`；格式化、4项桌面聚焦测试及全targets/features Clippy均退出0。定向暂存审查排除既有Dsh格式差异、母任务状态、其他编号任务删除及其它计划；截图约1.6MB仅含隔离假数据。自身槽位锁在各阶段结束后清理，缓存保留。

状态：**已完成（桌面列表、展示排序与过期响应隔离）**。真实窗口验收使用上述同产物临时bundle回退，Cargo原始窗口命令的101保留如实记录，不冒称其退出0。下一步批次53；任务52编号文件验收后删除且不提交该删除。

## 任务 53：桌面供应商增删改复制

2026-11-07。批次 52 提交 `d59bfc858` 及正式证据已核对；母任务 1、3 已完成，母任务 2 沿 provider-contracts.md 既有底座子计划 1–93 开工裁定消费实际接口，不冒称整体完成。继续指令明确交付统一基础壳与完整应用草稿的组合，解除最初“基础字段不足以构造应用配置”的阻塞；没有引入未配置档案生命周期或默认模型/地址/官方模式。所有 plan.md 只读。

### 来源与实现边界

只读来源 HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，status 干净。已读 AddProviderDialog::handleSubmit、EditProviderDialog::handleSubmit、BasicFormFields、App.tsx 直接调用者及三个相邻组件测试；来源的 websiteUrl 是展示网址，settingsConfig 来自专属表单，不能当作 baseUrl。RustCodeGraph status/explore 已执行，图未返回目标片段后按技能回退定向搜索；修改前已读 system_settings_common 与 custom_model_modal。

- `vw-desktop/src/app/{components,state,message}/app_providers/edit.rs` 与直接父注册、SettingsMessage、既有 list 入口构成同一弹窗壳。列表提供新增、编辑、复制、删除；取消不保存，失败保留表单/目标，只有成功刷新目录。请求和返回均校验原 scope、generation 与当前 Gateway/租户/项目/主机，迟到响应不修改新上下文。删除复用正式 SDK 与既有授权主机引用检查，不自造删除所有者。
- 应用表单通过当前 scope 的 `ListState.new_draft` 提供现有强类型 AppProviderCreate 草稿；切换 scope 清除该上下文。没有草稿、缺少 model/baseUrl 或请求 live 激活时禁用新增保存并显示待补配置说明。此次真实调用者为隔离完整假配置 fixture；生产专属表单由 54–67 接到同一壳，后续按实际原生 DTO/应用适配扩充桥接。不能把本次通用 DTO 夹具当作 14 应用全部配置变体的新增/复制/密钥编辑验收。
- 编辑只从服务端获取 `ProviderBasic`（name/category/notes/websiteUrl），不读取原始配置或密钥；Keep/Replace/Clear 明确表达。替换输入使用 secure 输入框，空值与脱敏标记禁用提交；新建草稿中夹带的凭据被改为 Keep，唯一新密钥来自本次明确输入。请求 Debug 隐藏密钥。原生专属凭据变体仍须应用表单/适配处理，通用路径拒绝不支持的替换，不把脱敏值或空串伪作旧值。
- 新增最小正式 DTO `vw-api-types/src/app_providers/basic.rs`，SDK `app_provider_basic/app_provider_basic_write`，路由 `GET/PUT /v1/app-providers/{app}/{id}/basic`。父 Gateway 鉴权与租户作用域沿用正式注册；未知字段拒绝、16 KiB 限制、错误出站固定安全内容，不含请求密钥或服务端原始响应体。没有任意文件/命令/namespace 接口。
- `vw-agent-apps/src/providers/basic.rs` 在唯一领域所有者中校验基础字段；网址只接受无内嵌凭据的 http(s) 地址，与 baseUrl 分开。修改保留未知字段、排序和应用配置；原生 profile 内用于摘要的 name 同步更新。`vw-agent/src/app_providers/basic.rs` 沿原租户 settings/SQLite/PostgreSQL 所有者，新增/复制用 insert-if-absent，更新用 compare-exchange，一次写入基础字段；复制复用既有白名单并排除账号/未知授权元数据。基础操作不激活、不改原生文件，不宣称 live 成功。

### TDD 与自动验证

红阶段动态槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`；`CARGO_BUILD_BUILD_DIR` 同值。确切命令 `cargo test -p vw-desktop --lib cc04_ui_edit_ -- --nocapture` 编译接线成功，两个指定测试分别在“脱敏值不得提交”“删除失败不能关闭选择与确认”的业务断言失败，**0 passed / 2 failed，退出 101**。修复后原两项转绿，继续增加草稿、秘密意图与真实 SDK 调度断言。首次 Clippy 报 DTO large_enum_variant，已将 Create 草稿装箱，未放宽 lint。

末审补充消息诊断保密断言：`cargo test -p vw-desktop --lib cc04_ui_edit_redacted_secret_is_not_submitted -- --nocapture` 在 Key 消息自动 Debug 会打印假密钥的业务断言失败，0 passed / 1 failed，退出 101。随后将整个 EditMessage Debug 改为固定脱敏输出；该断言并入原指定测试，不增加有效测试计数。最后仅调整消息诊断输出与独立断言，视图/调度路径保持上述实际窗口验证范围。

所有阶段先按 mkdir 原子动态领取槽位并记录任务/会话/PID，两个 Cargo 目录始终使用对应绝对路径；中间独立 Gateway 阶段动态槽位 **3**：`/Users/xiangmin/worker/code/shiju/target/rust-slot-3`，2 项 HTTP/DB 测试通过，退出 0。最终收口与窗口阶段槽位 **1**；审查补充后重新格式化并执行下表，不重复累计先前轮次。

| 确切命令（默认 features；Clippy 全 features） | 退出码 | 有效结果 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-desktop --lib cc04_ui_edit_ -- --nocapture` | 0 | 6 passed |
| `cargo test -p vw-desktop --lib cc04_ui_list_ -- --nocapture` | 0 | 4 passed |
| `cargo test -p vw-agent --lib cc04_ui_edit_ -- --nocapture` | 0 | 3 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |

独立桌面测试 `message/app_providers/edit_tests.rs` 覆盖指定两项、无完整草稿拒绝、Keep 不带 value、明确 Clear、复制来源/新身份、草稿夹带密钥剥离、HTTP SDK PUT 与 DELETE 错误结果。Gateway 独立 `basic_tests.rs` 使用真实回环 HTTP、正式 SDK→生产路由→临时 SQLite→重开存储，断言新增/编辑/复制结果、websiteUrl 与 baseUrl 分离、model/密钥/未知元数据保留及复制授权白名单；Basic 未授权、外租户、脱敏凭据拒绝均无写入。原生 Claude 记录另断言其专属 config 不变、嵌套名称与正式摘要同步、通用密钥替换拒绝后内容不变。SDK 成功 HTTP 测试沿父路由、租户中间件，独立 Basic 拒绝测试验证保护；不将其称为真实账号或执行主机的实机授权验收。

### macOS 真实窗口

macOS **27.0（26A428）/ arm64**，workspace **0.6.0**。沿任务 52 的显式原生入口，case 仅支持 list/edit、theme 仅 dark/light，未知值直接报错。edit 使用完整且显式的假应用配置（fixture model/API 地址）和假 token；仅本地 HTTP 服务被替代，真实 Iced 组件、Message 和正式 SDK 执行，未读取/修改真实用户配置。

确切窗口命令如下，均在槽位 1 内执行并手动关闭，**退出 0**：

```bash
SHIJU_CC04_WINDOW_CASE=edit SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window
SHIJU_CC04_WINDOW_CASE=edit SHIJU_CC04_WINDOW_THEME=light cargo test -p vw-desktop --test cc04-provider-window
```

为使 CUA 能稳定定位原生进程，额外继承 `CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER=/tmp/shiju-53-window-runner.sh`；runner 仅把 Cargo 传入的同一槽位测试二进制复制到临时 `/tmp/CC04Edit53Acceptance.app` 后 exec，Cargo 全程等待并返回其真实退出码。没有把原始命令启动、零测试或强制杀进程当作窗口通过。两个窗口分别在 CUA 中观察关闭，fixture 线程随应用退出并 join 后才释放槽位。日志有系统 sandbox_extension_issue_file_to_process 提示及既有链接器 __eh_frame 提示，实际窗口正常退出，无字形整形 panic。

暗色实际操作：中文名称/网址与 80 遍中文长备注保存，重新编辑重读；name=fail 触发 HTTP403，toast 与可编辑表单保留；取消后重开仍是已保存中文名称。选择替换密钥、输入脱敏占位值后保存无动作，再输入明确假密钥保存；复制为新 ID 并看到中文副本；使用完整应用草稿新增并看到中文记录。删除先取消再确认成功移除副本；删除当前 provider-0 得到 HTTP409，确认框和目标保留，取消后列表直连/原生当前仍为 provider-0。浅色实际操作：中文名称/分类/80 遍长备注保存重读，切换明确清除意图再取消该意图，HTTP403 失败保留、取消后中文已保存记录仍在；删除确认可取消。只滚动内容，标题和底部操作固定；输入/复选框/按钮复用统一明暗样式，4px 滚动条两侧各 7px 留白；中文均走默认字体。

真实截图：[暗色表单](evidence/task-53/dark-form.png)、[保存](evidence/task-53/dark-saved.png)、[失败](evidence/task-53/dark-failure.png)、[密钥意图与滚动](evidence/task-53/dark-key-intent.png)、[复制](evidence/task-53/dark-copy.png)、[组合新增表单](evidence/task-53/dark-create-form.png)、[新增结果](evidence/task-53/dark-created.png)、[删除失败](evidence/task-53/dark-delete-failure.png)、[保留选择](evidence/task-53/dark-selection-preserved.png)、[浅色表单](evidence/task-53/light-form.png)、[保存](evidence/task-53/light-saved.png)、[失败](evidence/task-53/light-failure.png)、[删除确认](evidence/task-53/light-delete-confirm.png)。均为真实原生绘制，未用 Element 构造测试替代。

边界：窗口服务端是本地假数据夹具，真实持久化由上述隔离 HTTP/SQLite 测试证明；不将二者合称真实生产 Gateway/账号/原生客户端消费通过。54–67 专属配置表单、14 应用完整新增与原生密钥变体、当前供应商 live/首次激活、真实账号/Keychain、非 macOS 均未在本任务验收。

状态：**已完成（统一基础壳、组合草稿及正式基础元数据接口）**。最终格式化、6/4/3 项聚焦验证及 Clippy 均退出 0；明暗窗口均退出 0。自身所有阶段槽位 1/3 锁已清理，缓存保留。仅提交本任务实现、独立测试、必要接线及本条证据/截图。排除既有 Dsh 格式差异、母任务状态、其他编号任务删除和其它计划；计划文件保持只读。本编号文件在最终证据齐备后删除，不提交该删除；下一步批次 54。

## 任务 54：桌面 Claude 供应商表单

2026-11-07。批次 53 提交 `ffb6a2d9d` 及其正式证据已核对，沿用批次 52 的窗口入口和批次 26 的 Claude 执行端适配。母任务 1、3 已完成，母任务 2 依 `provider-contracts.md` 的底座子计划 1–93 开工裁定消费实际接口，完整领域迁移未收口。所有 plan.md 保持只读。

来源：冻结上游 `ca58c0313360eae7c851fbd7553c1614c7af03e7`，核对 `ClaudeFormFields` 的 `apiKeySection`、`endpointSection`、`authFieldSelect`、主模型输入及 `ProviderForm` 的 Claude 分支与相邻 `ClaudeFormFields.test.tsx`。上游仅作为行为来源，未运行其测试。代码导航使用 RustCodeGraph，桌面编辑前对照统一设置控件和 custom_model_modal。

- `vw-desktop/src/app/{components,state,message}/app_providers/claude.rs` 接现有新增、编辑弹窗。官方使用 Claude Code 已有登录；账号绑定及模型发现明确暂不可用。第三方可编辑 `ANTHROPIC_BASE_URL`、`ANTHROPIC_AUTH_TOKEN` 或 `ANTHROPIC_API_KEY`、`ANTHROPIC_MODEL`；不回显已存密钥。云配置、托管账号或旧格式不做隐式转换，保留基础资料编辑。失败保留草稿；过期代次、主机和工作上下文响应不覆盖当前表单。目录写入和执行端应用结果分别展示。
- 新增强类型 `ClaudeFormView/ClaudeFormWrite`、SDK `claude_form_{read,create,update}`，走既有 Claude Gateway 鉴权、租户及主机授权。读仅返回表单投影；写经 `providers/claude_form.rs` 还原专属 profile，复用现有字段校验、基础资料校验、catalog CAS、Claude JSON 投影、执行端事务。换 API 地址必须替换旧密钥，避免把原凭据发给另一上游。客户端未实现地址/密钥策略，也不能提交任意配置文件或命令。
- 可选应用只接现有 `First` 或 `IfCurrent`，由执行端判断是否实际应用。真实回环 SDK → 生产 Gateway handler/授权 → 临时 SQLite/HOME 用例证明：首次激活写入地址、密钥和模型，保留用户 env/hook；当前编辑复用原密钥、切换认证键后清掉旧字段；改为官方后移除第三方字段和目录密钥；第二条首激活请求仅存目录，文件字节保持一致。坏网址、换地址且保留密钥、错误 token 和外租户均拒绝，目录记录、事务状态和原生文件保持一致。读响应不含假密钥。

### 红绿验证

各阶段从仓库根用 mkdir 原子动态领取槽位，记录任务/计划/会话/PID。红绿及自动回归使用 **槽位 1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`；窗口使用 **槽位 3**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。两阶段的 `CARGO_BUILD_BUILD_DIR` 同设对应绝对路径，避免仓库 build-dir 合用中间产物。

初次红测夹具在未启动服务时初始化 App，等待后终止；不计验证。改为独立回环夹具后，`cargo fmt --all` 退出 0；`cargo test -p vw-desktop --lib cc04_ui_claude_ -- --nocapture` 退出 **101**，1 passed / 1 failed，失败在 Claude 草稿未通过正式 SDK 触达校验的业务断言。实现后真实 SDK 收到 HTTP 422，名称、模型和本次新密钥草稿保留；旧主机或代次的返回不能关闭新表单。

| 命令 | 退出码 | 有效结果 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-desktop --lib cc04_ui_claude_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-desktop --lib cc04_ui_edit_ -- --nocapture` | 0 | 6 passed |
| `cargo test -p vw-agent --lib gateway::api::handlers::app_providers::claude_tests:: -- --nocapture` | 0 | 13 passed，含本任务 2 项 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 targets/features |
| `SHIJU_CC04_WINDOW_CASE=claude SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际交互并关闭 |
| `SHIJU_CC04_WINDOW_CASE=claude SHIJU_CC04_WINDOW_THEME=light cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际交互并关闭 |

### macOS 窗口

macOS **27.0（26A428）/ arm64**，workspace **0.6.0**。两个命令均通过 `CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER=/tmp/shiju-54-window-runner.sh` 在同一槽位构建后将 Cargo 传入的二进制放到隔离临时 bundle，Cargo 等待实际窗口退出。真实 Iced 组件、Message 和正式 SDK 接回环 HTTP 假数据，窗口夹具只替代服务端；真实事务和文件结果由上述独立 Gateway 测试验证。未使用真实 Claude 账号、Keychain、用户配置或 CLI 消费；非 macOS 未验收。

- 暗色：新增第三方，输入中文名称、长备注和长模型，切换认证键。无效地址触发 HTTP 422，toast 可见，草稿及遮蔽的假密钥仍在；修正地址后保存，列表重读；再次编辑后取消并重开，仍为已保存值，未回显密钥。[失败及长表单](evidence/task-54/dark-failure.jpg)、[保存](evidence/task-54/dark-saved.jpg)、[取消重读](evidence/task-54/dark-cancel.jpg)。
- 浅色：新增官方并保存中文动态备注和模型，重读后切换第三方、填入坏地址触发 HTTP 422，取消并重开，官方选项和原模型仍在。[官方表单](evidence/task-54/light-official.jpg)、[保存](evidence/task-54/light-saved.jpg)、[失败及长表单](evidence/task-54/light-failure.jpg)、[取消重读](evidence/task-54/light-cancel.jpg)。

两个主题均实际滚动内容，标题及底部按钮固定，4px 滚动条两侧预留 7px，中文与动态文本使用默认字体。截图和正常退出证明本次操作未见字形整形 panic；仅出现既有链接器 `__eh_frame` 与系统 sandbox extension 提示。两阶段进程退出后已清理本任务锁并保留编译缓存；差异审查排除了预先存在的 Dsh 格式差异、其它任务和计划文件。

状态：**已完成**。下一步批次 55；本任务编号文件按约定验收后删除，删除不入提交。

## 任务 55：桌面 Gemini 供应商表单

2026-11-07。批次 54 提交 `babf326d5` 及其验收记录已核对。母任务 1、3 已完成，母任务 2 沿 `provider-contracts.md` 的既有底座子计划 1–93 开工裁定消费实际接口，完整领域迁移仍待收口。沿用批次 31 Gemini 适配和批次 52 显式窗口入口。

- 来源为冻结上游 `ca58c0313360eae7c851fbd7553c1614c7af03e7`，工作区干净。读取 `GeminiFormFields`、`ProviderForm` 调用分支及相邻 `GeminiFormFields.test.tsx`。相邻用例侧重 Agy 模型发现，未运行来源测试。RustCodeGraph 定位既有 SDK，再核对执行端与桌面调用者。
- `vw-desktop/src/app/{components,state,message}/app_providers/gemini.rs` 接供应商列表的新增与编辑弹窗。第三方编辑 `GOOGLE_GEMINI_BASE_URL`、写入专用的 `GEMINI_API_KEY` 和 `settings.json` 的 `model.name`，官方使用 Gemini CLI 已有的 Google 登录。账号授权、绑定和模型发现明确暂不可用，非原生或托管档案可编辑基础资料。共享弹窗负责状态与 toast，HTTP 校验失败保留草稿，旧代次或主机响应不能关闭新表单。
- SDK `gemini_form_{read,create,update}` 对应 `POST /v1/app-providers/gemini/{id}/form/read`、`POST /form` 和 `PATCH /form`。Gateway 在读取档案或接触原生文件前检查租户、主机和读写授权。执行端 `providers/gemini_form.rs` 返回脱敏投影，保持原密钥仅允许同一 API 地址，写入复用基础资料校验、目录 CAS 和已有双文件事务。可选应用由执行端按新增 `First` 或编辑 `IfCurrent` 判定，桌面分别反馈目录保存和执行端应用结果。
- 正式 SDK 经回环 HTTP → Gateway → 隔离 SQLite/HOME。首次激活写入端点和假密钥，当前编辑保留同端点密钥并更新 `model.name`。保留 dotenv 注释、用户变量、MCP 和 UI 设置。第二条首次激活请求只保存目录；改为官方后写入 `oauth-personal` 并移除第三方凭据。读响应无原密钥。错误令牌、换端点且保留密钥、重复创建和编辑不存在的档案均被拒绝，目录、事务状态和原生文件无变化。

### 红绿验证

阶段均以原子 mkdir 动态领取 **槽位 1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。红测 `cargo fmt --all` 退出 0，`cargo test -p vw-desktop --lib cc04_ui_gemini_ -- --nocapture` 退出 101（1 passed / 1 failed），失败在 Gemini 草稿尚未通过 SDK 提交到校验端的行为断言。实现后桌面测试通过回环 HTTP 夹具注入 422，验证名称、地址、模型及新输入的假密钥均保留；Gateway 测试使用正式 handler 和真实文件。

| 命令 | 退出码 | 有效结果 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-desktop --lib cc04_ui_gemini_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-agent --lib gateway::api::handlers::app_providers::gemini_tests:: -- --nocapture` | 0 | 7 passed，含本任务 1 项 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 targets/features |
| `SHIJU_CC04_WINDOW_CASE=gemini SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并手动关闭 |
| `SHIJU_CC04_WINDOW_CASE=gemini SHIJU_CC04_WINDOW_THEME=light cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并手动关闭 |

### macOS 窗口

macOS **27.0 (26A428) / arm64**，workspace **0.6.0**。窗口命令使用 `CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER=/tmp/shiju-55-window-runner.sh` 将 Cargo 传入的二进制放入隔离临时 bundle，并等待实际窗口退出。实际 Iced 组件、Message 与正式 SDK 连接回环 HTTP 假数据；窗口只替代服务端，原生双文件写入由上述独立 Gateway 测试验证。真实 Google 账号、Gemini CLI 版本及其消费配置、非 macOS 平台未验证。

- 深色：新增第三方，填写中文名称、长备注和动态模型。错误地址收到 422，toast 显示失败，模型和遮蔽的假密钥仍在。改正地址后保存并重读列表，改名后取消，重开后显示原有值和保留密钥选项。[失败及长表单](evidence/task-55/dark-failure.jpg)、[保存](evidence/task-55/dark-saved.jpg)、[取消后重开](evidence/task-55/dark-cancel.jpg)。
- 浅色：新增官方，填写中文长备注和模型后保存。重开后切换第三方并提交无效地址，收到 422，取消并重读后仍为官方和原模型。[官方表单](evidence/task-55/light-official.jpg)、[保存](evidence/task-55/light-saved.jpg)、[失败及长表单](evidence/task-55/light-failure.jpg)、[取消后重开](evidence/task-55/light-cancel.jpg)。

明暗两次实际滚动时标题和操作区固定，滚动条宽 4px，两侧各预留 7px。中文和动态文本沿默认字体路径绘制，两次正常退出且未出现字形整形 panic。日志包含既有链接器 `__eh_frame` 及系统 sandbox extension 提示。定向差异审查排除并行终端与计划改动，自身槽位锁全部释放，缓存保留。

状态：**已完成**。下一步批次 56。本编号文件验收后删除，删除不入提交。

## 任务 56：桌面 Codex 配置编辑器

2026-11-07。批次 55 提交 `1474abb89` 及其正式验收已核对；母任务 1、3 已完成，母任务 2 沿 `provider-contracts.md` 的底座子计划 1–93 开工裁定消费现有接口，完整领域迁移仍未收口。沿用批次 28–30 的 Codex 投影、登录处理和事务适配，以及批次 52 的显式窗口入口。

- 来源：冻结上游 `ca58c0313360eae7c851fbd7553c1614c7af03e7`，工作区干净。读取 `CodexConfigEditor`、`CodexConfigSections`、`ProviderForm` 的 Codex 分支、`CodexFormFields::renderCatalogRows` 和相邻 `CommonConfigModalBehavior.test.tsx`。上游只读，未运行其测试。
- 桌面 `vw-desktop/src/app/{components,state,message}/app_providers/codex.rs` 接供应商列表的新增和编辑。编辑第三方选路与模型 TOML、菜单显示名和请求模型；失效字段只显示执行端返回的路径。密钥单独遮蔽输入。400/422 保留草稿，作用域或代次过期的响应被忽略；目录保存和应用结果分别反馈。托管、官方和不支持的凭据形态仅可修改基础资料；官方授权、模型发现/推荐、外部目录内容与代理模型转换不在本项。
- SDK `codex_form_{read,create,update}` 对应 `POST /v1/app-providers/codex/{id}/form/read`、`POST /form`、`PATCH /form`。Gateway 复用租户和主机读写授权，执行端 `vw-agent-apps/src/providers/codex_catalog/form.rs` 处理 TOML 安全投影、校验、同端点密钥保留、模型映射；密钥、未公开的 TOML 值和注释不进入读响应。现有映射按同端点和模型 ID 保留既有元数据；目录仍由模型目录服务持有，原生变更沿既有 Codex pending 事务。显式勾选应用时，新增仅首次激活，编辑仅应用当前直连档案。
- 正式 SDK → 回环 HTTP → 正式 Gateway handler → 隔离 SQLite/HOME。读出失效字段 `mcp_servers.中文夹具`，未返回其值、注释或假密钥。编辑当前档案后，`config.toml` 出现新模型并保留原密钥与用户注释，失效字段仍保存在档案，未写入原生配置；`shiju-model-catalog.json` 写入中文显示名并保留模型元数据。首次激活实际更新指针；已有指针时新增只保存目录。错误令牌、重复新增、编辑缺失档案、无效 TOML、换地址且保留旧密钥分别被拒绝，档案、原生文件和指针不变。

### 红绿验证

原子 mkdir 动态领取：红测及首轮绿测槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`；最终验证槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。`CARGO_BUILD_BUILD_DIR` 同各自槽位。红测格式化退出 0，桌面过滤命令退出 101（0 passed / 2 failed），失败在尚未通过 SDK 提交和有效响应尚不能结束保存。绿测的桌面回环 HTTP 夹具返回 422；真实执行端文件证据由 Gateway 用例给出。

| 命令 | 退出码 | 有效结果 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 格式化 |
| `cargo test -p vw-desktop --lib cc04_ui_codex_ -- --nocapture` | 0 | 2 passed |
| `cargo test -p vw-agent --lib gateway::api::handlers::app_providers::codex_catalog_tests:: -- --nocapture` | 0 | 7 passed，含本任务 1 项 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 targets/features |
| `SHIJU_CC04_WINDOW_CASE=codex SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并手动关闭 |
| `SHIJU_CC04_WINDOW_CASE=codex SHIJU_CC04_WINDOW_THEME=light cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并手动关闭 |

### macOS 窗口

macOS **27.0 (26A428) / arm64**，workspace **0.6.0**。隔离 runner 将槽位产物放入临时 app bundle；真实 Iced 组件、Message 和 SDK 连接本机回环 HTTP 夹具，使用假档案。两种主题均从供应商列表打开现有 Codex 档案，输入中文 TOML 草稿和中文映射，提交无效 TOML 并确认错误提示及草稿保留；修正后保存，重开、改映射名后取消，再重开确认保留已保存映射。滚动查看了中文失效路径和长表单，标题与底部按钮固定，沿用 4px 滚动条和两侧各 7px 留白。两次正常关闭，未出现字形整形 panic。真实 Codex CLI/账号、外部模型目录和非 macOS 平台未验证。

- 深色：[失败与映射](evidence/task-56/dark-failure.jpg)、[保存](evidence/task-56/dark-saved.jpg)、[取消后重开](evidence/task-56/dark-cancel.jpg)。
- 浅色：[失败与映射](evidence/task-56/light-failure.jpg)、[保存](evidence/task-56/light-saved.jpg)、[取消后重开](evidence/task-56/light-cancel.jpg)。

差异审查仅纳入 Codex 读写契约、桌面接线、测试和本任务证据。链接器仍提示既有 `__eh_frame` 警告，未影响行为测试；槽位锁释放，保留缓存。

状态：**已完成**。

## 任务 57：桌面 Grok 配置编辑器

2026-11-07。批次 56 提交 `7267ce2b3` 已验收。母任务 1、3 已完成。母任务 2 按 `provider-contracts.md` 的底座子计划 1–93 开工裁定执行，完整领域迁移仍待收口。沿用任务 32 的 Grok 原生事务和任务 52 的显式窗口入口。

- 冻结来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 干净。核对 `GrokBuildProviderForm::{handleRawConfigChange,handleSubmit}`、`ProviderForm` 分支、相邻 `GrokBuildProviderForm.test.tsx`、`grok_editor::{view,plan_save}`。来源只读。
- 桌面新增和编辑复用基础弹窗。第三方档案编辑 `models.default` 及其模型表的可公开字段，使用独立遮蔽密钥输入。失效字段展示执行端投影给出的路径。目录验证失败时保留草稿，过期主机或代次的响应丢弃。官方及托管档案可编辑基础资料。官方授权、模型知识和全局配置编辑留给对应任务。
- 正式 SDK `grok_form_{read,create,update}` 连接 `POST /v1/app-providers/grokbuild/{id}/form/read`、`POST /form` 和 `PATCH /form`。执行端 `vw-agent-apps/src/providers/grok_form.rs` 复用 Grok 模型表选择和配置校验。读取响应只返回允许编辑的字段和失效路径，保留原凭据和不公开的 TOML 字段。已存密钥和不公开的模型选项仅随相同地址保留。
- SDK → 回环 HTTP → 正式 Gateway 鉴权和 handler → 隔离 SQLite、HOME 和 Grok pending 事务。首次激活更新指针，已有指针时只新增目录项。应用当前档案的模型表重命名会移除旧表，保留本机注释、其它表、密钥及该模型的不公开选项。旧失效字段保存在档案，未写入原生文件。错误 token、重复新增、更新缺失档案、无效 TOML 及携带旧凭据改地址均拒绝，相关档案、文件和指针保持原状。

### 红绿验证

红测和绿测动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。窗口阶段领取槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。`CARGO_BUILD_BUILD_DIR` 与各自槽位相同。各阶段均已释放自身锁并保留缓存。

| 阶段 | 命令 | 退出码 | 有效结果 |
| --- | --- | --- | --- |
| 红 | `cargo fmt --all` | 0 | 格式化 |
| 红 | `cargo test -p vw-desktop --lib cc04_ui_grok_ -- --nocapture` | 101 | 0 passed、2 failed。草稿未提交给 SDK、当前有效响应未结束保存 |
| 绿 | `cargo fmt --all` | 0 | 格式化 |
| 绿 | `cargo test -p vw-desktop --lib cc04_ui_grok_ -- --nocapture` | 0 | 2 passed |
| 绿 | `cargo test -p vw-agent --lib gateway::api::handlers::app_providers::grok_tests:: -- --nocapture` | 0 | 6 passed，含本任务 2 项 |
| 绿 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 targets/features |
| 窗口 | `cargo fmt --all` | 0 | 格式化 |
| 窗口 | `SHIJU_CC04_WINDOW_CASE=grok SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并关闭 |
| 窗口 | `SHIJU_CC04_WINDOW_CASE=grok SHIJU_CC04_WINDOW_THEME=light cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并关闭 |

### macOS 窗口

macOS **27.0 (26A428) / arm64**，桌面版本 **0.6.0**。临时 app bundle 启动真实 Iced 组件、Message 和 SDK，连接使用假档案的回环 HTTP 夹具。夹具返回实际表单契约，服务校验和原生写入另由正式 Gateway 用例验证。真实 Grok CLI/账号和非 macOS 平台未验证。

深浅主题均从列表打开当前直连档案。提交 `context_window = 0` 后显示失败提示并保留中文草稿。修正数值并勾选同时应用后保存，重开后替换成另一份草稿并取消，再重开确认保存值仍在。滚动查看中文失效字段和表单顶部，标题及底部按钮固定，复用 4px 滚动条和两侧各 7px 的内容留白。两次通过窗口关闭按钮正常退出，日志无字形整形 panic。日志有既有的 `__eh_frame` 和系统 sandbox extension 提示。

- 深色：[失败与保留草稿](evidence/task-57/dark-failure.jpg)、[保存反馈](evidence/task-57/dark-saved.jpg)、[取消后重开](evidence/task-57/dark-cancel.jpg)。
- 浅色：[失败与保留草稿](evidence/task-57/light-failure.jpg)、[保存反馈](evidence/task-57/light-saved.jpg)、[取消后重开](evidence/task-57/light-cancel.jpg)。

差异审查限本任务代码、独立测试、必要接线及本条证据。排除并行终端和其它计划的改动。状态为 **已完成**，下一步批次 58。

## 任务 58：桌面 OpenCode 供应商编辑

2026-11-07。核对批次 57 提交 `4fd67e19f` 与上节验收。母任务 1、3 已完成。母任务 2 沿用 `provider-contracts.md` 记录的底座子计划 1–93 开工裁定，完整领域迁移仍待收口。沿用任务 35 的 OpenCode 原生事务与任务 52 的显式窗口入口。

- 冻结来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7`。核对 `OpenCodeFormFields`、`ProviderForm` 的 V1 字段及 V2 原生编辑分支、`OpenCodeFormFields.test.tsx` 的模型和请求字段案例。桌面复用现有弹窗，编辑单个 V1 `provider` 或 V2 `providers` 节点。新增可同时添加成员，编辑可同步已添加成员，列表分别提供添加与移除。官方授权、模型发现和专用请求选项控件留给各自任务。
- `vw-gateway-client` 的 `opencode_form_{read,create,update}` 调用 `POST /v1/app-providers/opencode/{id}/form/read`、`POST /form` 和 `PATCH /form`。执行端 `providers/opencode_form.rs` 投影公开字段、保留原位的受保护字段、处理 V1 密钥保留/替换/清除、校验 JSON/JSONC，并以公开内容修订检查并发修改。变更已识别的连接字段时拒绝沿用受保护配置。托管档案可以编辑基础资料。成员操作直接复用原 SDK 和事务。
- SDK → 回环 HTTP → 正式 Gateway 权限检查和 handler → 隔离 SQLite、HOME 与原生 JSONC。新增并添加、编辑既有成员、V2 仅保存目录后显式添加、移除成员保留目录、重新添加均检查实际结果。未点名供应商、JSONC 注释、`small_model` 与原凭据保留。验证失败、无写权限及过期公开修订拒绝提交。桌面聚焦用例用假回环响应证明 422 保留草稿，过期主机或代次的响应被忽略。

### 红绿验证

红测与最终聚焦验证动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。窗口阶段领取槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。`CARGO_BUILD_BUILD_DIR` 与各自槽位相同。各阶段退出后释放自身锁并保留缓存。

| 阶段 | 命令 | 退出码 | 有效结果 |
| --- | --- | --- | --- |
| 红 | `cargo fmt --all` | 0 | 格式化 |
| 红 | `cargo test -p vw-desktop --lib cc04_ui_opencode_ -- --nocapture` | 101 | 2 项业务断言失败。草稿未提交给 SDK，有效响应未结束保存 |
| 绿 | `cargo fmt --all` | 0 | 格式化 |
| 绿 | `cargo test -p vw-desktop --lib cc04_ui_opencode_ -- --nocapture` | 0 | 2 passed |
| 绿 | `cargo test -p vw-agent --lib gateway::api::handlers::app_providers::opencode_tests:: -- --nocapture` | 0 | 7 passed，含本任务 3 项 |
| 绿 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 targets/features |
| 窗口 | `SHIJU_CC04_WINDOW_CASE=opencode SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并关闭 |
| 窗口 | `SHIJU_CC04_WINDOW_CASE=opencode SHIJU_CC04_WINDOW_THEME=light cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并关闭 |

### macOS 窗口

macOS **27.0 (26A428) / arm64**，桌面版本 **0.6.0**。临时 app bundle 运行真实 Iced 组件、Message 和 SDK，连接使用假档案的回环 HTTP 夹具。夹具返回表单和目录契约，真实 Gateway 鉴权、事务与文件结果由独立集成用例证明。系统 PATH 未找到 OpenCode CLI，CLI 版本、真实安装消费及账号未验证。

两种主题从供应商列表打开已添加的中文档案，提交错误模型节点后提示失败并保留中文草稿。修正后保存并同步，输入另一份草稿后取消，再重开核对已保存值。长表单滚动时标题和底部按钮固定，滚动条 4px，内容区两侧各预留 7px。移除后档案仍在目录中，重新添加后成员恢复。窗口均正常退出，中文动态文本无字形整形 panic。日志有系统 sandbox extension 与既有链接提示。

- 深色 [失败保留](evidence/task-58/dark-failure.jpg)、[保存](evidence/task-58/dark-saved.jpg)、[取消后重开](evidence/task-58/dark-cancel.jpg)、[滚动](evidence/task-58/dark-scroll.jpg)、[移除](evidence/task-58/dark-removed.jpg)、[重新添加](evidence/task-58/dark-added.jpg)。
- 浅色 [失败保留](evidence/task-58/light-failure.jpg)、[保存](evidence/task-58/light-saved.jpg)、[取消后重开](evidence/task-58/light-cancel.jpg)、[滚动](evidence/task-58/light-scroll.jpg)、[移除](evidence/task-58/light-removed.jpg)、[重新添加](evidence/task-58/light-added.jpg)。

差异审查限本任务的实现、必要接线、独立测试及上述证据。状态为 **已完成**，下一步批次 59。

## 任务 59：桌面 OpenClaw 供应商编辑

2026-11-07。核对批次 58 提交 `9e183e69d` 和正式验收。母任务 1、3 已完成，母任务 2 按底座子计划 1–93 的既有开工裁定执行。沿用任务 36 的增量事务和任务 52 的显式原生窗口入口。

- 冻结来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7`。核对 `OpenClawFormFields`、`ProviderForm` 的 OpenClaw 分支、`useOpenclawFormState` 的模型与 User-Agent 保存逻辑及相邻表单测试。桌面在既有弹窗编辑单个 `models.providers` 节点，提供模型配置、API 密钥保留/替换/清除和浏览器 User-Agent。官方授权和模型发现暂不可用。
- `openclaw_form_{read,create,update}` 通过 SDK 调用 `POST /v1/app-providers/openclaw/{id}/form/read`、`POST /form` 与 `PATCH /form`。Gateway 按目标主机权限读写。执行端 `providers/openclaw_form.rs` 校验 JSON/JSONC，投影公开字段和公开修订，按模型 ID 恢复保留项。密钥和自定义请求头留在执行端。修改地址或协议时，执行端拒绝沿用受保护的值。托管或缺少节点的档案可编辑基础资料。
- 新建可同时添加到原生配置，编辑可同步现有成员。供应商列表复用任务 36 SDK 增量添加与移除，保留目录行。SDK → 回环 HTTP → 真实 Gateway → 临时 SQLite/HOME 测试确认，仅更新点名成员，保留其它供应商、注释、工作区字段、密钥与自定义请求头。非法模型、过期公开修订和权限拒绝均保留数据库与文件前像。仅保存目录时不写原生文件。
- 桌面聚焦用例使用假回环 HTTP 的 422 响应证明失败保留中文草稿，并验证上一代或其它执行主机的响应不会关闭正在保存的表单。

### 红绿验证

红绿验证动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。窗口阶段动态领取槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。两阶段的 `CARGO_BUILD_BUILD_DIR` 随各自槽位设置。进程退出后释放各自占用并保留缓存。

| 阶段 | 确切命令 | 退出码 | 有效结果 |
| --- | --- | --- | --- |
| 红 | `cargo fmt --all` | 0 | 格式化 |
| 红 | `cargo test -p vw-desktop --lib cc04_ui_openclaw_ -- --nocapture` | 101 | 2 项业务断言失败，草稿未发给 SDK，有效响应未结束保存 |
| 绿 | `cargo fmt --all` | 0 | 格式化 |
| 绿 | `cargo test -p vw-desktop --lib cc04_ui_openclaw_ -- --nocapture` | 0 | 2 passed |
| 绿 | `cargo test -p vw-agent --lib gateway::api::handlers::app_providers::openclaw_tests:: -- --nocapture` | 0 | 5 passed，含本任务 2 项 |
| 绿 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 targets/features |
| 窗口 | `SHIJU_CC04_WINDOW_CASE=openclaw SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并关闭 |
| 窗口 | `SHIJU_CC04_WINDOW_CASE=openclaw SHIJU_CC04_WINDOW_THEME=light cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并关闭 |

### macOS 原生窗口

macOS **27.0 (26A428) / arm64**，桌面 **0.6.0**。临时 app bundle 通过实际 Iced 组件、Message 与 SDK 调用隔离回环 HTTP 夹具。窗口夹具返回脱敏表单和目录契约，真实 Gateway 权限、事务和落盘结果由上表集成测试验证。PATH 未找到 OpenClaw CLI，真实安装版本、账号和 CLI 消费配置未验证。

两种主题均从列表打开已添加的中文档案，提交空模型 ID，确认服务拒绝且中文草稿仍显示。修正后保存并同步，输入新草稿后取消，再次打开确认仍显示已保存的模型。拖动长表单滚动条时标题和底部按钮固定，滚动条宽 4px，内容两侧各预留 7px。移除原生成员后档案仍在目录，重新添加后状态恢复。中文名称、备注和模型文本均实际绘制，窗口正常退出，无字形整形 panic。日志有既有链接提示、系统 sandbox extension 提示及深色窗口的一条 macOS 输入法端口提示。

- 深色 [失败保留](evidence/task-59/dark-failure.png)、[保存](evidence/task-59/dark-saved.png)、[取消前](evidence/task-59/dark-cancel-draft.png)、[取消后重开](evidence/task-59/dark-cancel.png)、[滚动](evidence/task-59/dark-scroll.png)、[移除](evidence/task-59/dark-removed.png)、[重新添加](evidence/task-59/dark-added.png)。
- 浅色 [失败保留](evidence/task-59/light-failure.png)、[保存](evidence/task-59/light-saved.png)、[取消前](evidence/task-59/light-cancel-draft.png)、[取消后重开](evidence/task-59/light-cancel.png)、[滚动](evidence/task-59/light-scroll.png)、[移除](evidence/task-59/light-removed.png)、[重新添加](evidence/task-59/light-added.png)。

差异审查限本任务实现、独立测试、必要接线及证据。状态为 **已完成**，下一步批次 60。

## 任务 60：桌面 Hermes 供应商编辑

2026-11-07。核对批次 59 提交 `a76f7f120` 及正式验收。母任务 1、3 已完成，母任务 2 沿用 `provider-contracts.md` 的底座子计划 1–93 开工裁定，完整领域迁移仍待收口。沿用批次 37 的 Hermes 增量事务与批次 52 的原生窗口入口。

- 核对冻结来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `HermesFormFields`、`ProviderForm` 的 Hermes 分支、`useHermesFormState` 和相邻 `HermesFormFields.test.tsx`。桌面复用供应商弹窗及设置控件，使用单节点 JSON 表单编辑 API 地址、协议、模型及请求间隔。模型上下文和图片输入标志随节点保存。官方授权和模型发现暂不可用。
- 桌面 `components/state/message/app_providers/hermes.rs` 接入新增、编辑、保存及列表成员操作。草稿原文经 SDK `hermes_form_read`、`hermes_form_create`、`hermes_form_update` 调用 `POST /v1/app-providers/hermes/{id}/form/read`、`POST /form`、`PATCH /form`。Gateway 按租户和目标主机检查读写权限，执行端 `providers/hermes/form.rs` 校验草稿、投影公开字段和修订。API 密钥支持保留、替换、清除，`key_env` 与未知字段留在执行端。涉及已保存凭据的地址或协议变更由执行端拒绝不安全的复用。托管或只读字典节点不可编辑。仅在原生文件存在的可写成员可直接同步编辑并建立目录档案。
- 新建可同时添加到原生文件。编辑可只保存目录或同步现有成员。列表使用既有 `hermes_provider_add` 和 `hermes_provider_remove_live`，移除已入目录的成员后保留档案。执行端复用批次 37 的 YAML 补丁、目录 CAS、文件前像校验及恢复。真实 SDK → 回环 HTTP → Gateway → 临时 SQLite/HOME 用例确认，`applied` 后只有指定成员改变，保留相邻节点、注释、根字段和受保护的配置。授权失败、非法模型和过期修订均保留目录及文件前像。

### 红绿验证

红绿阶段动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。窗口阶段动态领取槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。各阶段的 `CARGO_BUILD_BUILD_DIR` 与相应槽位一致。退出后释放本任务的锁并保留缓存。

| 阶段 | 确切命令 | 退出码 | 有效结果 |
| --- | --- | --- | --- |
| 红 | `cargo fmt --all` | 0 | 格式化 |
| 红 | `cargo test -p vw-desktop --lib cc04_ui_hermes_ -- --nocapture` | 101 | 两项业务断言失败，草稿未提交给 SDK，有效响应未结束保存 |
| 绿 | `cargo fmt --all` | 0 | 格式化 |
| 绿 | `cargo test -p vw-desktop --lib cc04_ui_hermes_ -- --nocapture` | 0 | 2 passed，服务拒绝保留草稿，旧代或其它主机响应被忽略 |
| 绿 | `cargo test -p vw-agent --lib gateway::api::handlers::app_providers::hermes_tests:: -- --nocapture` | 0 | 8 passed，含本任务 3 项 |
| 绿 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 targets/features |
| 窗口 | `SHIJU_CC04_WINDOW_CASE=hermes SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并关闭 |
| 窗口 | `SHIJU_CC04_WINDOW_CASE=hermes SHIJU_CC04_WINDOW_THEME=light cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并关闭 |

### macOS 原生窗口

macOS **27.0 (26A428) / arm64**，桌面 **0.6.0**。临时 app bundle 通过真实 Iced 组件、Message 与 SDK 调用使用假数据的回环 HTTP 夹具。上表的 Gateway 集成用例另行验证真实授权、目录和 YAML 写入。PATH 未发现 Hermes CLI，真实安装版本、账号及 CLI 消费配置未验证。

两种主题均从供应商列表打开已添加的中文档案，提交空模型 ID 后收到 422，中文草稿保留。修正后保存并同步，修改模型后取消，再打开仍显示已保存的模型。实际拖动 4px 滚动条，内容两侧各留 7px，标题及底部操作固定。移除成员后档案仍在目录，重新添加后状态恢复。中文动态模型、名称和备注均正常绘制，窗口正常退出，无字形整形 panic。日志含既有链接提示和系统 sandbox extension 提示，深色窗口另有一条 macOS 输入法端口提示。

- 深色 [校验失败](evidence/task-60/dark-failure.png)、[保存同步](evidence/task-60/dark-saved.png)、[取消后重开](evidence/task-60/dark-cancel.png)、[滚动](evidence/task-60/dark-scroll.png)、[移除](evidence/task-60/dark-removed.png)、[重新添加](evidence/task-60/dark-added.png)。
- 浅色 [校验失败](evidence/task-60/light-failure.png)、[保存同步](evidence/task-60/light-saved.png)、[取消后重开](evidence/task-60/light-cancel.png)、[滚动](evidence/task-60/light-scroll.png)、[移除](evidence/task-60/light-removed.png)、[重新添加](evidence/task-60/light-added.png)。

差异审查限本任务实现、必要接线、独立测试及证据。状态为 **已完成**，下一步批次 61。

## 任务 61：桌面 Pi 供应商编辑

2026-11-07。核对批次 60 提交 `1f2b2ba81` 和正式验收。母任务 1、3 已完成；母任务 2 沿用底座子计划 1–93 的既有开工裁定，完整领域迁移仍待收口。复用批次 38 的 Pi 增量事务和批次 52 的原生窗口入口。

- 来源快照 `ca58c0313360eae7c851fbd7553c1614c7af03e7`：核对 `PiProviderForm`、`buildPiSettingsConfig`、`ProviderForm` 的 Pi 分支及相邻表单测试。桌面 `components/state/message/app_providers/pi.rs` 复用统一弹窗，编辑 `models.json` 中单个 `providers` 节点。新增需要地址、协议和模型，既有原生覆盖可保留缺省字段。官方授权和模型发现暂不可用，端点与请求选项归批次 68。
- `pi_form_read/create/update` 通过 SDK 调用 `POST /v1/app-providers/pi/{id}/form/read`、`POST /form` 与 `PATCH /form`。Gateway 校验租户、主机和读写权限，执行端 `providers/pi/form.rs` 校验草稿并比较公开修订。API 密钥可保留、替换或清除；拒绝可执行的密钥输入。请求头、兼容选项与未投影的模型字段留在执行端。同步时保留现有原生凭据，拒绝将其沿用到变更后的传输配置。
- 新建可同时加入 Pi；编辑可仅保存目录或同步现有成员。列表复用 `pi_provider_add` 和 `pi_provider_remove_live`。移除保留档案及展示名，原生节点中的独立名称保留在声明中。真实 SDK → 回环 HTTP → Gateway → 临时 SQLite/HOME 用例验证：仅改动指定节点，保留其它成员、根字段、注释、设置与登录文件。无效、过期或无权的请求保留目录和文件前像。

### 红绿验证

红测和首次绿测使用动态槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。最终桌面聚焦与窗口用槽位 **4**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-4`。最终 Gateway 用槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。最终 Clippy 回到槽位 **2**。各阶段的 `CARGO_BUILD_BUILD_DIR` 同相应槽位，进程退出后释放自己的锁，保留缓存。

| 阶段 | 确切命令 | 退出码 | 有效结果 |
| --- | --- | --- | --- |
| 红 | `cargo fmt --all` | 0 | 格式化 |
| 红 | `cargo test -p vw-desktop --lib cc04_ui_pi_ -- --nocapture` | 101 | 2 项业务断言失败：草稿未提交给 SDK，有效响应未结束保存 |
| 绿 | `cargo fmt --all` | 0 | 格式化 |
| 绿 | `cargo test -p vw-desktop --lib cc04_ui_pi_ -- --nocapture` | 0 | 2 passed：失败保留草稿，忽略过期响应 |
| 绿 | `cargo test -p vw-agent --lib gateway::api::handlers::app_providers::pi_tests:: -- --nocapture` | 0 | 8 passed，含本任务 3 项 |
| 绿 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 targets/features |
| 窗口 | `SHIJU_CC04_WINDOW_CASE=pi SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并关闭 |
| 窗口 | `SHIJU_CC04_WINDOW_CASE=pi SHIJU_CC04_WINDOW_THEME=light cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并关闭 |

首次绿测时工作区并行改动曾导致 `point_below_tests.rs` 缺 `capture_keyboard` 字段，重试已通过。首次 Gateway 测试的测试夹具存在借用冲突，修正后重跑。槽位 1 随后的 Clippy 读到了缺 Pi 表单类型的依赖产物，退出 101；使用槽位 2 重跑成功。

### macOS 原生窗口

macOS **27.0 (26A428) / arm64**，桌面 **0.6.0**。临时 app bundle 经真实 Iced 组件、Message 和 SDK 连接隔离回环 HTTP 夹具；真实权限、目录及原生 JSONC 文件由 Gateway 集成测试验证。PATH 未找到 Pi CLI，未验证真实安装、账号或 CLI 消费配置。

两种主题均从供应商列表打开已加入的中文档案，提交空模型 ID 后收到 422 并保留草稿；修正后保存和同步，再修改草稿并取消，重新打开确认已保存值仍在。实际拖动长表单的 4px 滚动条，内容两侧各留 7px，标题及底部按钮固定。移除后仍可看到档案，重新加入后恢复成员。中文动态文本实际绘制，无字形整形 panic。日志含既有链接提示和 macOS sandbox extension 提示，深色窗口另有一条输入法端口提示。

- 深色 [校验失败](evidence/task-61/dark-failure.jpg)、[保存同步](evidence/task-61/dark-saved.jpg)、[取消前](evidence/task-61/dark-cancel-draft.jpg)、[取消后重开](evidence/task-61/dark-cancel.jpg)、[滚动](evidence/task-61/dark-scroll.jpg)、[移除](evidence/task-61/dark-removed.jpg)、[重新添加](evidence/task-61/dark-added.jpg)。
- 浅色 [校验失败](evidence/task-61/light-failure.jpg)、[保存同步](evidence/task-61/light-saved.jpg)、[取消前](evidence/task-61/light-cancel-draft.jpg)、[取消后重开](evidence/task-61/light-cancel.jpg)、[滚动](evidence/task-61/light-scroll.jpg)、[移除](evidence/task-61/light-removed.jpg)、[重新添加](evidence/task-61/light-added.jpg)。

差异审查限本任务的实现、独立测试、必要接线及上述证据。状态为 **已完成**，下一步批次 62。

## 任务 62：桌面 OMP 供应商编辑

2026-11-07。核对批次 61 提交 `d621fc325` 及正式验收。母任务 1、3 已完成，母任务 2 沿用 `provider-contracts.md` 的底座子计划 1–93 开工裁定。复用批次 39 的 OMP 增量事务和批次 52 的原生窗口入口。

- 核对冻结来源 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `PiProviderForm`、`handleOmpTextChange`、`ProviderForm` 的 OMP 分支，以及 `PiProviderForm.test.tsx` 的 OMP 作用域与发现配置用例。桌面 `components/state/message/app_providers/omp.rs` 复用统一弹窗，编辑 `models.yml` 中由当前供应商标识确定的单个 YAML 节点。可修改地址、协议、认证类型、模型和发现配置。官方授权及桌面模型发现请求暂不可用。
- `OmpFormWrite/View` 通过 SDK `omp_form_read/create/update` 调用 `POST /v1/app-providers/omp/{id}/form/read`、`POST /form`、`PATCH /form`。Gateway 先检查租户、执行主机及读写权限。执行端 `providers/omp/form.rs` 校验 YAML 作用域、字段和公开修订，合并原节点中保留的字段。根 API 密钥通过单独的保留、替换、清除操作处理。地址、协议、认证或发现配置变更受保留凭据约束，原请求头和兼容选项留在执行端。
- 新增可同时加入，编辑可只存目录或同步现有成员。列表复用 `omp_provider_add`、`omp_provider_remove_live`，移除保留档案与展示名。真实 SDK → 回环 HTTP → Gateway → 临时 SQLite 和隔离 HOME 验证了 200 保存及应用、422 拒绝无效 YAML 或不安全变更、409 拒绝过期修订，以及授权拒绝。仅目标成员变化，其他成员、根字段、外部注释、设置与登录文件保留。桌面测试通过回环夹具注入 422，验证草稿保持和旧作用域或请求代次不会覆盖当前草稿。

### 自动验证

各阶段原子领取槽位并记录任务、会话和 PID，`CARGO_BUILD_BUILD_DIR` 同各阶段的绝对 `CARGO_TARGET_DIR`。

| 槽位 | CARGO_TARGET_DIR | 用途 |
|---|---|---|
| 1 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` | 首轮红测尝试、最终桌面聚焦及窗口 |
| 2 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-2` | 有效红测、最终 Gateway |
| 4 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-4` | 首轮绿测及最终严格 Clippy |

| 阶段 | 命令 | 退出码 | 行为证据 |
|---|---|---:|---|
| 红 | `cargo test -p vw-desktop --lib cc04_ui_omp_ -- --nocapture` | 101 | 0 passed / 2 failed，失败在草稿未提交给 SDK、有效响应未结束保存 |
| 格式化 | `cargo fmt --all` | 0 | 格式化检查步骤 |
| 绿 | `cargo test -p vw-desktop --lib cc04_ui_omp_ -- --nocapture` | 0 | 2 passed，失败保留草稿、忽略过期响应 |
| 绿 | `cargo test -p vw-agent --lib gateway::api::handlers::app_providers::omp_tests:: -- --nocapture` | 0 | 7 passed，含本任务新增 2 项 |
| 静态检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 targets/features |
| 深色 | `SHIJU_CC04_WINDOW_CASE=omp SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作后正常退出 |
| 浅色 | `SHIJU_CC04_WINDOW_CASE=omp SHIJU_CC04_WINDOW_THEME=light cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作后正常退出 |

槽位 1 首轮红测和窗口尝试遇到旧依赖产物，退出 101。有效红测由槽位 2 获得，窗口重试刷新了 `iced_term` 依赖。首次 Gateway 编译的 OMP 类型导入错误及首次 Clippy 检出的 OMP 保存响应接线错误已修正。替换前的窗口编译主动中断并释放槽位，退出 143。各阶段退出后释放自身锁，保留编译缓存。

### macOS 原生窗口

macOS **27.0 (26A428) / arm64**，桌面 **0.6.0**。临时 app bundle 运行真实 Iced 组件、Message 和 SDK，连接使用假档案的隔离回环 HTTP 夹具。真实 Gateway 授权、事务与 YAML 落盘由集成用例验证。PATH 未找到 OMP CLI，真实安装、账号及 CLI 消费配置未验证。

两种主题均从列表加入假档案，进入编辑，拖动长表单的 4px 滚动条。内容两侧各留 7px，标题及底部按钮固定。提交空模型 ID 收到 422 并保留中文草稿，修正后保存并同步。再次修改后取消，重开保留已保存的中文模型，再从 OMP 移除成员，档案仍在列表。中文动态文本实际绘制，无字形整形 panic。深色首次操作遇到窗口点击定位错误，重启后完成同一操作序列。浅色补拍时并行改动一度导致 `app_init/new/settings.rs` 编译错误，退出 101。改用同槽位已通过窗口命令构建的测试二进制补拍，退出 0。日志有既有链接器、macOS sandbox extension 和输入法端口提示。

- 深色 [滚动](evidence/task-62/dark-scroll.jpg)、[校验失败](evidence/task-62/dark-failure.jpg)、[保存同步](evidence/task-62/dark-saved.jpg)、[取消前](evidence/task-62/dark-cancel-draft.jpg)、[取消后重开](evidence/task-62/dark-cancel-reopened.jpg)、[加入](evidence/task-62/dark-added.jpg)、[移除](evidence/task-62/dark-removed.jpg)。
- 浅色 [滚动](evidence/task-62/light-scroll.jpg)、[校验失败](evidence/task-62/light-failure.jpg)、[保存同步](evidence/task-62/light-saved.jpg)、[取消前](evidence/task-62/light-cancel-draft.jpg)、[取消后重开](evidence/task-62/light-cancel-reopened.jpg)、[加入](evidence/task-62/light-added.jpg)、[移除](evidence/task-62/light-removed.jpg)。

状态 **已完成**。差异审查限于本任务实现、测试、接线和证据。下一步批次 63。

## 任务 63：桌面 Mcode 供应商编辑

2026-11-07。核对批次 62 提交 `e5f875f67` 和正式验收。母任务 1、3 已完成；母任务 2 沿用 provider-contracts.md 的底座子计划 1–93 开工裁定，完整领域迁移未收口。复用批次 40 的 Mcode 增量事务和批次 52 的原生窗口入口；所有 plan.md 保持只读。

- 来源为冻结快照 `ca58c0313360eae7c851fbd7553c1614c7af03e7` 的 `McodeProviderForm`、`ProviderForm` Mcode 分支及 `McodeProviderForm.test.tsx` 的原生字段、未知选项保留、失败草稿和标识锁定用例。
- 桌面 `components/state/message/app_providers/mcode.rs` 接入统一弹窗和列表成员操作。用户编辑 `config.yaml` 中单个 `custom_provider` 节点的 YAML；名称与基础资料、API、地址、模型及原生字段通过 SDK 发送，Iced 不实现 YAML 或事务策略。官方授权、模型发现暂不可用。
- `McodeFormWrite/View`、`GatewayClient::mcode_form_read/create/update` 对应 `POST /v1/app-providers/mcode/{id}/form/read`、`POST /form`、`PATCH /form`。执行端 `providers/mcode_form.rs` 投影公开字段，隐藏值以原位置保留标记表示，API 密钥独立 Keep/Replace/Clear；Mcode 的必需密钥约束仍由原校验器拒绝无效 Clear。公开目录与原生节点修订共同参与陈旧检测，哈希不包含秘密值。保存时还原隐藏值、检验保留凭据的地址/协议变更、校验原生结构，并复用现有授权、存储 CAS、文件锁和事务。
- 新增可只存目录或同时加入；编辑可只存目录或同步现有成员。列表调用既有 add/remove-live，移除仍保留档案和展示名。非自定义原生节点、托管授权、冲突和无权限路径明确拒绝。
- 回环 HTTP → 真实 SDK/Gateway → 临时 SQLite 与隔离 HOME 验证了保存及同步、凭据和请求头保留、中文模型变更、其他成员/注释/设置/登录文件不变、422 无效 YAML、409 陈旧修订、授权拒绝。桌面回环夹具仅替代执行端边界，返回 422 或 catalogSaved/live 形状，证明失败保留草稿、过期作用域和代次不覆盖当前表单。

### 自动验证

各阶段原子领取共享槽位，记录计划、任务、会话、PID；`CARGO_BUILD_BUILD_DIR` 与绝对 `CARGO_TARGET_DIR` 相同，退出后释放自己的锁并保留缓存。

| 槽位 | CARGO_TARGET_DIR | 用途 |
|---|---|---|
| 1 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` | 深色窗口、9 项 Gateway 与桌面复跑、首次 Clippy |
| 2 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-2` | 基线、红测、首轮绿测与 Gateway、浅色窗口、最终聚焦验证及 Clippy |

| 阶段 | 命令 | 退出码 | 有效数量/结果 |
|---|---|---:|---|
| 基线 | `cargo test -p vw-desktop --lib cc04_ui_mcode_ -- --nocapture` | 0 | 0 项，不计验收 |
| 红 | 同上 | 101 | 0 passed / 2 failed，保存未提交给 SDK、有效响应未结束保存 |
| 绿 | 同上 | 0 | 2 项 |
| 格式化 | `cargo fmt --all` | 0 | 完成 |
| Gateway | `cargo test -p vw-agent --lib gateway::api::handlers::app_providers::mcode_tests:: -- --nocapture` | 0 | 9 项，含新增 2 项 |
| Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 targets/features |
| 深色 | `SHIJU_CC04_WINDOW_CASE=mcode SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作后关闭 |
| 浅色 | `SHIJU_CC04_WINDOW_CASE=mcode SHIJU_CC04_WINDOW_THEME=light cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作后关闭 |

Gateway 首轮 8 passed / 1 failed（101）：既有原生账号用例检出非自定义节点保护回归，恢复 Mcode 的 key_exists 保护后重跑。首次严格 Clippy 退出 101，检出新增分支的 collapsible_if；修正后重新格式化，运行 `cargo test -p vw-agent --lib cc04_ui_mcode_ -- --nocapture`（2 passed，退出 0）及桌面上述两项过滤测试（2 passed，退出 0），最终 Clippy 退出 0。测试链接器有既有 unwind 提示；自身锁已释放，缓存保留。

### macOS 原生窗口

macOS **27.0（26A428）/ arm64**，桌面 **0.6.0**。临时 app bundle 运行真实 Iced、Message 和 SDK，连接假档案回环 HTTP 夹具；不改真实用户配置。真实 Gateway 授权、事务及 YAML 落盘由上述集成测试验证。PATH 未找到 Mcode CLI，真实安装、账号和 CLI 消费配置未验证。

明暗主题均完成：列表加入 → 编辑并拖动长表单 → 提交空 models 得到 422 且保留中文草稿 → 修正后保存并同步 → 修改中文名称后取消 → 重开显示原已存名称 → 移除后档案保留。真实绘制中文动态模型、表单和 toast，未发生字形整形 panic；使用默认字体，滚动条 4px、内容两侧各留 7px，标题与底部按钮固定。窗口正常关闭且 Cargo 退出 0；日志仅有既有 macOS sandbox extension/输入法端口提示。

- 深色：[加入](evidence/task-63/dark-added.png)、[滚动](evidence/task-63/dark-scroll.png)、[失败](evidence/task-63/dark-failure.png)、[保存](evidence/task-63/dark-saved.png)、[取消前](evidence/task-63/dark-cancel-draft.png)、[重开](evidence/task-63/dark-cancel-reopened.png)、[移除](evidence/task-63/dark-removed.png)。
- 浅色：[加入](evidence/task-63/light-added.png)、[滚动](evidence/task-63/light-scroll.png)、[失败](evidence/task-63/light-failure.png)、[保存](evidence/task-63/light-saved.png)、[取消前](evidence/task-63/light-cancel-draft.png)、[重开](evidence/task-63/light-cancel-reopened.png)、[移除](evidence/task-63/light-removed.png)。

状态：**已完成**。只提交本任务实现、测试、必要接线和证据，排除其他会话的设置/终端变更和编号任务删除。下一步批次 64。

## 任务 64：桌面 Kimi 供应商编辑

2026-11-07。已核对批次 63 提交 `eccf7557a` 及上述正式验收。母任务 1、3 已完成；母任务 2 沿用 provider-contracts.md 的底座子计划 1–93 开工裁定，完整领域迁移仍未收口。复用批次 41 的 Kimi 增量事务和批次 52 的原生窗口入口；所有 plan.md 保持只读。

- 来源 `/Users/xiangmin/worker/code/cc-switch` HEAD 为冻结快照 `ca58c0313360eae7c851fbd7553c1614c7af03e7`，工作区干净。阅读 `KimiProviderForm`、`ProviderForm` 的 Kimi 分支及相邻测试的 TOML 编辑、非法作用域、模型上下文、官方账号连接保护行为。RustCodeGraph status/explore 确认现有原生调用链。
- 新增桌面 `components/state/message/app_providers/kimi.rs`，接入统一编辑弹窗、Message 分发与列表成员操作。草稿是单个 `provider` 与关联 `models` 的 TOML 声明；Iced 不解析 TOML、不决定事务策略。模型标识、显示名、上下文和原生扩展交由执行端校验。官方授权、模型发现明确暂不可用；官方/托管节点不作为可编辑连接回填。
- `KimiFormWrite/View/Config` 与 SDK `kimi_form_read/create/update` 对应 `POST /v1/app-providers/kimi/{id}/form/read`、`POST /form`、`PATCH /form`。SDK 保留 Kimi 原有身份编码与敏感凭据头处理。执行端 `providers/kimi_form.rs` 负责脱敏投影、原位置隐藏值还原、公开目录及原生成员修订、Keep/Replace/Clear、保留凭据时的地址/协议变更拒绝；秘密值不参与返回的修订哈希。复用既有身份/原生校验、主机授权、存储 CAS、文件锁和事务所有者。
- 新建可只存目录或同时加入；编辑可只存目录或同步现有成员。列表增删消费原 SDK；移除后目录和展示名保留。既有原生默认模型、关联模型及官方账号连接保护继续生效。
- 回环 HTTP → 真实 SDK/Gateway → 临时 SQLite 与隔离 HOME，验证了更新同步、中文模型/基础资料、凭据及请求头保留、其他供应商/注释/默认模型/登录文件不变、422 无效 TOML、409 陈旧修订、无权限与地址变更拒绝，以及仅存目录不落原生文件。桌面单测的回环服务器仅替代执行端网络边界，接收真实 SDK 请求并返回 422，证明失败保留草稿和代次/主机过期响应被忽略。

### 自动验证

每阶段原子 mkdir 动态领取仓库共享槽位，记录任务/会话/PID；`CARGO_BUILD_BUILD_DIR` 与绝对 `CARGO_TARGET_DIR` 相同。Cargo 和窗口进程退出后释放自身锁，缓存保留。

| 槽位 | CARGO_TARGET_DIR | 阶段 |
|---|---|---|
| 1 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` | 红测、格式化、绿测、Gateway、Clippy |
| 2 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-2` | 窗口首次编译、补依赖后的格式化、深色与浅色窗口 |

| 阶段 | 命令 | 退出码 | 有效数量/结果 |
|---|---|---:|---|
| 红 | `cargo test -p vw-desktop --lib cc04_ui_kimi_ -- --nocapture` | 101 | 0 passed / 2 failed，业务断言：草稿未送达 SDK、有效响应未结束保存 |
| 格式化 | `cargo fmt --all` | 0 | 实现后与补窗口测试依赖后均完成 |
| 绿 | `cargo test -p vw-desktop --lib cc04_ui_kimi_ -- --nocapture` | 0 | 2 passed |
| Gateway | `cargo test -p vw-agent --lib gateway::api::handlers::app_providers::kimi_tests:: -- --nocapture` | 0 | 8 passed，含新增 2 项 |
| Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 targets/features |
| 深色 | `SHIJU_CC04_WINDOW_CASE=kimi SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作后关闭 |
| 浅色 | `SHIJU_CC04_WINDOW_CASE=kimi SHIJU_CC04_WINDOW_THEME=light cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作后关闭 |

首次深色窗口编译退出 101：夹具使用 TOML 但桌面未声明该测试依赖；仅补 `dev-dependencies` 中的既有 toml 0.9 及锁文件后，格式化并重跑成功。生产 Iced 路径没有 TOML 解析。测试链接器有既有 unwind 提示，严格 Clippy 退出 0。未执行全量单测、fmt check、wasm 或 Makefile。

### macOS 原生窗口

macOS **27.0（26A428）/ arm64**，桌面 **0.6.0**。显式窗口测试通过临时 app bundle runner 运行真实 Iced、Message 和 SDK，连接隔离假数据 HTTP 夹具，不改真实用户配置。真实授权、原生 TOML 落盘和事务由上述 Gateway 集成测试证明。PATH 未找到 Kimi CLI，真实安装、账号与 CLI 消费配置未验证。

明暗主题分别完成：为中文供应商添加成员 → 打开编辑并拖动长表单 → 提交空 models 返回 422，中文草稿保留 → 修正中文动态模型后保存并同步 → 修改中文名称后取消 → 重开确认原名称 → 移除后档案保留。中文动态文本和长表单实际绘制，未发生字形整形 panic；默认字体、统一输入/按钮样式、4px 滚动条和内容两侧 7px 留白，标题与操作区固定。两个窗口均手动关闭、进程退出 0；日志仅出现既有 macOS sandbox extension/输入法端口提示。

- 深色：[添加](evidence/task-64/dark-added.png)、[滚动](evidence/task-64/dark-scroll.png)、[失败](evidence/task-64/dark-failure.png)、[保存](evidence/task-64/dark-saved.png)、[取消草稿](evidence/task-64/dark-cancel-draft.png)、[重开](evidence/task-64/dark-cancel-reopened.png)、[移除](evidence/task-64/dark-removed.png)。
- 浅色：[添加](evidence/task-64/light-added.png)、[滚动](evidence/task-64/light-scroll.png)、[失败](evidence/task-64/light-failure.png)、[保存](evidence/task-64/light-saved.png)、[取消草稿](evidence/task-64/light-cancel-draft.png)、[重开](evidence/task-64/light-cancel-reopened.png)、[移除](evidence/task-64/light-removed.png)。

状态：**已完成**。差异审查限于本任务代码、独立测试、必要接线、测试依赖与正式证据；排除其他会话设置/终端变更、其它计划及编号任务删除。自身槽位 1、2 锁已释放，缓存保留。下一步批次 65。

## 任务 65：桌面 Dsh 供应商编辑

2026-11-07。批次 64 提交 `4e32b597d` 与上节正式验收已核对。母任务 1、3 已完成；母任务 2 沿用底座子计划 1–93 开工裁定，不冒称完整迁移完成。本任务初次登记缺少原生读取/移除依赖，继续指令明确授权在 Dsh 领域所有者补齐这些最小能力；所有 plan.md 保持只读。

### 实现与边界

- 来源冻结快照 `ca58c0313360eae7c851fbd7553c1614c7af03e7`，工作区干净。阅读 `DshAccountCard`/`DshAccountLogin`、ProviderList/AuthCenterPanel 调用者和相邻账号卡测试；官方账号卡不作为可编辑供应商。原生编辑/移除沿 `dsh_config::{save_and_commit,remove_provider,provider_key_exists}` 和批次 42 的真实 profile 集合、凭据引用、多目录事务。
- 新增 `components/state/message/app_providers/dsh.rs`，接入既有列表、编辑弹窗、Message 和 SDK；Iced 只维护 YAML 草稿，不解析原生配置、不实现事务策略。支持新建、编辑、保存到目录或同步现有成员、添加与移除；官方授权和模型发现明确暂不可用。失败保留草稿；主机作用域/代次过期响应被忽略。
- 新增 `DshFormWrite/View/Config`，SDK `dsh_form_read/create/update` 对应 `POST /v1/app-providers/dsh/{id}/form/read`、`POST/PATCH /form`；`dsh_provider_remove_live` 对应 `POST /remove-live`。沿用敏感凭据头、租户/主机读写权限和错误白名单；不添加客户端路径、环境或任意命令参数。
- `providers/dsh_edit.rs` 在执行端把原生 models 数组、安全容量字段与供应商节点转换为编辑声明，核对反向投影一致性；不同 profile 同名节点分歧、不支持的原生布局或凭据引用明确拒绝。`dsh_form.rs` 仅投影安全可编辑字段，API 密钥不出站，apiKeyEnv/请求头/未知扩展以原位置保留标记回填；支持 Keep/Replace/Clear，未满足必需凭据的 Clear 仍由原校验拒绝。公开修订不哈希秘密值；写入时重新读取锁内原生节点并还原最新隐藏值，保留凭据时拒绝地址/协议变更。
- 移除只补丁选中成员，保留其它节点、插件、周围注释、共享凭据 refs 与原生账号/device records。保留目录档案及展示名，允许重新添加。存在默认选择/引用、动态标签等无法安全确认的配置时拒绝移除；不重写默认字段或擅自清理共享凭据。既有文件锁、目录 CAS、前像复核、多文件 pending/恢复继续由 `DshFiles` 与 `ProviderTransactions` 负责。
- Gateway 新用例使用真实回环 HTTP、正式 SDK/handler、临时 SQLite 和隔离 HOME：中文模型更新同步、凭据/请求头/未知字段保留且不出站、陈旧修订和非法草稿拒绝、原生独有节点读取、只读权限允许读取但拒绝写入/移除、移除保留兄弟成员/凭据/档案且可重新添加。无真实用户配置或凭据参与。

### 自动验证

各阶段原子 mkdir 动态领取仓库共享槽位，记录计划、任务、会话和 PID；绝对 `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 相同。Cargo、窗口及子进程结束后清理自身锁，保留缓存。

| 槽位 | CARGO_TARGET_DIR | 阶段 |
|---|---|---|
| 1 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` | 红测、格式化、桌面绿测、执行端/Gateway、Clippy |
| 2 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-2` | 深色原生窗口 |
| 5 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-5` | 浅色原生窗口 |

| 命令 | 退出码 | 有效数量/证据 |
|---|---:|---|
| `cargo test -p vw-desktop --lib cc04_ui_dsh_ -- --nocapture`（红） | 101 | 0 passed / 2 failed：草稿未交给 SDK、有效响应未结束保存 |
| 同上（绿） | 0 | 2 passed，失败保留草稿与陈旧响应隔离 |
| `cargo fmt --all` | 0 | 格式化，无全量 fmt check |
| `cargo test -p vw-agent-apps --lib providers::dsh_tests:: -- --nocapture` | 0 | 8 passed，含新增 2 项及既有多 profile/恢复/外部改动用例 |
| `cargo test -p vw-agent --lib cc04_dsh_ -- --nocapture` | 0 | 既有 3 项回归 |
| `cargo test -p vw-agent --lib gateway::api::handlers::app_providers::dsh_tests:: -- --nocapture` | 0 | 5 passed，含新增 2 项；与上行去重 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全 targets/features，不计行为测试 |
| `SHIJU_CC04_WINDOW_CASE=dsh SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并手动关闭 |
| `SHIJU_CC04_WINDOW_CASE=dsh SHIJU_CC04_WINDOW_THEME=light cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并手动关闭 |

### macOS 原生窗口

macOS **27.0（26A428）/ arm64**，桌面 **0.6.0**。`sw_vers`/`uname -m` 退出 0。PATH 未找到 Dsh CLI（`command -v dsh` 退出 1），真实 Dsh 安装版本、账号登录和应用消费未验证；不将窗口夹具等同真实上游运行。Windows/Linux、其它架构和 PostgreSQL 实际服务未验收。

显式窗口入口的 `dsh` case 使用临时 app bundle runner，真实 Iced/Message/SDK 接假档案回环 HTTP；执行端真实落盘由上述 Gateway 测试证明。明暗主题分别完成：列表添加中文供应商 → 打开编辑、拖动长表单 → 空 models 返回 422、保留中文失败草稿 → 修正中文动态模型、保存并同步 → 修改中文名称后取消 → 重开显示原名称 → 移除后档案仍在列表。统一弹窗/输入/按钮/toast，4px 滚动条、内容两侧各 7px 留白、固定标题和操作区；中文及动态文本使用默认字体，实际整形无 panic。两个窗口正常关闭、Cargo 退出 0；仅出现既有 macOS sandbox extension 提示，测试链接器有既有 unwind 提示。

- 深色：[添加](evidence/task-65/dark-added.png)、[长表单](evidence/task-65/dark-scroll.png)、[失败](evidence/task-65/dark-failure.png)、[保存](evidence/task-65/dark-saved.png)、[取消草稿](evidence/task-65/dark-cancel-draft.png)、[重开](evidence/task-65/dark-cancel-reopened.png)、[移除](evidence/task-65/dark-removed.png)。
- 浅色：[添加](evidence/task-65/light-added.png)、[长表单](evidence/task-65/light-scroll.png)、[失败](evidence/task-65/light-failure.png)、[保存](evidence/task-65/light-saved.png)、[取消草稿](evidence/task-65/light-cancel-draft.png)、[重开](evidence/task-65/light-cancel-reopened.png)、[移除](evidence/task-65/light-removed.png)。

状态：**已完成**。最终格式化后桌面 2 项复验与严格 Clippy 均退出 0，去重共 15 项行为测试通过。自身槽位 1、2、5 占用记录已清理，缓存保留；定向差异检查通过，只读 plan.md 无差异。只提交本任务领域实现、桌面接线、独立测试和正式证据，排除其它会话设置/终端变更与编号任务删除。未执行全量单元测试、fmt check、wasm 或 Makefile。下一步批次 66。

## 任务 66：桌面 Agy 供应商编辑

2026-11-07。核对批次 65 提交 `e8070f531` 与正式证据；母任务 1、3 已完成，母任务 2 沿用底座子计划 1–93 开工裁定，不冒称整体完成。继续指令明确沿任务 43 实现目录档案新增/编辑/删除，配置在下次授权终端启动投影至持久私有 profile；全局 current、live 成员操作不适用，OAuth 账号增删不纳入。所有 plan.md 保持只读。

### 实现与边界

- 冻结来源 HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`，工作区干净。阅读 AgyOAuthSection/AgyAccountSelect、ProviderForm 直接调用点、相邻 AgyOAuthSection/ProviderForm.agyLogin 测试和 `agy_direct::{is_official,live_settings,terminal_launch,key_resume_target}`；沿任务 43 的 `prepare_checked` 持久 profile 与进程租约，不新增全局启用语义。
- 新增桌面 components/state/message 的 `app_providers/agy.rs`，接既有列表、统一编辑弹窗、Message 与正式 SDK。编辑模型显示名、可选端点、基础资料和秘密三态；失败保留草稿，主机作用域/代次过期响应被忽略。保存提示下次授权终端启动生效；全局切换、live 成员、官方授权、模型发现明确不可用，复制按钮禁用。取消不提交；删除使用专属安全接口。
- 新增 AgyFormView/AgyFormWrite；`POST /v1/app-providers/agy/{id}/form/read`、`POST/PATCH /form` 和 `POST /form/delete` 对应 SDK `agy_form_read/create/update/delete`。鉴权沿敏感 token 头和租户/主机读写权限，客户端没有路径、命令或环境参数。只读权限可读取，写入/删除拒绝。非 macOS 返回不支持。
- 表单读取不返回秘密，公开修订只摘要基础字段、模型、地址和秘密存在位，不哈希秘密内容。更新用公开修订校验及数据库 CAS；Keep 读取执行端最新密钥，地址变化不得沿用旧密钥；Replace 拒绝掩码/占位符；Clear 在自定义地址仍存在时拒绝，清空地址后可使用已有原生登录，不发起授权。只更新所属字段，保留未知顶层/配置/env 字段和展示元数据；不支持的托管绑定、请求覆盖或原生形状明确拒绝。
- Agy 启动持有授权私有根下的独立 0600 目录记录共享 flock，覆盖 API profile 和原生登录；删除取得独占租约后检查记录和未知引用元数据，以完整前像条件删除，避免删除更新后的记录。锁 no-follow/所有者/权限/硬链接检查；租约保留至实际子进程回收，异常退出随句柄释放。没有削弱通用删除 guard；没有删除持久 profile、会话、原生登录或 config 链接。保护范围是本执行端授权私有根内的 Agy 启动；不声称监测外部手工启动或其它主机进程。
- 真实回环 HTTP + 正式 SDK/handler + 临时 SQLite + 隔离 HOME 验证保存后配置未变、下一次受控 Agy PTY 进程启动时模型更新、已有会话/登录保留、运行时删除拒绝、退出后只删目录记录、非法 Clear/端点重定向/旧修订无目录配置副作用及权限拒绝。只替代外部 Agy 程序和终端连接边界，真实文件/存储/权限/进程生命周期未 mock；凭据均为假值。

### 验证记录

每阶段原子 mkdir 动态领取共享槽位，记录任务/会话/PID；`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均指向领取的绝对目录。Cargo、窗口与子进程结束后释放自身锁并保留缓存。

| 槽位 | CARGO_TARGET_DIR | 阶段 |
|---|---|---|
| 1 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` | 红测、初轮绿测、执行端/API 回归、浅色窗口 |
| 2 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-2` | 深色窗口、最终回归与 Clippy |

- `cargo test -p vw-desktop --lib cc04_ui_agy_ -- --nocapture`：红测退出 101，0 passed / 2 failed，业务断言为草稿未交 SDK、有效响应未结束保存；接线后退出 0，2 passed。
- 初轮 `cargo test -p vw-agent --lib cc04_agy_ -- --nocapture` 退出 0，2 passed；`cargo test -p vw-agent-apps --lib cc04_agy_ -- --nocapture` 退出 0，6 passed。第一次执行端编译发现本次新增锁名摘要 LowerHex 不受支持，改用既有 base64 编码后通过；该编译失败不计红测。
- `cargo fmt --all` 退出 0；最终 `cargo test -p vw-agent --lib gateway::api::handlers::agent_apps::launch::agy_tests:: -- --nocapture` 退出 0、3 passed；`cargo test -p vw-agent-apps --lib cc04_agy_ -- --nocapture` 退出 0、6 passed；`cargo test -p vw-desktop --lib cc04_ui_agy_ -- --nocapture` 退出 0、2 passed，去重共 11 项。
- `cargo clippy --all-targets --all-features -- -D warnings` 两次均退出 101：首轮被其它会话新增的 `system_settings_common/select.rs` 缺少测试模块及未使用导入阻塞；观察该处修复后重新动态领取槽位 2 重试，又被 `system_settings_common/select/tests.rs` 的 7 个 E0599（overlay Element 的 layout/update/draw 调用）阻塞。未修改或提交该无关模块，不宣称完整 Clippy 通过。
- 最初裸二进制窗口无法由原生自动化工具识别，终止该次测试（退出 101，不计窗口通过），随后沿既有临时 app bundle runner 启动正式显式窗口命令，无仓库构建配置变更。

### macOS 原生窗口

macOS **27.0（26A428）/ arm64**，桌面 **0.6.0**，`sw_vers`/`uname -m` 退出 0。`command -v agy` 退出 1，真实 Agy 安装版本、真实账号/钥匙串及上游应用消费未验证；Windows/Linux、其它架构、PostgreSQL 实际服务未验收。

`SHIJU_CC04_WINDOW_CASE=agy SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window` 和对应 `light` 命令均实际操作并关闭窗口，退出 0。临时 runner 仅包装同一槽位二进制为可识别 app bundle；`agy` case 使用真实组件/Message/SDK，回环 HTTP 提供假目录数据，未知 case 仍拒绝。执行端落盘由上述真实 Gateway 测试证明，不把窗口夹具当真实上游运行。

明暗主题均完成：新增中文档案 → 滚动长表单 → 非法模型返回 422 并保留草稿 → 修正中文动态模型保存 → 重开编辑 → 修改名称后取消 → 重开显示已保存名称 → 确认删除并观察列表移除。浅色额外验证既有档案 PATCH 保存名称更新。统一圆角输入/按钮/toast、固定标题与底部操作区，4px 滚动条及两侧各 7px 留白；中文与动态文本默认字体，实际绘制无整形 panic。仅有既有 macOS sandbox extension 和测试链接器 unwind 提示。

- 深色：[失败与长表单](evidence/task-66/dark-failure.png)、[保存](evidence/task-66/dark-saved.png)、[取消草稿](evidence/task-66/dark-cancel-draft.png)、[重开](evidence/task-66/dark-cancel-reopened.png)、[删除后](evidence/task-66/dark-deleted.png)。
- 浅色：[失败与长表单](evidence/task-66/light-failure.png)、[保存](evidence/task-66/light-saved.png)、[取消草稿](evidence/task-66/light-cancel-draft.png)、[重开](evidence/task-66/light-cancel-reopened.png)、[删除后](evidence/task-66/light-deleted.png)。

状态：**已完成，待回归**。自身槽位 1、2 占用记录均已清理，缓存保留；其它会话重新领取的槽位不触碰。保留编号任务文件，待其它会话修复上述编译错误后重新运行严格 Clippy，通过后再完成归档。只提交本任务实现、测试和证据，排除其它会话设置/终端改动及编号任务状态；只读 plan.md 无差异。未执行全量单元测试、fmt check、wasm 或 Makefile。


## 任务 67：桌面 Desktop 直连与映射表单

2026-11-07，状态：**已完成**。批次 66 实现提交 `b3f0f04e2` 已核对；调度明确允许消费其“已完成，待回归”实现，历史无关 Clippy 阻塞进入统一回归。母任务 1、3 已完成；母任务 2 沿底座子计划 1–93 开工裁定，不冒称整体完成。所有 plan.md 保持只读。

### 来源与实现

- 来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`：阅读 `ClaudeDesktopProviderForm`、`useDesktopConfigMessages`、直接调用者 ProviderForm/SwitchModePanel 与相邻测试。实现前检查统一 settings 控件及 custom_model_modal，沿任务 33 直连事务与任务 34 映射目录契约。
- 新建 components/state/message 的 `app_providers/desktop.rs`，接入列表新增、逐条“直连与映射”、状态和 Message 分发。支持直连基础字段、已有官方登录、首次激活/当前供应商应用、默认路由、模型与中文显示名、1M 标记、兼容导入。统一弹窗、输入、按钮、toast；固定标题与保存/取消区，4px 滚动条、两侧各 7px 留白，重开滚动复位。
- 新增脱敏 `DesktopFormView` 与 SDK `claude_desktop_form_read`，调用 `POST /v1/app-providers/claude-desktop/{id}/form/read`。执行端先验证敏感 token、租户和主机读取权限，再读目录并调用领域 `form_view`；不返回密钥，拒绝含秘密 query 等非法端点。保存密钥需重新完整输入，事件/状态 Debug 脱敏。UI 不调用文件系统或命令，所有提交走既有 SDK/API；过期主机/代次响应丢弃，忙碌时禁止重复提交。
- 直连保存/切换消费既有事务 API，分别呈现目录保存、未应用、已应用、失败和恢复状态；只有真实返回 Applied 才提示完全退出重启 Desktop。映射保存仅写目录，明确未激活；非空路由/代理模式仍不能直接激活。路由运行时、实时诊断、模型发现、OAuth 发起尚不可用，未伪造支持。
- Gateway 测试使用真实 SDK/HTTP/handler、临时 SQLite 与隔离 HOME，证明读取脱敏、权限拒绝无副作用、端点秘密拒绝、映射目录保存/导入、兼容原生档案到四文件事务，以及直连切换/编辑/reapply/官方配置。仅外部上游和原生窗口 HTTP 响应使用假数据，凭据均为测试值。

### 验证记录

每阶段以原子 mkdir 动态领取共享槽位，记录计划/任务/会话/PID；`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 同为下表绝对目录。所有 Cargo 和原生窗口子进程结束后已清理自身锁，保留缓存。

| 槽位 | CARGO_TARGET_DIR | 阶段 |
|---|---|---|
| 1 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` | 红绿测试、首次 API/Clippy、浅色窗口、最终严格 Clippy |
| 2 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-2` | 最终 UI/API/事务测试、最终深色窗口 |
| 4 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-4` | 最终格式化 |

| 确切命令 | 退出码 | 有效行为结果 |
|---|---|---|
| `cargo test -p vw-desktop --lib cc04_ui_desktop_ -- --nocapture` 红测 | 101 | 0 passed / 2 failed，SDK 未提交的业务断言，非编译失败 |
| 同上，最终绿测 | 0 | 2 passed：mapping_validation_is_visible、import_uses_compatibility_api |
| `cargo test -p vw-agent --lib gateway::api::handlers::app_providers::desktop_mapping_tests:: -- --nocapture` | 0 | 4 passed |
| `cargo test -p vw-agent --lib cc04_claude_desktop_sdk_api_switch_edit_reapply_and_official -- --nocapture` | 0 | 1 passed |
| `cargo fmt --all` | 0 | 格式化，不计行为测试 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 最终完整严格检查通过，不计行为测试 |
| `SHIJU_CC04_WINDOW_CASE=desktop SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并手动关闭 |
| `SHIJU_CC04_WINDOW_CASE=desktop SHIJU_CC04_WINDOW_THEME=light cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作并手动关闭 |

去重共 **7 项行为测试**。首次实现编译因闭包参数缺少 bool 类型退出 101，补充后通过，不计红测；首次深色 fixture 错误匹配其它 `/config` 路由导致退出 101，限制应用路径前缀后完成最终明暗窗口，两次均退出 0。没有执行全量单元测试、fmt check、wasm 或 Makefile。

### macOS 原生绘制

macOS **27.0（26A428）/ arm64**，桌面 **0.6.0**。窗口命令额外继承 `CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER=/tmp/shiju-task67-window/runner`，仅将同槽位二进制包装到临时 Desktop67Acceptance.app，未修改仓库构建配置。显式 desktop case 使用真实 Iced 组件、Message、SDK 与回环假 HTTP；未知 case 仍拒绝。真实 Claude Desktop 账号、安装版本与最终上游消费未验证，Windows/Linux 未验收；不把 fixture 当作真实路由运行时或上游应用通过。

明暗主题均操作：打开含 12 条中文动态映射的长表单 → 非法 model 返回 422 并保留草稿 → 修正并保存中文显示名 → 再改草稿后取消 → 重开确认已保存值；新建中文直连档案 → 非法 URL 失败 → 修正后应用 → 显示完全退出重启提示 → 修改名称后取消并刷新，确认原保存名称。深色额外点击默认路由，回读显示 claude-fable-5。实际滚动、弹窗和动态中文整形无 panic；只见既有 sandbox extension 与链接器 unwind 提示。

- 深色：[映射失败](evidence/task-67/dark-failure.png)、[保存长表单](evidence/task-67/dark-saved.png)、[取消重开](evidence/task-67/dark-cancel-reopened.png)、[默认路由](evidence/task-67/dark-defaults.png)、[直连失败](evidence/task-67/dark-direct-failure.png)、[应用重启提示](evidence/task-67/dark-direct-applied.png)、[取消后列表](evidence/task-67/dark-direct-cancel.png)。
- 浅色：[映射失败](evidence/task-67/light-failure.png)、[保存长表单](evidence/task-67/light-saved.png)、[取消草稿](evidence/task-67/light-cancel-draft.png)、[取消重开](evidence/task-67/light-cancel-reopened.png)、[直连失败](evidence/task-67/light-direct-failure.png)、[应用重启提示](evidence/task-67/light-direct-applied.png)、[取消后列表](evidence/task-67/light-direct-cancel.png)。

定向差异检查通过，只提交本任务实现、独立测试、原生夹具和正式证据，排除其它会话设置/终端变更和编号任务状态。编号任务证据齐备后删除，不单独提交状态变更。下一步批次 68 与计划统一回归。

## 任务 68：桌面端点与请求选项

2026-11-07，状态：**已完成**。已核对批次 67 提交 `3fb3ccfc5` 和本文件证据；母任务 1、3 完成，母任务 2 沿底座子计划 1–93 开工裁定，不冒称整体迁移完成。首次检查发现 Legacy PATCH 不兼容原生档案后停止；调度随后明确授权补齐 Claude/Codex 请求选项所有者契约，本轮依此完成，不修改任何 plan.md。

### 来源、实现与消费边界

来源 `/Users/xiangmin/worker/code/cc-switch` HEAD `ca58c0313360eae7c851fbd7553c1614c7af03e7`；读取 `EndpointField`、`RequestHeadersEditor`、`LocalProxyRequestOverridesField`，及 ClaudeFormFields/OpenCodeFormFields 调用与 `requestOverrides.test.ts` 相邻行为。导航使用 RustCodeGraph，未覆盖的新增文件定向读取；编辑前对照 system_settings_common 与 custom_model_modal。

- 桌面 components/state/message 的 `app_providers/endpoints.rs` 接入供应商列表逐项“端点与请求选项”。地址簿增删、基础/完整 URL、刷新、标记使用均消费既有 SDK；完整路径及末尾斜杠原样提交，时间戳来自服务。地址簿不切换当前供应商，标记使用不代表成功请求。主机/租户/代次过期响应丢弃，忙碌时阻止重复提交；失败保留弹窗和草稿。
- Claude/Codex 显示只写普通头与代理覆盖 JSON 输入。留空保留、`{}` 清除对应组；先要求每个非空字段是单个完整 JSON 对象，拒绝注入相邻选项；结构反序列化失败保留草稿，头名称/值、大小、受保护代理头及协议字段继续使用执行端 `providers/validation::{validate,merge}` 的唯一校验。保存成功清空草稿，刷新/重开只回读是否配置，绝不以脱敏占位值回填后覆盖秘密。Form/Event Debug 固定脱敏，SDK 凭据头标记 sensitive，错误不返回响应正文。
- 新增强类型 `RequestOptionsTarget/Write/View`，SDK `app_provider_request_options_read/save`，API `POST /v1/app-providers/{app}/{id}/request-options/read` 与 `PUT /v1/app-providers/{app}/{id}/request-options`。先通过既有 provider_editor_context 校验 token、租户、执行主机及 read/write 权限，再访问固定档案命名空间。写入复用档案 CAS；不支持应用/非原生记录明确拒绝，不走不兼容 Legacy PATCH，不增加配置路径、命令接口或业务存储。
- `providers/request_options.rs` 负责原生共同选项视图与合并，`AppProviderStore` 负责持久化；Claude/Codex 原生保存不再删除 `requestOptions`，并在自身 CAS 前验证它。现有档案、凭据、原生配置与其他元数据继续由原所有者处理。
- **普通头与代理覆盖均仅保存，返回 `application: not_applied`。** 普通头供原生适配消费，但本入口尚未将它投影到原生配置；代理覆盖只用于路由/聚合代理，生产代理未接入。本任务不声称发送请求、原生消费或 live 生效，没有提前建设路由引擎。其它应用本次只开放已有端点地址簿，不新增它们的请求选项契约。
- 弹窗复用统一输入/checkbox/按钮、toast；标题、刷新、取消、保存按钮固定，仅内容滚动，4px 滚动条、两侧各 7px 留白。重开复位滚动；所有中文、URL 与动态状态使用默认字体，无中文等宽字体。

### SDK→HTTP→执行端证据

新 Gateway 测试使用真实 SDK、回环 HTTP、正式 handler/权限策略与临时 SQLite；原生档案由既有 Claude/Codex 保存所有者创建。只使用假凭据和隔离 HOME，未触碰真实用户配置。

1. 请求选项 PUT 携带普通 Authorization 测试头、X-Title、代理 X-Proxy 和 temperature；响应只有 `headersConfigured/overridesConfigured/application`，不含头名/值、正文或密钥。随后经 SDK 执行 Claude 原生表单更新及 Codex 原生保存，独立重读数据库确认选项完整保留、头名规范化及名称更新；重新打开数据库仍可读出配置状态。隔离 `.claude/settings.json` 和 `.codex/config.toml` 均未创建，不把目录保存当原生应用。
2. 无效 token、错误 host、受保护代理 Authorization 被拒绝，数据库档案字节不变；普通头显式清除保留代理覆盖，再清除代理覆盖。既有端点回归另证明 Basic 认证拒绝、非法输入无副作用与跨租户/应用隔离。
3. 原生档案地址簿通过 SDK 添加完整 `/v1/messages/`，MarkUsed 回读服务时间，独立数据库 `customEndpoints` 与响应一致，原生凭据保持；含 URL 凭据的非法端点拒绝且档案不变，删除后空列表。窗口回环 fixture 只替代远端响应，不能取代以上真实存储证据。

### 验证记录

每阶段依次原子 mkdir 动态领取共享槽位并记录计划/任务/会话/PID；`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 同为以下绝对路径。所有 Cargo、窗口及产物子进程结束后已释放自身锁，保留缓存。

| 槽位 | CARGO_TARGET_DIR | 阶段 |
|---|---|---|
| 1 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` | 红测、首次绿测、最终 API/端点回归与 Clippy、浅色窗口 |
| 2 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-2` | 格式化、桌面绿测、深色窗口、JSON 边界首次复验与最终 Clippy |
| 3 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-3` | JSON 边界补强后桌面最终复验 |

| 确切命令 | 退出码 | 有效结果 |
|---|---|---|
| `cargo fmt --all` | 0 | 每轮 Rust 修改后执行，不计行为测试 |
| `cargo test -p vw-desktop --lib cc04_ui_endpoints_ -- --nocapture` 红测 | 101 | 0 passed / 2 failed；SDK 未提交的业务断言，非编译失败 |
| 同上，JSON 边界补强前绿测 | 0 | 2 passed：invalid_response_stays_in_modal、submit_keeps_full_url_choice |
| `cargo test -p vw-agent --lib cc04_ui_endpoints_ -- --nocapture` | 0 | 3 passed：原生保存保留/脱敏、权限校验/清除、地址簿/时间戳 |
| `cargo test -p vw-agent --lib cc04_endpoints_ -- --nocapture` | 0 | 2 passed：既有认证拒绝无副作用、SDK 持久化/隔离 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | JSON 边界补强前完整严格检查通过 |
| `SHIJU_CC04_WINDOW_CASE=endpoints SHIJU_CC04_WINDOW_THEME=dark cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作、绘制并手动关闭 |
| `SHIJU_CC04_WINDOW_CASE=endpoints SHIJU_CC04_WINDOW_THEME=light cargo test -p vw-desktop --test cc04-provider-window` | 0 | 实际操作、绘制并手动关闭 |

JSON 边界补强前去重通过 **7 项行为测试**（5 项新增、2 项已有回归）。补强新增 `cc04_ui_endpoints_json_field_cannot_inject_sibling_options`，最终 `cargo fmt --all` 退出 0；同一桌面过滤命令退出 101，因并行改动 `system_settings_common/mod_ui_refresh_01_tests.rs:417` 对共享引用中的 renderer 调用可变 screenshot（E0596），未执行任何测试，新增测试尚未验收。未修改该无关文件；该并行错误随后由所属会话修复；槽位 3 重跑相同过滤命令退出 0，3 passed，包含新增 JSON 注入拒绝测试。最终严格 Clippy 在槽位 2 退出 0。最终去重 **8 项行为测试**（6 项新增、2 项已有），未改窗口布局或有效输入处理，不重复窗口验收。窗口不计单元测试数。窗口命令额外继承 `CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER=/tmp/shiju-task68-window/runner`，仅把同槽位二进制包装为临时 Endpoints68Acceptance.app；未改仓库构建设置。看到既有链接器 unwind/sandbox extension 提示，窗口命令均退出 0；没有 panic。未运行全量 cargo test、fmt check、wasm 或 Makefile。

### macOS 原生绘制

macOS **27.0（26A428）/ arm64**，桌面 **0.6.0**。两种主题均实际操作：供应商列表 → 端点与请求选项 → 中文无效端点返回 422、保留草稿 → 修正为完整 `/v1/messages/` 并勾选完整 URL → 添加、标记使用、刷新，显示服务返回 12345 → 滚动 9 条端点长表单 → 输入中文动态普通头、受保护代理头，返回 400 且草稿保留 → 修正并保存，状态显示两组已配置且未应用、输入框清空 → 输入清除草稿后取消 → 重开确认两组配置仍保留、没有敏感值回填。真实绘制中文、弹窗、滚动与动态文本，无字形整形 panic。

- 深色：[端点失败](evidence/task-68/dark-endpoint-failure.png)、[完整 URL 与服务时间](evidence/task-68/dark-full-url-used.png)、[请求覆盖失败](evidence/task-68/dark-options-failure.png)、[保存状态](evidence/task-68/dark-options-saved.png)、[取消重开](evidence/task-68/dark-cancel-reopened.png)。
- 浅色：[端点失败](evidence/task-68/light-endpoint-failure.png)、[完整 URL 与服务时间](evidence/task-68/light-full-url-used.png)、[请求覆盖失败](evidence/task-68/light-options-failure.png)、[保存状态](evidence/task-68/light-options-saved.png)、[取消重开](evidence/task-68/light-cancel-reopened.png)。

真实第三方应用版本/账号、上游网络、普通头原生投影、代理消费和非 macOS 平台均未验收。截图中的时间及 HTTP 返回属于显式窗口 fixture；真实服务时间/权限/持久化由上述 Gateway 测试证明。差异检查通过，只提交任务实现、独立测试、夹具和本条正式证据；其它会话设置/终端改动及编号任务状态不纳入提交。下一步批次 69 与计划统一回归。
