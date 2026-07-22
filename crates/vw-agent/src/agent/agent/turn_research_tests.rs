use super::*;
use crate::app::agent::config::{ResearchPhaseConfig, ResearchTrigger};

fn research_config(max_iterations: usize, show_progress: bool) -> ResearchPhaseConfig {
    ResearchPhaseConfig {
        enabled: true,
        trigger: ResearchTrigger::Always,
        max_iterations,
        show_progress,
        ..ResearchPhaseConfig::default()
    }
}

#[tokio::test]
async fn turn_injects_research_context_after_memory_context() {
    let provider = ProviderHandle::with_responses(vec![
        Ok(tool_response()),
        Ok(text_response("[RESEARCH COMPLETE]\n- tool-out found")),
        Ok(text_response("answered")),
    ]);
    let mut agent = default_agent(
        provider.native_provider(),
        Box::new(StaticMemoryLoader { result: Ok("[Memory context]\n".to_string()) }),
    );
    agent.research_config = research_config(2, true);

    let response = agent.turn("find facts").await.expect("turn should use research context");

    assert_eq!(response, "answered");
    let requests = provider.seen_messages();
    assert_eq!(requests.len(), 3);
    let user = final_user_message(&requests[2]);
    assert!(user.content.contains("[Memory context]"));
    assert!(user.content.contains("[RESEARCH COMPLETE]\n- tool-out found"));
    assert!(user.content.contains("] find facts"));
}

#[tokio::test]
async fn turn_injects_research_context_without_memory_context() {
    let provider = ProviderHandle::with_responses(vec![
        Ok(text_response("[RESEARCH COMPLETE]\n- standalone fact")),
        Ok(text_response("answered")),
    ]);
    let mut agent = default_agent(
        provider.provider(),
        Box::new(StaticMemoryLoader { result: Ok(String::new()) }),
    );
    agent.research_config = research_config(1, false);

    let response = agent.turn("research it").await.expect("turn should use research-only context");

    assert_eq!(response, "answered");
    let requests = provider.seen_messages();
    let user = final_user_message(&requests[1]);
    assert!(user.content.starts_with("[RESEARCH COMPLETE]\n- standalone fact"));
    assert!(user.content.contains("] research it"));
}

#[tokio::test]
async fn turn_skips_empty_research_context() {
    let provider =
        ProviderHandle::with_responses(vec![Ok(text_response("")), Ok(text_response("answered"))]);
    let mut agent = default_agent(
        provider.provider(),
        Box::new(StaticMemoryLoader { result: Ok(String::new()) }),
    );
    agent.research_config = research_config(1, false);

    let response = agent.turn("empty research").await.expect("turn should ignore empty research");

    assert_eq!(response, "answered");
    let requests = provider.seen_messages();
    let user = final_user_message(&requests[1]);
    assert!(!user.content.contains("[RESEARCH COMPLETE]"));
    assert!(user.content.contains("] empty research"));
}

#[tokio::test]
async fn turn_continues_when_research_phase_fails() {
    let provider = ProviderHandle::with_responses(vec![
        Err("research failed".to_string()),
        Ok(text_response("fallback")),
    ]);
    let mut agent = default_agent(
        provider.provider(),
        Box::new(StaticMemoryLoader { result: Ok(String::new()) }),
    );
    agent.research_config = research_config(1, false);

    let response = agent.turn("keep going").await.expect("research failure should not fail turn");

    assert_eq!(response, "fallback");
    assert_eq!(provider.seen_messages().len(), 2);
}

#[tokio::test]
async fn turn_with_stream_injects_research_context_after_memory_context() {
    let provider = ProviderHandle::with_responses(vec![
        Ok(tool_response()),
        Ok(text_response("[RESEARCH COMPLETE]\n- stream tool fact")),
        Ok(text_response("stream answer")),
    ]);
    let mut agent = default_agent(
        provider.native_provider(),
        Box::new(StaticMemoryLoader { result: Ok("[Memory context]\n".to_string()) }),
    );
    agent.research_config = research_config(2, true);
    let (tx, rx) = tokio::sync::mpsc::channel(16);

    let response = agent
        .turn_with_stream("stream research", tx)
        .await
        .expect("streaming turn should use research context");

    assert_eq!(response, "stream answer");
    let requests = provider.seen_messages();
    assert_eq!(requests.len(), 3);
    let user = final_user_message(&requests[2]);
    assert!(user.content.contains("[Memory context]"));
    assert!(user.content.contains("[RESEARCH COMPLETE]\n- stream tool fact"));
    assert!(user.content.contains("] stream research"));
    assert!(!collect_chunks(rx).await.is_empty());
}

#[tokio::test]
async fn turn_with_stream_injects_research_context_without_memory_context() {
    let provider = ProviderHandle::with_responses(vec![
        Ok(text_response("[RESEARCH COMPLETE]\n- stream standalone fact")),
        Ok(text_response("stream answer")),
    ]);
    let mut agent = default_agent(
        provider.provider(),
        Box::new(StaticMemoryLoader { result: Ok(String::new()) }),
    );
    agent.research_config = research_config(1, false);
    let (tx, rx) = tokio::sync::mpsc::channel(8);

    let response = agent
        .turn_with_stream("stream no memory", tx)
        .await
        .expect("streaming turn should use research-only context");

    assert_eq!(response, "stream answer");
    let requests = provider.seen_messages();
    let user = final_user_message(&requests[1]);
    assert!(user.content.starts_with("[RESEARCH COMPLETE]\n- stream standalone fact"));
    assert!(user.content.contains("] stream no memory"));
    assert!(!collect_chunks(rx).await.is_empty());
}

#[tokio::test]
async fn turn_with_stream_continues_when_research_phase_fails() {
    let provider = ProviderHandle::with_responses(vec![
        Err("research failed".to_string()),
        Ok(text_response("stream fallback")),
    ]);
    let mut agent = default_agent(
        provider.provider(),
        Box::new(StaticMemoryLoader { result: Ok(String::new()) }),
    );
    agent.research_config = research_config(1, false);
    let (tx, rx) = tokio::sync::mpsc::channel(8);

    let response = agent
        .turn_with_stream("stream keep going", tx)
        .await
        .expect("research failure should not fail streaming turn");

    assert_eq!(response, "stream fallback");
    assert_eq!(provider.seen_messages().len(), 2);
    assert!(!collect_chunks(rx).await.is_empty());
}
