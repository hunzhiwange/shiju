# 模型恢复的原生窗口验收

macOS 显式运行（需要当前用户已登录图形会话）：

```sh
python3 scripts/verify_model_recovery_window.py --output /tmp/model-recovery-evidence
```

输出目录必须尚不存在。脚本动态领取仓库共享 Cargo 槽位 1–10，设置绝对
`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR`，精确选择 Cargo JSON 中的测试目标，
不会运行普通 `shi-ju` 二进制。结束、失败或中断后，先结束自己启动的进程组，再释放锁；
编译缓存保留。测试不会更改 HOME、用户 bootstrap 配置或生产数据库。

后端是独立的 `vw-agent` 测试进程：临时 SQLite 和配置根目录、生产客户端路由及令牌鉴权、
真实 provider 解析、processor、session runtime 和恢复预算。仅模型 HTTP 边界是本地可控服务：
首次 503 后成功、永久模型缺失、持续 503 耗尽、等待期间显式取消。

桌面入口在主线程运行真实 Iced 窗口，使用 `App::new`、生产聊天流订阅、`App::update`、
聊天面板、输入控件及 toast。驱动器通过同一应用消息接口建立请求、触发取消，
以 Iced 窗口截图任务记录实际绘制；不使用系统点击或外部原生 UI 自动化工具。
恢复进度的截图完成后才释放成功响应，取消后等待真实 Gateway 停止确认。

四场景分别执行浅色、深色主题，共 8 项。每种主题核验模型 HTTP 调用数：
恢复成功 2 次、真实缺失 0 次、耗尽 3 次、取消 1 次。核验状态清理、
失败原因留在 System 消息、尝试与重试次数，并生成 12 张窗口截图。

输出包含 `slot.json`、`window-results.json`、`model-calls.json`、构建日志、窗口日志和 PNG。
成功必须退出 0，不能把已生成部分截图或编译成功当作验收通过。

这是恢复链路的自动化窗口验收，不声称人工点击验收。驱动器只订阅实际聊天流，
不启动与目标无关的系统菜单、终端和全局轮询；系统菜单的阻塞接收器会阻止独立
Iced 测试运行时退出。测试不覆盖菜单或整个桌面壳层的生命周期。
