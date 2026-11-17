# 任务238：修复gateway-desktop的desktop_skills_tests测试第2组

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次2】 依赖：任务237（同文件写入）

状态：未开始

来源：原任务19全量实际执行；证据编号464, 465, 466, 467。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据464：`desktop_skills::desktop_skills_tests::resolve_skill_detail_returns_personal_toml_skill_and_missing_errors`

位置：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs:302`；结果：FAIL。

```text
crates/libs/gateway-desktop/src/desktop_skills_tests.rs:312:6:
    personal detail: "未找到技能: custom-skill"
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据465：`desktop_skills::desktop_skills_tests::resolve_skill_detail_returns_built_in_and_local_variants`

位置：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs:334`；结果：FAIL。

```text
crates/libs/gateway-desktop/src/desktop_skills_tests.rs:364:6:
    local detail: "未找到技能: create-task-plans"
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据466：`desktop_skills::desktop_skills_tests::set_local_skill_enabled_toggles_disabled_marker`

位置：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs:639`；结果：FAIL。

```text
crates/libs/gateway-desktop/src/desktop_skills_tests.rs:656:6:
    disable skill: "未找到可管理的本地技能: demo"
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据467：`desktop_skills::desktop_skills_tests::set_local_skill_enabled_noops_for_current_state_and_errors_for_missing_skill`

位置：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs:675`；结果：FAIL。

```text
crates/libs/gateway-desktop/src/desktop_skills_tests.rs:686:6:
    enable already enabled skill: "未找到可管理的本地技能: demo"
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p gateway-desktop --lib 'desktop_skills::desktop_skills_tests::resolve_skill_detail_returns_personal_toml_skill_and_missing_errors' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-desktop --lib 'desktop_skills::desktop_skills_tests::resolve_skill_detail_returns_built_in_and_local_variants' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-desktop --lib 'desktop_skills::desktop_skills_tests::set_local_skill_enabled_toggles_disabled_marker' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-desktop --lib 'desktop_skills::desktop_skills_tests::set_local_skill_enabled_noops_for_current_state_and_errors_for_missing_skill' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。
