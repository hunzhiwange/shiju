//! APIQL 客户端规范路径、查询编码与参数拒绝测试。

use serde_json::json;

use super::ApiQlQuery;
use crate::client::test_support;

#[tokio::test]
async fn apiql_client_uses_canonical_query_routes() {
    let server = test_support::server(vec![
        (200, json!({"data":[]})),
        (200, json!({"data":[]})),
        (200, json!({"data":3})),
        (
            200,
            json!({"data":[],"page":{"per_page":20,"current_page":2,"total_page":null,"total_record":null,"total_macro":0,"from":21,"to":null}}),
        ),
    ]);
    let list_query = ApiQlQuery::from_parameters(["column[]=id", "orderBy=id asc", "limit=2"])
        .expect("list query");
    let page_query = ApiQlQuery::from_parameters(["page=2", "page_size=20"]).expect("page query");

    let blank = server
        .client()
        .apiql_list(" ", &ApiQlQuery::default())
        .await
        .expect_err("blank entity class must fail before a request");
    assert!(blank.contains("must not be empty"));
    assert!(ApiQlQuery::from_parameters(["page=0"]).is_err());
    assert!(ApiQlQuery::from_parameters(["=value"]).is_err());

    server.client().apiql_list("product:product", &list_query).await.expect("list");
    server
        .client()
        .apiql_list_only("product:product", &ApiQlQuery::default())
        .await
        .expect("list only");
    server
        .client()
        .apiql_list_count(
            "product:product",
            &ApiQlQuery::from_parameters(["active=1"]).expect("count query"),
        )
        .await
        .expect("list count");
    server.client().apiql_list_page("product:product", &page_query).await.expect("list page");

    let list = server.take_request();
    let only = server.take_request();
    let count = server.take_request();
    let page = server.take_request();
    assert_eq!(list.path, "/apiQL/product%3Aproduct?column%5B%5D=id&orderBy=id+asc&limit=2");
    assert_eq!(only.path, "/apiQL/product%3Aproduct/list-only");
    assert_eq!(count.path, "/apiQL/product%3Aproduct/list-count?active=1");
    assert_eq!(page.path, "/apiQL/product%3Aproduct/list-page?page=2&page_size=20");
    for request in [&list, &only, &count, &page] {
        assert_eq!(request.method, "GET");
        assert!(!request.path.split('?').next().expect("path").ends_with('/'));
    }
    server.join();
}
