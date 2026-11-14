# WebDAV 上传提交合同

macOS 网关入口 `device_sync::webdav_upload::upload_webdav_snapshot` 接收真实数据库、授权快照选择、技能根目录及当前连接；调用者必须在 `run_with_sync_lock` 内以刚读取的凭据执行，并保证 preview/baseline 与目的地、profile 和兼容版本一致。入口不自行持久化基线；只有完整成功返回的基线才能保存。

上传前重新导出并比对冲突决策，准备原生版本目录后再次读取远端；依次 PUT config.json、skills.zip、manifest.json。任一失败立即返回，禁止保存成功基线。内容相同仅校验远端，不 PUT。发布后读回 manifest、按摘要和大小下载校验两制品，再复核 manifest；制品读回失败或摘要不符显式返回 SyncConflict。

固定 key 采用 best-effort 一致性，无跨设备原子事务或远端互斥承诺。部分失败可能留下新制品与旧 manifest，调用者必须保留原基线并显式处理冲突。此接口供后续任务接入调度/API，本任务未扩展其职责。

本轮按用户要求仅实现并静态审阅，未运行 Cargo、测试或实机验证；未领取槽位，CARGO_TARGET_DIR 未设置。编译与行为验证留待统一执行。
