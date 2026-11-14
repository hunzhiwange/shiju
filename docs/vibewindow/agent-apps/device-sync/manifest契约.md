# 原生 manifest 校验入口

`vw_agent_apps::device_sync::manifest::SyncManifest::from_slice` 接受原始 JSON 字节与共享快照所有者提供的 `supported_data_version: u32`。仅支持 `shiju-device-sync`、协议版本 1 和精确匹配的数据版本；无默认版本或旧 CC 回退。共享快照生产版本尚未交付时不得推测数值或接入生产同步。

JSON 使用 snake_case 字段，闭合 schema；artifacts 对象恰含 `config.json` 和 `skills.zip`，各有小写 64 位十六进制 sha256 与 u64 size。缺失、重复、未知字段或制品均拒绝。snapshot_id 是按制品名字典序连接 `name:sha256`（分隔符 `|`）后的 SHA256 小写十六进制值，必须匹配。size 是声明值；实际制品摘要、长度和制品大小上限由后续任务9校验，不能将 manifest 校验当成制品完整性证明。

manifest 原始输入超过 1MiB 拒绝；传输层还必须在读取时执行相同上限。created_at 必须是 RFC3339。device_name 去除 Unicode 控制字符后保留前64个 Unicode 标量，中文按字符计数；它只用于显示，不构成授权身份、主机标识或路径。使用 `normalize` 校验本地构造值后再发布；出站编码同样限制1MiB。

本轮仅实现并人工审阅代码，依最新指令未运行 Cargo、测试、实机检查或最终回归，未领取构建槽位或设置缓存目录。后续统一验证需覆盖格式与版本拒绝、闭合制品及重复键、snapshot_id 篡改、1MiB边界、中文设备名和控制字符处理。没有编译或行为通过证据。
