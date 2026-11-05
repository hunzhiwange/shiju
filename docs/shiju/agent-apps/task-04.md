# 任务04：安装实例共享类型

2026-11-06，macOS；纯 DTO，无 UI 或安装执行副作用。

## 依赖与边界

已核对任务01至03证据与真实产物：共享 AppId、14应用目录、ACP只读投影均已存在，vw-agent-apps 为正式 crate。参考只读 upstream/misc.rs 的 ToolInstallation 与 ToolInstallationReport。使用仓库 RustCodeGraph 技能检查索引与相关符号，新增文件未在图中时以文件读取补充。本任务无认证、操作或存储调用，不依赖未交付的执行主机接口，不创建主机 RPC。来源、总计划保持只读。

新增 agent_apps/installations.rs，并由既有 agent_apps 模块导出；lib.rs 已有公开模块，无需重复修改。独立测试在 installations_tests.rs。

每处安装独立包含服务器分配的实例 ID、AppId、目标主机、macOS/Windows/Linux/WSL 环境（WSL 必带发行版）、入口及可选真实路径、来源、默认入口、版本与运行状态。来源覆盖上游 nvm/homebrew/volta/fnm/mise/bun/pnpm/scoop/pip/system，并保留 native/unknown 的明确区分。非零退出携带 exitCode，与启动失败、超时等探测失败分离，避免把探测失败误判为安装损坏。错误仅为固定枚举，不传原始 stderr 或凭据。

DTO 路径为目标 OS 的诊断字符串，不在客户端解释为本机路径。DTO 可反序列化不代表可信；没有接收路径的执行请求或执行能力。后续操作必须在认证主机上下文按实例 ID 重新解析并核验，不能把路径或 targetHostId 当作授权。本任务不增加冲突策略、升级命令或报告衍生业务逻辑。

## TDD 与验证

红、绿两个阶段各自以原子 mkdir 动态领取槽位2，owner 记录任务、会话 shell PID。两个环境变量均为绝对路径：

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

后者覆盖 .cargo/config.toml 中独立的 build.build-dir，避免中间产物逃离槽位。阶段 shell 使用 EXIT trap 清理自己的 owner 和锁；编译缓存保留。

红：`cargo test -p vw-api-types --lib cc_apps_installation_roundtrip -- --nocapture`，退出101，E0432：缺少 AppInstallationReport 目标契约，0项执行。测试先于实现加入；不是工具或无关编译错误。

`cargo fmt --all`，退出0。审查后没有无关格式化差异。

绿：`cargo test -p vw-api-types --lib cc_apps_installation_roundtrip -- --nocapture`，退出0，3/3通过，68项过滤。覆盖同一报告四种环境及独立来源/主机/实例ID、Windows路径和缺少真实路径/版本、未知来源保留、未知枚举及缺失必填字段拒绝、WSL缺少发行版拒绝、非零退出与超时/启动失败区分、空报告往返。Windows/WSL为在macOS运行的序列化夹具，不代表其他平台实机执行证据。

`cargo clippy --all-targets --all-features -- -D warnings`，退出0，1m23s，无警告。`git diff --check`退出0。审查范围仅新增DTO、独立测试、模块声明及本证据；前序任务1至3的既有删除不纳入提交。所有验证子进程已结束，槽位2锁已释放，缓存保留。状态：已完成；推荐执行任务5。
