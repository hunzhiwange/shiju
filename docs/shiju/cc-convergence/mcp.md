# MCP 配置与第三方同步

桌面端在设置的「MCP 服务器」中管理识矩全局配置。新增、编辑和列表右键同步使用同一套应用选择。全局配置通过网关保存，模板和可用应用来自 `GET /v1/global/mcp/options`。

`POST /v1/global/mcp/sync` 按名称读取已保存的全局配置。网关验证租户、目标主机和原生文件写入权限，再通过各应用的适配器更新所选应用，返回逐项结果。应用已有未受管的同名配置时返回冲突。

项目 ACP 通过 `POST /v1/project-mcp/definitions` 创建项目定义，通过 `PUT /v1/project-mcp/definitions/{id}` 更新定义或审核 stdio 配置。运行时检查与会话工具调用仍使用各自的入口。
