# Hermes macOS 实机验收（已完成，待回归）

2026-11-06 14:11–14:19，Asia/Shanghai。已读只读plan.md、来源与接口、任务53、批次52及task-16/31/32/34证据。前序待回归不视为符号缺失。本次无Rust修改，未验收Windows/WSL/Linux。

## 环境与来源

macOS 27.0 (26A428)、arm64、Node v26.13.0、npm 11.20.0。开始command -v hermes为空；没有覆盖或卸载个人安装、读取个人凭据。0700随机隔离根内独立HOME/config、prefix、Applications、uv缓存与Python安装目录。0600策略只授权p/a/host本机生命周期，随机测试令牌仅临时保存，Gateway仅127.0.0.1:19453。

生产配方native_hermes_agy.rs指定官方来源：
https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.sh

本次下载脚本SHA256：abf338605dae6dd28d101673ac6aed7995c955745db46d47676bc591a7957209。脚本支持--non-interactive、--skip-browser、--skip-computer-use、--dir、--commit及HERMES_HOME；没有编造下载地址。默认stable解析v0.23.0，检出2bd378ed1b8d6bf2192cb26e0a55ff3091e6bd19。运行：

```text
HOME=<isolated-home> HERMES_HOME=<isolated-home>/.hermes XDG_CONFIG_HOME=<isolated-config> UV_CACHE_DIR=<isolated-cache>/uv UV_PYTHON_INSTALL_DIR=<isolated-home>/.local/share/uv/python CI=1 bash <downloaded-official-install.sh> --non-interactive --skip-browser --skip-computer-use
```

退出0，官方日志见hermes-official-install.txt。隔离下载uv 0.13.5、Python 3.14.9与锁定依赖，并构建官方TUI/web；日志Install complete。实际hermes --version退出0：Hermes Agent v0.23.0 (2026.9.24)、stable、local 2bd378ed1b8d、install method git、Python 3.14.9、OpenAI SDK 2.54.0，详见hermes-runtime.json。

## 实际证据与缺口

请求、脱敏响应见hermes-api.json；生产GatewayClient输出见hermes-sdk.txt。

- 初始catalog 200，Hermes/native_cli；installations=[]、launch-identities=[]均200。正式安装preview返回422空响应，未创建操作。
- 官方隔离安装后，将管理员策略read.search_paths和write.prefix明确指向隔离home/.local，npm/node链接既有工具，仅使前缀满足现有校验。API发现真实入口，最终version=0.23.0、runtimeStatus=runnable、source=unknown。初次安装尚未完成时两秒版本探测曾超时，直接命令20秒预算成功但伴随上游stable源SSL超时提示；不把初次probe_failed当安装失败。
- 实际update/uninstall preview均422空响应。local.rs::preview的Install仅调用npm::install_args；scoped_discover仅识别npm manifest，selected拒绝unknown或超出prefix目标。Hermes原生配方存在，但尚未被此本地写入适配器接通。这是当前实现缺口，不能将其宣称为官方不支持Hermes生命周期。未扩展修改共享执行器。
- 将本任务创建的入口临时移到备份、替换为指向隔离不存在路径的断链，diagnostics 200、probe_failed/damaged_installation；finally恢复原入口。未破坏个人安装。
- SDK由已有rlib编译临时驱动，3次运行退出0，分别读取空基线、官方安装后及清理后，目录与API一致。前两次链接了不匹配Tokio缓存，退出101/reactor错误；根据gateway fingerprint选取tokio-f993a923b732abac后恢复，不将失败算通过。命令见hermes-sdk-command.txt。
- 清理本次创建的Hermes安装、配置与缓存后，API与SDK均空列表。这里是隔离测试目录清理，不能代替正式API卸载、指定实例与剩余实例保持验收。

默认features cargo run已进入Running。独立Task53.app从同一产物复制，CUA实际见“识矩 - 项目”，AX只暴露窗口/菜单；设置图标按两种缩放坐标点击及super+comma均未进入设置。首次系统启动bundle使用个人配置目录，未操作账号或凭据；截图含个人账户信息，已删除且不提交。随后直接以隔离HOME/config启动bundle，进程自行退出且日志为空。未取得Hermes卡片、连接测试Gateway、进度/取消、断线重读toast及SDK/Iced一致性截图，不能以项目入口代替这些证据。

launch-identities两次为空，缺专用授权供应商/官方身份；真实默认终端程序/cwd/环境、供应商/适用官方账号和临时材料清理未验证。安装/更新/卸载422导致操作进度、取消及双实例保持未验证。未做有可观察版本变化的升级；最新版本不能替代升级证据。以上保留待回归，不宣称完整通过。

## Cargo与清理

原子mkdir动态领取slot-3，owner记录task53/会话/控制PID76199。CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为/Users/xiangmin/worker/code/shiju/target/rust-slot-3；后者覆盖.cargo/config.toml的build.build-dir=target，未绕过槽位。

| 确切命令 | 退出码 | 有效自动测试 |
| --- | --- | --- |
| cargo build -p vw-cli --bin shiju | 0 | 0，构建 |
| cargo build -p vw-desktop --bin shi-ju | 0 | 0，构建 |
| cargo run -p vw-desktop --bin shi-ju | -15，Running后主动结束 | 0，人工场景 |

临时SDK rustc编译退出0；本项为实机场景驱动，有效自动测试0，不把构建或零测试当行为通过。无Rust变更，按任务无需Clippy；既有ld __eh_frame警告未改。未运行全量test、fmt检查、wasm或Makefile。已用RustCodeGraph status/explore核对地图，源码/未索引变更直接读取。

控制器记录构建/桌面/Gateway退出[0,0,-15,-15]；独立bundle不再运行。全部槽位产物进程退出后，自有slot-3锁已释放，缓存保留。临时安装、策略、令牌与测试根清理；plan.md保持只读。任务文件保留供缺口修复后回归。
