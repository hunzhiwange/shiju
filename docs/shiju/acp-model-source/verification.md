# 真实 CLI 离线模型验收

此工具调用生产 `do_stream_request_acp`，不实现 ACP agent。Python HTTP fixture 只模拟
模型上游；即使工具自测通过，也不代表任何供应商 CLI 已支持。当前 22 个 driver 均默认
拒绝接管；没有对应 driver 及审查过的原生配置配方时，真实验收不得通过。

先按编号任务的共享槽位规则，以原子 mkdir 领取 `target/rust-slot-locks/slot-N.lock`，
写入会话、任务、PID，再设置绝对 `CARGO_TARGET_DIR=$PWD/target/rust-slot-N`。
脚本检查目录和占用记录，不领取或创建槽位，不接受任意 Cargo 命令。
调用方在脚本及全部子进程结束后释放自己的锁，保留编译缓存。

```sh
python3 -m unittest discover -s scripts -p 'acp_model_source_fixture_tests.py'
python3 scripts/verify_acp_model_source.py --agent codex --mode session-model \
  --manifest /absolute/path/to/reviewed-cli-versions.json \
  --output /absolute/path/to/session-model-result.json --offline-fixture
```

第二种来源使用 `--mode cli-local`。未设置 `--offline-fixture` 时仍只运行本地 fixture；
没有生产/付费上游选项。两种模式各固定选择一个具名 ignored Rust 测试，要求恰好
1 passed、实际 HTTP 请求、认证/模型/路径正确、两次工具结果和全部生命周期断言。
未注入 manifest 时直接运行 ignored 入口会失败，不能跳过或返回零测试成功。
普通契约验证命令为：

```sh
cargo test -p vw-agent --lib --all-features acp_model_source_harness_contract -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

## Manifest 与配置隔离

基线为 `cli-versions.json` 的 `entries`。按 agent key 唯一选择条目，只运行已安装的
`local_command_candidates`，核对 `distribution.release` 与 `installed.command` 的固定
`--version` 探测；独立底层 CLI 还需 `underlying.command`（同样只能是 `--version`）和
`underlying.release`。不运行 npx/npm/uvx、不隐式安装、不使用 latest 启动。
版本声明和基线快照不能代替当前已安装版本证据。
当前真实运行仅启用 POSIX：Windows 尚未验证原生文件 ACL 和子进程组清理，明确拒绝。
Cargo 运行结束、超时或异常退出时都会清理其进程组，避免遗留 CLI 占用槽位产物。

每个 driver 任务必须在用于验收的 manifest 副本添加经该实际版本源码审查的
`offline_acceptance.session-model` 和 `offline_acceptance.cli-local` 配方。配方包含：

- `config_root_env`：CLI 官方专属原生配置目录变量，禁止 HOME、USERPROFILE、PATH。
- `files`：相对此目录的原生配置文件名到文本的映射；禁止绝对路径、路径穿越或外部 URL。
- 文本占位符：`{{endpoint}}`、`{{key}}`、`{{model}}`、`{{root}}`。

这里定义工具输入契约，不为任何 CLI 臆测配置格式。没有经审查的配方即 unavailable；
对于只支持参数配置目录而没有专属环境变量的 CLI，当前工具明确拒绝，需在对应 driver
任务证明入口后扩展。配方必须隔离全部原生配置和账号目录，不能只覆盖模型字段后仍读取
用户账号。版本探测前就设置专属目录。HOME 保留原值，不重定位用户主目录。
文件使用独占创建、目录 0700、文件 0600，临时原生配置不会读取或更改用户 CLI 配置。

session-model 的原生配置故意注入错误模型和错误 fixture 凭据，生产 profile 必须覆盖它们；
回退本地账号会被拒绝。cli-local 原生配置指向本地 fixture；更换客户端模型后，CLI 仍须
请求原生配置中的 primary 模型。上游始终使用 Chat fixture，其他 CLI 协议必须经过生产
中继转换；不以模型服务兼容性伪装缺失的 driver。

## 证据边界

Rust 入口使用唯一会话标识调用生产请求链：首次请求、第二轮、模型请求发出后的取消、
关闭进程后通过已有记录恢复、同来源更换客户端模型。fixture 使用真实 CLI 宣告的
read/read_file/readfile 工具发起两次读取 `challenge.txt`；不认识的工具 schema 或未返回
两轮工具结果会失败，不伪造成功。真实 CLI 的能力差异应由对应 driver 任务适配。
应用 ACP 会话记录仍遵循生产持久化路径，使用随机会话名；原生 CLI 配置单独隔离。

JSON 的 `evidence_kind=real-cli` 仅标识执行路径，必须同时 `passed=true` 才是通过证据。
`unavailable`、`version-mismatch`、`handshake-only`、`failed` 均非成功。工具自测属于
mock-flow，只证明工具不会接受假成功。fixture 证据由 HTTP 处理器独立生成，不采用 CLI
输出中的 model 字段。错误认证、未知路径、错误模型及辅助请求都会使验收失败。
输出只保存模型、路径、认证布尔值、工具轮数、生命周期结果及固定命令；不保存令牌、
请求体或供应商 stdout/stderr。超时/构建失败也不会回显原始日志。控制凭据与模型凭据分离。

本工具的合同测试不能代替 22×2 项真实验收，也不能将缺 driver、缺 CLI、只握手或负向
错误记为支持。发布全量能力前还需逐项保存两个来源的成功结果并完成任务 35 回归。
