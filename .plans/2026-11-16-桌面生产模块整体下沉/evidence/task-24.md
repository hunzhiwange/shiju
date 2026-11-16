# 任务24实施证据（尚未完成）

使用 do-task-plans、rustcodegraph、git-commit 技能。已读取公共计划；严格未运行 Cargo、测试或绘制。

## 生产调用链

- App 继续持有 McpSettingsState，但定义、默认值与局部页面状态代理归 management_state。
- mcp_management_list::input 仅收集选定网关、认证上下文、令牌和可信安装实例；不传入 App。
- 宿主 form/import/bindings::update → configuration_update::form/import/bindings → 对应领域 reducer；旧状态机移除。
- management_update 提供局部借用、catalog 可见性与具名反馈/刷新效果；真实 GatewayClient 执行认证请求。
- probe::invalidate 的状态清理与原授权取消归领域；宿主提供导航和变更信号并映射结果。
- connection_form_panel 的字段投影归 management_form::panel；宿主仅映射消息。
- list/resync/tools 原有领域 reducer 保持真实调用，宿主仅投影跨页输入。

## 已迁断言

原 catalog 测试整组 → host_module_24_tests::catalog；保留真实中文明暗 draw。
原 form 的 preset、modal draw、SDK conflict、graphical values、transport credentials → host_module_24_tests::form；测试名及断言保留，transport 用真实领域 Update 替代 App。
原 bindings 的 unsupported、partial/unverified、14 applications、old context/duplicate、single/bulk SDK → host_module_24_tests::bindings；测试名及断言保留。
原 import 的结果及 malformed 断言 → host_module_24_tests::import。
以上测试旧挂载已删除或缩减；新模块通过 lib.rs 独立注册。未运行。

## 未完成，不能删任务卡

- tools/probe/resync/import 仍有依赖 App 的领域断言、页面绘制和大型测试夹具，需要继续拆离领域与跨域契约。
- 尚未完成全部旧断言与前轮 desktop_split 测试逐断言去重及所有残留函数审计。
- 未编译、未测试，静态语法及差异审查不能替代运行证据。

## 当前统计

baseline task=24 splitlines：来源 3986 → 2986，净减少 1000；生产候选 1146 → 616，减少 530。本任务未新建宿主文件。共享 infrastructure 中的 MCP 状态迁移另外减少34行，不混入独占来源数字。

后续仅用户授权后：python3 scripts/cargo_slot.py cargo test -p desktop-settings-mcp --lib host_module_24_tests -- --test-threads=1；宿主 cargo check 亦未运行。
