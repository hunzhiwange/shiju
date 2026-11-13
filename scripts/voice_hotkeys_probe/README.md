# macOS 全局热键自动探针

从仓库根目录运行（入口自动使用 `scripts/cargo_slot.py`）：

```sh
python3 scripts/voice_hotkeys_probe.py
python3 scripts/voice_hotkeys_probe.py --suppression default
python3 scripts/voice_hotkeys_probe.py --source null
# 缩短窗口占用，仍执行八组断言（额外双击循环从10次改为1次）
python3 scripts/voice_hotkeys_probe.py --double-taps 1
```

要求当前执行进程已有输入监控和辅助功能权限。探针不申请权限，
只开启自己的临时输入窗口；每次注入前校验前台 PID，焦点改变即失败停止，
不会重新抢回焦点继续发送。结束时释放按键、关闭自建窗口并按条件恢复原应用。
配置在临时目录，仅注册测试组合 Control+Option+Command+A/C；冲突会失败。

探针直接编译仓库中的热键 Runtime、原生监听、权限、偏好及本机语言校验源码。
只为未调用的 Iced 消息类型声明最小编译上下文，不替代热键或系统权限行为。
离线使用根 Cargo.lock 的依赖版本，全部 Rust 产物继承共享槽位；临时源码和窗口程序结束后删除。

八组断言覆盖：长按/重复/释放，双击免提/再次结束，单击超时对照，
第二击标记为自动重复时的超时对照，连续十次双击，独立聊天/重复，
Escape 取消，停用/恢复/模拟休眠通知安全锁存。输出 POST/POLL 的单调时间戳，
只含测试键码、重复标志和动作，不读取用户文本，不录音、不播放、不访问服务。

两个超时对照用于验证探针能识别失败形状；第二击的重复标志是主动构造的，
不能推断历史失败也由该标志造成。模拟通知不是实际休眠。
本入口验证真实 CG tap/Carbon 与生产状态机，不能替代完整 ShiJu.app 的真实录音、
听写回填、聊天窗口启动、锁屏/休眠及权限撤销验收。
