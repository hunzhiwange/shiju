# WebDAV 上传

`WebDavUploadTarget::new` 消费执行侧凭据，校验启用配置与快照所有者提供的非零兼容版本，创建禁止重定向的客户端。此构造仅做本地校验；Gateway 仍须授权主机网络访问并解析凭据。

`upload` 只接受 `UploadArtifact::{Config, Skills, Manifest}` 及固定的暂存字节。路径复用原生 URL 构造器；config.json 为 application/json，skills.zip 为 application/zip，manifest.json 保持 application/octet-stream。制品上限 512 MiB，manifest 上限 1 MiB。仅 2xx 成功；重定向及其它状态返回脱敏错误，等待响应超时为 30 秒。不读取或显示响应体、URL、凭据及底层错误。

调用者负责使用过滤后的快照和归档、准备目录、先上传两份制品再发布 manifest。单次上传不提供事务性；超时可能已写入远端，不自动重试。

本轮按用户要求只实现和静态审阅，未执行 Cargo、测试或实机验证，未领取构建槽位。后续统一验证须覆盖 loopback PUT 字节及 Content-Type、非成功状态、重定向不跟随和响应超时。
