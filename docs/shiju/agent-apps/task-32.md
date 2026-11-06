# 任务32：官方账号终端启动

状态：当前行为完成；Claude目录/显式默认账号、Codex材料与轮换回写、AGY不安全隔离路径的明确拒绝均有聚焦证据。此处不宣告母任务3、母任务6全部交付，也不声称个人真实登录或终端窗口交接通过。

## 所有者与授权边界

用户授权补齐必要最小底座。账号元数据唯一落点为母任务6指定的 `vw-agent/src/agent_apps/auth_accounts.rs`，复用 `GatewaySettingsStore`，固定命名空间 `agent_apps.auth_accounts` 与 `agent_apps.auth_defaults`。秘密归 `vw-agent-apps/src/auth/credential_file.rs` 所有；apps/launch只消费材料，不复制账户库。元数据不保存token。存储门面复用SQLite/PostgreSQL，此次实际验证SQLite，不声称PostgreSQL实机通过。

根目录由可信执行主机提供，不是客户端路径。平台、公司、主机、应用服务与显式账号ID构成作用域；分段base64编码避免路径注入。同ID跨公司/主机/服务不串读，缺记录默认拒绝。目录0700、秘密文件0600，原子替换并sync；拒绝符号链接、非私有路径、超大/错配材料、跨作用域目录重绑定及账号身份/表示类型变化。

`AuthAccountStore::save` 与 `register_claude_directory` 是可信主机内部的显式导入/目录注册接口，调用者须先授权租户与主机；不暴露新HTTP导入API，不推断公司成员权限，不扫描或自动认领HOME、现有CLI登录、个人钥匙串或upstream库。未知元数据或秘密材料均拒绝；元数据写入后秘密落盘失败也不会回退任何其他账号。`prepare_launch` 在材料化前复核作用域记录和任务28的主机实例、授权目录、参数规则。

API/SDK及终端dispatch仍归对应任务。此实现返回持有资源与凭据锁的运行对象，后续终端适配必须维持相同生命周期，不能拿短命启动器退出当作CLI运行结束。

## Claude目录与默认账号

- 显式OAuth token只进入子进程环境，使用私有 `CLAUDE_CONFIG_DIR`，禁用其他settings来源，不继承API key、运行时hook或默认登录选择。
- `register_claude_directory` 由auth所有者创建作用域内稳定目录，不能传任意宿主路径；文件只存类型标记，目录由作用域重新推导成不可外部构造的 `ClaudeAccountDirectory`。不采用已有非所属目录，不把token记录悄然替换为目录记录。
- 目录账号在进程启动前设置 `CLAUDE_CONFIG_DIR`，不设置 `CLAUDE_CODE_OAUTH_TOKEN`；允许CLI按这个目录读取、刷新自己的目录/钥匙串凭据，目录和会话状态保持，启动临时资源清理。
- 来源 `upstream/src-tauri/src/services/claude_account/layout.rs:10–12,122–130` 指出macOS钥匙串service与配置目录的hash绑定；`misc.rs::claude_account_terminal_launch` 也要求启动前设置目录，不能晚写settings。未复制其账号库/钥匙串实现。
- `set_default` 仅选择已有作用域记录，按租户、主机与服务保存显式ID；`prepare_default_launch` 复用同一材料路径。未设置默认、指针损坏、记录缺失、错误主机均拒绝，不把宿主“当前登录”当作已授权默认账号。原来源None默认槽语义在多租户主机上不能安全复用，使用已授权的显式默认映射，始终保留其账号目录。

目录注册和夹具不是真实登录证明。真实账号授权、默认切换UI及个人账户实机证据仍由母任务6/后续macOS验收覆盖，本任务不会读取个人凭据来制造证据。

## Codex生命周期

显式ChatGPT tokens进入私有 `auth.json`，配置指定openai与file凭据存储，仅子进程设置 `CODEX_HOME`；拒绝项目 `.codex/config.toml` 覆盖。不继承OPENAI_API_KEY，不回退主机登录。

`AuthAccountLaunch` 持有auth所有者的 `CredentialLease`，从准备到子进程回收、轮换回写与临时清理全程保留。`credential_lease.rs` 使用现有libc的Unix `flock(LOCK_EX|LOCK_NB)`；导入、目录注册和账号运行共享同一锁。相同账号的并发导入/运行明确返回in-use；跨进程已验证。持久空锁文件不unlink，以免旧inode/新inode产生两个独占锁；fd设置CLOEXEC，内核在持有进程结束时释放，不存在需要猜测PID清理的陈旧占用目录。

进程正常或非零退出后，在删除临时目录前重新读取并验证auth.json；仅同一 `tokens.account_id` 的有效材料可原子写回原作用域账号。非零退出也可能已经刷新，必须先回写再报告进程失败。材料损坏、身份改变、持久化失败均明确报错并清理临时资源；spawn失败不回写，丢弃未运行材料会释放锁。生产运行对象强制经过回写接口；忽略回写的便捷方法只在独立测试中编译。

## AGY绑定账号：有证据的安全拒绝

这是当前授权主机上的隔离能力拒绝，不把“没有个人测试账号”当作上游不支持，也不把整个agy应用标为不支持。

证据：

- 只读迁移来源 `upstream/src-tauri/src/services/agy_account/keychain.rs:1–10,17–25` 固定service `gemini`、account `antigravity`，明确1.4.3官方登录只看钥匙串，oauth_creds.json不替代它；`misc.rs::switch_agy_to_bound_account` 实际覆盖该全局项。
- [Go keyring macOS实现](https://github.com/zalando/go-keyring/blob/master/keyring_darwin.go) 的Get调用绝对路径 `/usr/bin/security find-generic-password -s … -wa …`；Set也没有keychain-path参数。它没有供调用方指定进程私有钥匙串的入口。
- [Antigravity官方认证文档](https://antigravity.google/docs/install?tab=cli) 的Local silent keyring sign-in确认本地CLI读取OS安全钥匙串。文档将Gemini API key和企业ADC列作其他认证方式，不等同于这项个人官方绑定账号材料注入。

替代方案逐项核对：私有gemini_dir只隔离文件，不能改写固定钥匙串查询；临时私有钥匙串即便创建也不会被上述AGY查询选中；修改default/search-list后再恢复会改变同OS用户的全局可见状态，且并发/崩溃无法保证隔离；PATH包装不能覆盖其绝对security路径；不改HOME、不注入厂商二进制、不把API key/ADC伪装为原官方个人绑定账号。

因此材料导入与启动均在副作用前返回 `agy official accounts require an isolated OS keychain execution host`。没有读取或修改任何真实全局钥匙串，没有创建无法由AGY消费的临时钥匙串。测试用非敏感执行边界与宿主状态哨兵核对不启动、不写入与不创建临时资源；它不是AGY个人账号实机成功证明。

按AGENTS.md“对不支持或不安全路径明确返回错误”、本任务账户隔离目的及本轮授权“协议不支持可验证隔离时明确拒绝”的要求，此拒绝属于本任务已验收行为，无需因它维持代码阻塞。将来有独立OS用户/主机或厂商进程级账号注入契约时，再由正式账号所有者增加相应能力，不在当前代码中伪造支持。

## TDD与验证

全部Cargo阶段动态原子mkdir领取仓库共享槽位，owner记录计划/任务/会话/PID；EXIT/INT/TERM清理自己的锁，缓存保留。检查仓库build.build-dir并覆盖到同一槽位，未绕过缓存规则。

实际slot-2：

- `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`
- `CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`

行为红灯：

- 初始聚焦命令 `cargo test -p vw-agent-apps --lib cc_apps_account_launch_isolation -- --nocapture`，退出101，1有效测试失败于最小可编译签名 `official account launch not implemented`。此前修正夹具临时根0755→0700的问题未冒充所需行为红灯。
- 生命周期补充同命令，退出101，8有效测试中7通过、1失败：真实子进程写入新refresh token且非零退出，但原实现没有调用账号所有者回写；在“轮换材料必须清理前到达所有者”的断言失败。随后实现锁与回写，不是缺依赖/零测试。

最终验证（所有命令从仓库根运行，上述slot-2环境均继承）：

| 确切命令 | 退出码 | 有效数量/结果 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 无无关格式化差异 |
| `cargo test -p vw-agent-apps --lib cc_apps_account_launch_isolation -- --nocapture` | 0 | 12通过、0失败、148过滤；环境污染子进程1项及锁子进程2次不重复计入顶层数量 |
| `cargo test -p vw-agent --lib cc_apps_account_launch_isolation -- --nocapture` | 0 | 4通过、0失败、11444过滤 |
| `cargo test -p vw-agent-apps --lib cc_apps_launch_request_validation_rejects_stale_ambiguous_and_foreign_instances -- --nocapture` | 0 | 1通过、0失败、159过滤；macOS运行Windows/WSL外部实例显式夹具，非平台实机通过声明 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全仓通过，无Clippy诊断 |
| `git diff --check` / `git diff --cached --check` | 0 | 仅本任务源码、独立测试和证据；plan.md/upstream未修改 |

首轮生命周期Clippy发现本次unused_unit与两处collapsible_if，已修正并重新执行fmt、两crate聚焦及全仓Clippy，全部通过。Agent测试链接器仍提示 `__eh_frame section too large`，测试退出0；未修改链接设置隐藏提示，也未将其冒充Clippy成功。首次原始签名/夹具问题及这次风格诊断均已按本任务处理，无无关错误被悄悄修复。

真实文件/SQLite/子进程证据覆盖：作用域隔离与重开、官方账号材料而非API key、Claude持久目录及默认指针、Codex轮换后下一次运行实际读取新材料、非零退出回写、并发导入/运行拒绝、跨进程锁释放、身份变化/无效材料/回写失败清理、未运行丢弃/spawn失败、环境污染、项目覆盖及AGY副作用前拒绝。未使用真实个人账号或外网模型请求。

所有Cargo与测试子进程已结束，自己的slot-2锁已释放，缓存保留。没有运行无过滤cargo test、fmt检查、wasm检查或Makefile，无UI变更。后续按任务36接线授权终端执行，并由对应macOS任务取得真实账号/CLI实机证据；AGY拒绝保持显式能力边界。
