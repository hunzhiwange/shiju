# 任务45：远程隔离闭环回归

状态：已完成。2026-11-06，macOS。

已阅读 do-task-plans、仓库及 RustCodeGraph 约定、只读 plan.md、来源与接口.md、任务44证据与真实 SDK/API/执行主机实现。RustCodeGraph status 确认索引存在；新增接口使用源码核对。批次44的实际交付（7f76dc816）提供了所需启动底座，编号文件已删除；母计划旧状态标签不作为缺失符号证据。本任务不修改 plan.md、来源快照或远程业务协议，不和其他计划批次并行。

## 范围和断言

新增独立 remote_tests_tests.rs，注册一个 cc_apps_remote_host_isolation 前缀行为测试。仅将已有测试终端注入工厂扩大至父模块可见以复用正式 launch 路由；没有新增 remote.rs、RPC、依赖或配置。

两个随机 loopback HTTP 监听器使用两个独立的管理员策略、0700临时主机根、SQLite和生命周期存储，刻意使用相同 host ID。SDK通过真实HTTP调用两个网关，先后在各自主机完成安装、发现、启动和卸载，断言被选主机真实安装文件和启动子进程标记、另一主机没有安装或启动副作用。包管理器是隔离脚本，记录argv、cwd、HOME和npm前缀；终端OS边界替代为执行已验证 LaunchMaterial 的真实标记子进程，没有真实个人安装或外网包请求。

同一网关两个租户均有明确的主机操作授权，因此跨租户取消404来自操作所有权检查，不是缺少授权造成的假隔离。另一个网关也不能取消该操作。拒绝后原操作仍运行，所有者取消返回Cancelled后，/bin/kill -0确认原包管理器PID已经消失，延迟逃逸标记不存在。安装子进程非零退出产生Failed且无安装产物。跨网关安装ID启动和越出执行主机的客户端cwd均被拒绝，客户端临时根最终保持空目录。

测试仅覆盖网关分别在自己授权本地主机执行的远程客户端闭环，不宣称支持管理员 binding=remote 的任意远程执行，也不宣称Windows/WSL/Linux实机或厂商CLI实机验收。无需UI证据。

## 红灯调整

前序已交付完整生产行为，因此首轮新增回归直接通过（1通过，退出0），没有声称当前生产隔离存在缺陷。为证明断言能捕获假成功，随后仅在新测试夹具中将终端启动边界临时改为空返回：同一聚焦命令退出101，0通过/1失败，实际失败点为成功HTTP返回后缺少 launched 子进程文件。这个红灯是测试边界故障注入，不是缺失生产契约、工具/编译失败，也不是生产权限降级。恢复真实启动后补入安装失败和取消PID回收断言，再执行最终验证。

## Cargo共享槽位

从仓库根依次原子mkdir领取，实际槽位2：

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

第二项覆盖仓库 build.build-dir=target，所有Cargo及子进程继承，未使用--target-dir。锁记录计划/任务/会话与阶段持有PID，最终阶段设置EXIT/INT/TERM清理，只删除自己的owner和锁，保留缓存。格式化后审查差异，其他会话的全局设置/桌面改动和此前编号任务删除均不纳入本次提交。

| 确切命令 | 退出码 | 有效结果 |
| --- | --- | --- |
| cargo fmt --all | 0 | 每次Rust修改后执行，未保留无关格式差异 |
| cargo test -p vw-agent --lib cc_apps_remote_host_isolation -- --nocapture（初次） | 0 | 1通过，11468过滤 |
| cargo test -p vw-agent --lib cc_apps_remote_host_isolation -- --nocapture（测试边界空返回红灯） | 101 | 1失败，11468过滤，缺少进程标记 |
| cargo test -p vw-agent --lib cc_apps_remote_host_isolation -- --nocapture（最终） | 0 | 1通过，0失败，11468过滤 |
| cargo clippy --all-targets --all-features -- -D warnings | 0 | 全仓通过，无新增警告 |
| git diff --check / git diff --cached --check | 0 | 审查仅本任务4文件 |

最终测试0.25秒，全仓Clippy23.60秒。既有macOS链接器__eh_frame大小提示单列，不修改全仓链接设置。未运行无过滤cargo test、fmt检查、wasm检查或Makefile。最终阶段已结束、本人slot-2锁已释放、缓存保留。下一步逐应用macOS实机批次；本任务不宣告母任务整体完成。
