# 任务 23: DeepAgents 接入后实机验收

批次：【批次 23】 依赖：接入任务 2 已交付，并核对当前代码与脱敏接入证据；验收任务 22 已结束并恢复环境（通过或明确记录阻塞）。

线性顺序：原接入批次 1–21 → 新增验收批次 22–42；本批仅任务 23，全部串行，禁止争用网关、租户配置和验收目录。

状态：已阻塞

目的：证明 DeepAgents 通过系统安装、内置预设及指定会话模型，真实创建 Rue.js 编辑器文件；追加实测记录。

来源任务：`2-DeepAgents.md`；依据 `docs/dev/acp/ACP验收.md` 最近各项的“完整文件创建为终点”口径，不把浏览器行为、页面构建记为通过。

预计会话范围：只验收 `deepagents`，不重跑既有 ACP 项或其他新增接入，不扩展为上游能力全量认证。

## 文件与阅读顺序

1. 阅读仓库 AGENTS.md、`skills/do-task-plans/SKILL.md`、同目录只读 `plan.md`、本任务。
2. 阅读来源任务（若仍存在）、`差异清单.md`、`docs/dev/evidence/acp-onboarding-deepagents.md`（若已交付）及 `docs/dev/acp/ACP验收.md` 的最新记录。
3. 核对现有注册 `crates/vw-acp/src/agent_registry.rs`、安装白名单 `crates/vw-config-types/src/acp_install.rs`、派生预设 `crates/vw-agent/src/agent/catalog.rs` 及实际模型来源驱动；代码导航先用仓库 rustcodegraph 技能，图不覆盖时用 rg。
4. 修改：仅在 `docs/dev/acp/ACP验收.md` 末尾追加“DeepAgents 验收”章节，保留全部历史记录。
5. 运行产物：`/Users/xiangmin/worker/code/ruejs/acpdemo/deepagents/`；原始日志、请求、响应和快照只存本机 0700 私有临时目录。

## 执行约定

- 不修改 `plan.md`、历史接入任务、旧验收记录或其他任务状态；来源任务缺失不等于通过，须用当前实现、接入证据和真实执行确认前置条件。
- 常规、可逆、低风险事项自主决定；先调查根因并记录依据，局部阻塞继续可独立检查。无法消除的关键歧义或确需新增授权才集中询问，不将沉默视为批准。
- 不替换模型、租户或引擎，不降低权限检查，不把 unsupported、authentication_required、安装成功或 HTTP 200 当成真实会话成功。
- 本任务是验收而非行为实现，测试驱动开发不适用。发现产品缺陷时记录可复现失败与底层位置；超出小范围验收修复则保留阻塞证据，另建修复任务，不能绕过或伪造通过。
- 只有确需修复本接入的小范围底层缺陷时，先添加独立文件中的行为失败测试，记录红灯，再最小修复并取得原生聚焦通过证据；实际测试名与命令先根据现有代码确认后写入本任务，禁止空过滤或零测试充数。

## 构建与资源约定

- 工作目录 `/Users/xiangmin/worker/code/shiju`。优先使用能证明包含本接入的现有构建；核对二进制摘要和构建来源，不使用旧二进制判定新实现失败或成功。
- 需要新构建时，使用 `python3 scripts/cargo_slot.py sh -c 'cargo clippy --all-targets --all-features -- -D warnings && cargo build -p vw-cli --bin shiju --features vw-agent/default'`，同阶段共用脚本领取的槽位；记录两个绝对缓存路径、命令、退出码。
- 若无代码变更且复用已验证产物，不运行无关 Cargo 检查。发生代码变更则必须执行上述 Clippy 及与缺陷对应的原生聚焦测试，不运行全量 cargo test、fmt check、wasm check 或 Makefile。
- 启动临时网关前按当前 CLI 帮助和已有运行方式核实参数，使用空闲回环端口及现有开发配置，不重启用户服务、不回显配置或令牌。只结束自有进程；所有使用槽位产物的前台子进程退出后释放自有锁，保留缓存。

## 验收步骤

1. 核实前置接入已交付；读取已有证据不能替代本次执行。确认 Rue.js 工作区及其 `@rue-js/rue` 源码存在，记录其初始工作区状态与普通文件 SHA-256 快照（排除 .git、node_modules、target）。目标目录存在时先保留原产物，不覆盖用户文件；不能安全复用时记录阻塞。
2. 对所有请求使用 `X-Platform-ID: 10001`、`X-Company-ID: 100010005`、`X-Directory: /Users/xiangmin/worker/code/ruejs`。从当前网关读取 `GET /v1/global/config/acp/installed`、`GET /v1/agent?include_disabled=true` 和 `GET /v1/global/config/acp`，记录安装、启用及配置基线，私密环境值不得写入文档。
3. 调用 `POST /v1/desktop/acp-install/deepagents`，记录 HTTP、客户端退出码、耗时、安装前后状态、解析到的入口及版本。对已安装项核验真实入口；安装器明确不支持时保留拒绝原因，不临时使用任意 URL 或 shell 安装替代系统链路。
4. 按当前接口结构将对应 ACP 配置传给 `POST /v1/desktop/acp-diagnostic/acp-smoke-deepagents`；读取 `GET /v1/acp-model-source/deepagents` 确认 session_model 能力。普通诊断与真实模型会话分开记录；能力未实现或拒绝时不能改用 CLI 默认模型冒充验收。
5. 确认内置预设 `acp-deepagents`、builtin 标识及 backend `deepagents`；若未启用，通过 `PATCH /v1/global/config` 临时设置相应 agents 条目的 enabled=true，再 GET 重读确认保存；保留原值供结束恢复。不要改写其他预设或默认配置。
6. 向 `POST /v1/chat/stream` 发出下文请求，使用 Content-Type application/json 和上述三个租户/目录头，客户端限时 300 秒。记录完整 SSE、响应头、HTTP、退出码与耗时。超时先调查会话状态；仅在现有接口确认可用时使用原请求标识与 Last-Event-ID 续传，不重新提交同一创建任务。
7. 从实际响应的会话标识查询 `GET /v1/session/{session_id}/ui/summary` 和 `GET /v1/session/{session_id}/acp-options`，执行期间及结束后确认预设、backend、full_access、connected 和 selected_model 的 source、provider_id、完整模型 ID。不能仅凭模型目录或助手自述判断模型绑定。
8. 取得 chat.done 且无 chat.error，核对工具执行及实际产物。目录内应有完整可运行页面源码与 README；源码实际使用本仓库 @rue-js/rue，实现文本输入、实时字数与 localStorage 保存/恢复。检查文件内容而非只看回复；无需运行浏览器、页面构建或创建后的额外回归，不宣称这些行为已经实测。
9. 所有成功、失败和中断路径都恢复临时 enabled 配置并 GET 复查；停止自有网关后对比文件快照，确认仅目标目录发生本次变化。遇到并发外部变化记录具体文件和归因边界，不能删除别人的修改或声称无越界。
10. 追加本次日期、通过/阻塞结论、版本与构建摘要、安装及诊断、模型能力、请求与终态、会话绑定、产物清单、目录边界、配置恢复、清理、执行/未执行检查及私有证据目录。不提交凭据、完整配置环境值、原始敏感请求或供应商二进制。

## 请求体

```json
{
  "model": "alibaba-token-plan-cn/deepseek-v4.1-flash-1017",
  "agent": "acp-deepagents",
  "acp_agent": "deepagents",
  "acp_model_source": "session_model",
  "options": {
    "cwd": "/Users/xiangmin/worker/code/ruejs",
    "full_access": true
  },
  "messages": [
    {
      "role": "user",
      "content": "帮我用 Rue.js 实现一个纯文本编辑器，支持输入、实时显示字数、刷新后恢复内容。只写到 acpdemo/deepagents/，不修改目录外文件。使用本仓库的 @rue-js/rue；在目标目录内提供可运行页面与运行说明。实际调用工具完成文件后简短回复即结束，不运行额外测试。"
    }
  ]
}
```

## 验证与完成

- 成功标准：系统安装和内置预设保存重读成功、真实指定模型调用得到成功终态、会话身份与模型匹配、完整文件实际落盘、目录边界及配置恢复均有本次新证据。缺一不得标记通过。
- 仅使用状态 `未开始`、`已完成`、`已阻塞`、`待回归`。前置接入未交付或指定模型不支持则记录具体阻塞、已尝试路径及重跑条件，不静默跳过。无关构建失败才可记录待回归及确切重跑命令。
- 通过后标记已完成，在最终回复交接新证据并删除本任务文件；阻塞/待回归时保留本文件，记录已运行与未完成检查。
- 有实际文档、代码、测试或配置变更时使用 `$git-commit` 技能，只提交本任务相关差异；共享验收文档含其他改动时按差异精确暂存。只有本任务文件的状态或删除时不单独提交，不提交 Rue.js 工作区或其他任务的变更。

## 2026-11-13 本次执行

状态：已阻塞。任务 22 已记录阻塞并确认未改环境；本次通过 rustcodegraph 及当前源码副本核对注册、固定 0.1.42 安装映射和 builtin 派生。既有接入证据明确未发布 session_model；当前 drivers/mod.rs 无 DeepAgents configure、source_protocol、verified_release 分支，能力门控 UnsupportedCli。指定模型无法按要求绑定，不能以 cli_local 替代。

新增整个会话模型中继驱动及其安全审核超出验收小修范围，未扩大为功能开发。Rue.js 源码存在、目标目录不存在，保存初始 Git 状态，未写入产物。未执行安装/诊断/模型来源 API、配置启用保存、真实 SSE、会话查询、完整文件快照及后续验收；未运行 Cargo、Clippy 或测试。未启动进程或修改配置，无自有资源需要恢复。详情见 ACP验收.md 末尾 DeepAgents 验收章节，私有证据 `/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-deepagents-blocked-a3la5u4n`。先交付并验证模型驱动，再使用已验证新构建从步骤 1 重跑；本文件保留。

## 2026-11-13 阻塞恢复复核

状态保持已阻塞。按追加指示重读接入证据、DeepAgents 审核补丁、真实 CLI 夹具、model_source/mod.rs、驱动分派及现有 CodeBuddy 私有注入实现，逐项评估安全路径：

- 系统安装和普通诊断只能证明受管入口/协议可启动，无法补齐指定 session_model 模型绑定；不能作为成功替代。
- CLI 的 --model 与 OPENAI_BASE_URL/OPENAI_API_KEY 在既有夹具中仅验证 openai:gpt-4o 和虚拟回环服务。cli_local 绕过宿主 model_source 模块，不能证明指定租户、provider/model、短期令牌及会话身份绑定。没有改用此路径或替换指定模型。
- 自定义 args/env 不能修复现有 session_model：prepare 固定调用 drivers::configure，configure 先调用 launch_route，source_protocol 对 deepagents 明确拒绝；其后 configure 与 verified_release 同样无分支。改预设、启用状态或重装受管包均不改变此边界，别名冒充其他 backend 也不符合要求。
- 已有 deepagents 补丁解决宿主工具权限、文件回退、工作目录与 CLI 本地 Chat API 选择，不包含宿主会话中继驱动。仅加入 capability/verified_release 登记会伪造支持，不能视为小修；仅复制其他驱动也不能证明该 CLI 的模型参数、原生配置优先级、取消和恢复不会绕过绑定。
- 允许的小范围修复评估：当前没有已实现驱动中的局部参数错误可修。最小完整交付仍需固定版本的 DeepAgents driver、源协议路由、准确模型选择和禁止回退、私有端点与短期令牌隔离、会话复用及拒绝路径测试、真实指定模型请求证据，再发布能力；必须具备独立失败测试、原生聚焦验证与整仓 Clippy。该范围属于新增功能，超出本任务验收小修授权。

所需新增用户决策：是否授权单独实施“DeepAgents session_model 私有中继驱动与验证”，保持 alibaba-token-plan-cn/deepseek-v4.1-flash-1017、既定租户、DeepAgents 原生引擎和权限策略；交付后再恢复任务 23。未获得此授权前保持已阻塞，不扩大为功能开发。

本轮环境确认：两轮均未启动/停止网关或供应商进程、执行安装器、PATCH 配置、领取 Cargo 槽位或写入 Rue.js 工作区；没有本会话临时配置或进程需要恢复，未触碰其他会话环境。目标 acpdemo/deepagents/ 仍不存在。复核源码摘要仍与首轮证据一致；仅追加此任务文件，不单独提交。未运行 Cargo、运行时 API 或实机创建，不将静态检查记为通过。
