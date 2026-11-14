# 项目组合 macOS 交付证据

## 任务 11：应用供应商槽位

2026-11-14：已实现指定组供应商槽位编排与模块注册，合同见 [profile-provider-apply.md](profile-provider-apply.md)。null 不调用领域入口；同一有效供应商不调用写回调；缺失引用逐项 Missing。切换保留当前模式，核对实际目标、attached、stack 和非活动指针；发布待恢复不能返回成功。文件投影、登录事务、路由发布与权限仍归原领域执行端，生产回调由后续应用编排接线，当前不构成平台闭环。

静态阅读了任务 10 预览、模式观察、三个应用原生事务及 Claude 路由发布。按本轮用户覆盖要求，未编写或执行测试，未执行 Cargo、构建、检查、实机测试或最终回归；无 Cargo 槽位，两个缓存环境变量未设置。无编译或测试通过结论。后续补充并执行合同列出的 profile_t11_ 场景，再验收 macOS 实际事务与路由。

## 任务 1：领域接入契约

2026-11-14：契约文档编写完成，见 [profile-contracts.md](profile-contracts.md)。本条仅记录静态盘点，不代表 macOS 功能验收通过。

- 已阅读计划、任务 1、来源映射和母计划执行分线；逐项提取任务 2–30 的文件落点和实际父模块。
- 来源 `git -C /Users/xiangmin/worker/code/cc-switch rev-parse HEAD` 退出 0，得到 `ca58c0313360eae7c851fbd7553c1614c7af03e7`。
- 已静态核对三组 ProfileScope、未拍选择/切走补拍、项目作用域与路径设置所有者、四类领域入口、主机策略和锁顺序。来源到后续任务的对应关系见契约首表。
- 依赖缺项与下一步见契约“当前生产入口与缺项”；组合 DTO/API/SDK/Iced 尚待本计划实施，Desktop MCP 等不支持能力不能伪装成功。
- 用户本轮明确禁止执行检查、构建、单元测试、实机测试和最终回归，因此任务卡的 Python 关键字检查也未运行。没有测试通过数、编译通过或平台验收结论；以上只为源码阅读证据。
- 后续推荐先实施任务 2 的分组/快照 DTO；验证禁令解除后补各卡聚焦验证，28–31 再汇总 macOS 闭环证据。

## 任务 2：分组与快照 DTO

2026-11-14：实现完成，未验证。`vw-api-types::project_profiles` 定义三个受限组、独立分槽、组合 ID/实体/版本、四类引用快照及逐项/整体应用结果；已在 crate 根注册。复用 AppId 映射，拒绝不支持的组及未知载荷字段，Desktop wire key 为 `claude-desktop`。

- null/省略槽位表示不动，空 MCP/技能集合表示清空启用集，空提示词 ID 表示明确停用；任意非 null 类别表示该组已拍摄。`merge_scope_from` 只替换指定组。
- 技能引用保存 installationId，app_id 由其所在的组确定，两者共同定位安装；供应商/MCP/提示词保存领域 ID，启用集合表达启用位，不复制密钥或正文。未拍组和已拍空组可区分。
- selected 与 applied 分离；逐项状态包含成功、缺失、冲突、拒绝、失败、取消、不支持。调用者只有确认全部所需步骤后才可返回 applied；取消保留已提交结果。
- 已编写 5 个 `profile_t02_` 独立测试，覆盖 null/空集合、三组映射和独立覆盖、空提示词、非法组拒绝、部分成功序列化。源码审阅不等于测试通过。
- 按本轮明确覆盖要求，未执行 Cargo、构建、检查、任何测试、实机验收或最终回归；未领取 Cargo 槽位，CARGO_TARGET_DIR 未设置，无测试通过数量或退出码。后续解除禁令后通过仓库 cargo_slot.py 运行任务卡原定聚焦测试。

## 任务 14：应用提示词槽位

2026-11-14：实现指定组 `project_profiles/apply_prompt.rs` 与 `PromptsStore::apply_profile_prompt` 执行端适配。null 不读写；空串停用；相同激活项不重写；目标缺失返回 Missing，保留当前选择。读取在原生槽锁内取得目录 revision 和文件前像；激活调用母任务 13 的 activate_ordinary，停用调用 reconcile_ordinary，保留原事务、授权和冲突检查。无 current 标记或快照写入。调用者仍须完成执行主机路径解析、项目与物理槽授权及外层操作串行化；尚未接生产应用 API，不构成平台闭环。

按用户本轮覆盖要求，仅写代码及必要合同，未编写或运行测试，未执行 Cargo、构建、检查、实机或最终回归。未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。实现未经编译或行为验证。后续需验证 profile_t14_ 的 null、空串、重复应用零写入、缺失引用保留、版本/原生前像冲突、授权拒绝与组隔离，再验收 macOS。

## 任务 19：列表、当前状态与创建入口

2026-11-14：新增 GET/POST `/agent-apps/project-profiles` 及 GET `/agent-apps/project-profiles/current`。camelCase 参数为 projectId、targetHostId、scope，创建另需 name。租户只取认证 Extension，执行主机只取现有本地主机策略授权结果；不接受请求目录或租户。scope 仅 claude、claude-desktop、codex，未知字段拒绝。策略 grant 新增独立 `project_profiles_read`、`project_profiles_create` 项目 ID 数组，默认空；不从 workspace/provider 权限推导组合权限。列表实体按契约在租户内共享，当前标记额外检查项目占用。无授权在构造存储前拒绝，坏记录整次返回 profile_state_invalid，不伪装空快照。所有响应 no-store。

缺项：任务 7/8 提供同步快照回调合同，但四类实际状态读取尚无完整统一生产适配。创建调用现有 create_from_current 服务，读取明确返回 Unavailable，HTTP 503 snapshot_unavailable，零实体写入；当前不能成功创建，不能据此宣称任务故事或平台闭环完成。后续由领域所有者接入已授权实际状态读取，再补至少四个 profile_t19_ 场景（权限/租户/项目拒绝、非法 scope 零读取、坏记录、成功创建不激活），不得以目录期望状态替代原生有效状态。

遵照本轮覆盖仅写代码及合同、删除编号卡；未编写或运行测试，未执行 Cargo、构建、检查、实机测试或回归。未领取槽位，CARGO_TARGET_DIR 和 CARGO_BUILD_BUILD_DIR 未设置。没有编译、行为或验收通过结论。
