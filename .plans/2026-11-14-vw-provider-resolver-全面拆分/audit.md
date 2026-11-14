# 提供商解析源码与依赖审计

本卡仅完成子计划准备，未迁移业务代码。2026-11-15 静态读取；RustCodeGraph status 有索引，explore 泛化命中无关符号，缺失区域以 rg/源码补充。未运行 Cargo、测试或实机。

## 清单与已有库复用

包仅一个 src/lib.rs 入口，无 build.rs、features、显式 target、生成源码或额外测试目录。清单依赖：anyhow、once_cell、serde derive、serde_json、tokio rt/sync/time/macros、tracing、strsim、directories、vw-shared、vw-config-types。非 wasm reqwest 0.13 default-features=false/json/rustls、tokio rt-multi-thread/fs、tempfile；wasm reqwest json、vw-gateway-client、web-sys Window/Location。迁移时按实际调用裁剪，不能把非 wasm依赖升为通用。

真实契约在 vw-shared/provider/{models,protocols,reasoning,types,state}.rs、vw-shared/auth/{mod,store}.rs、vw-config-types/{paths,config,gateway}.rs。models.rs 和 provider.rs 目前仅再导出纯数据与转换；不重复新建类型库。前序 shared 11—17 和 config 1/29 的库均为计划依赖，详见 plan.md，不假装现成可用。

已读取 crates/libs/app-config-paths/src/lib.rs、project-storage-paths/src/lib.rs、app-provider-validation/src/lib.rs、memory-response-cache/src/lib.rs：分别是应用配置路径、项目运行路径、应用供应商配置校验、记忆响应缓存，与本包全局缓存版本/模型目录/进程 OnceCell 不同，不复用这些不相干语义。当前 crates/libs 没有 model-*、auth-*、provider-* 同义职责库。

## 模块图与顺序依据

箭头 A → B 表示 A 使用 B：

```text
error → serde/serde_json
protocol_routes → shared.protocols + reqwest::Url
installation → flag
flag → once_cell/env
cache → once_cell/tokio.sync
global → config.paths/directories/once_cell
auth → global + shared.auth.store
config → config.Config；wasm → gateway-client/web-sys
models → flag/global/protocol_catalog/native(models_cache)
protocol_catalog → shared.models/shared.protocols + protocols.json
models_cache → models(解析/覆盖/快照) + installation + reqwest/tokio/tempfile
provider → cache/config/models/auth/shared.state/shared.types/remote_models
remote_models → provider(类型/转换再导出) + models(目录/get/精确元数据)
```

当前 models/native 双向为同领域父子实现关系，拆成基线叶子与快照缓存后消除。provider/remote_models 的环来自再导出，发现库直接依赖共享契约及模型库即可去环，无需注册器或新增 trait。

排序按拓扑深度及外依赖：错误（serde 两项）先行，路由是契约叶子但携带 reqwest URL；目录、缓存、身份无原包依赖，随后基线（共享契约/资源）、快照（网络/磁盘）、配置/认证，最后发现与组装。独立叶子因清单写冲突仍串行。不是以行数决定迁移顺序。

共享可变状态：PATHS、cache::STATE、flag 的三个 Lazy、protocol_catalog::CATALOG、models::CACHE/ModelsCache 的 gate/state/revision、wasm WASM_GATEWAY_ENDPOINT、provider CONFIG_FINGERPRINT。各只迁移一次；门面再导出不得再创建副本。provider 的 filtered/settings key 保持原值。

## 候选库及外部消费者

| 卡 | 目标库 | 直接消费者及依赖方向 |
|---|---|---|
| 1 | provider-errors | agent/provider/mod.rs；只依赖 serde/serde_json |
| 2 | provider-protocol-routes | agent/model_relay、LLM drivers/convert、model_check；只向共享协议和 URL |
| 3 | provider-runtime-paths | 认证/快照库、agent 模型验收测试；向配置目录契约 |
| 4 | provider-state-cache | 解析组装；向 tokio sync，不向业务类型 |
| 5 | provider-request-identity | 模型缓存；flag 与 installation 同职责链 |
| 6 | provider-model-catalog | 快照/发现、agent-apps 原始模型类型、资源测试；向共享模型/协议 |
| 7 | provider-model-cache | agent、agent-apps、desktop 元数据/刷新；向 3/5/6 |
| 8 | provider-config-reader | 组装、desktop wasm 初始化；向配置契约和 wasm 网关 |
| 9 | provider-auth-reader | 组装；向 3 和共享认证存储 |
| 10 | provider-model-discovery | agent、agent-apps、desktop 发现/余额凭据筛选；向 6/7 与共享类型/转换，绝不向 11 |
| 11 | provider-resolution | desktop provider 门面；向 4/7/8/9/10 与共享契约 |
| 12 | vw-provider-resolver 保留 | 发布模块路径及跨库矩阵测试；仅依赖职责库，不留运行时实现 |

每个候选的确切符号、外部依赖、测试归属、目标路径与后续验收在对应卡列出；全引用快照见 consumer-refs.md。大文件 models.rs 按解析/运行时分卡，provider.rs 按 query/state/native/gateway/overrides 分文件，remote_models.rs 按 HTTP/合并分文件，同领域一张聚焦迁移卡。没有将整个大包作为单卡搬迁。

## 逐源文件归属

| 当前 src/ 文件 | 卡与归属 |
|---|---|
| auth.rs | 9：provider-auth-reader |
| auth_tests.rs | 9：独立认证测试 |
| cache.rs | 4：provider-state-cache |
| cache_tests.rs | 4：独立缓存测试 |
| config.rs | 8：provider-config-reader |
| config_tests.rs | 8：独立配置测试 |
| error.rs | 1：provider-errors |
| error_tests.rs | 1：独立错误测试 |
| flag.rs | 5：provider-request-identity/flag |
| flag_tests.rs | 5：独立 flag 测试 |
| global.rs | 3：provider-runtime-paths |
| global_tests.rs | 3：独立目录测试 |
| installation.rs | 5：provider-request-identity/installation |
| installation_tests.rs | 5：独立身份测试 |
| lib.rs | 12：薄模块再导出与矩阵测试挂载 |
| models.rs | 6：解析/资源合并/精确元数据；7：刷新类型、快照与运行时缓存；共享类型直接复用 |
| models_cache.rs | 7：provider-model-cache/native |
| models_cache_tests.rs | 7：独立原生缓存测试 |
| models_tests.rs | 6：纯目录/资源/协议断言；7：url/cache_path/init；辅助 fixture 按真实使用迁移 |
| protocol_catalog.rs | 6：provider-model-catalog 协议补充 |
| protocol_matrix_tests.rs | 12：保留跨库证据/路由集成测试，不压进路由叶子 |
| protocol_routes.rs | 2：provider-protocol-routes |
| protocol_routes_tests.rs | 2：独立路由测试 |
| provider.rs | 11：provider-resolution query/state/native/gateway/overrides |
| provider_tests.rs | 11：按查询/覆盖/指纹/发现分独立文件，fixture 单一维护 |
| remote_models.rs | 10：provider-model-discovery HTTP 与合并模块 |
| remote_models_tests.rs | 10：按 HTTP/合并分独立文件，保留所有断言 |

## 资源和 cfg

- assets/model/api.json、shiju.json、protocols.json 由卡 6 迁至新库同层 assets/model/，保持 include_str! 字节内容。shiju_official_sources.json、shiju-sources.md 作为来源证据同迁，不遗留无归属资源。
- models_tests.rs 的官方来源与协议 fixture、provider_tests.rs 的 shiju fixture、agent aisdk/reasoning_tests.rs 与 desktop reasoning_selector_tests.rs 的跨包 fixture 路径同步调整。protocol_matrix_tests.rs 依赖 docs/dev/model-protocols 下库存及六组 research，保留仓库文档，只调整相对路径。
- 原生 models_cache、remote_models 及其测试挂载保持 not(wasm32)；wasm 配置网关和模型 Mutex 缓存不被删掉。原生 FS/tempfile、tokio fs/rt-multi-thread 继续 target 条件。
- 保留门面有现有公开模块与发布契约理由；只留再导出和跨库测试，内部消费者不能继续经门面获取所有依赖。未拆业务：无；暂留仅发布边界和矩阵集成测试。

## 完成证据与未验证项

全部 27 个 Rust 源文件、五个资源及唯一清单已归属；12 张实施卡逐一映射 prompt，全部串行且每卡有明确前置批次。静态审计可确定迁移边界，不能证明编译、行为、安全性能或实机通过。未来统一验收覆盖各卡行为与消费者、原生/wasm 条件分支；本次均未验证。
