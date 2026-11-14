# 任务 40：技能深链预览

已实现启动队列中的 skill 资源预览，兼容 ccswitch 与 shiju scheme。
预览显示仓库、分支、当前项目目录和目标主机；拒绝才移出队列，继续时交接到仓库确认弹窗。
交接先读取仓库版本，显式确认通过现有 gateway_skill_repos_request → skill_repo_save SDK 保存仓库，不自动安装技能。
项目、网关和主机变化由现有仓库状态作用域检查拒绝，SDK 失败保留预览供重试。

直接依赖：任务 38 的 deep_links 队列已存在；任务 8 的应用入口已存在。
外部契约使用当前工作区 vw-gateway-client 的 SkillRepoEntry、SkillReposResponse、SaveSkillRepoRequest，不新增接口。
实现文件：skill_link_preview.rs，以及系统设置消息、弹窗和模块注册入口。

本轮用户要求只写代码：未运行 Cargo、单元测试、实机测试或回归；无槽位领取，无测试通过结论。
后续统一验证应覆盖冷/热启动、两种 scheme、无项目、非法字段、拒绝、确认保存、SDK 失败重试及作用域变化。
