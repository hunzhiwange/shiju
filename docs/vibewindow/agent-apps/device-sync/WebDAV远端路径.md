# WebDAV 远端路径

`device_sync::webdav_url::build_artifact_url` 接受地址、root、profile、共享快照兼容版本及固定制品名，生成 `<base>/<root>/<profile>/shiju-device-sync/v1/data-v<版本>/<制品>`。兼容版本由快照所有者提供，零值拒绝；不推测当前支持的版本。

沿用 WebDAV 配置的默认 root/profile 与校验规则。root 可由斜杠分段，profile 只允许单段；空格和中文按 UTF-8 逐段编码。百分号编码先解码一次，再交给 URL 路径段编码器，避免双重编码；空中间段、点段、编码斜杠、反斜杠、嵌套编码及 URL 分隔符直接拒绝。仅 base 被解析为 URL，任何路径输入都不能替换 origin 或跳出原生命名空间。制品只接受 config.json、skills.zip、manifest.json。

传输调用者使用 `build_client`，逐请求附加凭据；所有重定向均不跟随，应作为错误处理，禁止自行向 Location 重放认证信息。此模块不授予联网权限，也不执行连接验证。

本轮仅实现与静态审阅，按用户要求未执行编译、单元测试、实机测试或其它测试；未领取 Cargo 槽位。空格、中文、斜杠、危险编码与重定向行为均待统一验证。
