---
name: computer-control
description: 使用识矩真实 computer_control 工具观察、输入、拖动和停止已授权的 macOS 原生应用；用于本机应用操作及驱动、权限诊断。
---

# 原生应用电脑操控

先检查当前工具目录中是否存在 `computer_control`。技能只编排该工具，不提供 CLI 或 MCP server；工具未注册时报告“当前会话没有 computer_control 工具”，请用户检查识矩宿主与工具启用状态，不虚构调用或改用 shell 注入输入。

使用当前已确认应用的 bundle_id 和 process_id；不得猜测身份或使用其他应用替代授权目标。下面 com.example.fixture 与 42 仅为格式占位，实际调用前替换。会话、凭证与权限版本由宿主提供，不能放入工具参数。

## 观察、动作、重新观察

1. 确认用户授权目标和操作范围。先调用观察，读取可访问性文本与截图。
2. 坐标来自最近一次同一应用截图的像素坐标。窗口改变、目标变化或观察过期时先重新观察；不得按屏幕全局坐标猜测。
3. 每次执行一个有界动作，立即重新观察核对结果。动作返回成功只代表执行完成，不能代替回读。
4. 结束或用户要求停止时调用 stop；发生取消或权限错误后停止动作，不循环重试。

以下 JSON 均为 `computer_control` 的完整参数，可在替换真实目标后调用：

```json
{"target":{"kind":"native_app","bundle_id":"com.example.fixture","process_id":42},"action":{"action":"native_observe","screenshot":true}}
```

```json
{"target":{"kind":"native_app","bundle_id":"com.example.fixture","process_id":42},"action":{"action":"native_input","input":{"kind":"click","x":120,"y":80,"button":"left"}}}
```

```json
{"target":{"kind":"native_app","bundle_id":"com.example.fixture","process_id":42},"action":{"action":"native_input","input":{"kind":"type_text","text":"示例文字"}}}
```

拖动使用截图内的路径：2–128 个整数坐标点，duration_ms 为 1–5000，button 为 left、right 或 middle。不可拆成无释放保障的按下/移动指令。拖动后重新观察；失败时停止，不盲目重复拖动。

```json
{"target":{"kind":"native_app","bundle_id":"com.example.fixture","process_id":42},"action":{"action":"native_input","input":{"kind":"drag","path":[[120,80],[160,100],[200,120]],"button":"left","duration_ms":400}}}
```

```json
{"target":{"kind":"native_app","bundle_id":"com.example.fixture","process_id":42},"action":{"action":"stop"}}
```

stop 停止当前会话的控制。用户在界面停止会话或撤销权限也应立即生效；不得重新建立控制绕过停止意图。其他原生输入仅按实际工具 schema 使用 activate、scroll、press，不运行脚本。Chrome、Excel、Word 需要独立连接和授权，不以本技能的原生操作流程声称已支持其完整链路。

## 缺少驱动、权限与失败处理

- 工具存在不等于执行服务可用。原生执行依赖识矩包内 `Contents/Helpers/ShiJu Computer Use.app/Contents/MacOS/shiju-computer-use`；缺失时请用户安装包含该服务的完整识矩包，不能复制 Codex 驱动或借用其授权。
- 插件安装不会自动取得 TCC。权限由实际执行的 ShiJu Computer Use.app 持有；用户应在系统设置的隐私与安全性中核对该进程的辅助功能（新版系统可能称设备控制与数据访问）和屏幕录制权限，按系统要求重启，再重新观察。不要静默申请或修改 TCC。
- PermissionDenied 也可能来自识矩策略或目标授权，先核对原生控制开关和应用授权，再由用户核对系统权限；不能直接断言是 TCC。StalePermission 需重读授权状态；TargetChanged 需重新确认目标；Cancelled 后尊重停止；Unsupported 不宣称已支持锁屏或其他系统。
- 当前服务端可能将启动、连接及系统权限失败归为 BackendFailure。仅凭此错误不能断言驱动缺失、辅助功能或屏幕录制权限不足；明确报告“执行后端失败，具体原因未确定”，列出包内服务、运行状态与权限的人工排查步骤。不得把资源安装 status 当作驱动在线或权限通过证据。
- 没有成功的观察与动作回读，不声称电脑操控已就绪。不要输出凭证、socket 鉴权数据或敏感截图内容到诊断日志。
