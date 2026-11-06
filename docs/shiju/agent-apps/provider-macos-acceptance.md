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
