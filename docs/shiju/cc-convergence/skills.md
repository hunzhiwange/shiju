# 技能插件唯一安装路径

2026-11-14，任务7。依据本轮用户要求仅做实现及静态审查，不运行任何 Cargo、构建、单元测试或实机验证。当前已是按职责划分的唯一所有者，本任务采用零删除路径，不制造行为变化。

## 当前调用链与权威所有者

路径均相对仓库根。

|入口/职责|实际生产符号及存储|保留的边界|
|---|---|---|
|外部应用技能安装|`crates/vw-desktop/src/app/session_gateway.rs::gateway_skills_install_view` → SDK `client/agent_apps_skills/install_api.rs::agent_apps_skills_install` → 网关 `api/handlers/agent_apps_skills/install_api.rs::execute`|提交一次安装请求，按返回任务ID观察终态，不由Iced直接写执行主机目录|
|安装权威记录|`crates/vw-agent/src/gateway/agent_apps_skills_store.rs::SkillInstallsStore`，`agent_apps.skills.installed` namespace|可信TenantScope/ProjectScope及target_host_id定位记录；SSOT由执行端授权解析，不接受客户端任意本机路径|
|本地/Git/ZIP来源|`install_local_with_source`、`install_repo`及后台repo任务、`install_from_zip` → `commit_install`|不同来源预检保留；ZIP逐项成功/跳过/失败结果不伪装整体原子；发布和settings提交使用既有回滚|
|物理目录发布|`crates/vw-agent-apps/src/skills/local_install.rs::LocalInstall`|prepare阶段受控复制、物理根锁、排他rename；commit失败显式rollback，成功finish；不执行安装内容|
|应用绑定投影|同一Store的绑定/启停入口及 `skills/{copy_deploy,link_deploy,disable_deploy}.rs`|权威安装与应用投影职责分离，不能把一次分发当成第二权威安装|
|ShiJu项目技能|`gateway/api/handlers/desktop_skills.rs::install_builtin_post`、`install_built_in_skill`；`skill/install.rs::handle_command`|工作区技能目录、内置资源及CLI来源审计；与外部应用按主机/项目管理的SSOT不同，不强制合并|
|插件包|`crates/vw-agent/src/plugins/install.rs::install_plugin_from_source`、`install_plugin_from_marketplace` → 私有 `install_plugin`|插件清单、市场身份、依赖及包审计独立；插件提供的技能只汇入目录视图，不因此改写外部应用安装记录|

`execute` 在初始化存储、查询来源及发起网络之前调用 `authorize_skill_install_owner`。本地来源校验目录/文件类型，Git要求已保存且启用的仓库。`SkillInstallsStore::commit_install` 在 `GatewaySettingsStore::update_with_preimages` 中检查版本及大小写不敏感的目录身份冲突，再发布、提交或回滚。以上为源码事实，不代表当前版本已运行验证。

## 来源与删除集合

正式包 `crates/vw-agent-apps/src/lib.rs` 明确将 `upstream` 作为参考快照，正式模块树注册 `pub mod skills`。来源 `upstream/src-tauri/src/services/skill.rs`、DAO及TS hooks的调用者仍属于来源自身模块树；不得在本任务删除来源片段或把参考中的调用边当成正式运行双写。

最小删除集合为空。现有项目技能、插件安装及ACP会话挂载具有不同权限和存储职责，保留这些路径；来源整体归档与移除由任务21、22处理。未增加兼容层、后门、迁移或新领域接口。

## 静态证据与待统一验证

已读取准入表、母技能执行合同、本任务卡与只读计划。RustCodeGraph status退出0；explore发现图主要返回来源和已有desktop技能，正式新技能文件未被有效命中，因此用rg及原文件读取补齐真实API、SDK、Store、安装实现与Iced调用。旧母技能执行合同关于skills模块尚不存在的陈述已过时，以当前生产模块为准。

现有行为测试可供后续统一验证：`gateway/api/handlers/agent_apps_skills/install_api_tests.rs` 的 `cp12_install_api_success`、`cp12_install_api_rejects_without_side_effects`、`cp12_install_api_archive_preserves_individual_results`、`cp12_install_api_git_requires_enabled_saved_repository`，以及原生local/repo/ZIP安装测试。当前均未运行，未新增镜像实现或字符串匹配测试，未生成脱敏夹具及数据前后哈希；跨租户拒绝、失败回滚、重启读取、ACP挂载隔离仍待统一行为验证。

本任务无业务数据和运行代码改动，无需数据回退。恢复时仅逆转本证据文件，不覆盖工作区已有修改（包括SDK repos_api.rs的其他任务变更）。准入基线、来源哈希与恢复约束见[准入与所有者](准入与所有者.md)。
