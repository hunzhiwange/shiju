# 任务 48：更新下载进度

2026-11-14：实现交付，未验证。

直接依赖任务 47 的 acceptance-47.md 已交付。沿用工作区 vw_shared::update::download_desktop_installer、UpdateProgress 和 Iced 消息契约，无新增 SDK。底层使用响应 chunk 的实际字节数上报进度，校验清单大小与 SHA-256；失败尝试由临时目录清理，重试使用保留的发布快照创建新下载。

修复 AppUpdatePrepared 先于订阅轮询到达时直接丢弃进度队列的问题：完成时收取最后一条真实进度，再释放接收器。成功或失败都保留真实计数，不人为补成 100%。既有错误路径恢复下载按钮，重试清空旧进度。

新增 desktop_integration_48_preserves_expected_behavior，覆盖实时字节进度、超过单次轮询预算的尾部事件、失败后的进度保留和重试状态。仅编写，未运行。

按本轮用户覆盖要求，未执行 Cargo、脚本、单元测试、实机测试或最终回归；未领取槽位，未设置 CARGO_TARGET_DIR，无通过数或发布包实测证据。任务卡按要求删除。后续统一运行 desktop_integration_48_ 定向验证及 macOS 发布包人工验收。
