# 任务 24: DimCode 接入后实机验收

批次：【批次 24】 依赖：接入任务 3 已交付，并核对当前代码与脱敏接入证据；验收任务 23 已结束并恢复环境（通过或明确记录阻塞）。

线性顺序：原接入批次 1–21 → 新增验收批次 22–42；本批仅任务 24，全部串行，禁止争用网关、租户配置和验收目录。

状态：未开始

目的：证明 DimCode 通过系统安装、内置预设及指定会话模型，真实创建 Rue.js 编辑器文件；追加实测记录。

来源任务：`3-DimCode.md`；依据 `docs/dev/acp/ACP验收.md` 最近各项的“完整文件创建为终点”口径，不把浏览器行为、页面构建记为通过。

预计会话范围：只验收 `dimcode`，不重跑既有 ACP 项或其他新增接入，不扩展为上游能力全量认证。

## 验证阶段

Rust 编译资源和会话调度按 `.plans/references/rust.md` 的项目覆盖规则执行；下方必要定向单元测试仍在任务内完成。

- 开发阶段完成本任务和直接受影响行为的必要定向单元测试，优先复用现有测试；确认有效测试数及结果，不积压到最终回归。
- 基础检查限定受影响包和目标；同一目标已被定向测试成功编译时不重复 check。Rust 不能只编译单个 `.rs` 文件，测试过滤也可能编译整个 lib-test 目标。
- 本任务不运行整仓构建、全目标全特性 Clippy、全仓格式检查或全量测试。统一回归只补跨任务集成及公共检查，不机械重跑未受后续改动影响的已通过单元测试。
- 仅同仓库 Rust 重型编译串行排队、复用缓存；其它语言的普通定向单元测试不套用此限制。必要定向测试通过后按原完成规则收尾；仅具体集成待验收或客观外部阻塞保留待回归。

## 文件与阅读顺序

1. 阅读仓库 AGENTS.md、`skills/do-task-plans/SKILL.md`、同目录只读 `plan.md`、本任务。
2. 阅读来源任务（若仍存在）、`差异清单.md`、`docs/dev/evidence/acp-onboarding-dimcode.md`（若已交付）及 `docs/dev/acp/ACP验收.md` 的最新记录。
3. 核对现有注册 `crates/vw-acp/src/agent_registry.rs`、安装白名单 `crates/vw-config-types/src/acp_install.rs`、派生预设 `crates/vw-agent/src/agent/catalog.rs` 及实际模型来源驱动；代码导航先用仓库 rustcodegraph 技能，图不覆盖时用 rg。
4. 修改：仅在 `docs/dev/acp/ACP验收.md` 末尾追加“DimCode 验收”章节，保留全部历史记录。
5. 运行产物：`/Users/xiangmin/worker/code/ruejs/acpdemo/dimcode/`；原始日志、请求、响应和快照只存本机 0700 私有临时目录。

## 执行约定

- 不修改 `plan.md`、历史接入任务、旧验收记录或其他任务状态；来源任务缺失不等于通过，须用当前实现、接入证据和真实执行确认前置条件。
- 常规、可逆、低风险事项自主决定；先调查根因并记录依据，局部阻塞继续可独立检查。无法消除的关键歧义或确需新增授权才集中询问，不将沉默视为批准。
- 不替换模型、租户或引擎，不降低权限检查，不把 unsupported、authentication_required、安装成功或 HTTP 200 当成真实会话成功。
- 本任务是验收而非行为实现，测试驱动开发不适用。发现产品缺陷时记录可复现失败与底层位置；超出小范围验收修复则保留阻塞证据，另建修复任务，不能绕过或伪造通过。
- 只有确需修复本接入的小范围底层缺陷时，先添加独立文件中的行为失败测试，记录红灯，再最小修复并取得原生聚焦通过证据；实际测试名与命令先根据现有代码确认后写入本任务，禁止空过滤或零测试充数。

## 构建与资源约定

- 工作目录 `/Users/xiangmin/worker/code/shiju`。优先使用能证明包含本接入的现有构建；核对二进制摘要和构建来源，不使用旧二进制判定新实现失败或成功。
- 运行本任务必要定向单元测试，审查差异并记录有效测试数和结果；不重复公共整仓检查。
- 若无代码变更且复用已验证产物，不运行无关 Cargo 检查。发生代码变更则必须执行上述 Clippy 及与缺陷对应的原生聚焦测试，不运行全量 cargo test -j 1、fmt check、wasm check 或 Makefile。
- 启动临时网关前按当前 CLI 帮助和已有运行方式核实参数，使用空闲回环端口及现有开发配置，不重启用户服务、不回显配置或令牌。只结束自有进程；所有使用槽位产物的前台子进程退出后释放自有锁，保留缓存。

## 验收步骤

1. 核实前置接入已交付；读取已有证据不能替代本次执行。确认 Rue.js 工作区及其 `@rue-js/rue` 源码存在，记录其初始工作区状态与普通文件 SHA-256 快照（排除 .git、node_modules、target）。目标目录存在时先保留原产物，不覆盖用户文件；不能安全复用时记录阻塞。
2. 对所有请求使用 `X-Platform-ID: 10001`、`X-Company-ID: 100010005`、`X-Directory: /Users/xiangmin/worker/code/ruejs`。从当前网关读取 `GET /v1/global/config/acp/installed`、`GET /v1/agent?include_disabled=true` 和 `GET /v1/global/config/acp`，记录安装、启用及配置基线，私密环境值不得写入文档。
3. 调用 `POST /v1/desktop/acp-install/dimcode`，记录 HTTP、客户端退出码、耗时、安装前后状态、解析到的入口及版本。对已安装项核验真实入口；安装器明确不支持时保留拒绝原因，不临时使用任意 URL 或 shell 安装替代系统链路。
4. 按当前接口结构将对应 ACP 配置传给 `POST /v1/desktop/acp-diagnostic/acp-smoke-dimcode`；读取 `GET /v1/acp-model-source/dimcode` 确认 session_model 能力。普通诊断与真实模型会话分开记录；能力未实现或拒绝时不能改用 CLI 默认模型冒充验收。
5. 确认内置预设 `acp-dimcode`、builtin 标识及 backend `dimcode`；若未启用，通过 `PATCH /v1/global/config` 临时设置相应 agents 条目的 enabled=true，再 GET 重读确认保存；保留原值供结束恢复。不要改写其他预设或默认配置。
6. 向 `POST /v1/chat/stream` 发出下文请求，使用 Content-Type application/json 和上述三个租户/目录头，客户端限时 300 秒。记录完整 SSE、响应头、HTTP、退出码与耗时。超时先调查会话状态；仅在现有接口确认可用时使用原请求标识与 Last-Event-ID 续传，不重新提交同一创建任务。
7. 从实际响应的会话标识查询 `GET /v1/session/{session_id}/ui/summary` 和 `GET /v1/session/{session_id}/acp-options`，执行期间及结束后确认预设、backend、full_access、connected 和 selected_model 的 source、provider_id、完整模型 ID。不能仅凭模型目录或助手自述判断模型绑定。
8. 取得 chat.done 且无 chat.error，核对工具执行及实际产物。目录内应有完整可运行页面源码与 README；源码实际使用本仓库 @rue-js/rue，实现文本输入、实时字数与 localStorage 保存/恢复。检查文件内容而非只看回复；无需运行浏览器、页面构建或创建后的额外回归，不宣称这些行为已经实测。
9. 所有成功、失败和中断路径都恢复临时 enabled 配置并 GET 复查；停止自有网关后对比文件快照，确认仅目标目录发生本次变化。遇到并发外部变化记录具体文件和归因边界，不能删除别人的修改或声称无越界。
10. 追加本次日期、通过/阻塞结论、版本与构建摘要、安装及诊断、模型能力、请求与终态、会话绑定、产物清单、目录边界、配置恢复、清理、执行/未执行检查及私有证据目录。不提交凭据、完整配置环境值、原始敏感请求或供应商二进制。

## 请求体

```json
{
  "model": "alibaba-token-plan-cn/deepseek-v4.1-flash-1017",
  "agent": "acp-dimcode",
  "acp_agent": "dimcode",
  "acp_model_source": "session_model",
  "options": {
    "cwd": "/Users/xiangmin/worker/code/ruejs",
    "full_access": true
  },
  "messages": [
    {
      "role": "user",
      "content": "帮我用 Rue.js 实现一个纯文本编辑器，支持输入、实时显示字数、刷新后恢复内容。只写到 acpdemo/dimcode/，不修改目录外文件。使用本仓库的 @rue-js/rue；在目标目录内提供可运行页面与运行说明。实际调用工具完成文件后简短回复即结束，不运行额外测试。"
    }
  ]
}
```

## 验证与完成

- 成功标准：系统安装和内置预设保存重读成功、真实指定模型调用得到成功终态、会话身份与模型匹配、完整文件实际落盘、目录边界及配置恢复均有本次新证据。缺一不得标记通过。
- 仅使用状态 `未开始`、`已完成`、`已阻塞`、`待回归`。前置接入未交付或指定模型不支持则记录具体阻塞、已尝试路径及重跑条件，不静默跳过。无关构建失败才可记录待回归及确切重跑命令。
- 通过后标记已完成，在最终回复交接新证据并删除本任务文件；阻塞/待回归时保留本文件，记录已运行与未完成检查。
- 有实际文档、代码、测试或配置变更时使用 `$git-commit` 技能，只提交本任务相关差异；共享验收文档含其他改动时按差异精确暂存。只有本任务文件的状态或删除时不单独提交，不提交 Rue.js 工作区或其他任务的变更。
