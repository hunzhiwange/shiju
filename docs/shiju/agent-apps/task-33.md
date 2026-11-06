# 任务33：授权主机的应用读取与诊断API

状态：已完成。2026-11-06，macOS。

## 生产接入与部署

当前桌面端已有[本机免配置初始化](local-setup.md)，无需手动设置下述文件和环境变量；下述管理员显式策略仍用于高级部署，并优先于本机默认策略。

最初核对发现：已有TenantScope只解析客户端头，Basic/skey只认证网关，没有认证主体到租户/主机的可信授权来源。用户随后明确授权补齐管理员登记、默认拒绝的最小生产底座。SHIJU账户/profile接口未提供可消费的执行主机归属与在线分发契约，因此本任务没有把平台登录等同于本机管理权。

新增读取路由挂在实际 `gateway/router.rs::handler_router` 已注册的 `handlers::agent_apps::router` 中，保持现有图标库端点。原 `handlers/agent_apps.rs` 移入同名目录的 `mod.rs`，只增加读取路由合并；原仓库并非在 `gateway/api/mod.rs` 注册客户端路由，遵循真实接线，不添加第二个不被调用的注册入口。

管理员在启动网关前设置绝对路径 `SHIJU_AGENT_APPS_ACCESS_FILE`，指向JSON授权文件。未设置、文件缺失/无效均默认拒绝。该变量只在服务端路由构建时读取；没有HTTP登记、自动认领或请求指定授权文件路径的入口。文件热读支持撤权和轮换；管理员应以原子替换更新。

授权文件形状（占位符必须由管理员替换，不能直接用作有效凭证）：

```json
{
  "principals": [{
    "id": "deployment-reader",
    "token_sha256": "<64位小写SHA256十六进制摘要>",
    "grants": [{"platform_id": "<平台>", "company_id": "<公司>", "host_id": "<本机登记ID>"}]
  }],
  "hosts": [{
    "id": "<本机登记ID>", "online": true, "binding": "local",
    "home": "<管理员批准的绝对home路径>",
    "search_paths": ["<管理员批准的绝对CLI目录>"],
    "applications": "<管理员批准的系统Applications绝对目录>"
  }]
}
```

凭证应使用独立随机高熵值（至少32字符），文件只存SHA256摘要，凭证本身通过 `x-agent-apps-token` 请求头发送；保留网关既有Basic要求，部署者需沿用HTTPS等既有安全传输边界。这里不调用外部服务、不采集个人主机归属或凭据。文件只允许当前OS用户/root拥有，模式0600或更严格，父目录不得允许组/其他用户写入；拒绝符号链接、非普通文件、超64KiB文件、重复主机/主体/摘要与未知字段。非Unix暂不支持该私有文件认证，默认拒绝。

每个principal只通过摘要认证，不允许请求指定主体；随后精确匹配平台、公司、主机grant，再检查主机在线和local绑定。未授权/未知主机统一403，不泄露注册表；已授权离线503；明确remote绑定501，绝不回退本机。不读取客户传入的路径、命令、参数或环境变量。仅有网关密码或任意TenantScope不足以获得读取权限。

`search_paths` 是管理员显式批准的本执行端搜索范围（最多64项），不会自动增加宿主HOME、登录Shell、全局PATH或其他个人安装目录。管理员应按期望安装范围登记相应目录。home仅用于本地主机可用性和Claude Desktop配置/应用存在观察，不代表注册或认领该HOME中的官方账号。CLI符号链接和真实路径是观察数据，不授予后续写入/启动权限。

## API契约与职责

在现有客户端管理API及 `/v1` 等价前缀下：

- `GET /agent-apps/catalog?targetHostId=…`：授权后返回已有14应用 `AppCatalogResponse`，不探测应用文件或进程。
- `GET /agent-apps/installations?targetHostId=…&appId=…`
- `GET /agent-apps/diagnostics?targetHostId=…&appId=…`
- `GET /agent-apps/versions?targetHostId=…&appId=…`

后三个读取视图共享 `AppReadResponse`，包含 `installations` 及可选 `desktop`。CLI复用catalog固定命令名、已有受限 `--version` 子进程探测（2秒超时、输出上限、丢弃stderr、回收进程组）、报告去重与来源推断。返回每个实例路径、真实路径、版本、来源及明确runtimeStatus；缺失CLI返回空列表，非零退出/无效输出/损坏安装等保留实际诊断。服务器不接受任意命令执行输入；未知查询字段直接400。

Claude Desktop复用既有只读presence探测，返回应用包和配置目录观察，不把配置目录当成安装证明、不把目录存在当成可运行或CLI版本。Windows/WSL没有本任务真实本地适配器，明确拒绝，不冒充macOS/Windows/WSL实机证据。

应用探测/聚合归vw-agent-apps，DTO归vw-api-types，网关只认证、授权和调度阻塞工作。不改供应商、账户、生命周期、SDK或UI。后续任务34–36必须继续使用真实授权边界，不能把本任务GET权限直接视为写/启动授权。

## 验证记录

所有Cargo阶段从仓库根动态原子mkdir领取槽位，记录任务/会话/PID并通过退出trap释放。实际全部取得slot-2：

- `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`
- `CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`

已检查 `.cargo/config.toml`：原 `build.build-dir = "target"` 被上述环境变量覆盖，避免中间产物逃离槽位。没有新增构建目录，没有占用其他任务的锁；本阶段所有Cargo及测试子进程结束后自己的slot-2锁已释放，编译缓存保留。

| 阶段与确切命令 | 退出码 | 有效数量/结果 |
| --- | --- | --- |
| 初始红灯：`cargo test -p vw-agent --lib cc_apps_read_api_authorization -- --nocapture` | 101 | 2有效测试均失败：最小路由返回503，分别不满足授权拒绝401和读取成功200；不是缺失底座、编译失败或零测试 |
| 首次实现：同一聚焦命令 | 0 | 2通过、0失败、11448过滤 |
| 补充边界：同一聚焦命令 | 101 | 6通过、1失败；Desktop夹具把系统Applications与用户Applications设为相同目录，返回2条观察而夹具预期1；将夹具系统目录改为独立SystemApplications，未修改原presence模块掩盖结果 |
| 修正夹具后：同一聚焦命令 | 0 | 7通过、0失败、11448过滤；生产接入子进程另运行1项，不重复计入顶层数量 |
| 最终 `cargo fmt --all` | 0 | 无无关格式化差异 |
| 最终 `cargo test -p vw-agent --lib cc_apps_read_api_authorization -- --nocapture` | 0 | 7通过、0失败、11448过滤；包含独立子进程生产路由验证 |
| `cargo test -p vw-agent --lib cc_base_icon_api_read_scoped_sqlite -- --nocapture` | 0 | 1通过、0失败、11454过滤；原图标库SQLite读取回归 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全仓通过，无Clippy诊断 |
| `git diff --check` / `git diff --cached --check` | 0 | 仅任务源码、独立测试和本记录进入提交；总计划/来源快照无改动 |

最终命令均在同一slot-2阶段顺序执行（fmt→聚焦→图标回归→Clippy），阶段退出0；此前各阶段也动态领取并释放slot-2。Agent测试链接器仍有既有 `__eh_frame section too large` 提示，实际测试退出0；未更改链接设置隐藏提示，无新增Clippy警告。

真实临时文件与CLI子进程覆盖：14应用目录、固定--version的2.3.4结果与宿主ID、非零退出7诊断、缺失安装空列表、Claude Desktop观察、跨平台/公司/主机拒绝、未知目标、离线及remote拒绝、缺失/错误/重复令牌、租户头和查询边界、逐请求撤权、默认空配置拒绝、文件缺失/权限过宽/符号链接/超大内容/无效模式，以及重复注册、未知字段、相对根和未登记grant。拒绝场景的CLI执行标记保持不存在；代码路径在授权返回前只读取管理员策略文件，不进入任何应用home/目录/元数据或进程探测。目录接口授权后也不探测应用文件或进程。

生产接入子进程只为该子进程设置管理员配置路径，实际构建已注册 `handler_router`，保留TenantScope与Basic中间件：单有应用令牌仍被Basic拒绝，满足两层认证及精确授权后读取真实CLI版本。没有测试专用“已授权”扩展，没有修改HOME或使用个人安装/账户。文件权限、打开目录句柄和相对openat检查均走生产加载路径，父目录与文件符号链接不受信任。

本次没有UI变更，不需要渲染证据；未运行无过滤cargo test、fmt检查、wasm检查或Makefile。macOS临时夹具是当前API验收证据，不冒充个人应用、真实官方账号或其他平台实机验收。后续推荐任务34接入同一管理员授权来源，并按写操作范围明确授权，不能直接把本任务的读取grant等同于任意生命周期执行权限。
