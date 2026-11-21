# 任务 5 适配 MCP 和技能的会话生命周期

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 1】 依赖：无

状态：未开始

来源任务：无

目的：复用现有挂载设施，提供原生与外挂共存、隔离和撤除。

## 文件与范围

修改 `crates/libs/agent-app-services/src/mcp/acp_mount.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/mod.rs` 的共用视图部分及相邻独立测试。逐适配器分派由任务 48 修改。

## 上下文

ACP 按会话提供服务与技能目录。视图及撤权应沿用现有生命周期。

## 实施

1. 核对 stdio、HTTP/SSE 各适配器已用传输的路径及权限。复用当前会话名单和撤权逻辑。保持原生目录，按源处理同名冲突和显式拒绝。
2. 核对 skill 视图在 Linux 的目录创建、中文路径、同名冲突、非法链接和清理。无法证明隔离的挂载失败关闭。
3. 测试先读原生技能和外挂技能的独立标记，验证新建、恢复、支持时的分叉及撤除。MCP 验证独立执行回执及结果续轮，撤除后零调用。

## 验证

- 先写独立回归测试，使用 `acp_platform_mount_lifecycle` 作为测试名或前缀。旧行为预期失败点为恢复沿用了旧挂载或外挂遮蔽原生条目。
- 后续在 Linux Docker 容器内使用同一红绿命令 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_mount_lifecycle`。仅列为后续命令，本轮不执行。绿灯要求命中本任务测试且零失败。
- 本轮静态逐项检查测试断言、调用者接线及本任务差异。记录受影响目标和待运行项。

MCP 目录及撤销专属红绿测试另用 `python3 scripts/cargo_slot.py cargo test -j 1 -p agent-app-services --lib acp_platform_mcp_revoke`。测试撤权时并发定位旧服务的行为，绿灯须命中用例且零失败。
