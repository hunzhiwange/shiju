# 任务36：剩余设置页面与安全状态静态交付

状态：完整静态职责交付；本记录替代早期阶段记录。未编译、未测试、未运行实机、未回归；以下计数均为源码统计，不是测试通过数。阶段提交61dbb3877的未完成项已在本次收口。

## 实际生产路径

- 安全与电脑控制页面继续直接借用desktop-settings-security的真实状态、局部消息、视图、更新和配置写入；App仅提供当前端点、会话/项目资源和Toast映射。安全标签像素/交互断言迁shell，与真实general页面比较。
- voice设置仍委派desktop-settings-voice，App只连接转录、回放、热键、音频取消及Onboarding生命周期；保留切换设置路由取消真实转录的宿主契约。
- companion页面与尺寸弹窗由appearance渲染，shell测试注入desktop-task-board的真实SVG/精灵素材；App只拥有原生窗口、持久化缩放与ViewMessage。Scene、截图、实际字体整形、局部布局和英文断言已迁出。
- device_sync的授权库存由宿主按当前host/project连接providers、MCP、prompts、skills与项目组合；行标签、组合去重及含未解析技能引用不可选策略归memory。S3/WebDAV的可编辑性由领域ViewState统一决定。传输、预览、任务面板及配置写入继续调用memory既有入口。
- prompts的所有局部UI→更新事件转换移入skills::prompts::ui_route；宿主仅处理Agents/Projects两条跨页导航和消息信封。没有让shell吸收提示词业务。
- 搜索目标类型、page映射、稳定排序、标签本地化、字段扩展与导航激活判定在shell::search；宿主只组合多个领域的描述符和Qdrant可见性。
- channels的项目名称回退、QR草稿分流归channels::update；技能任务结束/备份恢复的库存失效归skills::update；工作树所有请求代次重置归workspace；归档项目批量删除的结果校验、下一项决策、摘要和reconcile集合归sessions。宿主仅执行跨域缓存、目录选择、请求任务和通知效果。
- providers的模型重试10秒计时归providers::managed_shiju，宿主将代次映射回AccountMessage。账号当前身份核对、跨页刷新和Toast保留在宿主。目录、发现、补全、路由队列均继续调用providers真实入口。

未新增App字段或AppState，未复制App，未引入巨型trait或新生产总控库。没有反向path/include加载宿主，没有新能力开关，没有改变密钥写入、授权作用域、取消/过期结果或持久化策略。

## 全部来源、残留及调用者

[逐文件清单](task-36-files.json)覆盖baseline任务36的全部152个来源，逐项记录：原始SHA核对、splitlines前后值、迁出位置、每个残留函数和App字段组、保留理由。152个来源均与固定baseline SHA一致。保留函数只做正向领域借用/信封转换、真实跨域效果装配、系统资源所有权或最小宿主契约。

必须保留的真实边界包括：账号与模型凭据、语音活动互斥、多个领域的导航目录、WeCom/Cron联动、真实SDK反馈到Toast、route/runtime共享关闭flush、原生目录选择、多会话缓存与MCP面板取消。具体文件和函数均在清单中列出，没有把局部算法以“跨域”名义保留。

必要父模块注册在components/mod.rs、system_settings、skills、themes及各领域lib.rs；Cron原有列表测试的错误领域path已修复，6个局部用例挂到cron::host_module_19_tests::rendering::list_tests，4个完整侧栏/页面/弹窗集成用例挂到宿主ui_refresh_27_tests::list_tests。原本未使用的cron/list、history、templates、delivery宿主包装删除。原生picker的8场景daemon夹具迁cron，仍须显式运行，不自动启动窗口。

共享绘制夹具的归属：editor→desktop-editor::settings_test_support，providers→providers::view_test_support，companion→appearance::companion_test_support，完整Cron外壳→shell::cron_render_test_support。宿主测试只正向导入这些泛型夹具；领域源码无宿主引用。cron和providers的cfg(test) self别名只用于同一份共享夹具在领域/宿主两侧引用同一领域类型。

## 完整断言映射

[逐测试映射](task-36-test-map.json)记录全部89个原测试函数，每个名称仍存在，并逐项记录原断言数和实际新文件断言数；390个测试函数体断言宏逐一数量一致。卡片选择用例的3条窗口所有权断言拆到宿主mini_companion_selection_events_preserve_window，局部事件/布局/绘制仍保留原名。

[夹具断言映射](task-36-helper-assertions.json)另外覆盖13个有断言的旧辅助函数，共19条断言：环境隔离、真实端点、预览实际绘制、状态加载、原生取消、唯一内容滚动、轮询终止、HTTP请求约束和保存后刷新。409个原断言宏全部有明确去向，无删除、弱化或用文件名推断等价。

目录编辑、发现与补全测试使用真实领域状态和SDK，仅替换HTTP服务；宿主仅保留真实HostInput取消轮询、SDK失败Toast及其App资源夹具。Iced异步Feedback必须在断言前实际驱动：保存失败宿主契约和非静默补全夹具显式消费真实反馈事件；保存后的轮询跳过成功提示，继续等待真实HTTP结果。未替换被测算法。

UI原明暗主题、窄屏、多语言、像素/几何、鼠标、遮罩、字体与截图断言源码保留；本轮没有实际绘制，不能据此承诺没有glyph panic。

## 行数与静态检查

固定152来源：7437→4545行，净减少2892行；其中沿用baseline的“生产候选”标记为3443→2831行。此分类只是文件名初筛，不冒充准确生产编译行数。

额外宿主调用者ui_refresh_27_tests增加21行必要测试适配；保守扣除后可归属宿主净减少至少2871行，其它父模块删除的注册行未额外计入收益。没有新建宿主Rust文件，没有将实现挪到宿主tests/examples。新代码留在既有领域库；仓库总代码行数不作为收益。

本轮仅执行源码读取、独立rustfmt（79个本次变更Rust文件语法解析，非类型检查）、正向模块path存在性、旧测试名称/断言静态计数、依赖方向和git diff --check。没有Cargo命令、测试执行或通过数。Cargo.lock仅补充shell测试使用真实task-board素材的既有包依赖边，不改动其它任务内容。

## 以后经用户明确要求才运行

所有命令均为保存的验证入口，本轮没有执行：

- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-shell --lib host_module_36_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-channels --lib host_module_36_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-providers --lib host_module_36_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-security --lib host_module_36_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-workspace --lib host_module_36_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-cron --lib host_module_19_tests::rendering::list_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-network --lib host_module_21_tests::tunnel_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::components::system_settings_cron::ui_refresh_27_tests::list_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::components::editor_settings_fields_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::components::system_settings_search_tests -- --test-threads=1`

其余保留宿主契约按逐测试映射中的名称做单个有效过滤，不能运行零测试并声称通过。原生picker::run、音频/窗口/发布验收继续沿原生显式路径，须另行授权，不随上述静态交付自动运行。
