# 技能插件 macOS 交付证据

审查日期：2026-11-14。任务57状态：已完成（仅静态交付审查）。用户本轮要求不执行 Cargo、单元测试、实机或最终回归，不切分支、不建工作树、不提交。本文记录代码与断言位置，不声称编译、测试或macOS行为通过；母任务12未由本任务标记完成。

## 直接依赖及外部产物

直接依赖43、51、44、45、46、52、47、48、53、54、50、49、55的编号文件均已不在当前目录；删除不能代替完成证据。静态可见以下产物：

|任务|桌面产物（相对 crates/vw-desktop/src/app/）或服务产物|本次结论|
|---|---|---|
|43|message/settings/skills/repo_modal.rs；components/system_settings_skills/repo_modal.rs|实现与独立测试文件在位，未运行|
|44|同目录 market_view.rs|实现与独立测试文件在位，未运行|
|45|同目录 install_view.rs|实现与独立测试文件在位，未运行|
|46|同目录 import_modal.rs|实现文件在位；未发现同名独立状态测试，补测范围待统一验证确认|
|47|同目录 bindings_view.rs|实现与独立测试文件在位，未运行|
|48|同目录 updates_view.rs|实现与独立测试文件在位，未运行|
|49|同目录 backup_modal.rs|实现与独立测试文件在位，未运行|
|50|同目录 storage_modal.rs|实现与独立测试文件在位，未运行|
|51|同目录 claude_view.rs|实现与独立测试文件在位，未运行|
|52|同目录 jobs_view.rs|实现文件在位；未发现同名独立状态测试，补测范围待统一验证确认|
|53|message/settings/skills/deeplink.rs、deeplink_tests.rs|解析/预览与拒绝断言在位，OS接入需母20|
|54|crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/macos_api_tests.rs|本地生命周期、仓库发现/Git安装/远程更新/提交前取消及SQLite/PostgreSQL入口在位，未运行|
|55|同目录 macos_files_tests.rs|生产存储适配与隔离文件系统夹具在位，未运行|

公共入口：crates/vw-agent-apps/src/skills/mod.rs 注册原生模块；crates/vw-agent/src/gateway/api/handlers/agent_apps/mod.rs 合并技能API；crates/vw-gateway-client/src/client/agent_apps_skills/mod.rs 注册SDK；crates/vw-agent/src/gateway/agent_apps_skills_store.rs 负责生产设置存储适配。来源固定提交 ca58c0313360eae7c851fbd7553c1614c7af03e7，30项文件哈希重核一致；许可和来源清单保留。API类型、SDK与UI映射见来源覆盖.md。

## 自动证据边界

以下均为静态阅读的验收代码，实际有效测试数、运行退出码、平台/应用版本与槽位均未取得，不能记为通过。

- SDK 的 agent_apps_skills/*_api_tests.rs 使用 client::test_support::server 返回预设HTTP响应：可验请求路径、token头、DTO和脱敏错误，不证明真实安装。install_api_tests.rs::cp12_install_api_success 覆盖 Local/Archive/Git 请求序列化及逐项结果。
- macos_api_tests.rs::cp12_macos_api_success 使用临时真实文件、生产路由及真实GatewayClient，覆盖本地安装、库存/详情、更新拒绝、任务断连重读、终态取消及卸载备份；cp12_macos_api_rejects_without_side_effects 包含非法token、项目、主机、客户端路径及穿越拒绝。HTTP服务仅绑定127.0.0.1，不访问公共网络。安装包内install.sh应保持惰性。
- macos_api_tests.rs::remote_update 使用本地HTTP服务替代外部GitHub归档响应，仍经过生产路由、真实SDK、安全解压、生产设置存储与文件发布。cp12_macos_api_remote_update_and_precommit_cancel 和 cp12_macos_api_postgres_remote_update_and_precommit_cancel 分别提供SQLite/PostgreSQL入口，均遍历更新成功和取消两种分支。断言覆盖单仓库发现、Git安装、更新检查、拒绝其他项目/主机、更新后版本与哈希变化、下载阻塞时请求取消、取消后旧库存与字节保持、HTTP服务重启后重读终态及卸载。取消分支断言请求时仍为Started，最终为Interrupted且不要求协调；不是把取消请求响应当成取消完成。
- cp12_macos_api_postgres_success 及 cp12_macos_api_postgres_remote_update_and_precommit_cancel 需要 VIBEWINDOW_TEST_POSTGRES_URL，缺少时显式失败。双数据库验收代码入口在位；本轮未连接数据库，未获取SQLite/PostgreSQL运行证据。
- 取消与归属实现已静态复核：skills/job_cancel.rs::request_cancel 仅在Started/Staged记录请求，Committing及终态保持真实结果；agent_apps_skills_store.rs::set_job_phase 在持久事务中检查取消并串行化提交边界，start_skill_update_at 在网络准备期间轮询取消，提交前调用阶段转换。owned_job 在同一事务中匹配作用域、主机、principal及资源根目录，缺失归属的旧任务不暴露；updates_api.rs 的启动记录principal，状态重查先核验归属。jobs_api_tests.rs 包含其他owner查询/取消返回NOT_FOUND的断言。上述实现及断言均未运行。
- macos_files_tests.rs::cp12_macos_files_success 使用临时目录、SkillInstallsStore，断言Copy/Symlink、迁移、卸载备份、保护外部编辑和恢复；cp12_macos_files_rejects_without_side_effects 断言未授权迁移、旧版本卸载和越界恢复不改变目录或存储。它不等于启动真实应用/CLI。
- 桌面 *_tests.rs 的状态转移或Element构建断言不证明真实字形整形、暗黑主题、滚动条、弹窗布局和Finder打开成功。

## 明确缺口与人工交接

1. 原“缺少仓库发现/更新成功及提交前取消夹具”的TODO已由任务54补齐。默认生产端点仍为GitHub/codeload；服务内部router_at允许验收替换外部HTTP边界，客户端不能覆盖归档URL。新夹具不是访问真实GitHub，也不证明真实网络、应用副本更新回滚或所有提交竞态已验收；双数据库入口存在不等于双数据库验证通过。
2. 本轮未运行全部必要聚焦测试、类型检查与最终回归；后续统一验证需记录命令、有效数量和退出码，不能从任务删除推导通过。
3. 用户依母计划人工验收.md执行真实macOS应用/CLI、版本、应用目录与权限、Copy/Auto/Symlink、修改保护、Finder、中文动态文本与暗黑主题、弹窗滚动、来源链接确认以及取消/失败重试。GUI与CLI均待验，不因缺证据标成不支持。
4. 分发不添加ACP会话选择或full_access授权；会话挂载/撤权仍由既有ACP合同负责。真实会话集成尚待统一验证。

## 静态检查记录

已读取母子plan、能力覆盖、来源清单、母任务12、执行分线及来源与任务映射；保留规划文件原文。RustCodeGraph status退出0、explore退出0；图未给出本次新增技能链路完整上下文，继续用文件读取核对。30来源哈希核对退出0，结果30/30。git diff --check -- docs/vibewindow/agent-apps/skills 退出0；本文已重新读取，未以空diff代替未跟踪文档审查。没有运行任何测试。

任务54后续静态复核：已重新读取 macos_api_tests.rs、updates_api.rs、jobs_api.rs、install_api.rs、discovery_api.rs、agent_apps_skills_store.rs 的归属/阶段/更新执行代码，以及 skills/job_cancel.rs、job_cancel_tests.rs 和 jobs_api_tests.rs。更新远程更新、提交前取消、双数据库及任务归属描述，移除过时TODO；本次仅改文档，不重建任务卡、不运行Cargo或测试、不提交。
