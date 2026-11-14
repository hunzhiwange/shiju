# 任务 47：更新检查来源

日期：2026-11-14。实现已交付，验证未执行。

直接依赖任务 1 的盘点文档 acceptance-1.md 已存在。没有外部 SDK 或新增发布产物；沿用工作区 vw_shared::update 和 Iced 消息契约。

源码核对：UpdateSource 只允许 Shiju/Github，默认 Shiju，非法持久化来源拒绝使用。元数据固定来自 https://www.shiju.cloud/agents/shiju-agent/latest/manifest.json 或 https://api.github.com/repos/hunzhiwange/shiju/releases/latest，不使用 CLI 环境覆盖，不引入 CC 更新源。手动及后台结果校验请求编号与来源；RunAppUpdate 克隆已检测 DesktopRelease，prepare_desktop_update 按快照校验资产再下载，不重新读取 latest。原业务逻辑无需修改。

新增独立 desktop_integration_47_tests 模块，覆盖默认和合法来源、非法来源拒绝、错来源结果拒绝，以及两种来源的当前快照下载边界。空资产快照应在创建缓存及网络请求前报错，断言实际错误消息；这些仅为已编写的测试，未运行。

按本轮用户覆盖指令，未运行 Cargo、单元测试、实机测试或最终回归，未领取槽位，CARGO_TARGET_DIR 未设置，无退出码、通过数或实机证据。后续统一执行 desktop_integration_47_ 定向测试并人工验收。任务卡按用户要求删除，不表示验证通过。
