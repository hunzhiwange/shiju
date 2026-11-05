# 源码依据与产品语义

调研日期：2026-11-05。本次只创建计划，未修改业务代码、未运行 Cargo。后续执行必须重新检查工作区。

## 已确认的结果

用户已选择：智能体统一全局配置，项目只用于技能目录。原有已启用/官方目录、每行三张卡片、4px 滚动条及两侧 7px 留白继续保留。

智能体弹窗最终为“基础参数、身份、工具、技能、MCP”。首次新建和编辑均进入基础参数。基础参数包含名称、多行描述、推荐提示词、运行引擎、唯一的模型配置组、完全访问开关、现有适用的运行参数。删除“资料与默认参数”标签。

身份保留唯一的系统提示词编辑框与身份文件；删除资料里的“规则”框，不删除 system_prompt 字段、不清空已有正文。身份文件正文沿用即时保存，其他字段沿用保存/取消事务。

外层“技能”是安装、启停、管理资源的库；弹窗“技能”是该智能体的新会话选择，职责不同，均保留。重复的是资料区的默认技能与弹窗技能列表，合并到弹窗技能页。MCP 同理：弹窗新增独立 MCP 页，只选择服务；外层 MCP 继续管理服务连接、安装和诊断，不复制凭据表单。

## 当前实现证据

- `crates/vw-agent/src/config/schema/config_load.rs::load_config_for_tenant_from_store` 将项目 JSON 覆盖合入基础配置，当前并未排除 agents。`config/mod.rs::get_current` 按项目读取；仅移除 UI 项目框不足以实现全局定义。
- `crates/vw-agent/src/gateway/api/handlers/misc.rs::agent_list/agent_copy` 当前支持 directory，复制分全局与项目写入。`agent/mod.rs::load_state_with_disabled` 从 get_current 取定义。需要在配置加载/写入边界统一 agents 的来源，保持其他项目配置和权限上限。
- `crates/vw-desktop/src/app/message/settings/agents.rs` 的 refresh/config_task/persist_agents_patch 及 `agents_profile.rs` 当前按 agents_settings.project_path 读写；CatalogLaunch 还校验该路径与当前项目相同。
- `components/system_settings_agents/profile_modal.rs` 分出资料、基础参数，描述是单行输入，规则与 `identity.rs` 共用同一个 system_prompt_editor，属于同一字段的重复入口。
- `defaults.rs::view` 同时显示默认模型、JSON 权限、默认技能及默认 MCP；`skills.rs::view` 另有 allowed_skills 列表与 Strict/Unrestricted 策略。
- `agent/defaults.rs::resolve` 优先级为可信显式选择、Fixed、继承；Fixed([]) 与 Inherit 有不同含义。`session/agent_binding.rs::prepare` 在新会话固化选择，续聊保持绑定。
- `AgentDefinitionConfig.allowed_skills` 既是继承来源，也参与委托技能策略；`session/capability_policy.rs` 区分可见技能与默认注入。不能将 Strict 丢掉或把 Inherit 当作全选。
- `AgentDefaults.permission` 是策略 JSON；它与 `SessionRuntimeState.full_access_enabled` 不是同一个值。`message/chat/stream/permissions.rs::handle_toggle_full_access_permission`、请求队列及 ACP 的 `parsed.full_access` 已有完全访问流程。把开关写成 {"*":"allow"} 会误用权限字段，ACP 目前还会拒绝这种固定策略。
- `defaults.rs::patch` 当前遍历默认字段并操作 mode/value；新增布尔字段后必须只处理原有枚举字段，避免把 bool 当对象。

## 明确的配置契约

1. agents 定义在当前租户内全局唯一，包含启用、复制、新建、修改和删除。项目覆盖不再改变定义；项目接口收到 agents 修改应明确拒绝，不静默转存全局。项目目录仍用于会话工作区、技能发现、MCP 环境解析及既有安全策略；“项目只用于技能目录”指移走本页面的项目选择器，不删除系统其他功能的工作区。
2. 弹窗内技能项目选择只刷新候选目录，不切换整个应用项目，不修改 agents 的保存目标。全局智能体内保存的技能 ID 是全局默认：实际会话在自己的工作区校验可用性，缺失就明确报错；不能暗中按查看目录写出不同智能体定义。
3. 技能页只有一个候选列表和一个继承/固定选择器。Inherit 显示继承来源的有效预览，不能默认为全选；Fixed([]) 明确表示不预选。当前继承来源保持 allowed_skills，图形化编辑新会话默认统一写 defaults.skills，不再并排编辑两份默认列表。现有 skill_policy 作为独立访问边界保留，不由切换继承模式自动放宽；严格策略下越界选择显式拒绝。后端若有多个解析入口，应统一执行同一语义，包含委托子智能体。
4. MCP 页同样写 defaults.mcp，按服务稳定 ID 选择，不按动态工具 ID；继承解析当前工作区可用服务，固定空列表禁用默认 MCP，禁用或未知服务不能运行。
5. 基础参数只显示一个模型组。以 defaults.model 承载继承/固定默认；需要强制模型时在同一组复用所选 provider/model，不再出现另一套可独立冲突的选择。保留现有强制绑定行为和 ACP 能力检查；已有矛盾值显式提示并要求用户在唯一控件中修正，不自动猜测。
6. 为用户实际要求的新会话默认值，在 `AgentDefaults` 新增 `full_access: bool`，默认 false。它表示新会话的完全访问初值，不能复用 permission JSON。基础参数只显示一个“完全访问权限”开关：关闭走当前正常审批，开启沿用现有完全访问语义。已有底层 permission 和安全拒绝继续执行；界面不再提供继承、全部拒绝、逐项询问、JSON 策略等菜单，也不静默把旧策略改成允许。
7. 完全访问默认只在新会话初始化/选择智能体时使用，之后会话内的显式开关操作优先，不能每轮被默认值覆盖。默认缺失按 false 处理；不新建权限策略引擎。已有绑定、恢复会话及其他会话的权限状态不得被配置保存改变。

## 方案权衡

| 问题 | 方案 | 结论 |
| --- | --- | --- |
| 项目 | 只移动项目控件；保留项目级定义；全局定义与目录上下文分离 | 用户确认第三项；需要底层与 UI 一致 |
| 技能/MCP | 保留资料区与列表两套；删除独立列表；保留专属列表并合并继承/固定 | 采用第三项，资源管理库与会话选择保持不同职责 |
| 权限 | 把开关转换为 allow/deny JSON；复用现有完全访问运行开关并保存初值 | 采用第二项，避免关闭即拒绝全部及 ACP 不支持的伪行为 |

## 环境与验证入口

已检查根 Cargo.toml 的 workspace，vw-desktop/vw-config-types/vw-agent Cargo.toml；vw-desktop 默认 features=[]。聚焦单元/渲染测试使用默认 features；每项代码任务再跑 `cargo clippy --all-targets --all-features -- -D warnings`。

`crates/vw-desktop/Cargo.toml` 已注册 `agent-profile-window`（test=false，harness=false），入口为 `src/agent_profile_window_main.rs`，bootstrap 使用隔离端口 9。它适合最终原生交互验收，不连接用户业务网关；持久化验收另外使用临时数据库与隔离 HTTP 测试服务，不把隔离窗口的保存失败当成成功。

上一轮报告过工具相关 4 项、技能发现文案 1 项既有测试失败。本轮不继承该结论为当前基线；执行者先复核，只修本次造成的问题，不用 blanket skip 隐藏新增失败。工作区有前一轮 UI 与其他任务改动，提交只包含本任务拥有的差异。

本目录不修改 `.plans/2026-11-04-统一智能体入口/plan.md`，不以该旧计划的设计说明代替当前实现证据。新增任务执行采用顺序批次，避免共享文件和验证资源冲突。

## 需求覆盖与任务依赖

| 用户需求 | 编号任务 |
| --- | --- |
| 智能体全局，项目选择移入技能目录 | 1、2 |
| 技能重复收敛，继承/固定合并到技能页 | 4 |
| 描述改 textarea | 3 |
| 只保留身份中的系统提示词 | 3 |
| MCP 独立标签 | 5 |
| 权限简化为完全访问开关 | 6 |
| 资料、参数合入基础参数 | 3 |
| 保留三列卡片、滚动条与新建修改行为 | 2、7 |

任务均为单独顺序批次：1→2→3→4→5→6→7。虽然任务1底层与部分界面代码不同，但全局语义是后续界面前提；任务2至6共享草稿、消息与默认配置文件，不能并行。任务7在所有功能完成后执行统一回归。
