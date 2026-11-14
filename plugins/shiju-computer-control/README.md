# ShiJu 电脑操控本地资源

原生应用宿主链接在 ShiJu 桌面程序内。此插件只提供 Chrome Native
Messaging 中继、扩展和 Excel/Word 加载项；不声明独立 MCP server。
锁屏执行是否可用以桌面宿主探测结果为准，安装资源不代表授权或就绪。

## 本地安装（macOS）

先将 ShiJu.app 移到稳定位置，再使用 Python 3（需保持解释器安装位置有效）：

```sh
python3 "/Applications/ShiJu.app/Contents/Resources/computer-control/install_local.py" install
python3 "/Applications/ShiJu.app/Contents/Resources/computer-control/install_local.py" status
```

仅写入 `~/.shiju/computer-control`，权限 0700。安装记录保存每个自有文件的
SHA-256；重复安装可更新，未知文件冲突、符号链接和已修改资源直接拒绝。
status 仅报告资源完整性，不能作为后端运行或系统授权证据。

Chrome 106+：在桌面宿主创建的隔离 profile 中，通过开发者模式加载
`~/.shiju/computer-control/plugin/chrome`。扩展必须具有中继批准的 ID
`ogfgcnpdhnmkpcbaclbglefaclanipmg`；未提供稳定扩展签名 key，因此任意
unpacked 安装不保证取得该 ID。ID 不符时必须由维护者提供匹配签名的扩展，
不可放宽 allowed_origins。注册仅位于隔离 profile 的 NativeMessagingHosts，
不修改日常 Chrome profile 或组织策略。每个文档需点击扩展临时授权。

Office：使用支持 ExcelApi 1.2、WordApi 1.3 的 Excel/Word 客户端。
将安装目录下 `plugin/office` 静态托管于 `https://localhost:3000/office/`，
自行配置 localhost TLS 证书并由用户决定信任；安装器不会部署 HTTPS 服务、
生成或静默信任证书。通过 Office 支持的旁加载流程分别选择 excel/manifest.xml
与 word/manifest.xml，打开任务窗格。在 ShiJu 设置中按文档配对，令牌仅保存在
会话内；WebSocket 服务地址和一次性凭证由实际宿主提供。

原生控制需用户在系统设置中授予 ShiJu 辅助功能、屏幕录制权限；涉及应用自动化
时按系统提示授权。不得通过安装器改写 TCC。重启后应重新检查宿主状态、Chrome
扩展连接和 Office 配对，分别在临时文档验证操作与回读。本轮尚未执行这些验收。

## 停用与卸载

先在 ShiJu 设置停止会话、撤销授权并停用插件，再从隔离 Chrome 删除扩展、
从 Office 删除旁加载项并关闭任务窗格。之后运行：

```sh
python3 "/Applications/ShiJu.app/Contents/Resources/computer-control/install_local.py" uninstall
```

卸载只删除安装记录中哈希仍匹配的资源；保留用户文件、profile、socket 和目录。
卸载资源本身不替代宿主权限撤销，也不声称已完成工具撤销实机验收。
