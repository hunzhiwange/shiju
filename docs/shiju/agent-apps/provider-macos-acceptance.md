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
