# 宿主源码静态基线

## 范围与状态

按用户本次覆盖指令，仅生成源码静态清单和归属核对。不运行 Cargo、单元测试或实机测试；验证统一延期。任务1实现完成，删除编号任务文件；plan.md/prompt.md 保持只读。

- 基线 HEAD：`995c8028c00fbb65921f776f75e995f96d2c08c4`。
- 开始时工作区：`## shiju...origin/shiju [ahead 3]`（无文件变更）。
- Rust 版本：未采集，本次仅做源码静态清单，未调用工具链。
- RustCodeGraph status：退出码0，索引文件12099，索引时间戳1794811895249；静态查询未提供编译归属证据。
- 原计划的宿主 lib 实际测试数：未知；未运行清单命令，退出码不适用。
- 本次具名源码候选：19 条，函数定义均找到一次。清单见 before-host-tests.txt；候选路径由下述模块链推导，不能替代实际 lib-test 清单。
- 未运行：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib -- --list`，以及全部定向测试、构建、格式化和公共检查；无通过数、失败数或 ignored 数证据。

## 物理行数口径

本计划未规定历史数值，当前固定使用文件字节按 splitlines() 计物理行（含注释与空行，末行无换行仍计一行），只统计磁盘文件，不将其当作编译归属。

- `crates/vw-agent/src/**/*.rs`：1381 个文件，135833 行。
- 外部 path 挂载文件不重复计入上述宿主物理行数；其宿主注册归属单独记录。

| 文件 | 物理行数 |
| --- | ---: |
| `crates/libs/session-engine/tests/host_suites/prompt_tests.rs` | 100 |
| `crates/vw-agent/Cargo.toml` | 564 |
| `crates/vw-agent/src/gateway/api/handlers/config.rs` | 149 |
| `crates/vw-agent/src/gateway/api/handlers/config_tests.rs` | 1738 |
| `crates/vw-agent/src/lib.rs` | 382 |
| `crates/vw-agent/src/session/compaction.rs` | 67 |
| `crates/vw-agent/src/session/compaction_tests.rs` | 1011 |
| `crates/vw-agent/src/session/prompt.rs` | 17 |
| `crates/vw-agent/src/task_plan_run/mod.rs` | 58 |
| `crates/vw-agent/src/task_plan_run/runner_tests.rs` | 1869 |

## 指定用例的静态注册归属

- lib.rs 声明 `session`、`task_plan_run`；`gateway` 受 `not(target_arch = "wasm32")` 限制。Cargo.toml 的 `autotests = false` 和 `[lib] doctest = false` 不改变这些显式 lib 模块挂载。
- 任务2：`lib.rs → session/mod.rs → prompt.rs → prompt_tests`。prompt.rs 在 `cfg(test)` 下通过 `#[path = "../../../libs/session-engine/tests/host_suites/prompt_tests.rs"]` 挂载外部文件。磁盘位于领域目录，候选编译归属仍是 vw-agent；session-engine/src/lib.rs 未见该 host_suites/prompt_tests 注册。
- 任务3/4：`lib.rs → gateway/mod.rs → api/mod.rs → handlers/mod.rs → config.rs → config_tests`。config.rs 在 `cfg(test)` 下挂载本目录 config_tests.rs；仅非 wasm 宿主链。
- 任务5/6：`lib.rs → task_plan_run/mod.rs → runner_tests`。mod.rs 在 `cfg(test)` 下挂载本目录 runner_tests.rs。
- 任务7/8：`lib.rs → session/mod.rs → compaction.rs → compaction_tests`。compaction.rs 在 `cfg(test)` 下挂载本目录 compaction_tests.rs。
- 上述四个直接测试注册均为 path 模块，未见 include! 注册。这里只核对指定测试直接链，不审计全部宿主路径、领域断言或重复覆盖；各迁移任务负责自身断言映射。

## 采集方式与限制

使用 Python 标准库读取任务“指定测试”段、对应源码函数签名并生成清单；使用 rg 检索直接父模块的 mod/path/cfg/include 声明，读取宿主 lib.rs 和 Cargo.toml。未加载各大测试文件的行为正文。清单不评估宏展开、目标条件、依赖编译或实际测试发现；后续统一验证必须补充真实宿主/领域 lib-test 清单与有效测试结果。任务9静态对比应沿用此范围与行数口径，不将19条候选误作全宿主测试数。
