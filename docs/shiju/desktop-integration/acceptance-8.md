# Skills 页能力收口（任务 8）

2026-11-14：实现已写入共享 checkout，未验证。按本轮用户覆盖要求，未运行 Cargo、脚本、单元测试、实机或回归；未领取槽位，未创建工作树。删除任务卡不代表验收通过。

已阅读直接依赖 acceptance-2.md 与领域 docs/vibewindow/agent-apps/skills/macOS交付证据.md。来源版本沿用 ca58c0313360eae7c851fbd7553c1614c7af03e7。现有仓库、市场、安装、绑定和更新入口已有对应 SDK；不重做领域 CRUD。14 应用的绑定支持仍取自 SkillBindingSnapshot.capabilities，未验证的应用不标成不支持。

本次补齐“技能设置 → 应用技能绑定 → 刷新 → SDK 失败反馈”链路：message/settings/skills/bindings_view.rs 对网关初始化失败和缺少授权显示 toast；Completed 的错误进入弹窗 error 和 toast，不再丢失。session_gateway.rs 的 gateway_skill_bindings_request 保留绑定目标与库存读取的 SDK 错误，并避免通用重读提示覆盖已有错误。逐项结果、代次过滤与响应 scope 校验保留。

提交边界：bindings_view.rs 是其他领域任务尚未跟踪的完整文件，session_gateway.rs 同时存在其他会话尚未提交的新增实现。本任务只修改上述局部逻辑，不将其他会话文件整体纳入提交；代码留在共享 checkout，待依赖所有者整理提交。本任务仅独立提交此交接文档及任务卡删除。

后续统一验证：绑定入口初始化错误、缺少授权、SDK 读取拒绝/失败、迟到响应与作用域不匹配；核对错误提示和逐项结果保留；人工检查 macOS 明暗主题弹窗及 toast。未新增或运行测试，不声称通过。
