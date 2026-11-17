# 任务8：修复agent-app-services的prompt_data_migration_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：原任务19全量实际执行；证据编号27。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/agent-app-services/src/prompt_data_migration_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据27：`prompts_store::data_migration_tests::cc_data_12_owner_import_repeat_skip_replace_and_stale`

位置：`crates/libs/agent-app-services/src/prompt_data_migration_tests.rs:12`；结果：FAIL。

```text
crates/libs/agent-app-services/src/prompt_data_migration_tests.rs:29:95:
    called `Result::unwrap()` on an `Err` value: safety snapshot: StorageUnavailable
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p agent-app-services --lib 'prompts_store::data_migration_tests::cc_data_12_owner_import_repeat_skip_replace_and_stale' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 2026-11-17 执行记录

- 已阅读 prompt_data_migration.rs 的 commit_data_migration、底层 safety_snapshot_files.rs 和相邻 MCP 迁移夹具。底层 root 逐级 openat 使用 O_NOFOLLOW，macOS /var 为符号链接；测试传入未规范化临时目录会触发 StorageUnavailable。
- 仅修改本测试定义文件：规范化快照根路径，所有提交调用复用该路径；保留 0700 权限及重复、跳过、替换、过期和原文件未改动断言，不放宽底层安全策略。
- 修复前执行任务精确命令，日志 /tmp/shiju-task8-red.log；修复后同命令日志 /tmp/shiju-task8-after.log。两次槽位均为3，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 /Users/xiangmin/worker/code/shiju/target/rust-slot-3。
- 两次均退出101，实际执行0项：现有 rmcp 0.8.5 client-only 依赖组合报13个 E0432/E0433/E0425，涉及 base64、schemars、paste、handler::server、CapabilitiesBuilder。任务5也已记录同一编译阻塞；不接管无关共享依赖特性修复。
- 两次入口均已退出，未留下本任务 Cargo 子进程；缓存保留。已审查仅本任务测试差异，git diff --check 通过；工作区其他变更和暂存内容均保留。
- 尚无行为通过证据，保留编号文件。恢复条件：集成 rmcp 依赖特性修复后重跑上述精确命令，确认实际运行1项并通过；不运行全量测试。
