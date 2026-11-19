# 任务 17 · grok

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次 2】　依赖：任务 1

状态：待回归

目的：在既有权限内复验原生与 MCP 共存，按本卡定位并修复确认的缺陷。

## 基线与文件

- 固定版 1.0.77；另审 1.0.24。先核对当前注册、制品合约和实际启动命令。[分组审计](../../docs/dev/acp/mcp-policy-audit/5.md#grok) · [现有复验入口](../../docs/dev/acp/mcp-acceptance/grok.md)
- 原生路径与首个断点为「1.0.77 原生 search_tool/use_tool 路径可调用和续轮」。当前目录与执行证据为「直出触发后替换 MCP 发现入口，其他原生定义保留；原生回退保留动态发现」。
- 本任务回执写入 `docs/dev/acp/mcp-policy-audit/coexistence/grok.md`，逐轮记录工具名、ID、执行方、退出状态和缺证项。

## 实施与验证

1. 从现有复验入口取得固定版及已审命令，验证环境参数和权限模式，按共存验收口径运行三轮。保留审批和名单；先比较有无 Python 的仅拒绝与普通目录，再补同会话官方只读工具。
2. 当前补丁职责为「Unix 有 Python 时运行时直出已发现目录、重映射调用、按服务过滤与撤销；混合传输无法执行名单时拒绝」。保留原权限及受审入口。仅在实际复验确认适配器自身存在无意裁剪时，按公共规则为本适配器补红绿测试并局部修复。
3. 正向成功信号为挂载会话实际执行两类工具、ID 对应并续轮，撤除后目标服务零调用。逐项记录平台和入口边界「原版私有 TOML；驱动校验参数及模型，通用，Linux 源码有 Unix 分支，Windows ACL 拒绝」。使用 `git diff --no-index --check /dev/null docs/dev/acp/mcp-policy-audit/coexistence/grok.md` 检查新增回执的空白错误，以无空白错误诊断为通过。若当前制品缺失，记录入口和首个不可验证步骤。

## 复验与集成待办

- [共存回执](../../docs/dev/acp/mcp-policy-audit/coexistence/grok.md)记录 1.0.77 投影三轮同会话共存、1.0.24 对照、审批、取消、加载和恢复。投影 Python 测试 11 项通过，回执空白检查无诊断。未运行 Cargo、网关或其他平台。
- 直接原生入口模拟无 Python 回退。仅拒绝名单的 `mcp_denied` 在 ACP 单次批准后实际执行（`call_fixture_1`，服务 RPC `2`）；投影相同输入时该项未公布，调用为零。首个断点为共享启动接线未在选择原生入口前拒绝无法执行的 `shiju.grok.toolPolicy`。按公共规则交任务 43 落地共享门禁并复跑；需覆盖仅拒绝，以及无名单时原生发现和调用仍可用。证据在本机 `/tmp/shiju-grok17.f10MIl`，可用 `/usr/bin/python3 /tmp/shiju-grok17.f10MIl/probe.py 1.0.77 native deny` 重现原生行为。实际宿主复验需走无投影启动路径。
