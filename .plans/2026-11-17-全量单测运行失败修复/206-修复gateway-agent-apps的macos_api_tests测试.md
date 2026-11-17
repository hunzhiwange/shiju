# 任务206：修复gateway-agent-apps的macos_api_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：已阻塞

来源：原任务19全量实际执行；证据编号386, 387, 388, 389, 390。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/gateway-agent-apps/src/agent_apps_skills/macos_api_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据386：`agent_apps_skills::macos_api_tests::cp12_macos_api_postgres_remote_update_and_precommit_cancel`

位置：`crates/libs/gateway-agent-apps/src/agent_apps_skills/macos_api_tests.rs:322`；结果：FAIL。

```text
crates/libs/gateway-agent-apps/src/agent_apps_skills/macos_api_tests.rs:325:14:
    macOS API acceptance requires VIBEWINDOW_TEST_POSTGRES_URL: VIBEWINDOW_TEST_POSTGRES_URL is not configured; provide the real local PostgreSQL test URL

    Caused by:
        environment variable not found
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据387：`agent_apps_skills::macos_api_tests::cp12_macos_api_postgres_success`

位置：`crates/libs/gateway-agent-apps/src/agent_apps_skills/macos_api_tests.rs:151`；结果：FAIL。

```text
crates/libs/gateway-agent-apps/src/agent_apps_skills/macos_api_tests.rs:154:10:
    macOS API acceptance requires VIBEWINDOW_TEST_POSTGRES_URL: VIBEWINDOW_TEST_POSTGRES_URL is not configured; provide the real local PostgreSQL test URL

    Caused by:
        environment variable not found
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据388：`agent_apps_skills::macos_api_tests::cp12_macos_api_rejects_without_side_effects`

位置：`crates/libs/gateway-agent-apps/src/agent_apps_skills/macos_api_tests.rs:160`；结果：FAIL。

```text
crates/libs/gateway-agent-apps/src/agent_apps_skills/macos_api_tests.rs:177:79:
    called `Result::unwrap()` on an `Err` value: Http { status: 401 }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据389：`agent_apps_skills::macos_api_tests::cp12_macos_api_remote_update_and_precommit_cancel`

位置：`crates/libs/gateway-agent-apps/src/agent_apps_skills/macos_api_tests.rs:314`；结果：FAIL。

```text
crates/libs/gateway-agent-apps/src/agent_apps_skills/macos_api_tests.rs:236:25:
    called `Result::unwrap()` on an `Err` value: Http { status: 401 }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据390：`agent_apps_skills::macos_api_tests::cp12_macos_api_success`

位置：`crates/libs/gateway-agent-apps/src/agent_apps_skills/macos_api_tests.rs:144`；结果：FAIL。

```text
crates/libs/gateway-agent-apps/src/agent_apps_skills/macos_api_tests.rs:99:71:
    called `Result::unwrap()` on an `Err` value: Http { status: 401 }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'agent_apps_skills::macos_api_tests::cp12_macos_api_postgres_remote_update_and_precommit_cancel' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'agent_apps_skills::macos_api_tests::cp12_macos_api_postgres_success' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'agent_apps_skills::macos_api_tests::cp12_macos_api_rejects_without_side_effects' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'agent_apps_skills::macos_api_tests::cp12_macos_api_remote_update_and_precommit_cancel' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'agent_apps_skills::macos_api_tests::cp12_macos_api_success' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 2026-11-17 执行证据与阻塞

- 已读取执行技能、Rust 资源约定、代码导航技能、测试、HTTP handlers、相邻 backups/install 夹具，以及底层 access/gateway_execution。当前测试文件无既有 diff，未修改业务代码或断言。
- 原生精确命令已按上列顺序在一次 `python3 scripts/cargo_slot.py sh -c` 中执行；实际槽位6，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-6`。日志 `/tmp/shiju-task206/red-<后缀>.log`，后缀依次为 postgres_remote_update_and_precommit_cancel、postgres_success、rejects_without_side_effects、remote_update_and_precommit_cancel、success。
- 五条均实际执行1项：前两条因缺少 VIBEWINDOW_TEST_POSTGRES_URL 失败，退出101；rejects_without_side_effects 为1 passed/0 failed，退出0；remote_update_and_precommit_cancel 为0 passed/1 failed，退出101，在254行读取原夹具 ssot/tool/SKILL.md 报不存在；success 为0 passed/1 failed，退出101，100行本地安装返回403。入口及其测试进程已退出。
- 新失败证明共享底层契约已变化：当前未提交的 `app-access-service/src/access.rs` 在 execution_context 中忽略 path/target，gateway_host 改用 agent_paths 全局 HOME 的 `.agents/skills`，skill_install_files 为空。原夹具提供的私有根、主机与 selected 文件映射均不再生效。远程安装因此可能触及真实 HOME，不能继续用旧夹具运行。rejects 测试通过也不能证明原授权语义成立（本地来源映射缺失足以导致拒绝）。
- 公共规则要求共享写入排队，且禁止弱化安全/租户隔离断言。相邻任务202亦已记录 access.rs 被其他任务修改；此处不覆盖该共享变更，不恢复旧令牌授权，不把目标改成全局用户目录，不跳过 PostgreSQL 测试。
- 已检查 Docker 可用，存在 PostgreSQL 容器；尚未读取凭证或运行带 URL 的测试，因为 HTTP 场景必须先恢复隔离。需要共享底层修改所有者确认并稳定本地来源、执行根和目标/租户隔离的现行接口，然后据此调整本夹具（必要时进程隔离）并用真实 PostgreSQL 完成五条绿灯。本任务仅更新状态，无代码变更，不创建状态专用提交；保留任务文件。

## 2026-11-18 重试与部分修复

- 提交58e6f3c40：五项测试使用独立子进程和VIBEWINDOW_TEST_ROOT；原ssot观察路径链接到隔离HOME的实际技能根，避免写真实HOME。没有删改断言。
- 真实PostgreSQL来自已有本机Docker服务，只在进程环境传入URL，不输出或落盘凭证；连接显式sslmode=disable。发现同步驱动不能在异步测试内创建/析构，进一步将两项PostgreSQL测试改为同步测试、运行时外持有数据库夹具；远程用例仓库准备在spawn_blocking内执行。
- 原生五条精确过滤已运行；SQLite拒绝与远程更新取消均父子进程各1 passed/0 failed。SQLite本地安装为父子各1 failed，HTTP403。PostgreSQL本地安装在修复夹具后同样到达HTTP403（初始化及清理不再panic）。
- 修复后编译产物为target/rust-slot-3/debug/deps/gateway_agent_apps-bf5715e2a385139d；由于共享源码持续变化，再次原生重编由本任务入口SIGINT有界停止，入口退出130并清理其进程组。使用同一已编译产物在槽位3中补充精确运行：SQLite远程父子各1 passed；PostgreSQL远程父子各1 failed，在HTTP安装发生Transport错误，服务器同步PostgreSQL驱动运行时嵌套panic。日志/tmp/shiju-task206/artifact-{postgres_remote_update_and_precommit_cancel,remote_update_and_precommit_cancel}.log。不能把补充产物执行冒充最新源码Cargo绿灯。
- 本轮槽位先6后3；两缓存变量均为对应的/Users/xiangmin/worker/code/shiju/target/rust-slot-N。日志/tmp/shiju-task206/retry-*.log。相关入口均已结束，自有槽位锁已释放。未运行全量测试。
- 新根因：install_api.rs在async execute内直接new/get同步数据库并调用start_repo_install_at；PostgreSQL远程仍需修复生产同步数据库边界，不属于无关回归。
- 当前产品接口忽略旧策略文件，skill_install_files恒空，本地安装成功覆盖无法成立。已向用户提出必要方向决定：恢复受信本地导入入口，或明确变更验收范围使用Git安装。此决定未收到答复；不擅自恢复旧应用授权或删除本地安装断言。保留已阻塞状态；不是仅等待共享文件所有者。

- 第二组运行时边界修复尚未提交：多次git commit --only均退出128，最后一次55秒有界等待仍未取得索引；lsof证实真实活跃git PID80503持有.git/index.lock（此前PID57115已退出并被新的Git操作接替）。未删除他人锁或使用替代索引；实际代码仍保留在macos_api_tests.rs（12插入/8删除），diff --check通过。第一组提交58e6f3c40已成功；不能声称第二组已提交。恢复时先待该Git操作释放，再使用git-commit仅提交此文件。

## 2026-11-18 补交与导入方向复核

- 原活跃Git锁自然释放，未删除锁。已用git-commit提交第二组实际修复：c5083139c（test(skills): 修复PostgreSQL验收夹具的运行时边界），仅测试定义文件；该文件工作区现无差异。此前未提交记录为历史状态，已解除。
- 重查全部InstallSkillRequest/agent_apps_skills_install/SkillInstallSource::Local、Archive调用者：桌面desktop-settings-skills/src/update/install_view.rs仅构造Git来源；desktop-session只转发请求。Local/Archive仅保留在API/SDK和测试，生产树中没有注册skill_install_files的写入口。
- 已明确设计证据：5cb8b43dc“fix(apps): 移除本地操作重复授权与项目主机门槛”；当前execution_context忽略旧策略文件/目标主机，使用Gateway本机身份。不是其他任务未提交漂移，不能恢复旧授权以通过测试。
- 因此旧本地成功验收与已明确设计不兼容：当前无法从调用者推导新的受信文件选择入口。替换为Git安装会丢失原本地成功/打包hook不执行覆盖；恢复策略读取会重新引入已移除入口。是否新增本地导入能力或明确撤销该旧验收仍需产品方向决定，保留已阻塞且不削弱断言。真实PostgreSQL远程生产同步驱动边界问题仍未解决，不能标记待回归或完成。
- 本轮仅补交已验证的相同代码并做调用链、提交历史和diff轻量核查，没有新增实现，未重复Cargo。前轮测试证据仍按上述原始结果保留。
