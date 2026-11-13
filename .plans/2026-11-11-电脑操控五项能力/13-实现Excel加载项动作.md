# 任务 13: 实现Excel加载项动作

批次：【批次 13】 依赖：批次 12；线性前置任务 12

状态：已阻塞

## 继续执行复核（2026-11-13）

- 已核对现有实现提交 `39b64eafd`（`feat(computer-control): 新增Excel加载项范围动作与配对资源`），本轮未修改业务代码、共享协议及其他任务文件。
- 再次运行 `node --test plugins/shiju-computer-control/office/excel/actions.test.mjs plugins/shiju-computer-control/office/shared/connection.test.mjs`，退出码 0，22 项有效测试全部通过，0 失败、0 跳过；其中本任务 14 项。
- 再次执行 `find /Applications "$HOME/Applications" -maxdepth 3 -iname '*excel*' -print` 与 `mdfind 'kMDItemCFBundleIdentifier == "com.microsoft.Excel"'`，均无匹配；`ps -axo comm= | rg -i 'Microsoft Excel|Excel.app'` 无匹配。`list_document_sessions({surface:"excel"})` 仍返回 `executors: []`。计算机应用与浏览器清单未发现 Excel 应用或已打开的 Excel 工作簿；本轮未重复上一轮已证实受登录阻挡的网页创建流程。
- 仍缺可侧载本加载项的已授权 Microsoft Excel 会话。所需输入及验收步骤沿用下文：真实临时工作簿值、公式、格式回读及关闭后旧命令失败。保留“已阻塞”和任务文件；仅更新任务记录，不单独提交，不执行本任务明确排除的 Cargo。

## 本次实现与检查记录（2026-11-12）

- 当前为批次 13。依赖批次 12 实际产物已核对：`2098d43dc` 提供共享安全连接与传输，`97929c882` 提供文档会话生命周期；未仅凭前置任务文件已删除判定满足。
- 已实现四个列明文件。`createExcelActions` 沿用宿主现有 `read_range` / `write_range(values)` 协议；用户在窗格批准固定工作表、矩形范围、覆盖、受限公式与数字格式。公式通过现有字符串单元格传入，格式由连接时用户选择，不添加宿主不支持的命令或参数。返回实际地址、值、公式和数字格式；显式 load/sync，写入后重新加载回读。
- 默认拒绝覆盖、公式及改变格式；最多 1024 单元格、48 KiB 请求/结果、4096 字符单元格。拒绝保护表、只读、越界、危险公式、未知字段及手动计算模式下公式写入。仅支持 ExcelApi 1.2；本次未扩展 Rust 协议，不执行 Cargo（按本任务显式约定）。
- 每次配对使用新文档标识，代次由真实共享连接验证；URL 变化、另存为、保存未命名文档、窗格卸载或停止会使旧执行器失效。每条命令及异步阶段检查身份，连接关闭同步关闭执行器；不保存令牌或文件 URL，不启用宏。
- TDD 失败命令：`node --test plugins/shiju-computer-control/office/excel/actions.test.mjs`，退出码 1，缺少 `actions.mjs`，有效行为测试 0。实现后同一确切命令退出码 0，14 项有效测试通过、0 跳过，成功和拒绝断言均已执行。
- 集成验证命令：`node --test plugins/shiju-computer-control/office/excel/actions.test.mjs plugins/shiju-computer-control/office/shared/connection.test.mjs`，最终退出码 0，22 项通过、0 跳过。使用真实本地范围策略与共享传输实现，仅模拟 Office.js 与外部 WebSocket 边界，未将模拟当实机证据。
- `node --check plugins/shiju-computer-control/office/excel/actions.mjs` 及 `node --check plugins/shiju-computer-control/office/excel/actions.test.mjs` 均退出码 0。`npx --yes office-addin-manifest validate plugins/shiju-computer-control/office/excel/manifest.xml` 初次退出码 1（缺 SupportUrl/IconUrl），补齐后退出码 0，明确返回 `The manifest is valid`。为使清单资源完整，额外复制仓库现有 64px 图标至 `excel/icon.png`；未添加重型运行依赖。清单校验不是加载项实际运行证据。
- 已审查本任务差异：仅 Excel 加载项目录与本任务记录；未修改 plan.md、共享协议或其他任务文件。清单指定可信 `https://localhost:3000/office/excel/taskpane.html`，页面内提供静态资源映射、精确 Origin、WSS 一次性配对和临时公式回读步骤；服务启动、证书信任及 Office 侧载尚未实机验证。
- 实机功能阻塞：`find /Applications "$HOME/Applications" -maxdepth 3 -iname '*excel*' -print` 与 `mdfind 'kMDItemCFBundleIdentifier == "com.microsoft.Excel"'` 无匹配；`ps -axo comm= | rg -i 'Microsoft Excel|Excel.app'` 退出码 1，无 Excel 进程。`list_document_sessions({surface:"excel"})` 返回 `executors: []`。浏览器检查 Excel 网页版后点击“创建空白工作簿”，实际导航至 Microsoft 登录页，显示空的帐户输入框，没有可用的已登录会话。WPS 未当作 Office 成功；未购买、创建账号或输入凭证。
- 唯一必要外部条件：一个可侧载本加载项的已授权 Microsoft Excel 会话（桌面版或已登录网页版）。取得会话后，按窗格步骤在临时 A1:B2 写入值、`=SUM(A1:B1)` 与 0.00 数字格式，记录真实 Excel 版本、脱敏回读结果 3，以及关闭文档后旧命令失败。该证据尚缺，属于 Office 功能阻塞而非无关构建失败，因此保留本文件与“已阻塞”，不能标记完成或删除。


目的：在活动工作簿中完成结构化读取、写入和回读。

来源任务：用户要求截图五项全部支持；无外部任务清单。

预计会话范围：只处理本文件定义的行为与列明的模块；跨后端的实机验收已独立拆分。若发现新平台机制需要多会话，先记录具体阻塞，不能以界面或模拟替代执行。

## 文件

- 新建：`plugins/shiju-computer-control/office/excel/manifest.xml`
- 新建：`plugins/shiju-computer-control/office/excel/taskpane.html`
- 新建：`plugins/shiju-computer-control/office/excel/actions.mjs`
- 新建：`plugins/shiju-computer-control/office/excel/actions.test.mjs`

## 上下文

- 仓库：`/Users/xiangmin/worker/code/shiju`。新增路径均为本计划定义的待创建产物，现有模块编辑前重新核对。
- 依赖Office共享连接与会话协议；仅使用明确支持的 Office.js requirement set，不把离线文件处理当实时控制。
- 按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。
- 工作区已有大量其他任务变更，只改本任务列明的文件；保留其他变更，发现无关编译失败仅记录。

## 顺序与资源

- 线性顺序：任务1→2→3→4→5→6→7→8→9→10→11→12→13→14→15→16→17→18→19→20→21→22→23→24；当前任务13，批次13。
- 可并行执行批次：无。共享注册文件、配置类型和本机原生输入/Chrome/Office验证资源，采用每批一个任务，缓存分槽不改变源文件或实机互斥。
- 前置产物以实际代码和提交/证据为准；任务文件可能在完成后删除，不能只凭文件不存在判定完成。
- 锁屏依赖阻塞不阻止不依赖锁屏执行的任务18—22；任务23、24的五项完整验收仍保持阻塞。

Cargo：本任务不调用Cargo，不领取构建槽位。若必要相邻修复引入Rust/Cargo，先补入技能规定的完整共享槽位契约再执行。

## 测试计划

- 行为：指定工作簿内按范围读写值、公式和格式，写后回读，越权范围拒绝。
- 失败验证测试：`plugins/shiju-computer-control/office/excel/actions.test.mjs`；新增测试名称统一以 `cc_control_13_` 开头，后缀说明具体行为与结果。
- 失败验证命令：`node --test plugins/shiju-computer-control/office/excel/actions.test.mjs`
- 预期失败原因：现有代码缺少本任务行为，Office.js外部边界可模拟，但必须补临时工作簿实际写入公式并回读结果的记录。 实现前先新增测试并确认失败落在该行为；若为缺符号编译失败，集成后还须证明对应拒绝/成功断言实际运行。
- 通过验证命令：`node --test plugins/shiju-computer-control/office/excel/actions.test.mjs`
- 模拟策略：本地策略、注册表与传输使用真实实现；仅OS、Chrome API、Office.js和不可稳定控制的远端界面作为外部边界模拟。模拟证据不得替代实机成功证据。

## 步骤

1. 先在独立测试文件添加上述失败用例，运行失败验证命令，保存预期失败；仅实现这些用例证明的最小变化。
2. 创建 Excel 加载项清单和任务窗，注册 surface=excel、文档代次及实际支持的命令。
3. 实现范围读取、值/公式写入、基础格式与能力探测；明确异步 load/sync 时序及返回范围，限制请求规模。
4. 工作簿重开/另存为使旧命令失效；拒绝不支持的 API、保护区域和未经批准的破坏性操作，不启用宏。
5. 运行下列通过验证及所需实机检查，记录当前证据；重构只在行为测试持续通过时进行。

## 验证

- 运行：`node --test plugins/shiju-computer-control/office/excel/actions.test.mjs`
- features：不适用。
- 预期：至少5个 node 测试及临时工作簿值/公式/格式回读证据；关闭文档后命令失败。
- 所需证据：失败与通过的确切命令、退出码、有效测试数量、审查后的本任务差异；涉及真实后端时补实际版本、目标身份与脱敏回读。零测试、仅编译通过、跳过实机或HTTP 200均不算行为完成。

## 完成

- 仅使用状态：`未开始`、`已完成`、`已阻塞`、`已完成，待回归`。
- 新证据齐备才可标记已完成；不可用的OS/Office依赖是功能阻塞，不能当无关构建错误略过。无关既有构建失败可标记已完成，待回归并保留本任务文件，注明确切失败与已完成的验证。
- 阻塞先调查并在已有授权内修复必要相邻依赖；无法消除时只在本任务文件记录原因、尝试、命令和所需输入，不修改plan.md或其他任务状态。
- 已完成或已完成，待回归且有实际代码、测试、配置或文档变更，使用 `$git-commit` 技能检查 staged/unstaged/untracked，仅提交本任务实际变更。只有本任务文件状态更新或删除时不调用提交技能、不单独提交。
- 最终报告文件、符号、行为及完成证据；已完成后删除本任务文件，待回归和阻塞时保留。计划与提示词不作为进度来源，plan.md始终只读。
