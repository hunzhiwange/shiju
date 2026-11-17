# 任务30：修复agent-security的path_policy测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：原任务19全量实际执行；证据编号58, 59, 60, 61, 62, 63, 64, 65, 66, 67。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据58：`policy::legacy_security_tests::path_policy::absolute_paths_blocked_when_workspace_only`

位置：`crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:24`；结果：FAIL。

```text
crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:26:5:
    assertion failed: !p.is_path_allowed("/etc/passwd")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据59：`policy::legacy_security_tests::path_policy::checklist_all_system_dirs_blocked`

位置：`crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:254`；结果：FAIL。

```text
crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:260:9:
    System dir should be blocked: /etc
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据60：`policy::legacy_security_tests::path_policy::checklist_root_path_blocked`

位置：`crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:246`；结果：FAIL。

```text
crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:248:5:
    assertion failed: !p.is_path_allowed("/")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据61：`policy::legacy_security_tests::path_policy::checklist_sensitive_dotfiles_blocked`

位置：`crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:270`；结果：FAIL。

```text
crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:274:9:
    Sensitive dotfile should be blocked: ~/.ssh/id_rsa
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据62：`policy::legacy_security_tests::path_policy::checklist_workspace_only_blocks_all_absolute`

位置：`crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:289`；结果：FAIL。

```text
crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:291:5:
    assertion failed: !p.is_path_allowed("/any/absolute/path")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据63：`policy::legacy_security_tests::path_policy::forbidden_paths_blocked`

位置：`crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:99`；结果：FAIL。

```text
crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:101:5:
    assertion failed: !p.is_path_allowed("/etc/passwd")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据64：`policy::legacy_security_tests::path_policy::full_autonomy_still_respects_forbidden_paths`

位置：`crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:171`；结果：FAIL。

```text
crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:177:5:
    assertion failed: !p.is_path_allowed("/etc/shadow")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据65：`policy::legacy_security_tests::path_policy::path_home_tilde_ssh`

位置：`crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:154`；结果：FAIL。

```text
crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:156:5:
    assertion failed: !p.is_path_allowed("~/.ssh/id_rsa")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据66：`policy::legacy_security_tests::path_policy::path_symlink_style_absolute`

位置：`crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:147`；结果：FAIL。

```text
crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:149:5:
    assertion failed: !p.is_path_allowed("/proc/self/root/etc/passwd")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据67：`policy::legacy_security_tests::path_policy::path_var_run_blocked`

位置：`crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:164`；结果：FAIL。

```text
crates/libs/agent-security/src/policy/legacy_security_tests/path_policy.rs:166:5:
    assertion failed: !p.is_path_allowed("/var/run/docker.sock")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p agent-security --lib 'policy::legacy_security_tests::path_policy::absolute_paths_blocked_when_workspace_only' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p agent-security --lib 'policy::legacy_security_tests::path_policy::checklist_all_system_dirs_blocked' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p agent-security --lib 'policy::legacy_security_tests::path_policy::checklist_root_path_blocked' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p agent-security --lib 'policy::legacy_security_tests::path_policy::checklist_sensitive_dotfiles_blocked' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p agent-security --lib 'policy::legacy_security_tests::path_policy::checklist_workspace_only_blocks_all_absolute' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p agent-security --lib 'policy::legacy_security_tests::path_policy::forbidden_paths_blocked' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p agent-security --lib 'policy::legacy_security_tests::path_policy::full_autonomy_still_respects_forbidden_paths' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p agent-security --lib 'policy::legacy_security_tests::path_policy::path_home_tilde_ssh' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p agent-security --lib 'policy::legacy_security_tests::path_policy::path_symlink_style_absolute' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p agent-security --lib 'policy::legacy_security_tests::path_policy::path_var_run_blocked' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。
