use super::{test_support, ShijuCredential, ShijuKnowledgePageQuery};

#[tokio::test]
async fn knowledge_facade_forwards_page_and_preserves_type_identity() {
    let server = test_support::server(vec![(200, serde_json::json!({"items":[],"page":1,"pageSize":20,"total":0}))]);
    let query: gateway_shiju_knowledge::ShijuKnowledgePageQuery = ShijuKnowledgePageQuery::default();
    server.client().shiju_knowledge_bases(&ShijuCredential::new("secret").unwrap(), &query).await.unwrap();
    assert!(server.take_request().path.starts_with("/v1/shiju/knowledge/bases?"));
    server.join();
}
