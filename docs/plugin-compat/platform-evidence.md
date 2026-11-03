# 平台行为证据

本表仅描述已取得证据，不表示三平台完全兼容。当前记录见 [t64.json](evidence/t64.json)。

| 行为 | macOS arm64（本次） | Linux | Windows |
| --- | --- | --- | --- |
| 原包清单、中文/空格路径、真实安装发现 | 通过 | 未验证 | 未验证 |
| Python stdio 工具发现、真实调用、外部输出与 shutdown 请求 | 通过 | 未验证 | 未验证 |
| 宿主 hook 命令与包外数据 | unix 分支通过 | 未验证 | commandWindows 未执行 |
| 无效清单拒绝、来源隔离、包字节不变 | 通过 | 未验证 | 未验证 |
| 完整 OAuth 回调链 | 未验证 | 未验证 | 未验证 |
| Apps UI 显示/交互 | 本次未验证；历史 t31.json | 未验证 | 未验证 |

新增 `t64_tests` 三项测试及 `platform_tests` 一项测试通过真实实现执行，4 passed、0 failed、0 ignored；clippy 退出 0。夹具复用仓库编写的 MIT t61 包，不冒充公开原包。来源与预期值登记于 `crates/vw-agent/testdata/plugin_compat/t64/sources.json`。没有生产代码变更或目标行为红灯证据；新增验收首次即通过。

相邻 t32 测试退出 101，未产生完整测试汇总；日志见 `evidence/t64/adjacent.txt`。任务状态及后续验收条件仅记录在编号任务文件中。
