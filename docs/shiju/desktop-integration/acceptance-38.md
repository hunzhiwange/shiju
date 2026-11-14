# 任务 38：深链冷暖启动接收

实现：任务 33 的 macOS 单实例锁与 UnixDatagram 转发已存在。首实例 argv 与后续转发参数进入同一进程内编号队列；GUI 循环前注册 GURL Apple Event 处理器，接收 LaunchServices 的冷暖启动 URL。采用现有 objc2 0.6 与 Foundation 0.3 依赖，不新增 Cargo 依赖。

队列公开 `app::deep_links::pending()` 快照与 `acknowledge(id)`。读取不消费，未知或已确认编号返回 false；未确认的相同 URL 合并，确认后再次打开分配新编号。接收仅暂存，不解析或导入业务、不打印 URL。UI tick 唤醒窗口，不自动确认；预览与确认由后续任务 39 等消费者接入。队列存活范围为当前 GUI 进程，不承诺崩溃恢复。

macOS 安装包须注册 ccswitch/shiju URL scheme；本任务不更改协议所有权或签名。未确认隔离 app 的协议注册与系统事件投递。

遵照本轮用户要求：只完成代码，未运行 Cargo、单元测试、实机测试或回归；无槽位领取、无构建目录和通过证据。新增独立行为测试供未来统一验证，不能视为已通过。后续应核对 AppKit 冷暖启动投递、编号确认、预览接线与打包协议注册。
