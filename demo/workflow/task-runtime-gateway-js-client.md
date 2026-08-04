# JS 客户端任务运行时网关示例

本文展示下一版 JS 桌面客户端接入任务运行时的最小调用序列。示例只表达契约，不包含 UI 框架代码。



function gatewayHeaders(options: GatewayOptions): HeadersInit {


async function gatewayJson<T>(
  options: GatewayOptions,
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const response = await fetch(`${options.baseUrl}${path}`, {
    ...init,
    headers: {
      ...gatewayHeaders(options),
      ...(init.headers ?? {}),
    },
  });
  if (!response.ok) {
    throw new Error(`${init.method ?? "GET"} ${path} failed: ${response.status}`);
  }
  return response.json() as Promise<T>;
}
```

## 首屏加载

```ts
async function loadTaskBoard(options: GatewayOptions, projectPath: string) {
  const project = await gatewayJson<{ project: { id: string } }>(
    options,
    "/v1/projects/resolve",
    {
      method: "POST",
      body: JSON.stringify({ directory: projectPath, create_if_missing: false }),
    },
  );
  const projectId = encodeURIComponent(project.project.id);

  const [workflows, workflow, staff, board, worktrees] = await Promise.all([
    gatewayJson(options, "/v1/task-lifecycle/workflows"),
    gatewayJson(options, `/v1/agent-scheduler/projects/${projectId}/workflow`),
    gatewayJson(options, `/v1/staff/projects/${projectId}/projection`),
    gatewayJson(options, `/v1/agent-scheduler/projects/${projectId}/tasks`),
    gatewayJson(options, "/v1/task-pool/worktrees/snapshot", {
      method: "POST",
      body: JSON.stringify({ project_id: project.project.id }),
    }),
  ]);

  return { projectId: project.project.id, workflows, workflow, staff, board, worktrees };
}
```

## SSE 事件

```ts
function subscribeTaskBoardEvents(
  options: GatewayOptions,
  projectId: string,
  afterCursor: number | undefined,
  onEvent: (event: unknown) => void,
) {
  const cursor = afterCursor == null ? "" : `?after_cursor=${afterCursor}`;
  const encodedProjectId = encodeURIComponent(projectId);
  const source = new EventSource(
    `${options.baseUrl}/v1/agent-scheduler/projects/${encodedProjectId}/events/stream${cursor}`,
  );
  source.onmessage = (message) => onEvent(JSON.parse(message.data));
  source.onerror = () => {
    source.close();
  };
  return () => source.close();
}
```

断线后先调用 `GET /v1/agent-scheduler/projects/{project_id}/events?after_cursor={cursor}` 补齐，
再重新建立 SSE。事件只驱动刷新，最终卡片仍以 `/tasks` 快照为准。

## Command

```ts
async function moveTask(
  options: GatewayOptions,
  projectId: string,
  taskId: string,
  toStatusKey: string,
  orderNo: number,
) {
  const encodedProjectId = encodeURIComponent(projectId);
  return gatewayJson(options, `/v1/agent-scheduler/projects/${encodedProjectId}/commands`, {
    method: "POST",
    body: JSON.stringify({
      task_id: taskId,
      command: "move",
      dispatch_mode: "local",
      reason: "手动移动任务",
      payload: {
        move_task: {
          to_status_key: toStatusKey,
          order_no: orderNo,
        },
      },
    }),
  });
}

async function runStageAction(
  options: GatewayOptions,
  projectId: string,
  taskId: string,
  stageId: string,
  capabilityId: string,
  workflowUuid?: string,
) {
  const encodedProjectId = encodeURIComponent(projectId);
  return gatewayJson(options, `/v1/agent-scheduler/projects/${encodedProjectId}/commands`, {
    method: "POST",
    body: JSON.stringify({
      task_id: taskId,
      command: "run_stage_action",
      reason: "运行阶段动作",
      payload: {
        stage_action: {
          stage_id: stageId,
          capability_id: capabilityId,
          workflow_uuid: workflowUuid,
        },
      },
    }),
  });
}
```

## 日志与产物



hello wrold


```ts
async function loadTaskDetails(options: GatewayOptions, projectId: string, taskId: string) {
  const encodedProjectId = encodeURIComponent(projectId);
  const encodedTaskId = encodeURIComponent(taskId);
  const [logs, artifacts] = await Promise.all([
    gatewayJson(
      options,
      `/v1/agent-scheduler/projects/${encodedProjectId}/tasks/${encodedTaskId}/logs`,
    ),
    gatewayJson(
      options,
      `/v1/agent-scheduler/projects/${encodedProjectId}/tasks/${encodedTaskId}/artifacts`,
    ),
  ]);
  return { logs, artifacts };
}
```

日志和产物只展示 gateway 返回的数据；不要根据日志文本或本地文件扫描推导任务状态。