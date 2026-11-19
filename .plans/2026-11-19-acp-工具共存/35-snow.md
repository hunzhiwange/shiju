# 任务 35 · snow

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次 2】　依赖：任务 1

状态：待回归

目的：在既有权限内复验原生与 MCP 共存，按本卡定位并修复确认的缺陷。

## 基线与文件

- 固定版 0.9.14。先核对当前注册、制品合约和实际启动命令。[分组审计](../../docs/dev/acp/mcp-policy-audit/6.md#snow) · [现有复验入口](../../docs/dev/acp/mcp-acceptance/snow.md)
- 原生路径与首个断点为「原版保存 stdio，prompt 工具收集忽略会话列表」。当前目录与执行证据为「普通保留原生；挂载追加且冲突失败；无通用名单投影」。
- 本任务回执写入 `docs/dev/acp/mcp-policy-audit/coexistence/snow.md`，逐轮记录工具名、ID、执行方、退出状态和缺证项。

## 实施与验证

1. 从现有复验入口取得固定版及已审命令，验证环境参数和权限模式，按共存验收口径运行三轮。保留协议，只缩旧格式提示；先测 Chat 同轮官方读取，再补其他模型协议。
2. 当前补丁职责为「v3 加发现、单次审批、结果入历史和清理，Chat 提示使用真实目录」。保留原权限及受审入口。仅在实际复验确认适配器自身存在无意裁剪时，按公共规则为本适配器补红绿测试并局部修复。
3. 正向成功信号为挂载会话实际执行两类工具、ID 对应并续轮，撤除后目标服务零调用。逐项记录平台和入口边界「原版私有配置可选模；驱动无 v3 摘要前置，原包可能重现断点；通用」。使用 `git diff --no-index --check /dev/null docs/dev/acp/mcp-policy-audit/coexistence/snow.md` 检查新增回执的空白错误，以无空白错误诊断为通过。若当前制品缺失，记录入口和首个不可验证步骤。

## 本次复验

- 受管 0.9.14 v3 的四种模型协议均通过无挂载、挂载、撤除三轮，官方文件读取与 stdio MCP 在同一提示真实执行并续轮，撤除后零目标调用。Chat 另通过拒绝和两次 load 场景。逐轮证据见 [Snow 共存回执](../../docs/dev/acp/mcp-policy-audit/coexistence/snow.md)。
- Chat 等待单次审批时取消两次复现 `-32603`，服务调用均为零。首个断点是审批返回后的 `signal.throwIfAborted()` 错误未被外层映射为 `cancelled`。当前任务仅授权修复已确认的工具裁剪，本次目录比较未发现裁剪。直接 ACP 用迟到批准与宿主取消回包均复现，定位在 Snow 提示异常映射。
- 复验命令：`python3 /private/tmp/shiju-snow-task35.z1m19I/probe.py chat <受管 node_modules/.bin/snow>`。修复后应返回 `cancelled`，零服务调用且探针退出 0。真实上游、网关与其他平台未运行。

## 待回归交接

- 私有候选用 `controller.signal.aborted` 映射取消，Chat 两种审批回包各七项通过，其他三协议各三轮通过。仓库安装校验用当前共享合约拒绝该候选，更新摘要后同一校验通过。
- 按公共规则由任务 43 落地共享制品接线，同步 Snow 补丁规范与安装合约摘要，安装后用 `/private/tmp/shiju-snow-task35.z1m19I/cancel-trial.RxbNIk/probe.py` 重放。补充红绿轨迹和候选摘要已写入回执。
