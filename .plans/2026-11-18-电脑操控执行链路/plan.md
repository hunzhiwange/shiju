# 电脑操控执行链路计划

目标：让识矩以可识别、可授权、可取消的执行进程完成本机应用观察、输入及拖动，并可由技能调用真实工具。

范围：审计现有未验证改动；构建有稳定身份的本机控制服务、受鉴权 IPC、网关接线、授权诊断、macOS 打包及工具使用技能。以本机原生应用为本计划验收对象，核对其他项目可复用的机制。

范围外：复用 Codex 私有二进制、身份或 socket；Chrome 历史、CDP、WebMCP 及网站权限的完整实现；Office、锁屏支持、Windows/Linux。Chrome 共用须另立浏览器专项，不以本计划完成替代。

假设：当前目标是 macOS；继续使用现有 Rust 原生控制模块。独立 App 是选定的产品交付方案，并非操作系统或所有项目的必需条件。签名凭据可用性尚未证实，缺失时记录发布验收阻塞。

## 设计决策

- 技能＋CLI 最轻，但依赖既有驱动；采用现有 Rust 控制能力＋独立签名 App 保持授权身份；第三方驱动可缩短实现但增加分发与安全审计成本，本轮不新增。
- 网关保留用户和目标策略，控制 App 承担真实系统调用；不复制 OpenAI Bundle ID 或使用 Codex 已获授权。

## 架构说明

- 复用 `computer-control`、`tool-computer::NativeBackend` 与版本化授权；新增 IPC 承载执行，不改模型工具注册 key。
- 技能仅编排现有工具；CLI/MCP 是入口，均不能替代执行器、系统授权或目标校验。

## 依据与其他项目

- Cua Driver：技能调用已安装 CLI，另有 MCP 入口；其 macOS 权限属于实际驱动进程。https://github.com/trycua/cua/blob/main/docs/content/docs/cua-driver/quickstart.mdx
- Playwright：CLI＋技能或 MCP 调用浏览器执行能力；扩展连接现有标签页。共用浏览器不意味着重复启动相同 profile，也不意味着共用 Codex 的私有协议。https://github.com/microsoft/playwright-mcp/blob/main/README.md
- 本机 `/Users/xiangmin/.codex/computer-use/Codex Computer Use.app` 存在，执行文件 SkyComputerUseService，Bundle ID com.openai.sky.CUAService，Developer ID 签名；运行进程持有 computeruse.sock。本次仅观察，没有调用其私有协议。
- `/Users/xiangmin/worker/code/codex-main` 找到配置、发现和钩子，尚未找到上述服务的源码。识矩已有 NativeObserver、策略和 IPC，不应写成完全没有底层执行能力。

## 开发策略

行为变更先新增独立文件中的失败测试，再实现、取得通过证据，最后收敛修改。只模拟 macOS 权限、输入、截图等不可确定的 OS 边界；序列、策略、取消、IPC 尽量真实。物理桌面、TCC 和签名不能以模拟测试替代。

## 公共执行约定

- 每个编号任务会话负责自身实现、父模块/清单接线、验证、提交和任务文件清理；调度者只派发、等待、判定及恢复，不代改代码。
- 使用 `/Users/xiangmin/.codex/skills/do-task-plans/SKILL.md`。每项任务先读本文件与仓库 AGENTS.md；本文件生成后只读，进度仅写编号文件。
- 在当前分支和当前目录工作，不创建分支或 worktree。工作区已有大量修改，逐块识别本任务内容，不覆盖、暂存或提交无关改动。
- 无人值守推进常规、可逆、低风险决策；查源码及调用者后选择最小方案并记依据。关键歧义或新增授权才集中澄清，未回复不构成批准。
- 先排查底层原因，不削弱策略或扩大权限；明确不支持能力。局部受阻继续独立任务，记录失败证据、尝试及所需输入。
- 全局构建清单的新增 bin 由任务2负责，打包资源归任务3，工具工厂接线归任务5；任务4在任务2完成后写共享输入协议。不要将公共接线交给未编号协调者。

## 公共验证与资源规则

- 创建计划期间不运行测试、Cargo 或桌面构建。执行计划时，各任务列出的必要定向测试属于任务验收；仓库禁止默认全量测试、workspace 构建、全特性 Clippy、全仓格式、wasm 和 Makefile。
- 运行目录均为 `/Users/xiangmin/worker/code/shiju`。定向测试需记录有效测试数、退出码及当前证据；零测试或只编译成功不能算行为通过。同一目标已被测试编译，不重复 check。
- 非 Rust 使用现有 Python unittest 和 Node test 入口。物理输入与 TCC 授权要求实际用户会话，不自动批准系统授权，不控制无关应用。
- 仅格式化修改文件。统一回归只补跨任务集成，合并重复公共命令，不机械重跑未受后续修改影响的测试。

### Rust 专用资源规则

- 遵从本仓库 AGENTS.md，使用已核实的 `python3 scripts/cargo_slot.py cargo ...` 入口；这是对通用技能“原生 Cargo”要求的仓库专用等价调整。wrapper 将参数原样交给 Cargo。不要传 `--help` 或额外分隔符给 wrapper。
- 高内存编译串行排队，仅约束验证阶段，不把缓存竞争当任务依赖。Cargo 使用 `-j 1`，复用脚本动态领取的已有槽位和增量缓存，不固定按任务编号分配，不创建额外缓存或清缓存。
- 脚本实际支持1–20个共享槽位，继承校验 token/PID，设置 CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 到同一绝对路径；采用仓库脚本生命周期，不另建通用技能所述的10槽位池。
- 记录运行句柄、进程组、槽位、双缓存绝对路径和命令；超时或取消须经原句柄结束自有 Cargo/rustc/链接器及使用产物的子进程，确认退出才释放自有锁。禁止按进程名杀进程或删除他人锁。失败后不得无条件启动后续重型检查。

## 公共收尾规则

- 状态仅 `未开始`、`已完成`、`已阻塞`、`待回归`。完成需当前任务验收证据；回报证据后删除本编号文件，保留 plan.md 和 prompt.md。
- 合理排障仍受阻：保留编号文件并记录证据、尝试、所需输入。实现完成仅因无关条件无法验证可记待回归；自身接线没完成不能记待回归。
- 有实际代码、配置、测试或文档变更时，不论完成、待回归或阻塞，都先使用 `$git-commit`（`/Users/xiangmin/.codex/skills/git-commit/SKILL.md`）提交本任务变更；如实说明未完成验证。只有编号文件状态更新/删除或无实际改动不创建提交。提交失败保留任务文件记录原因。提交时仅暂存自身差异。
