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
