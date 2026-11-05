# 任务 77: 核对macOS来源覆盖与证据

批次：【批次 77】 依赖批次 76；此前各批已验收；共同前置为母任务 1–3

状态：未开始

目的：核对来源索引逐项已有实现/API/SDK/UI/测试证据和 14 应用 macOS 支持矩阵，明确未验证项。

来源任务：母任务 4；需求映射见本目录来源映射.md；源码入口如下。

预计会话范围：只证明“核对macOS来源覆盖与证据”这一个行为，新增接口仅服务本行为，不同时迁移其他应用或重构共享设施。依赖完成后无需阅读其他全部任务。

## 文件

- 新建或补充：`docs/shiju/agent-apps/provider-macos-acceptance.md`。

## 上下文

上游来源根为 `/Users/xiangmin/worker/code/cc-switch/`，快照提交见来源映射。新增路径是拟定落点，任务 1 核对后沿用同职责已有模块。仅验收 macOS；业务归执行端，API/SDK 走母任务 2 权限和错误契约。

- 先读来源：`/Users/xiangmin/worker/code/cc-switch/src-tauri/src/commands/provider.rs` 及其直接调用者和相邻测试。
- 先读来源：`/Users/xiangmin/worker/code/cc-switch/src/hooks/useProviderActions.ts` 及其直接调用者和相邻测试。
- 先读来源：`/Users/xiangmin/worker/code/cc-switch/src/components/providers/ProviderList.tsx` 及其直接调用者和相邻测试。

## 实现边界

- 核对来源索引逐项已有实现/API/SDK/UI/测试证据和 14 应用 macOS 支持矩阵，明确未验证项。
- 只做证据审查不补代码；真实应用缺安装、账号或路由运行时则登记未验证，不将本母能力标完整。更新 docs/shiju/agent-apps/provider-macos-acceptance.md，不修改任何 plan.md 或母任务状态。
- 先搜索真实调用者和相邻测试；行为相同的现有实现直接接通，不能复制永久业务所有者。缺少依赖只在本任务登记，不能伪造前置完成。

## 验证计划

- 测试驱动开发不适用：本任务只核对来源/契约/证据，不改变运行行为。Cargo 规则不适用，不领取槽位。
- 清单：每条来源索引均有唯一主责或明确交叉归属；支持项有 macOS 版本/应用版本/证据，缺失证据不计通过。
- 轻量检查命令：

```bash
python3 - <<'PY'
from pathlib import Path
p=Path('docs/shiju/agent-apps/provider-macos-acceptance.md')
s=p.read_text()
assert s.strip()
for key in ['macOS', 'API', 'SDK', 'Iced']:
    assert key in s, key
print('文档关键项检查通过；仍需人工逐项审查')
PY
```

## 步骤

1. 阅读只读 plan.md、执行索引.md、来源映射.md 与本任务的来源入口。
2. 对照真实实现/前置交付证据核对本任务清单，记录具体文件、符号、平台和差异。
3. 只更新上述文档；存在无法确认的契约或证据时在本任务记录缺项，不编造支持。
4. 执行轻量检查，人工检查链接和字段覆盖；保存审查结论。

## 验证

- 预期：文档字段检查退出码 0，人工清单所有项均有可追溯依据。自动字符串检查不等于契约/覆盖验收。
- 所需证据：来源提交、审查文件/符号、确切检查命令、退出码和缺项清单。任务 1 缺前置接口则已阻塞；最后覆盖审查存在未验证项不得宣布母任务完整。

## 完成

- 将本任务完成证据追加到 `docs/shiju/agent-apps/provider-macos-acceptance.md` 的本任务条目，包含来源函数、实现路径、API/SDK/UI 和 macOS 验证结果；该正式资料保留。不要改其他任务条目。

- 只有本任务要求的新证据齐备才标记 `已完成`；因无关既有错误不能回归时使用 `已完成，待回归` 并保留任务文件；真正缺依赖时 `已阻塞` 并仅在本任务记录原因、已尝试检查与恢复条件。
- 不修改任何 plan.md，不向 prompt.md 写进度。不得扩大到其他任务或全仓清理；有新需求或新设计先询问是否回写来源，不自行改母计划。
- 有代码/测试/配置/文档实际变更时使用 `$git-commit` 审查并只提交本任务变更；只有编号任务状态更新/删除不提交。最终回复报告证据后删除已完成任务文件；待回归或阻塞文件保留。正式证据文档不能随任务删除而丢失。
