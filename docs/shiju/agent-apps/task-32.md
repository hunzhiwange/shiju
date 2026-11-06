# 任务32：官方账号隔离启动阶段证据

状态：部分实现，任务仍阻塞；不宣称全部官方账号终端启动已交付。

## 已实现范围及真实所有者

经用户授权补齐母任务6必要最小底座。账号元数据唯一落点为计划指定的 `vw-agent/src/agent_apps/auth_accounts.rs`，复用 `GatewaySettingsStore` 的租户 SQLite/PostgreSQL存储门面，固定命名空间 `agent_apps.auth_accounts`，只保存 configured 标记，不存 token。此次实际验证 SQLite；未声称 PostgreSQL实机通过。

秘密文件所有者为 `vw-agent-apps/src/auth/credential_file.rs`，不是 apps/launch 或 providers 的账户副本。根目录由可信执行主机提供且预先要求私有权限；不读取、扫描或自动归属主机 HOME、CLI现存账号、钥匙串或只读upstream库。平台、公司、主机、应用服务与显式账号ID构成路径作用域；分段base64编码避免路径注入，同ID跨公司/主机/服务不串读。目录0700，临时文件原子替换且0600，拒绝符号链接、非私有路径、超大材料及服务错配；所有路径参数在创建文件之前验证。

`AuthAccountStore::save` 是可信主机内部的显式材料导入接口，调用者须先授权租户与主机；不对外新增账户导入API，也不声称实现成员授权、登录、默认账号选择、绑定CRUD或账号轮换。`prepare_launch` 在读取秘密前复核数据库中的作用域记录和任务28的授权主机实例/目录/参数规则；缺记录默认拒绝。未来API/SDK与终端dispatch仍归对应任务，不接受客户端指定凭据根目录或命令。

`apps/launch/account.rs` 消费 auth 提供的材料，启动观察到的实际程序：

- Claude：显式导入的OAuth token只进入子进程 `CLAUDE_CODE_OAUTH_TOKEN`，私有 `CLAUDE_CONFIG_DIR`，禁用其他settings来源。它是隔离的单次账号材料通路，未采用主机默认登录槽或现有目录，未实现默认账号/原有账号目录迁移。
- Codex：显式ChatGPT tokens材料进入私有 `auth.json`，配置强制openai与file凭据存储，仅子进程设置 `CODEX_HOME`；拒绝项目 `.codex/config.toml` 覆盖。没有API-key或主机默认账号回退。本阶段材料是一次性投影；CLI刷新后的凭据未回写账号所有者，跨进程轮换仍待母任务6实现，不能当作长期登录生命周期完整交付。
- AGY：在写入或启动前明确拒绝，原因是当前上游1.4.3官方登录读取固定 `gemini/antigravity` 系统钥匙串项。来源 `upstream/src-tauri/src/services/agy_account/keychain.rs` 开头明示 `--gemini_dir` 对应文件不能隔离官方账号；`misc.rs::switch_agy_to_bound_account` 实际改写全局项。当前授权禁止读取/自动归属个人全局凭据，不能将新文件接口伪装成AGY实际支持的账号注入。

进程环境清空后仅保留OS必要变量，凭据/运行时hook不继承；不修改父进程环境。输出不回传，错误固定，不实现Debug或Serialize以泄漏材料。临时目录持有到子进程回收，正常、非零、spawn失败、未运行丢弃均清理；正式账号凭据仍在auth所有者中。

## 尚未解除的真实边界

需要AGY可用的隔离OS账号/钥匙串执行主机及其可信分派能力，或上游正式提供进程级官方账号注入接口，再完成绑定账号启动。没有创建伪接口、切换个人钥匙串、修改系统钥匙串搜索列表或静默采用当前登录。单纯提供测试账号不能修复这个隔离限制。Claude默认/目录账号和Codex刷新回写仍需正式账号生命周期接口。故任务文件保留为已阻塞，后续计划不能以此证据宣告母任务3完整完成。

## TDD与验证

Cargo阶段均动态原子mkdir领取槽位，记录计划/任务/会话/PID；设置target和build-dir到同一绝对目录，EXIT/INT/TERM清理自己的锁，缓存保留。

实际使用slot-2：

- `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`
- `CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`

首次红灯暴露测试临时根默认0755，与真实私有根约束不符；修正夹具为0700后重新运行，未把这个夹具问题当作所需行为红灯。

- 行为红灯：`cargo test -p vw-agent-apps --lib cc_apps_account_launch_isolation -- --nocapture`，退出101；1项有效测试失败，失败点为最小可编译签名返回 `official account launch not implemented`。已先经过真实凭据持久化、重开及跨作用域拒绝断言，不是缺依赖/零测试。
- 最终 `cargo fmt --all`：退出0；只保留本任务模块注册和新增文件，无其他Rust格式差异。
- `cargo test -p vw-agent-apps --lib cc_apps_account_launch_isolation -- --nocapture`：退出0，7项通过、0失败、148过滤；继承环境用独立测试子进程执行1个有效子用例，不重复计入顶层7项。真实shell进程验证Claude token、Codex账号/auth内容、私有目录与文件权限、父环境不变、非零/无法启动/丢弃清理；验证错误服务、恶意参数、权限跳过和项目覆盖在临时副作用前拒绝；真实文件验证同ID跨租户隔离、错误主机/缺记录与无效材料拒绝。
- `cargo test -p vw-agent --lib cc_apps_account_launch_isolation -- --nocapture`：退出0，1项通过、0失败、11444过滤；真实SQLite重开后经AuthAccountStore读取材料→任务28主机校验→Claude子进程→临时目录清理；跨平台租户、跨公司、错误主机与缺账号记录均在进程副作用前拒绝。

上述均为本机macOS上的非敏感材料/子进程夹具验证，不是个人真实登录、AGY钥匙串切换或终端窗口交接证据。Windows/WSL仍由任务28执行环境校验拒绝，本轮没有跨平台实机运行。没有运行全量cargo test、fmt检查、wasm或Makefile，无UI变更。
- 必须检查 `cargo clippy --all-targets --all-features -- -D warnings`：退出0，无Clippy诊断。
- `git diff --check` 与 `git diff --cached --check`：退出0；暂存仅本任务10个代码/测试/证据文件，既有编号任务删除、任务32状态记录和plan.md均排除提交。未引入依赖。
- 最终验证阶段退出0，Cargo及测试子进程全部结束，自己的slot-2锁已释放，编译缓存保留。构建日志另有先前Agent链接器 `__eh_frame section too large` 平台提示，聚焦测试仍退出0；不修改链接配置掩盖提示。

下一步：交付正式账号生命周期（Claude目录/默认与Codex刷新），并提供可隔离AGY系统钥匙串的执行环境后继续任务32；此提交为可审查的部分进展，不删除编号任务文件，不修改其他母任务状态。
