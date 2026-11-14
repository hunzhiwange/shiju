# WebDAV 远端快照查询

`device_sync::webdav_remote::query_remote_snapshot` 由网关授权主机网络访问并提供凭据、共享快照数据兼容版本。只访问 URL 所有者生成的当前原生目录，不探测 CC 或其它版本目录。

一次有界 manifest GET 同时取得 ETag，无须额外 HEAD；沿用下载传输的 1 MiB 流限制、时限、禁重定向和凭据错误脱敏。404 返回 `Empty`；认证、网络、大小限制或缺失 ETag 等传输错误返回 `WebDavError`，不能当作空远端。

JSON、原生格式、摘要身份、时间等结构错误返回 `InvalidManifest`。结构有效但协议或数据版本不匹配，返回带 `UnsupportedProtocol` 或 `UnsupportedDataVersion` 的快照；协议不匹配优先。展示元数据包含规范化设备名、时间、snapshot ID、版本、兼容性、编码后的 manifest 路径和原始 ETag。路径不包含主机、凭据或查询串。

预览不读取 config.json / skills.zip，不证明制品存在或完整，不授权恢复。不兼容快照不能用于下载应用；实际下载仍须调用严格 manifest 校验并验证制品。ETag 和快照只表示本次观察，后续写操作仍需重检。

本轮依用户要求仅实现并审阅源码，未运行编译、单元测试、其它测试或实机验证，未领取 Cargo 槽位；后续统一验证应覆盖 404、损坏与不兼容区分、ETag、越界流及无制品请求。
