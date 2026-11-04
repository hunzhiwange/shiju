//! Restricted parallel iteration. Each item has its own pool; only leaves hold permits.
use super::*;

pub(super) fn parallel_limit(node: &WorkflowNode) -> Result<Option<usize>, String> {
    let parallel = node.data.get("parallel").and_then(Value::as_bool).unwrap_or(false)
        || string_field(&node.data, "mode").is_some_and(|s| s.eq_ignore_ascii_case("parallel"));
    if !parallel {
        return Ok(None);
    }
    let limit =
        crate::workflow::budget::VideoEditRunBudget::from_value(node.data.get("max_parallel"))?
            .limit();
    if iteration_error_strategy(node) != "terminate" {
        return Err("parallel iteration requires terminate error strategy".into());
    }
    Ok(Some(limit))
}
fn validate_graph(graph: &WorkflowGraph) -> Result<(), String> {
    for node in graph.nodes.values() {
        if !matches!(
            node.node_type.as_str(),
            "start"
                | "llm"
                | "template"
                | "template-transform"
                | "output"
                | "end"
                | "variable-aggregator"
                | "list-operator"
        ) {
            return Err(format!(
                "parallel iteration rejects node {} ({})",
                node.id, node.node_type
            ));
        }
    }
    Ok(())
}

pub(super) async fn execute<F>(
    runtime: &WorkflowRuntime,
    node: &WorkflowNode,
    pool: &VariablePool,
    on_event: &mut F,
) -> Result<Vec<Value>, String>
where
    F: FnMut(WorkflowRunEvent) + Send,
{
    let limit = parallel_limit(node)?
        .ok_or("parallel mode required")?
        .min(runtime.video_edit_budget.limit());
    let graph = iteration_subgraph(node)?;
    validate_graph(&graph)?;
    let selector =
        list_operator_input_selector(&node.data).ok_or("iteration 节点缺少 input_selector")?;
    let Some(Value::Array(items)) = pool.get_selector(&selector) else {
        return Err("iteration 节点输入必须是数组".into());
    };
    let output_selector = node
        .data
        .get("output_selector")
        .map(selector_from_value)
        .filter(|s| !s.is_empty())
        .ok_or("iteration 节点缺少 output_selector")?;
    let run_id = pool
        .get_selector(&["sys".into(), "workflow_run_id".into()])
        .and_then(Value::as_str)
        .unwrap_or("");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut pending = items.iter().cloned().enumerate();
    let mut tasks = JoinSet::new();
    let mut results = vec![Value::Null; items.len()];
    loop {
        while tasks.len() < limit && !runtime.video_edit_budget.is_cancelled() {
            let Some((index, item)) = pending.next() else {
                break;
            };
            let chunk_id = item
                .get("chunk_id")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .unwrap_or_else(|| format!("chunk-{index:06}"));
            let mut local = pool.clone();
            local.insert_node_output(&node.id, "item", item);
            local.insert_node_output(&node.id, "index", Value::from(index));
            let runtime = runtime.clone();
            let graph = graph.clone();
            let selector = output_selector.clone();
            let tx = tx.clone();
            let parent = node.id.clone();
            let run_id = run_id.to_owned();
            tasks.spawn(async move {
                let scope = WorkflowChildContext {
                    run_id,
                    parent_node_id: parent,
                    chunk_index: index,
                    chunk_id,
                    node_id: String::new(),
                    call_id: String::new(),
                };
                let result = execute_item(&runtime, &graph, &mut local, &tx, &scope).await;
                (
                    index,
                    result.map(|()| local.get_selector(&selector).cloned().unwrap_or(Value::Null)),
                )
            });
        }
        if runtime.video_edit_budget.is_cancelled() {
            stop(&mut tasks, &mut rx, on_event).await;
            return Err("video edit run cancelled".into());
        }
        if tasks.is_empty() {
            break;
        }
        tokio::select! {
            biased;
            _ = runtime.video_edit_budget.cancelled() => { stop(&mut tasks, &mut rx, on_event).await; return Err("video edit run cancelled".into()); }
            event = rx.recv() => { if let Some(event) = event { on_event(event); } }
            completed = tasks.join_next() => {
                match completed {
                    Some(Ok((index, Ok(value)))) => results[index] = value,
                    Some(Ok((_, Err(error)))) => { stop(&mut tasks, &mut rx, on_event).await; return Err(error); }
                    Some(Err(error)) => { stop(&mut tasks, &mut rx, on_event).await; return Err(format!("parallel iteration task failed: {error}")); }
                    None => break,
                }
            }
        }
    }
    drop(tx);
    while let Some(event) = rx.recv().await {
        on_event(event);
    }
    Ok(results)
}
async fn stop<F>(
    tasks: &mut JoinSet<(usize, Result<Value, String>)>,
    events: &mut mpsc::UnboundedReceiver<WorkflowRunEvent>,
    on_event: &mut F,
) where
    F: FnMut(WorkflowRunEvent) + Send,
{
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
    while let Ok(event) = events.try_recv() {
        on_event(event);
    }
}

async fn execute_item(
    runtime: &WorkflowRuntime,
    graph: &WorkflowGraph,
    pool: &mut VariablePool,
    events: &mpsc::UnboundedSender<WorkflowRunEvent>,
    scope: &WorkflowChildContext,
) -> Result<(), String> {
    let mut active = graph.start_node_ids.iter().cloned().collect::<BTreeSet<_>>();
    let mut executed = BTreeSet::new();
    let mut edges = BTreeSet::new();
    let handles = BTreeMap::new();
    while !active.is_subset(&executed) {
        let ready = next_ready_nodes(graph, &active, &executed, &edges, &handles);
        if ready.is_empty() {
            return Err("parallel iteration subgraph blocked".into());
        }
        let batch = workflow_execution_batch(graph, ready, runtime.video_edit_budget.limit());
        // Futures stay owned by this item. Dropping an item cancels all its branches.
        let mut futures = futures_util::stream::FuturesUnordered::new();
        for id in batch {
            let node = graph.nodes.get(&id).ok_or("parallel iteration node missing")?.clone();
            let mut local = pool.clone();
            let context = WorkflowChildContext {
                node_id: id,
                call_id: serde_json::to_string(&(
                    &scope.run_id,
                    &scope.parent_node_id,
                    scope.chunk_index,
                    &node.id,
                ))
                .expect("string tuple"),
                ..scope.clone()
            };
            let index = scope.chunk_index as u32;
            let run_id = scope.run_id.as_str();
            let events = events.clone();
            futures.push(async move {
                let mut emit = |event| {
                    let _ = events.send(WorkflowRunEvent::Child {
                        context: context.clone(),
                        event: Box::new(event),
                    });
                };
                emit_workflow_node_started(run_id, &node, index, &mut emit);
                let execution =
                    Box::pin(execute_node(runtime, graph, &node, &mut local, index, &mut emit))
                        .await;
                let dto = WorkflowNodeRunDto {
                    node_id: node.id.clone(),
                    node_type: node.node_type.clone(),
                    title: node.title.clone(),
                    status: execution.status.clone(),
                    inputs: redact_map(&execution.inputs),
                    outputs: redact_map(&execution.outputs),
                    selected_handle: execution.selected_handle.clone(),
                    error: execution.error.clone(),
                    elapsed_ms: execution.elapsed_ms,
                };
                emit(WorkflowRunEvent::NodeFinished(WorkflowNodeFinishedEvent {
                    node: dto,
                    index,
                }));
                (node, execution)
            });
        }
        while let Some((node, execution)) = futures.next().await {
            if execution.status == WorkflowNodeRunStatus::Failed {
                return Err(execution
                    .error
                    .unwrap_or_else(|| "parallel iteration child failed".into()));
            }
            for (key, value) in execution.outputs {
                pool.insert_node_output(&node.id, &key, value);
            }
            executed.insert(node.id.clone());
            activate_outgoing_edges(graph, &node, &handles, &mut edges, &mut active);
        }
    }
    Ok(())
}
