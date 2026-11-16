# 任务2：网关-api-handlers-agent_apps

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次2】 依赖：任务1

状态：未开始

来源任务：用户要求一次性完成代理宿主实质减负。

目的：完整迁出本文件族的领域职责与测试，宿主仅保留真实宿主端口和必要边界；不以迁几条用例收尾。

预计会话范围：同属网关现有文件族，共23个源文件、2538行；整个测试函数为最小移动单位。重导出文件不逐条扩读，仅读直接入口和测试实际依赖。

## 文件

以下为本任务唯一所有的现有来源（不是要求全部删除）：
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/access.rs`（2行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/aggregation_lifecycle_access.rs`（2行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/apps_usage_refresh.rs`（2行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/apps_usage_script.rs`（2行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/import_tests.rs`（39行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/launch.rs`（10行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/launch_tests.rs`（404行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs`（108行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle_tests.rs`（306行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/local_access_tests.rs`（155行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/mod.rs`（236行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/preferences.rs`（2行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/preferences_tests.rs`（137行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/read.rs`（2行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/read_tests.rs`（364行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/recovery_tests.rs`（62行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/remote_tests_tests.rs`（248行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_codex.rs`（2行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_native.rs`（2行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_read.rs`（2行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_refresh.rs`（2行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps_acceptance_tests.rs`（361行）
- `crates/vw-agent/src/gateway/api/handlers/agent_apps_read_tests.rs`（88行）

领域目标优先根据来源中现有直接调用选择：`crates/libs/gateway-agent-apps/src/`；其各自src/lib.rs、所属父模块和Cargo.toml的最小注册/依赖补丁由本任务排队完成。其它已有直接引用候选见scope.json本项targets，不默认向全部候选写入。
新建：实际接收库src/host_migration_2_tests.rs（测试多时在该模块下按行为分文件）、evidence/task-2.md。已有等价领域测试则逐项映射去重，不另建重复模块。
修改来源文件的直接父模块和调用方仅限消除旧挂载/转发与落实真实领域入口；使用前先读取当前内容。

## 上下文与符号

当前具体符号包括：`cc04_import_startup_consumes_only_authorized_empty_catalogs`、`terminal`、`failing_terminal`、`call`、`cc_apps_launch_api_target_scope_accepts_and_rejects_before_terminal`、`attached_terminal`、`cc_apps_launch_identity_list_sdk_http_tty_lifetime_and_account_rotation`、`cc_apps_launch_native_terminal_sdk_http_lifetime_and_rotation`、`identity_roundtrip`、`OwnedOperation`。核对其真实实现、现有Host trait、当前调用者及相邻测试；已有生产下沉不算本轮迁移收益。以同组全部函数/测试映射为验收单位，不只处理列出的示例符号。

## 步骤

1. 从上列来源进入，按 `cc04_import_startup_consumes_only_authorized_empty_catalogs`、`terminal`、`failing_terminal`、`call`、`cc_apps_launch_api_target_scope_accepts_and_rejects_before_terminal`、`attached_terminal`、`cc_apps_launch_identity_list_sdk_http_tty_lifetime_and_account_rotation`、`cc_apps_launch_native_terminal_sdk_http_lifetime_and_rotation`、`identity_roundtrip`、`OwnedOperation` 的直接定义/调用边界定位真实实现；将源清单全体用例加入本任务映射，不以示例符号代替全量范围。
2. 对照上列接收库现有入口实施公共“通用迁移动作”；按本任务注册名接入实际接收库，完整删除本任务来源中的旧定义/挂载，保留项逐符号说明。
3. 本任务负责来源直接父模块及接收模块的最小注册补丁、实际调用接线和依赖声明；证据逐项列出生产符号、领域测试与宿主边界，不把注册或清理交给收口任务。

## 静态验收及未来测试计划

本次不运行命令。保持行为迁移不制造红灯；现有用例名称/断言保留并逐条映射。若确有行为变更，先写失败测试代码并记录预期失败，未来同一精确过滤获取失败/通过证据。
未来接收库采用本任务注册名过滤（仅实际新建的库执行；等价去重则在证据列出本次读取到的完整真实测试名）：
- `python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib host_migration_2_tests -- --test-threads=1`
未来宿主薄用例按证据中保留的原完整名称过滤；整条迁出用真实宿主list证明消失，不能以零测试说通过。

当前所需证据：全文件/全用例映射；真实生产调用与依赖方向；领域自注册；原挂载删除；修改前后宿主及领域生产/测试分项行数；每个保留文件的具体职责及行数；未来命令与预期非零数；明确全部编译/行为检查未运行。不能把源文件没有业务实现写成生产迁移成果。

减量约束：优先迁整组领域测试，宿主保留必须逐符号说明。如果本组净减不足来源行数70%，必须在证据列出逐文件原因和继续可迁项，由收口任务验收；不能靠删除断言凑比例，也不能只删除任务文件就宣称达标。
