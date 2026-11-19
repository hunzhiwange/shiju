# GLM ACP 跨平台接入计划

目标：让 ShiJu 支持的 Windows、Linux 和 macOS 平台均可使用 GLM 会话模型与受策略约束的 MCP，官方工具可按现有权限共存。

范围：GLM 1.17.1 的固定 npm 包、受管补丁、安装、启动、状态目录、能力声明和端到端验收。按 `scripts/prepare_oss_release.py` 的六个 OS/架构组合验证。

范围外：其他 ACP 的移植，改写原任务文件，桌面构建和发布。

假设：用户提到的补丁指 GLM 的当前受管补丁。JavaScript 差异可以在目标平台共用，仍须以平台实测确认 npm 依赖与入口行为。

## 已核验的现状

- `crates/libs/acp-agent-registry/src/managed_runtime.rs` 的 `install` 仅在 macOS arm64 进入 GLM 受管安装。Linux、Windows 和 macOS x86_64 回退到全局原版 npm 路径。当前 JS 补丁没有系统或架构条件；现有安装依赖名为 `patch` 的外部程序，启动校验还假设 npm 的无后缀入口指向受审源码。
- `crates/libs/llm-runtime/src/acp/model_source/drivers/mod.rs` 当前只在 macOS arm64 宣告 `session_model`。GLM 私有状态目录驱动使用 Unix 权限校验，Windows 分支明确拒绝。Linux 有 Unix 实现。
- 原版 GLM 已原生接收模型配置和 ACP MCP。当前补丁执行 ShiJu MCP 服务级工具名单并处理错误事件；每个服务都有非空允许名单时还会启用 MCP-only。模型源启动的受审校验目前要求补丁产物。见 [现有验收](../../docs/dev/acp/mcp-acceptance/glm-acp-agent.md)。

## 设计决策

沿用固定发行包和受审制品链，实现同一受管路径跨平台。直接放行全局原版无法落实显式名单和入口校验。平台缺口单独补齐，原生工具按现有权限共存。

## 公共执行约定

- 从仓库根目录执行。读取仓库 `AGENTS.md` 和本计划，使用 `/Users/xiangmin/.codex/skills/do-task-plans/SKILL.md`。用仓库 RustCodeGraph 技能导航。现有计划及任务卡仅供参考，本计划和入口生成后只读。
- 各任务会话负责本任务接线、定向验证、结果、提交和收尾。共享文件按编号任务的所有权与依赖写入。主调度只派发、等待、判定和恢复。当前分支工作，保留其他工作区变更。
- 常规可逆事项自行解决。局部受阻时保留实际命令、错误和缺失证据，继续独立工作。涉及真实凭据时使用已授权隔离环境，日志只保存脱敏信息。禁止扩大权限、绕过受审校验、修改用户全局配置或执行发布。
- 行为改动先有失败用例，再用同一命令验证修复。仅在执行获得验证授权后运行相关包和目标。原生系统授权及运行验证使用目标 OS。编译成功、交叉编译或模拟协议无法代替实机。
- Rust 定向构建在 macOS/Linux 用仓库 `scripts/cargo_slot.py`，重型验证串行，复用缓存并限制 `-j 1`。脚本的运行入口明确拒绝 Windows。Windows 在原生隔离验证机用现有 `slot()` 上下文复用同一槽位约定，原生 Cargo 前台执行。退出前核对自有构建及产物进程已退出，保留缓存。记录实际槽位、两个缓存路径、命令、退出码及有效测试数。同目标已由测试编译时省略 check。
- 状态使用 `未开始`、`已完成`、`已阻塞`、`待回归`。有实际变更时用 `$git-commit` 提交本任务相关文件，排除无关变更，Git 暂存和提交串行。仅任务状态或清理时不建空提交。已完成须报告新证据后清理编号任务，阻塞、待回归或提交失败时保留对应任务文件。

### Windows 槽位入口

从原生 Windows 验证机的仓库根目录执行下例。各任务沿用同一入口并替换 `cargo` 后的参数。使用隔离验证机，超时或取消时先从运行会话收尾本任务的进程树。

```powershell
python -c 'import sys, subprocess
from scripts.cargo_slot import slot
with slot() as env:
    result = subprocess.run(sys.argv[1:], env=env)
raise SystemExit(result.returncode)' cargo test -j 1 -p acp-agent-registry --lib glm_managed_runtime_
```

## 批次与验收原则

批次 1 完成补丁语义、受管安装与 Windows 私有状态目录。这三个任务各有独立代码所有权，安装任务可用当前受审包验证安装机制。批次 2 接通模型来源能力，依赖前述产物和驱动。批次 3 执行跨平台真实路径并写验收记录。

六个目标是 macOS、Linux、Windows 各 x86_64 和 arm64。普通会话保留内置工具。无挂载、挂载、撤除分别开新会话。挂载确认 RustCodeGraph 真执行并回传给指定模型，官方工具可按原权限执行，显式 MCP 名单及宿主拒绝生效。负向覆盖受审产物被修改、状态目录不安全、未获权限或会话已撤除。实际运行证据按 OS 和架构记录，缺少环境标记待回归。
