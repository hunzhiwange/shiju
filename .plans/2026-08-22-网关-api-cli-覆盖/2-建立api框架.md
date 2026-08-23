# 任务 2: 建立 api CLI 基础框架

批次：【批次 2】 依赖批次 1

状态：已完成，待回归

目的：新增统一的远程 Gateway 命名空间、连接上下文、JSON 输入输出和危险操作确认。

来源任务：无

预计会话范围：只搭建一条示例只读命令所需的通用框架，不批量实现业务接口；框架需可被后续接口族复用。

## 文件

- 修改：`crates/vw-cli/src/cli.rs`
- 修改：`crates/vw-cli/src/lib.rs`
- 修改：`crates/vw-cli/src/handlers/mod.rs`
- 新建：`crates/vw-cli/src/handlers/api/mod.rs`
- 新建：`crates/vw-cli/src/handlers/api/context.rs`
- 新建：`crates/vw-cli/src/handlers/api/input.rs`
- 新建：`crates/vw-cli/src/handlers/api/output.rs`
- 测试：`crates/vw-cli/src/handlers/api/tests.rs`

## 上下文

- `GatewayEndpoint` 已支持 host、port、Bearer skey 与 `TenantScope`。
- 全局 API 参数必须包含 host/port、platform/company、skey 环境变量或文件、directory、output 和 `--yes`。
- 复杂 JSON 从文件或 `-` 代表的 stdin 读取；禁止把密钥设计成命令行明文参数。

## 测试计划

- 行为：`shiju api` 能构造租户化客户端、读取安全输入、稳定输出 JSON/JSONL，并在未确认时拒绝危险操作。
- 失败验证测试：`crates/vw-cli/src/handlers/api/tests.rs` 中的 `api_cli_context_input_output_and_confirmation_are_stable`
- 失败验证命令：`cargo test -p vw-cli api_cli_context_input_output_and_confirmation_are_stable --lib`
- 预期失败原因：仓库尚无 `Commands::Api`、共享 Gateway 上下文或结构化输出边界。
- 通过验证命令：`cargo test -p vw-cli api_cli_context_input_output_and_confirmation_are_stable --lib`
- 模拟策略：以本地一次性 HTTP 服务器验证真实请求头和输出 writer；只模拟 Gateway 网络边界。

## 步骤

1. 先写解析、租户头、认证来源、stdin/file 输入、输出和确认失败测试。
2. 在 `cli.rs` 只增加薄的 `Api` 入口，把实现放入 `handlers/api`。
3. 实现共享上下文，默认 `local/local` 租户，拒绝空 ID、空认证文件和非 JSON 输入。
4. 实现 JSON 默认输出、JSONL 流输出及非交互危险操作默认拒绝。
5. 用只读示例命令贯通解析到 GatewayClient，再运行验证。

## 验证

- 运行：`cargo test -p vw-cli api_cli_context_input_output_and_confirmation_are_stable --lib`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：基础命令解析和真实测试请求通过，认证不出现在输出，危险操作无 `--yes` 时失败，clippy 通过。
- 所需证据：失败验证缺少 Api 入口；通过后请求头断言、输出快照、拒绝分支均成功；测试 0 失败；clippy 0 警告。

## 完成

`crates/vw-cli/src/handlers/api/` 成为后续接口族唯一共享 CLI 边界，`cli.rs` 与 `lib.rs` 仅做薄注册和分发。

## 回归记录（2026-08-23）

- 已实现 `shiju api provider list` 示例及共享 context/input/output/确认边界。
- 失败验证已确认缺少 `Commands::Api`、context/input/output 与确认函数。
- 隔离执行同一目标测试：1 通过、0 失败；真实请求头、输出快照、认证脱敏和拒绝分支均通过。
- `cargo test -p vw-cli api_cli_context_input_output_and_confirmation_are_stable --lib` 已重试三次，均在编译 `vw-agent` 时被本任务外错误截断；最近一次为 PostgreSQL/存储层 102 个编译错误，未进入 `vw-cli`。
- `cargo clippy --all-targets --all-features -- -D warnings` 已运行，因 `vw-agent` 的 102 个库错误及 489 个测试目标错误退出 101；首批错误包括多个 PostgreSQL 文件的悬空文档注释、类型推断失败，以及 Lark 测试支持符号缺失。
- 未修改上述无关文件；待工作区相关并发改动恢复可编译后，重新运行两条规定命令。
