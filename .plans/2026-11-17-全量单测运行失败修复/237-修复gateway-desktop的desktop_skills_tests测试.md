# 任务237：修复gateway-desktop的desktop_skills_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：原任务19全量实际执行；证据编号452, 453, 454, 455, 456, 457, 458, 459, 460, 461, 462, 463。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据452：`desktop_skills::desktop_skills_tests::built_in_skill_helpers_read_metadata_and_mark_recommended_group`

位置：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs:283`；结果：FAIL。

```text
crates/libs/gateway-desktop/src/desktop_skills_tests.rs:285:9:
    bundled skill metadata; discovered: []
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据453：`desktop_skills::desktop_skills_tests::collect_catalog_skills_includes_personal_items_and_preserves_bundled_defaults`

位置：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs:494`；结果：FAIL。

```text
crates/libs/gateway-desktop/src/desktop_skills_tests.rs:508:72:
    personal item
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据454：`desktop_skills::desktop_skills_tests::collect_catalog_skills_orders_personal_items_by_title_then_id`

位置：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs:557`；结果：FAIL。

```text
crates/libs/gateway-desktop/src/desktop_skills_tests.rs:574:5:
    assertion `left == right` failed
      left: []
     right: ["task-50-alpha", "task-50-zeta"]
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据455：`desktop_skills::desktop_skills_tests::collect_catalog_skills_uses_local_metadata_for_built_in_override`

位置：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs:521`；结果：FAIL。

```text
crates/libs/gateway-desktop/src/desktop_skills_tests.rs:537:76:
    catalog entry
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据456：`desktop_skills::desktop_skills_tests::create_new_skill_scaffold_uses_next_available_name`

位置：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs:240`；结果：FAIL。

```text
crates/libs/gateway-desktop/src/desktop_skills_tests.rs:253:5:
    assertion `left == right` failed
      left: Some("new-skill")
     right: Some("new-skill-2")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据457：`desktop_skills::desktop_skills_tests::delete_local_skill_removes_directory`

位置：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs:711`；结果：FAIL。

```text
crates/libs/gateway-desktop/src/desktop_skills_tests.rs:727:6:
    delete skill: "未找到可管理的本地技能: demo"
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据458：`desktop_skills::desktop_skills_tests::discover_local_skills_ignores_non_skills_and_reports_disabled_state`

位置：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs:612`；结果：FAIL。

```text
crates/libs/gateway-desktop/src/desktop_skills_tests.rs:630:76:
    disabled skill
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据459：`desktop_skills::desktop_skills_tests::discover_local_skills_reads_workspace_skills_and_deduplicates`

位置：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs:578`；结果：FAIL。

```text
crates/libs/gateway-desktop/src/desktop_skills_tests.rs:603:5:
    assertion `left == right` failed
      left: "Duplicate Demo"
     right: "Primary Demo"
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据460：`desktop_skills::desktop_skills_tests::install_built_in_skill_copies_nested_resources`

位置：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs:478`；结果：FAIL。

```text
crates/libs/gateway-desktop/src/desktop_skills_tests.rs:489:5:
    assertion failed: skill_dir.join("SKILL.md").is_file()
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据461：`desktop_skills::desktop_skills_tests::install_built_in_skill_copies_once_and_collect_catalog_marks_installed`

位置：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs:403`；结果：FAIL。

```text
crates/libs/gateway-desktop/src/desktop_skills_tests.rs:423:76:
    catalog entry
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据462：`desktop_skills::desktop_skills_tests::installed_skills_preserve_market_source_metadata`

位置：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs:442`；结果：FAIL。

```text
crates/libs/gateway-desktop/src/desktop_skills_tests.rs:468:10:
    installed market skill
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据463：`desktop_skills::desktop_skills_tests::resolve_skill_detail_reports_disabled_local_skill_capabilities`

位置：`crates/libs/gateway-desktop/src/desktop_skills_tests.rs:376`；结果：FAIL。

```text
crates/libs/gateway-desktop/src/desktop_skills_tests.rs:391:6:
    disabled local detail: "未找到技能: disabled-detail"
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p gateway-desktop --lib 'desktop_skills::desktop_skills_tests::built_in_skill_helpers_read_metadata_and_mark_recommended_group' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-desktop --lib 'desktop_skills::desktop_skills_tests::collect_catalog_skills_includes_personal_items_and_preserves_bundled_defaults' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-desktop --lib 'desktop_skills::desktop_skills_tests::collect_catalog_skills_orders_personal_items_by_title_then_id' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-desktop --lib 'desktop_skills::desktop_skills_tests::collect_catalog_skills_uses_local_metadata_for_built_in_override' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-desktop --lib 'desktop_skills::desktop_skills_tests::create_new_skill_scaffold_uses_next_available_name' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-desktop --lib 'desktop_skills::desktop_skills_tests::delete_local_skill_removes_directory' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-desktop --lib 'desktop_skills::desktop_skills_tests::discover_local_skills_ignores_non_skills_and_reports_disabled_state' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-desktop --lib 'desktop_skills::desktop_skills_tests::discover_local_skills_reads_workspace_skills_and_deduplicates' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-desktop --lib 'desktop_skills::desktop_skills_tests::install_built_in_skill_copies_nested_resources' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-desktop --lib 'desktop_skills::desktop_skills_tests::install_built_in_skill_copies_once_and_collect_catalog_marks_installed' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-desktop --lib 'desktop_skills::desktop_skills_tests::installed_skills_preserve_market_source_metadata' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-desktop --lib 'desktop_skills::desktop_skills_tests::resolve_skill_detail_reports_disabled_local_skill_capabilities' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。
